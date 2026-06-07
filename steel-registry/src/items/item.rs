//! Item-related types.
//!
//! Dynamic item behavior has been moved to `steel-core::behavior`.
//! This file contains data structures that are needed by other crates.

use std::io::{self, Cursor, Read, Result, Write};
use std::str::FromStr;

use steel_utils::serial::{ReadFrom, WriteTo};
use steel_utils::{BlockPos, codec::var_int::VarInt};

use crate::blocks::properties::Direction;

use glam::DVec3;

use simdnbt::borrow;
use simdnbt::owned::{NbtCompound, NbtTag};

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

#[derive(Debug, PartialEq, Clone)]
pub struct StatusEffectInstance {
    pub id: VarInt,
    pub amplifier: u8,
    pub duration: VarInt,
    pub flags: u8,
}

impl WriteTo for StatusEffectInstance {
    fn write(&self, writer: &mut impl Write) -> io::Result<()> {
        self.id.write(writer)?;
        writer.write_all(&[self.amplifier])?;
        self.duration.write(writer)?;
        writer.write_all(&[self.flags])?;
        Ok(())
    }
}

impl ReadFrom for StatusEffectInstance {
    fn read(reader: &mut Cursor<&[u8]>) -> io::Result<Self> {
        Ok(Self {
            id: VarInt::read(reader)?,
            amplifier: {
                let mut b = [0];
                reader.read_exact(&mut b)?;
                b[0]
            },
            duration: VarInt::read(reader)?,
            flags: {
                let mut b = [0];
                reader.read_exact(&mut b)?;
                b[0]
            },
        })
    }
}

#[derive(Debug, Clone, PartialEq)]
pub enum ConsumeEffect {
    ApplyEffects { effects: Vec<StatusEffectInstance> },
    RemoveEffects { effects: Vec<VarInt> },
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
                    let mut effect_nbt = NbtCompound::new();
                    effect_nbt.insert("id", effect.id.0); // i32
                    effect_nbt.insert("amplifier", effect.amplifier as i8); // bool
                    effect_nbt.insert("duration", effect.duration.0); // i32
                    effect_nbt.insert("flags", effect.flags as i8);
                    effects_list.push(NbtTag::Compound(effect_nbt));
                }
                compound.insert("effects", NbtTag::List(effects_list.into()));
            }
            Self::RemoveEffects { effects } => {
                compound.insert("type", "minecraft:remove_effects");

                let effects_list: Vec<NbtTag> = effects.iter().map(|v| NbtTag::Int(v.0)).collect();
                compound.insert("effects", NbtTag::List(effects_list.into()));
            }
            Self::ClearAllEffects => {
                compound.insert("type", "minecraft:clear_all_effects");
            }
            Self::TeleportRandomly { diameter } => {
                compound.insert("type", "minecraft:teleport_randomly");
                compound.insert("diameter", *diameter); // f32
            }
            Self::PlaySound { sound_event } => {
                compound.insert("type", "minecraft:play_sound");
                compound.insert("sound_event", sound_event.clone()); // String
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
                        effects.push(StatusEffectInstance {
                            id: VarInt::from(eff_comp.get("id")?.int()?),
                            amplifier: eff_comp.get("amplifier")?.byte()? as u8,
                            duration: VarInt::from(eff_comp.get("duration")?.int()?),
                            flags: eff_comp.get("flags")?.byte()? as u8,
                        });
                    }
                }
                Some(Self::ApplyEffects { effects })
            }
            "minecraft:remove_effects" => {
                let list = compound.get("effects")?.list()?;
                let mut effects = Vec::new();

                if let Some(ints) = list.ints() {
                    for id in ints {
                        effects.push(VarInt::from(id));
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
                    effects.push(StatusEffectInstance::read(data)?);
                }

                Ok(Self::ApplyEffects { effects })
            }
            1 => {
                let len = VarInt::read(data)?.0 as usize;
                let mut effects = Vec::with_capacity(len);
                for _ in 0..len {
                    effects.push(VarInt::read(data)?);
                }
                Ok(Self::RemoveEffects { effects })
            }
            2 => Ok(Self::ClearAllEffects),
            3 => {
                let mut buf = [0; 4];
                data.read_exact(&mut buf)?;
                let diameter = f32::from_be_bytes(buf);
                Ok(Self::TeleportRandomly { diameter })
            }
            4 => {
                let len = VarInt::read(data)?.0 as usize;
                let mut buf = vec![0; len];
                data.read_exact(&mut buf)?;
                let sound_event = String::from_utf8(buf)
                    .map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e))?;
                Ok(Self::PlaySound { sound_event })
            }
            _ => Err(io::Error::new(
                io::ErrorKind::InvalidData,
                format!("Unknown ConsumeEffect type ID: {type_id}"),
            )),
        }
    }
}

impl WriteTo for ConsumeEffect {
    fn write(&self, writer: &mut impl Write) -> io::Result<()> {
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
                for &effect_id in effects {
                    effect_id.write(writer)?;
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
