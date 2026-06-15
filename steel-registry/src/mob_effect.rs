use rustc_hash::FxHashMap;
use steel_utils::Identifier;
use steel_utils::codec::VarInt;
use steel_utils::serial::{PrefixedRead, PrefixedWrite, ReadFrom, WriteTo};

use std::boxed::Box;
use std::io::{Cursor, Result, Write};

use crate::REGISTRY;

use simdnbt::borrow;
use simdnbt::owned::NbtCompound;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum MobEffectCategory {
    Beneficial,
    Harmful,
    Neutral,
}

#[derive(Debug, PartialEq, Eq, Hash)]
pub struct MobEffect {
    pub key: Identifier,
    pub category: MobEffectCategory,
    pub color: i32,
}

impl WriteTo for MobEffect {
    fn write(&self, writer: &mut impl Write) -> Result<()> {
        self.key.write(writer)?;

        Ok(())
    }
}

pub type MobEffectRef = &'static MobEffect;

pub struct MobEffectRegistry {
    effects_by_id: Vec<MobEffectRef>,
    effects_by_key: FxHashMap<Identifier, usize>,
    allows_registering: bool,
}

impl Default for MobEffectRegistry {
    fn default() -> Self {
        Self::new()
    }
}

impl MobEffectRegistry {
    #[must_use]
    pub fn new() -> Self {
        Self {
            effects_by_id: Vec::new(),
            effects_by_key: FxHashMap::default(),
            allows_registering: true,
        }
    }

    pub fn register(&mut self, effect: MobEffectRef) {
        assert!(
            self.allows_registering,
            "Cannot register mob effects after the registry has been frozen"
        );
        let idx = self.effects_by_id.len();
        self.effects_by_key.insert(effect.key.clone(), idx);
        self.effects_by_id.push(effect);
    }

    pub fn iter(&self) -> impl Iterator<Item = (usize, MobEffectRef)> + '_ {
        self.effects_by_id
            .iter()
            .enumerate()
            .map(|(id, &effect)| (id, effect))
    }

    pub fn effect_by_key(&self, identifier: &Identifier) -> MobEffectRef {
        self.effects_by_id
            .get(*self.effects_by_key.get(identifier).unwrap())
            .unwrap()
    }

    pub fn effect_id_by_key(&self, identifier: &Identifier) -> &usize {
        self.effects_by_key.get(identifier).unwrap()
    }
}

crate::impl_registry!(
    MobEffectRegistry,
    MobEffect,
    effects_by_id,
    effects_by_key,
    mob_effects
);

#[derive(Debug, Clone)]
pub enum LazyMobEffect {
    Raw(Identifier),
    Resolved(MobEffectRef),
}

impl PartialEq for LazyMobEffect {
    fn eq(&self, other: &Self) -> bool {
        match (self, other) {
            (LazyMobEffect::Resolved(a), LazyMobEffect::Resolved(b)) => a == b,
            (a, b) => a.key() == b.key(),
        }
    }
}

impl LazyMobEffect {
    pub fn key(&self) -> &Identifier {
        match self {
            LazyMobEffect::Raw(key) => key,
            LazyMobEffect::Resolved(effect) => &effect.key,
        }
    }

    pub fn resolve(&self) -> MobEffectRef {
        match self {
            LazyMobEffect::Resolved(effect) => effect,
            LazyMobEffect::Raw(key) => REGISTRY.mob_effects.effect_by_key(key),
        }
    }
}

#[derive(Debug, PartialEq, Clone)]
pub struct MobEffectInstance {
    effect: LazyMobEffect,
    amplifier: i32,
    duration: Option<i32>,
    pub ambient: bool,
    pub visible: bool,
    pub show_icon: bool,
    pub hidden_effect: Option<Box<MobEffectInstance>>,
}

impl MobEffectInstance {
    pub fn new(effect: LazyMobEffect, amplifier: i32) -> Self {
        Self {
            effect,
            amplifier,
            duration: None,
            ambient: true,
            visible: true,
            show_icon: true,
            hidden_effect: None,
        }
    }

    pub fn with_duration(mut self, duration: Option<i32>) -> Self {
        self.duration = duration;

        self
    }

    pub fn with_ambient(mut self, ambient: bool) -> Self {
        self.ambient = ambient;

        self
    }

    pub fn with_visible(mut self, visible: bool) -> Self {
        self.visible = visible;

        self
    }

    pub fn with_show_icon(mut self, show_icon: bool) -> Self {
        self.show_icon = show_icon;

        self
    }

    pub fn with_hidden_effect(mut self, hidden_effect: Option<Box<MobEffectInstance>>) -> Self {
        self.hidden_effect = hidden_effect;

        self
    }

    #[allow(dead_code)]
    pub fn is_instantaneous(&self) -> bool {
        if let Some(duration) = self.duration {
            duration == 0
        } else {
            false
        }
    }

    #[allow(dead_code)]
    pub fn is_infinite_duration(&self) -> bool {
        self.duration.is_none()
    }

    #[allow(dead_code)]
    pub fn effect(&self) -> MobEffectRef {
        self.effect.resolve()
    }

    #[allow(dead_code)]
    pub fn key(&self) -> &Identifier {
        self.effect.key()
    }

    pub fn to_nbt_compound(&self) -> NbtCompound {
        let mut compound = NbtCompound::new();

        compound.insert("effect", self.effect.key().to_string());
        compound.insert("amplifier", self.amplifier);
        compound.insert("duration", self.duration);
        compound.insert("ambient", self.ambient as i8);
        compound.insert("visible", self.visible as i8);
        compound.insert("show_icon", self.show_icon as i8);

        if let Some(hidden_effect) = &self.hidden_effect {
            compound.insert("hidden_effect", hidden_effect.to_nbt_compound());
        }

        compound
    }

    pub fn from_nbt_compound(compound: &borrow::NbtCompound) -> Option<Self> {
        Some(MobEffectInstance {
            effect: LazyMobEffect::Resolved(REGISTRY.mob_effects.effect_by_key(
                &Identifier::vanilla(compound.get("key")?.string().unwrap().to_string()),
            )),
            amplifier: compound.get("amplifier")?.int()?,
            duration: {
                if let Some(duration) = compound.get("duration") {
                    Some(duration.int()?)
                } else {
                    None
                }
            },
            ambient: compound.get("ambient")?.byte()? == 1,
            visible: compound.get("visible")?.byte()? == 1,
            show_icon: compound.get("show_icon")?.byte()? == 1,
            hidden_effect: {
                if let Some(hidden_effect) = compound.get("hidden_effect")?.compound() {
                    Some(Box::new(Self::from_nbt_compound(&hidden_effect)?))
                } else {
                    None
                }
            },
        })
    }

    pub fn amplifier(&self) -> i32 {
        self.amplifier
    }

    pub fn duration(&self) -> Option<i32> {
        self.duration
    }
}

impl WriteTo for MobEffectInstance {
    fn write(&self, writer: &mut impl Write) -> Result<()> {
        self.effect()
            .key
            .to_string()
            .write_prefixed::<VarInt>(writer)?;

        writer.write_all(&self.amplifier.to_be_bytes())?;

        if let Some(duration) = self.duration {
            duration.write(writer)?;
        } else {
            i32::write(&-1, writer)?;
        }

        self.ambient.write(writer)?;

        // yea, it may look incorrect, but check ModEffectInstance.Details
        self.visible.write(writer)?;

        self.show_icon.write(writer)?;

        if let Some(effect) = &self.hidden_effect {
            bool::write(&true, writer)?;

            effect.write(writer)?;
        } else {
            bool::write(&false, writer)?;
        }

        Ok(())
    }
}

impl ReadFrom for MobEffectInstance {
    fn read(reader: &mut Cursor<&[u8]>) -> Result<Self> {
        Ok(Self {
            effect: LazyMobEffect::Raw(Identifier::vanilla(String::read_prefixed::<VarInt>(
                reader,
            )?)),
            amplifier: VarInt::read(reader)?.0,
            duration: {
                let duration = VarInt::read(reader)?.0;

                if duration == -1 { None } else { Some(duration) }
            },
            ambient: bool::read(reader)?,
            visible: bool::read(reader)?,
            show_icon: bool::read(reader)?,
            hidden_effect: {
                if bool::read(reader)? {
                    Some(Box::new(MobEffectInstance::read(reader)?))
                } else {
                    None
                }
            },
        })
    }
}
