use std::clone::Clone;
use std::cmp::PartialEq;
use std::io::{Cursor, Result, Write};
use std::sync::OnceLock;

use steel_utils::{
    Identifier,
    hash::{ComponentHasher, HashComponent},
    serial::{ReadFrom, WriteTo},
};

use crate::item_stack::{ItemStack, ItemStackTemplate};

use simdnbt::owned::{NbtCompound, NbtTag};

use crate::REGISTRY;

#[derive(Debug, Clone, PartialEq)]
pub struct UseRemainder {
    identifier: Identifier,
    template: OnceLock<ItemStackTemplate>,
}

impl UseRemainder {
    pub const fn new(identifier: Identifier) -> Self {
        Self {
            identifier,
            template: OnceLock::new(),
        }
    }

    pub fn template(&self) -> &ItemStackTemplate {
        self.template.get_or_init(|| {
            let item_ref = REGISTRY
                .items
                .item_by_key(self.identifier.clone())
                .expect("Failed to resolve item in UseRemainder lazy init");

            ItemStackTemplate::new(item_ref)
        })
    }

    pub fn convert_into_remainder<F>(
        &self,
        used_stack: &ItemStack,
        has_infinite_materials: bool,
        on_extra_created_remainder: F,
    ) -> ItemStack
    where
        F: Fn(&ItemStack),
    {
        if has_infinite_materials {
            return used_stack.clone();
        }

        let remainder = self.template().0.clone();

        if used_stack.is_empty() {
            remainder
        } else {
            if !remainder.is_empty() {
                on_extra_created_remainder(&remainder);
            }

            used_stack.clone()
        }
    }
}

impl WriteTo for UseRemainder {
    fn write(&self, writer: &mut impl Write) -> Result<()> {
        // Клієнт Майнкрафту чекає ванільний ItemStack, тому шлемо дані з шаблону
        self.template().0.write(writer)?;
        Ok(())
    }
}

impl ReadFrom for UseRemainder {
    fn read(data: &mut Cursor<&[u8]>) -> Result<Self> {
        let template = ItemStackTemplate::read(data)?;

        let identifier = template.0.item.key.clone();

        let cell = OnceLock::new();
        let _ = cell.set(template);

        Ok(UseRemainder {
            identifier,
            template: cell,
        })
    }
}

impl simdnbt::ToNbtTag for UseRemainder {
    fn to_nbt_tag(self) -> simdnbt::owned::NbtTag {
        let mut compound = NbtCompound::new();
        compound.insert("convert_info", self.template().0.clone().to_nbt_tag());
        NbtTag::Compound(compound)
    }
}

impl simdnbt::FromNbtTag for UseRemainder {
    fn from_nbt_tag(tag: simdnbt::borrow::NbtTag) -> Option<Self> {
        let template = ItemStackTemplate::from_nbt_tag(tag)?;

        let cell = OnceLock::new();
        let _ = cell.set(template.clone());

        Some(UseRemainder {
            identifier: template.0.item.key.clone(),
            template: cell,
        })
    }
}

impl HashComponent for UseRemainder {
    fn hash_component(&self, hasher: &mut ComponentHasher) {
        hasher.start_map();
        self.identifier.hash_component(hasher);
        hasher.end_map();
    }
}
