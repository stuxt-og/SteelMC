use std::clone::Clone;
use std::cmp::PartialEq;
use std::io::{Cursor, Read, Result, Write};

use steel_utils::{
    Identifier,
    hash::{ComponentHasher, HashComponent},
    serial::{ReadFrom, WriteTo},
};

use simdnbt::owned::{NbtCompound, NbtTag};

#[derive(Debug, Clone, PartialEq)]
pub struct UseCooldown {
    pub seconds: f32,

    // useless, probably should just skip bytes
    pub cooldown_group: Option<Identifier>,
}

impl UseCooldown {
    pub fn ticks(&self) -> u32 {
        (self.seconds * 20.0) as u32
    }
}

impl WriteTo for UseCooldown {
    fn write(&self, writer: &mut impl Write) -> Result<()> {
        // Format: nutrition (VarInt), saturation (f32), can_always_eat (bool)
        writer.write_all(&self.seconds.to_be_bytes())?;
        self.cooldown_group.write(writer)?;

        Ok(())
    }
}

impl ReadFrom for UseCooldown {
    fn read(data: &mut Cursor<&[u8]>) -> Result<Self> {
        Ok(Self {
            seconds: {
                let mut seconds_bytes = [0u8; 4];
                data.read_exact(&mut seconds_bytes)?;

                f32::from_be_bytes(seconds_bytes)
            },
            cooldown_group: Option::read(data)
                .expect("Failed to read Option<Identifier> from packet"),
        })
    }
}

impl simdnbt::ToNbtTag for UseCooldown {
    fn to_nbt_tag(self) -> simdnbt::owned::NbtTag {
        let mut compound = NbtCompound::new();
        compound.insert("seconds", NbtTag::Float(self.seconds));

        if let Some(cooldown_group) = self.cooldown_group {
            compound.insert("cooldown_group", cooldown_group.to_nbt_tag());
        }

        NbtTag::Compound(compound)
    }
}

impl simdnbt::FromNbtTag for UseCooldown {
    fn from_nbt_tag(tag: simdnbt::borrow::NbtTag) -> Option<Self> {
        let compound = tag.compound()?;
        Some(Self {
            seconds: compound.get("seconds")?.float()?,
            cooldown_group: Identifier::from_nbt_tag(compound.get("cooldown_group").unwrap()),
        })
    }
}

impl HashComponent for UseCooldown {
    fn hash_component(&self, hasher: &mut ComponentHasher) {
        // For now, hash as empty map since full implementation requires proper codec
        hasher.start_map();
        // TODO: Add proper field hashing when FoodProperties codec is implemented
        hasher.end_map();
    }
}
