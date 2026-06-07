use std::any::Any;
use std::clone::Clone;
use std::cmp::PartialEq;
use std::fmt;
use std::io::{Cursor, Read, Result, Write};
use std::str::FromStr;
use std::sync::Arc;
use std::vec::Vec;

use steel_utils::{
    codec::VarInt,
    console,
    hash::{ComponentHasher, HashComponent},
    serial::{ReadFrom, WriteTo},
};

use crate::items::item::{ConsumeEffect, ItemUseAnimation};

use simdnbt::owned::{NbtCompound, NbtTag};

#[derive(Debug, Clone, PartialEq)]
pub struct FoodProperties {
    pub nutrition: VarInt,
    pub saturation: f32,
    pub can_always_eat: bool,
}

impl WriteTo for FoodProperties {
    fn write(&self, writer: &mut impl Write) -> Result<()> {
        // Format: nutrition (VarInt), saturation (f32), can_always_eat (bool)
        self.nutrition.write(writer)?;
        writer.write_all(&self.saturation.to_be_bytes())?;
        writer.write_all(&[self.can_always_eat as u8])?;

        Ok(())
    }
}

impl ReadFrom for FoodProperties {
    fn read(data: &mut Cursor<&[u8]>) -> Result<Self> {
        let mut saturation_bytes = [0u8; 4];
        data.read_exact(&mut saturation_bytes)?;

        let mut can_always_eat_bytes = [0u8; 1];
        data.read_exact(&mut can_always_eat_bytes)?;

        Ok(Self {
            nutrition: VarInt::read(data)?,
            saturation: f32::from_be_bytes(saturation_bytes),
            can_always_eat: can_always_eat_bytes[0] != 0,
        })
    }
}

impl simdnbt::ToNbtTag for FoodProperties {
    fn to_nbt_tag(self) -> simdnbt::owned::NbtTag {
        let mut compound = NbtCompound::new();
        compound.insert("nutrition", NbtTag::Int(self.nutrition.0));
        compound.insert("saturation", NbtTag::Float(self.saturation));
        compound.insert("can_always_eat", NbtTag::Byte(self.can_always_eat as i8));
        NbtTag::Compound(compound)
    }
}

impl simdnbt::FromNbtTag for FoodProperties {
    fn from_nbt_tag(tag: simdnbt::borrow::NbtTag) -> Option<Self> {
        let compound = tag.compound()?;
        Some(Self {
            nutrition: VarInt::from(compound.get("nutrition")?.int()?),
            saturation: compound.get("saturation")?.float()?,
            can_always_eat: compound.get("can_always_eat")?.byte()? != 0,
        })
    }
}

impl HashComponent for FoodProperties {
    fn hash_component(&self, hasher: &mut ComponentHasher) {
        // For now, hash as empty map since full implementation requires proper codec
        hasher.start_map();
        // TODO: Add proper field hashing when Tool codec is implemented
        hasher.end_map();
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct ConsumableData {
    pub consume_seconds: f32,
    pub animation: ItemUseAnimation,
    pub has_consume_particles: bool,
    pub on_consume_effects: Vec<ConsumeEffect>,
}

impl Default for ConsumableData {
    fn default() -> Self {
        Self {
            consume_seconds: 1.6,
            animation: ItemUseAnimation::Eat,
            has_consume_particles: true,
            on_consume_effects: vec![],
        }
    }
}

impl ReadFrom for ConsumableData {
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

impl WriteTo for ConsumableData {
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

impl simdnbt::FromNbtTag for ConsumableData {
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
                ItemUseAnimation::None
            }),
            has_consume_particles: compound.get("has_consume_particles")?.byte()? != 0,
            on_consume_effects: effects,
        })
    }
}

impl simdnbt::ToNbtTag for ConsumableData {
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

/// Some types are in steel-core, so here we do this to make it compile
/// see implementation in steel-core (ConsumableImpl)
pub trait ConsumableBehavior: Send + Sync + 'static {}

pub trait Consumable: Send + Sync + 'static {
    fn as_any(&self) -> &dyn Any;
    fn clone_arc(&self) -> Arc<dyn Consumable>;
}

impl<T> Consumable for T
where
    T: ConsumableBehavior + Clone,
{
    fn as_any(&self) -> &dyn Any {
        self
    }

    fn clone_arc(&self) -> Arc<dyn Consumable> {
        Arc::new(self.clone())
    }
}

pub struct ConsumableComponent {
    pub handler: Option<Arc<dyn Consumable>>,
    pub data: ConsumableData,
}

impl ConsumableComponent {
    pub fn new(data: ConsumableData) -> Self {
        Self {
            handler: None,
            data,
        }
    }

    pub fn with_handler(&mut self, handler: impl Consumable + 'static) {
        self.handler = Some(Arc::new(handler));
    }

    pub fn is_handler_set(&self) -> bool {
        self.handler.is_some()
    }

    pub fn downcast_ref<T: 'static>(&self) -> Option<&T> {
        self.handler.as_deref()?.as_any().downcast_ref::<T>()
    }

    pub fn consume_ticks(data: &ConsumableData) -> u32 {
        (data.consume_seconds * 20.0) as u32
    }
}

// Manually implementing Debug, PartialEq, Clone because of handler field
impl fmt::Debug for ConsumableComponent {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("ConsumableComponent")
            .field("data", &self.data)
            .field("handler", &"Arc<dyn Consumable>")
            .finish()
    }
}

impl PartialEq for ConsumableComponent {
    fn eq(&self, other: &Self) -> bool {
        self.data == other.data
    }
}

impl Clone for ConsumableComponent {
    fn clone(&self) -> Self {
        Self {
            handler: self.handler.as_deref().map(|h| h.clone_arc()),
            data: self.data.clone(),
        }
    }
}

impl HashComponent for ConsumableComponent {
    fn hash_component(&self, hasher: &mut ComponentHasher) {
        // For now, hash as empty map since full implementation requires proper codec
        hasher.start_map();
        // TODO: Add proper field hashing when Tool codec is implemented
        hasher.end_map();
    }
}

impl ReadFrom for ConsumableComponent {
    fn read(data: &mut Cursor<&[u8]>) -> Result<Self> {
        Ok(Self {
            data: ConsumableData::read(data)?,
            handler: None,
        })
    }
}

impl WriteTo for ConsumableComponent {
    fn write(&self, writer: &mut impl Write) -> Result<()> {
        self.data.write(writer)?;

        Ok(())
    }
}

impl simdnbt::FromNbtTag for ConsumableComponent {
    fn from_nbt_tag(tag: simdnbt::borrow::NbtTag) -> Option<Self> {
        Some(Self {
            data: ConsumableData::from_nbt_tag(tag)?,
            handler: None,
        })
    }
}

impl simdnbt::ToNbtTag for ConsumableComponent {
    fn to_nbt_tag(self) -> simdnbt::owned::NbtTag {
        self.data.to_nbt_tag()
    }
}
