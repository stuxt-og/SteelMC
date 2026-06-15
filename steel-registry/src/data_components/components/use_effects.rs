use std::clone::Clone;
use std::cmp::PartialEq;
use std::io::{Cursor, Read, Result, Write};

use steel_utils::{
    hash::{ComponentHasher, HashComponent},
    serial::{ReadFrom, WriteTo},
};

use simdnbt::owned::{NbtCompound, NbtTag};

#[derive(Debug, Clone, PartialEq)]
pub struct UseEffects {
    pub can_sprint: bool,
    pub interaction_vibrations: bool,
    pub speed_multiplier: f32,
}

impl WriteTo for UseEffects {
    fn write(&self, writer: &mut impl Write) -> Result<()> {
        // Format: can_sprint (bool), interaction_vibrations (bool), speed_multiplier (f32)
        writer.write_all(&[self.can_sprint as u8])?;
        writer.write_all(&[self.interaction_vibrations as u8])?;
        writer.write_all(&self.speed_multiplier.to_be_bytes())?;

        Ok(())
    }
}

impl ReadFrom for UseEffects {
    fn read(data: &mut Cursor<&[u8]>) -> Result<Self> {
        let mut can_sprint_bytes = [0u8; 1];
        data.read_exact(&mut can_sprint_bytes)?;

        let mut interaction_vibrations_bytes = [0u8; 1];
        data.read_exact(&mut interaction_vibrations_bytes)?;

        let mut speed_multiplier_bytes = [0u8; 4];
        data.read_exact(&mut speed_multiplier_bytes)?;

        Ok(Self {
            can_sprint: can_sprint_bytes[0] != 0,
            interaction_vibrations: interaction_vibrations_bytes[0] != 0,
            speed_multiplier: f32::from_be_bytes(speed_multiplier_bytes),
        })
    }
}

impl simdnbt::ToNbtTag for UseEffects {
    fn to_nbt_tag(self) -> simdnbt::owned::NbtTag {
        let mut compound = NbtCompound::new();
        compound.insert("can_sprint", NbtTag::Byte(self.can_sprint as i8));
        compound.insert(
            "interaction_vibrations",
            NbtTag::Byte(self.interaction_vibrations as i8),
        );
        compound.insert("speed_multiplier", NbtTag::Float(self.speed_multiplier));
        NbtTag::Compound(compound)
    }
}

impl simdnbt::FromNbtTag for UseEffects {
    fn from_nbt_tag(tag: simdnbt::borrow::NbtTag) -> Option<Self> {
        let compound = tag.compound()?;
        Some(Self {
            can_sprint: compound.get("can_sprint")?.byte()? != 0,
            interaction_vibrations: compound.get("interaction_vibrations")?.byte()? != 0,
            speed_multiplier: compound.get("speed_multiplier")?.float()?,
        })
    }
}

impl HashComponent for UseEffects {
    fn hash_component(&self, hasher: &mut ComponentHasher) {
        // For now, hash as empty map since full implementation requires proper codec
        hasher.start_map();
        // TODO: Add proper field hashing when UseEffects codec is implemented
        hasher.end_map();
    }
}
