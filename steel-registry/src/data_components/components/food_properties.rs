use std::clone::Clone;
use std::cmp::PartialEq;
use std::io::{Cursor, Read, Result, Write};

use steel_utils::{
    codec::VarInt,
    hash::{ComponentHasher, HashComponent},
    serial::{ReadFrom, WriteTo},
};

use simdnbt::owned::{NbtCompound, NbtTag};

#[derive(Debug, Clone, PartialEq)]
pub struct FoodProperties {
    pub nutrition: i32,
    pub saturation: f32,
    pub can_always_eat: bool,
}

impl WriteTo for FoodProperties {
    fn write(&self, writer: &mut impl Write) -> Result<()> {
        // Format: nutrition (VarInt), saturation (f32), can_always_eat (bool)
        VarInt::from(self.nutrition).write(writer)?;
        writer.write_all(&self.saturation.to_be_bytes())?;
        writer.write_all(&[self.can_always_eat as u8])?;

        Ok(())
    }
}

impl ReadFrom for FoodProperties {
    fn read(data: &mut Cursor<&[u8]>) -> Result<Self> {
        Ok(Self {
            nutrition: VarInt::read(data)?.0,
            saturation: {
                let mut saturation_bytes = [0u8; 4];
                data.read_exact(&mut saturation_bytes)?;

                f32::from_be_bytes(saturation_bytes)
            },
            can_always_eat: {
                let mut can_always_eat_bytes = [0u8; 1];
                data.read_exact(&mut can_always_eat_bytes)?;

                can_always_eat_bytes[0] != 0
            },
        })
    }
}

impl simdnbt::ToNbtTag for FoodProperties {
    fn to_nbt_tag(self) -> simdnbt::owned::NbtTag {
        let mut compound = NbtCompound::new();
        compound.insert("nutrition", NbtTag::Int(self.nutrition));
        compound.insert("saturation", NbtTag::Float(self.saturation));
        compound.insert("can_always_eat", NbtTag::Byte(self.can_always_eat as i8));
        NbtTag::Compound(compound)
    }
}

impl simdnbt::FromNbtTag for FoodProperties {
    fn from_nbt_tag(tag: simdnbt::borrow::NbtTag) -> Option<Self> {
        let compound = tag.compound()?;
        Some(Self {
            nutrition: compound.get("nutrition")?.int()?,
            saturation: compound.get("saturation")?.float()?,
            can_always_eat: compound.get("can_always_eat")?.byte()? != 0,
        })
    }
}

impl HashComponent for FoodProperties {
    fn hash_component(&self, hasher: &mut ComponentHasher) {
        // For now, hash as empty map since full implementation requires proper codec
        hasher.start_map();
        // TODO: Add proper field hashing when FoodProperties codec is implemented
        hasher.end_map();
    }
}
