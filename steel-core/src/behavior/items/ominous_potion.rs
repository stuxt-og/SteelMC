use crate::behavior::item::ItemBehavior;
use steel_macros::item_behavior;
use crate::behavior::context::{InteractionResult, UseOnContext};
use super::consumable::ConsumableImpl;
use steel_registry::data_components::vanilla_components::{CONSUMABLE};

//#[item_behavior]
pub struct OminousPotionItem;

impl ItemBehavior for OminousPotionItem {
    fn use_on(&self, context: &mut UseOnContext) -> InteractionResult {
        InteractionResult::Success
    }
}
