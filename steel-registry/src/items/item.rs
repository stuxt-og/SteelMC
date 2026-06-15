//! Item-related types.
//!
//! Dynamic item behavior has been moved to `steel-core::behavior`.
//! This file contains data structures that are needed by other crates.

use std::io::{self, Cursor, Result, Write};
use std::str::FromStr;

use steel_utils::serial::{PrefixedRead, PrefixedWrite, ReadFrom, WriteTo};
use steel_utils::{BlockPos, codec::var_int::VarInt};

use crate::blocks::properties::Direction;
use crate::mob_effect::MobEffectInstance;

use glam::DVec3;

/// Result of a ray cast hitting a block.
///
/// This is kept in steel-registry because it's used by steel-protocol
/// for packet deserialization.
#[derive(Debug, Clone)]
pub struct BlockHitResult {
    /// The exact location where the ray hit the block.
    pub location: DVec3,
    /// The face of the block that was hit.
    pub direction: Direction,
    /// The position of the block that was hit.
    pub block_pos: BlockPos,
    /// Whether this is a miss (no block hit).
    pub miss: bool,
    /// Whether the hit location is inside the block.
    pub inside: bool,
    /// Whether the world border was hit.
    pub world_border_hit: bool,
}

impl ReadFrom for BlockHitResult {
    fn read(data: &mut Cursor<&[u8]>) -> io::Result<Self> {
        let block_pos = BlockPos::read(data)?;
        let direction = Direction::read(data)?;
        // Click coordinates are relative to the block position (0.0 to 1.0 range)
        let click_x = f32::read(data)?;
        let click_y = f32::read(data)?;
        let click_z = f32::read(data)?;
        let inside = bool::read(data)?;
        let world_border_hit = bool::read(data)?;

        // Convert to absolute world coordinates by adding block position
        // (matching Java's FriendlyByteBuf.readBlockHitResult)
        let location = DVec3::new(
            f64::from(block_pos.x()) + f64::from(click_x),
            f64::from(block_pos.y()) + f64::from(click_y),
            f64::from(block_pos.z()) + f64::from(click_z),
        );

        Ok(BlockHitResult {
            location,
            direction,
            block_pos,
            miss: false,
            inside,
            world_border_hit,
        })
    }
}

/// Represents the animation that plays when item is used
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[repr(i32)]
pub enum ItemUseAnimation {
    None = 0,
    Eat = 1,
    Drink = 2,
    Block = 3,
    Bow = 4,
    Trident = 5,
    Crossbow = 6,
    Spyglass = 7,
    TootHorn = 8,
    Brush = 9,
    Bundle = 10,
    Spear = 11,
}

impl ItemUseAnimation {
    #[inline]
    pub fn id(self) -> i32 {
        self as i32
    }

    // getting name as str (equivalent of Java's StringRepresentable)
    pub fn as_str(self) -> &'static str {
        match self {
            Self::None => "none",
            Self::Eat => "eat",
            Self::Drink => "drink",
            Self::Block => "block",
            Self::Bow => "bow",
            Self::Trident => "trident",
            Self::Crossbow => "crossbow",
            Self::Spyglass => "spyglass",
            Self::TootHorn => "toot_horn",
            Self::Brush => "brush",
            Self::Bundle => "bundle",
            Self::Spear => "spear",
        }
    }

    #[inline]
    pub fn custom_arm_transform(self) -> bool {
        matches!(self, Self::Eat | Self::Drink | Self::Spear)
    }

    // parser from id
    pub fn from_id(id: i32) -> Option<Self> {
        match id {
            0 => Some(Self::None),
            1 => Some(Self::Eat),
            2 => Some(Self::Drink),
            3 => Some(Self::Block),
            4 => Some(Self::Bow),
            5 => Some(Self::Trident),
            6 => Some(Self::Crossbow),
            7 => Some(Self::Spyglass),
            8 => Some(Self::TootHorn),
            9 => Some(Self::Brush),
            10 => Some(Self::Bundle),
            11 => Some(Self::Spear),
            _ => None,
        }
    }
}

impl FromStr for ItemUseAnimation {
    type Err = io::Error;

    // parser from nbttag's name
    fn from_str(name: &str) -> Result<Self> {
        match name {
            "none" => Ok(Self::None),
            "eat" => Ok(Self::Eat),
            "drink" => Ok(Self::Drink),
            "block" => Ok(Self::Block),
            "bow" => Ok(Self::Bow),
            "trident" => Ok(Self::Trident),
            "crossbow" => Ok(Self::Crossbow),
            "spyglass" => Ok(Self::Spyglass),
            "toot_horn" => Ok(Self::TootHorn),
            "brush" => Ok(Self::Brush),
            "bundle" => Ok(Self::Bundle),
            "spear" => Ok(Self::Spear),
            _ => Err(io::Error::other(format!(
                "Unknown item use animation: '{name}'"
            ))),
        }
    }
}

impl WriteTo for ItemUseAnimation {
    fn write(&self, writer: &mut impl Write) -> io::Result<()> {
        VarInt::from(*self as i32).write(writer)?;

        Ok(())
    }
}

impl ReadFrom for ItemUseAnimation {
    fn read(reader: &mut Cursor<&[u8]>) -> io::Result<Self> {
        Self::from_id(VarInt::read(reader)?.0).ok_or_else(|| {
            io::Error::new(
                io::ErrorKind::InvalidData,
                "failed to convert id to ItemUseAnimation",
            )
        })
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct Potion {
    pub name: String,
    pub effects: Vec<MobEffectInstance>,
}

impl Potion {
    pub fn new(name: String, effects: Vec<MobEffectInstance>) -> Self {
        Self { name, effects }
    }

    pub fn has_instant_effects(&self) -> bool {
        for effect in &self.effects {
            if effect.is_instantaneous() {
                return true;
            }
        }

        false
    }
}

impl ReadFrom for Potion {
    fn read(reader: &mut Cursor<&[u8]>) -> io::Result<Self> {
        Ok(Self {
            name: String::read_prefixed::<VarInt>(reader)?,
            effects: {
                let len = VarInt::read(reader)?.0 as usize;
                let mut effects = Vec::with_capacity(len);

                for _ in 0..len {
                    effects.push(MobEffectInstance::read(reader)?);
                }

                effects
            },
        })
    }
}

impl WriteTo for Potion {
    fn write(&self, writer: &mut impl Write) -> io::Result<()> {
        self.name.write_prefixed::<VarInt>(writer)?;

        VarInt::from(self.effects.len() as i32).write(writer)?;

        // effects
        for effect in &self.effects {
            effect.write(writer)?;
        }

        Ok(())
    }
}
