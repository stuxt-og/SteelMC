use std::clone::Clone;
use std::cmp::PartialEq;
use std::io::{Cursor, Read, Result, Write};

use steel_utils::{
    Identifier,
    codec::VarInt,
    hash::{ComponentHasher, HashComponent},
    serial::{ReadFrom, WriteTo},
};

use simdnbt::owned::{NbtCompound, NbtTag};

use crate::mob_effect::MobEffectInstance;
use crate::potion::PotionRef;

use crate::REGISTRY;

#[derive(Debug, Clone, PartialEq)]
pub struct PotionContents {
    pub potion: Option<PotionRef>,
    pub custom_color: Option<i32>,
    pub custom_effects: Vec<MobEffectInstance>,
    pub custom_name: Option<String>,
}

impl WriteTo for PotionContents {
    fn write(&self, writer: &mut impl Write) -> Result<()> {
        if let Some(potion) = &self.potion {
            bool::write(&true, writer)?;
            VarInt::from(potion.id).write(writer)?;
        } else {
            bool::write(&false, writer)?;
        }

        if let Some(custom_color) = &self.custom_color {
            bool::write(&true, writer)?;
            i32::write(custom_color, writer)?;
        } else {
            bool::write(&false, writer)?;
        }

        VarInt::from(self.custom_effects.len() as i32).write(writer)?;
        for effect in &self.custom_effects {
            effect.write(writer)?;
        }

        if let Some(custom_name) = &self.custom_name {
            bool::write(&true, writer)?;
            VarInt::from(custom_name.len() as i32).write(writer)?;
            writer.write_all(custom_name.as_bytes())?;
        } else {
            bool::write(&false, writer)?;
        }

        Ok(())
    }
}

impl ReadFrom for PotionContents {
    fn read(data: &mut Cursor<&[u8]>) -> Result<Self> {
        Ok(Self {
            potion: {
                if bool::read(data)? {
                    Some(REGISTRY.potions.potion_by_id(VarInt::read(data)?.0))
                } else {
                    None
                }
            },
            custom_color: if bool::read(data)? {
                Some(i32::read(data)?)
            } else {
                None
            },

            custom_effects: {
                let len = VarInt::read(data)?.0 as usize;
                let mut effects = Vec::with_capacity(len);
                for _ in 0..len {
                    effects.push(MobEffectInstance::read(data)?);
                }
                effects
            },

            custom_name: {
                if bool::read(data)? {
                    let len = VarInt::read(data)?.0 as usize;
                    let mut buf = vec![0; len];
                    data.read_exact(&mut buf)?;
                    Some(String::from_utf8(buf).expect("Failed to decode custom name"))
                } else {
                    None
                }
            },
        })
    }
}

impl simdnbt::ToNbtTag for PotionContents {
    fn to_nbt_tag(self) -> simdnbt::owned::NbtTag {
        let mut compound = NbtCompound::new();
        if let Some(potion) = self.potion {
            compound.insert("potion", NbtTag::String(potion.key.to_string().into()));
        }

        if let Some(custom_color) = self.custom_color {
            compound.insert("custom_color", NbtTag::Int(custom_color));
        }

        let mut effects_list = Vec::new();
        for effect in self.custom_effects {
            effects_list.push(NbtTag::Compound(effect.to_nbt_compound()));
        }

        compound.insert("custom_effects", NbtTag::List(effects_list.into()));

        NbtTag::Compound(compound)
    }
}

impl simdnbt::FromNbtTag for PotionContents {
    fn from_nbt_tag(tag: simdnbt::borrow::NbtTag) -> Option<Self> {
        let compound = tag.compound()?;
        Some(Self {
            potion: {
                if let Some(key) = compound.get("potion") {
                    let full_id_str = key.string()?.to_str();

                    let id = if full_id_str.starts_with("minecraft:") {
                        Identifier::vanilla(full_id_str.replace("minecraft:", ""))
                    } else {
                        Identifier::vanilla(full_id_str.into())
                    };

                    Some(REGISTRY.potions.potion_by_key(&id))
                } else {
                    None
                }
            },
            custom_color: {
                if let Some(custom_color) = compound.get("custom_color") {
                    Some(custom_color.int()?)
                } else {
                    None
                }
            },
            custom_effects: {
                if let Some(effects_tag) = compound.get("custom_effects") {
                    let list = effects_tag.list()?;
                    let mut effects = Vec::new();
                    for eff_comp in list.compounds()? {
                        effects.push(MobEffectInstance::from_nbt_compound(&eff_comp)?);
                    }
                    effects
                } else {
                    Vec::new()
                }
            },
            custom_name: {
                if let Some(custom_name) = compound.get("custom_name") {
                    Some(custom_name.string()?.into())
                } else {
                    None
                }
            },
        })
    }
}

impl HashComponent for PotionContents {
    fn hash_component(&self, hasher: &mut ComponentHasher) {
        // For now, hash as empty map since full implementation requires proper codec
        hasher.start_map();
        // TODO: Add proper field hashing when FoodProperties codec is implemented
        hasher.end_map();
    }
}
