use std::clone::Clone;
use std::cmp::PartialEq;
use std::io::{Cursor, Error, ErrorKind, Read, Result, Write};
use std::str::FromStr;
use std::vec::Vec;

use steel_utils::{
    Identifier,
    codec::VarInt,
    console,
    hash::{ComponentHasher, HashComponent},
    serial::{PrefixedRead, ReadFrom, WriteTo},
};

use crate::items::item::ItemUseAnimation;

use simdnbt::borrow;
use simdnbt::owned::{NbtCompound, NbtTag};

use crate::REGISTRY;
use crate::mob_effect::{LazyMobEffect, MobEffectInstance};

#[derive(Debug, Clone, PartialEq)]
pub enum ConsumeEffect {
    ApplyEffects { effects: Vec<MobEffectInstance> },
    RemoveEffects { effects: Vec<LazyMobEffect> },
    ClearAllEffects,
    TeleportRandomly { diameter: f32 },
    PlaySound { sound_event: Identifier },
}

impl ConsumeEffect {
    pub fn to_nbt_compound(&self) -> NbtCompound {
        let mut compound = NbtCompound::new();

        match self {
            Self::ApplyEffects { effects } => {
                compound.insert("type", "minecraft:apply_effects");

                let mut effects_list = Vec::new();
                for effect in effects {
                    effects_list.push(NbtTag::Compound(effect.to_nbt_compound()));
                }
                compound.insert("effects", NbtTag::List(effects_list.into()));
            }
            Self::RemoveEffects { effects } => {
                compound.insert("type", "minecraft:remove_effects");

                let effects_list: Vec<NbtTag> = effects
                    .iter()
                    .map(|v| NbtTag::String(v.key().to_string().into()))
                    .collect();
                compound.insert("effects", NbtTag::List(effects_list.into()));
            }
            Self::ClearAllEffects => {
                compound.insert("type", "minecraft:clear_all_effects");
            }
            Self::TeleportRandomly { diameter } => {
                compound.insert("type", "minecraft:teleport_randomly");
                compound.insert("diameter", *diameter);
            }
            Self::PlaySound { sound_event } => {
                compound.insert("type", "minecraft:play_sound");
                compound.insert("sound_event", sound_event.to_string());
            }
        }

        compound
    }

    pub fn from_nbt_compound(compound: &borrow::NbtCompound) -> Option<Self> {
        let type_str = compound.get("type")?.string()?;

        match type_str.to_str().as_ref() {
            "minecraft:apply_effects" => {
                let list = compound.get("effects")?.list()?;
                let mut effects = Vec::new();

                if let Some(compounds) = list.compounds() {
                    for eff_comp in compounds {
                        effects.push(MobEffectInstance::from_nbt_compound(&eff_comp)?);
                    }
                }
                Some(Self::ApplyEffects { effects })
            }
            "minecraft:remove_effects" => {
                let list = compound.get("effects")?.list()?;
                let mut effects = Vec::new();

                if let Some(strings) = list.strings() {
                    for string in strings {
                        effects.push(LazyMobEffect::Resolved(
                            REGISTRY
                                .mob_effects
                                .effect_by_key(&Identifier::vanilla(string.to_string()))
                                .unwrap(),
                        ));
                    }
                }
                Some(Self::RemoveEffects { effects })
            }
            "minecraft:clear_all_effects" => Some(Self::ClearAllEffects),
            "minecraft:teleport_randomly" => {
                let diameter = compound.get("diameter")?.float()?;
                Some(Self::TeleportRandomly { diameter })
            }
            "minecraft:play_sound" => Some(Self::PlaySound {
                sound_event: Identifier::vanilla(
                    compound.get("sound_event")?.string()?.to_str().into_owned(),
                ),
            }),
            _ => None,
        }
    }
}

impl ReadFrom for ConsumeEffect {
    fn read(data: &mut Cursor<&[u8]>) -> Result<Self> {
        let type_id = VarInt::read(data)?.0;

        match type_id {
            0 => {
                let len = VarInt::read(data)?.0 as usize;
                let mut effects = Vec::with_capacity(len);

                for _ in 0..len {
                    effects.push(MobEffectInstance::read(data)?);
                }

                Ok(Self::ApplyEffects { effects })
            }
            1 => {
                let len = VarInt::read(data)?.0 as usize;
                let mut effects = Vec::with_capacity(len);

                for _ in 0..len {
                    effects.push(LazyMobEffect::Resolved(
                        REGISTRY
                            .mob_effects
                            .effect_by_key(&Identifier::vanilla(String::read_prefixed::<VarInt>(
                                data,
                            )?))
                            .unwrap(),
                    ));
                }

                Ok(Self::RemoveEffects { effects })
            }
            2 => Ok(Self::ClearAllEffects),
            3 => {
                let mut buf = [0; 4];
                data.read_exact(&mut buf)?;

                Ok(Self::TeleportRandomly {
                    diameter: f32::from_be_bytes(buf),
                })
            }
            4 => {
                let mut buf = vec![0; VarInt::read(data)?.0 as usize];
                data.read_exact(&mut buf)?;

                Ok(Self::PlaySound {
                    sound_event: Identifier::vanilla(
                        String::from_utf8(buf)
                            .map_err(|e| Error::new(ErrorKind::InvalidData, e))?,
                    ),
                })
            }
            _ => Err(Error::new(
                ErrorKind::InvalidData,
                format!("Unknown ConsumeEffect type ID: {type_id}"),
            )),
        }
    }
}

impl WriteTo for ConsumeEffect {
    fn write(&self, writer: &mut impl Write) -> Result<()> {
        match self {
            Self::ApplyEffects { effects } => {
                // ID
                VarInt::from(0).write(writer)?;

                // len
                VarInt::from(effects.len() as i32).write(writer)?;

                // effects
                for effect in effects {
                    effect.write(writer)?;
                }
            }
            Self::RemoveEffects { effects } => {
                // ID
                VarInt::from(1).write(writer)?;

                // len
                VarInt::from(effects.len() as i32).write(writer)?;

                // effects
                for effect in effects {
                    let effect_ref = effect.resolve();

                    let effect_id = *REGISTRY.mob_effects.effect_id_by_key(&effect_ref.key) as i32;

                    VarInt::from(effect_id).write(writer)?;
                }
            }
            Self::ClearAllEffects => {
                // ID
                VarInt::from(2).write(writer)?;
            }
            Self::TeleportRandomly { diameter } => {
                // ID
                VarInt::from(3).write(writer)?;

                // diameter
                writer.write_all(&diameter.to_be_bytes())?;
            }
            Self::PlaySound { sound_event } => {
                // ID
                VarInt::from(4).write(writer)?;

                let path: String = sound_event.to_string();

                // string: VarInt len + UTF-8 bytes
                let bytes = path.as_bytes();
                VarInt::from(bytes.len() as i32).write(writer)?;
                writer.write_all(bytes)?;
            }
        }
        Ok(())
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct Consumable {
    pub consume_seconds: f32,
    pub animation: ItemUseAnimation,
    pub has_consume_particles: bool,
    pub on_consume_effects: Vec<ConsumeEffect>,
}

impl Default for Consumable {
    fn default() -> Self {
        Self {
            consume_seconds: 1.6,
            animation: ItemUseAnimation::Eat,
            has_consume_particles: true,
            on_consume_effects: vec![],
        }
    }
}

impl ReadFrom for Consumable {
    fn read(data: &mut Cursor<&[u8]>) -> Result<Self> {
        let mut consume_seconds_bytes = [0u8; 4];
        data.read_exact(&mut consume_seconds_bytes)?;

        let item_use_anim = ItemUseAnimation::read(data)?;

        // skip SoundEvent
        VarInt::read(data)?;

        let mut has_consume_particles_bytes = [0u8; 1];
        data.read_exact(&mut has_consume_particles_bytes)?;

        let size: usize = VarInt::read(data)?.0 as usize;

        let mut on_consume_effects = Vec::with_capacity(size);

        for _ in 0..size {
            on_consume_effects.push(ConsumeEffect::read(data)?);
        }

        Ok(Self {
            consume_seconds: f32::from_be_bytes(consume_seconds_bytes),
            animation: item_use_anim,
            has_consume_particles: has_consume_particles_bytes[0] != 0,
            on_consume_effects,
        })
    }
}

impl WriteTo for Consumable {
    fn write(&self, writer: &mut impl Write) -> Result<()> {
        // Format: nutrition (VarInt), saturation (f32), can_always_eat (bool)
        writer.write_all(&self.consume_seconds.to_be_bytes())?;
        self.animation.write(writer)?;

        // SoundEvents.GENERIC_EAT
        VarInt::from(697).write(writer)?;

        writer.write_all(&[self.has_consume_particles as u8])?;

        VarInt::from(self.on_consume_effects.len()).write(writer)?;

        for on_consume_effect in &self.on_consume_effects {
            on_consume_effect.write(writer)?;
        }

        Ok(())
    }
}

impl simdnbt::FromNbtTag for Consumable {
    fn from_nbt_tag(tag: simdnbt::borrow::NbtTag) -> Option<Self> {
        let compound = tag.compound()?;

        let mut effects = Vec::new();

        if let Some(compounds_iter) = compound.get("on_consume_effects")?.list()?.compounds() {
            for eff_comp in compounds_iter {
                if let Some(effect) = ConsumeEffect::from_nbt_compound(&eff_comp) {
                    effects.push(effect);
                }
            }
        }

        Some(Self {
            consume_seconds: compound.get("consume_seconds")?.float()?,
            animation: ItemUseAnimation::from_str(
                compound.get("animation")?.string()?.to_str().as_str(),
            )
            .unwrap_or_else(|err| {
                console!(
                    "{}",
                    err.to_string() + ". Defaulting to ItemUseAnimation::None."
                );
                ItemUseAnimation::Eat
            }),
            has_consume_particles: compound.get("has_consume_particles")?.byte()? != 0,
            on_consume_effects: effects,
        })
    }
}

impl simdnbt::ToNbtTag for Consumable {
    fn to_nbt_tag(self) -> simdnbt::owned::NbtTag {
        let mut compound = NbtCompound::new();
        compound.insert("consume_seconds", NbtTag::Float(self.consume_seconds));
        compound.insert("animation", NbtTag::String(self.animation.as_str().into()));
        compound.insert(
            "has_consume_particles",
            NbtTag::Byte(self.has_consume_particles as i8),
        );
        let effects_list: Vec<NbtTag> = self
            .on_consume_effects
            .iter()
            .map(|e| NbtTag::Compound(e.to_nbt_compound()))
            .collect();

        compound.insert("on_consume_effects", NbtTag::List(effects_list.into()));

        NbtTag::Compound(compound)
    }
}

impl HashComponent for Consumable {
    fn hash_component(&self, hasher: &mut ComponentHasher) {
        // For now, hash as empty map since full implementation requires proper codec
        hasher.start_map();
        // TODO: Add proper field hashing when Consumable codec is implemented
        hasher.end_map();
    }
}
