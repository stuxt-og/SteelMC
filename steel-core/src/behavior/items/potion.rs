use crate::behavior::context::{InteractionResult, UseOnContext};
use crate::behavior::item::ItemBehavior;
use steel_macros::item_behavior;
//use super::consumable::ConsumableImpl;
//use steel_registry::data_components::vanilla_components::{CONSUMABLE};

/// (stub) Behavior for items that are food
#[item_behavior]
pub struct PotionItem;

impl ItemBehavior for PotionItem {
    fn use_on(&self, _context: &mut UseOnContext) -> InteractionResult {
        InteractionResult::Success
    }
}
