use crate::behavior::context::{InteractionResult, UseItemContext};
use crate::behavior::item::ItemBehavior;
use crate::player::Player;
use steel_macros::item_behavior;

use steel_registry::data_components::vanilla_components::FOOD;
use steel_registry::item_stack::ItemStack;

/// Behavior for items that are food
#[item_behavior]
pub struct FoodItem;

impl FoodItem {
    fn can_consume(user: &Player, stack: &ItemStack) -> bool {
        let mut can_always_eat = false;

        if let Some(food_properties) = stack.get(FOOD) {
            can_always_eat = food_properties.can_always_eat;
        }

        user.can_eat(can_always_eat)
    }

    // fn emit_particles_and_sounds(
    //     &self,
    //     data: &ConsumableData,
    //     random: &mut RandomSource,
    //     player: &Player,
    //     item_stack: &ItemStack,
    //     particle_count: i32,
    // ) {
    //     let consumable_volume = if data.animation == ItemUseAnimation::Drink {
    //         0.5
    //     } else {
    //         if random.next_bool() { 0.5 } else { 1.0 }
    //     };
    //
    //     let consumable_pitch = if data.animation == ItemUseAnimation::Drink {
    //         (0.9 + random.next_f32() * 0.1) as f64
    //     } else {
    //         random.triangle(1.0, 0.2)
    //     };
    //
    //     // if data.has_consume_particles {
    //     //     player.spawn_item_particles(item_stack, particle_count);
    //     // }
    //
    //     // player.play_sound(
    //     //     REGISTRY
    //     //         .sound_events
    //     //         .get()
    //     //         .unwrap()
    //     //         .sound_events_by_id
    //     //         .get(sound_events::ENTITY_PLAYER_BURP),
    //     //     0.5,
    //     //     0.9 + random.next_f32() * 0.1,
    //     // );
    // }
}

impl ItemBehavior for FoodItem {
    fn use_item(&self, context: &mut UseItemContext) -> InteractionResult {
        // let (existing_handler, data) = if let Some(consumable) = stack.get(CONSUMABLE) {
        //     (consumable.handler.clone(), consumable.data.clone())
        // } else {
        //     println!("TODO: add Consumable component to FoodItems");
        //     return InteractionResult::Pass;
        // };

        let Some(binding) = context.player.inventory.try_lock() else {
            println!("Deadlock!");
            return InteractionResult::Fail;
        };

        if !Self::can_consume(context.player, binding.get_item_in_hand(context.hand)) {
            return InteractionResult::Fail;
        }

        drop(binding);

        context.player.start_using_item(&context.inv, context.hand);

        InteractionResult::Success
    }
}
