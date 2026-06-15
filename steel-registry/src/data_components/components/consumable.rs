use std::clone::Clone;
use std::cmp::PartialEq;
use std::io::{Cursor, Read, Result, Write};
use std::str::FromStr;
use std::vec::Vec;

use steel_utils::{
    codec::VarInt,
    console,
    hash::{ComponentHasher, HashComponent},
    serial::{ReadFrom, WriteTo},
};

use crate::data_components::components::ConsumeEffect;
use crate::items::item::ItemUseAnimation;

use simdnbt::owned::{NbtCompound, NbtTag};

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
