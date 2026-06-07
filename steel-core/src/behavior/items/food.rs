use super::consumable::ConsumableImpl;
use crate::behavior::context::{InteractionResult, UseItemContext};
use crate::behavior::item::ItemBehavior;
use std::sync::Arc;
use steel_macros::item_behavior;
use steel_registry::data_components::components::{Consumable, ConsumableData};
//use steel_registry::data_components::vanilla_components::CONSUMABLE;

/// Behavior for items that are food
#[item_behavior]
pub struct FoodItem;

impl ItemBehavior for FoodItem {
    fn use_item(&self, context: &mut UseItemContext) -> InteractionResult {
        let binding = match context.player.inventory.try_lock() {
            Some(x) => x,
            None => {
                println!("Deadlock!");
                return InteractionResult::Fail;
            }
        };

        // let (existing_handler, data) = if let Some(consumable) = stack.get(CONSUMABLE) {
        //     (consumable.handler.clone(), consumable.data.clone())
        // } else {
        //     println!("TODO: add Consumable component to FoodItems");
        //     return InteractionResult::Pass;
        // };

        let handler_arc = Arc::new(ConsumableImpl::new()) as Arc<dyn Consumable>;

        let handler_any = handler_arc.as_any();
        if let Some(consumable_impl) = handler_any.downcast_ref::<ConsumableImpl>() {
            drop(binding);
            consumable_impl.start_consuming(
                &ConsumableData::default(),
                context.player,
                &context.inv,
                context.hand,
            );
        } else {
            println!("Failed to get ConsumableImpl");
        }

        InteractionResult::Success
    }
}
