use std::clone::Clone;
use std::cmp::PartialEq;
use std::io::{Cursor, Error, ErrorKind, Read, Result, Write};

use steel_utils::{
    Identifier,
    codec::VarInt,
    serial::{PrefixedRead, PrefixedWrite, ReadFrom, WriteTo},
};

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
    PlaySound { sound_event: String },
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
                compound.insert("sound_event", sound_event.clone());
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
                                .effect_by_key(&Identifier::vanilla(string.to_string())),
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
            "minecraft:play_sound" => {
                let sound_event = compound.get("sound_event")?.string()?.to_str().into_owned();
                Some(Self::PlaySound { sound_event })
            }
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
                    effects.push(LazyMobEffect::Resolved(REGISTRY.mob_effects.effect_by_key(
                        &Identifier::vanilla(String::read_prefixed::<VarInt>(data)?),
                    )));
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
                    sound_event: String::from_utf8(buf)
                        .map_err(|e| Error::new(ErrorKind::InvalidData, e))?,
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
                    effect.key().to_string().write_prefixed::<VarInt>(writer)?;
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

                // string: VarInt len + UTF-8 bytes
                let bytes = sound_event.as_bytes();
                VarInt::from(bytes.len() as i32).write(writer)?;
                writer.write_all(bytes)?;
            }
        }
        Ok(())
    }
}
