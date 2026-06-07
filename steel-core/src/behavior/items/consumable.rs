use crate::behavior::context::{InteractionResult, InventoryAccess};
use crate::entity::{/*Entity,*/ LivingEntity};
use crate::player::Player;
// use steel_protocol::packets::game::SoundSource;
// use steel_registry::REGISTRY;
use steel_registry::data_components::components::{
    ConsumableBehavior, /*ConsumableComponent,*/ ConsumableData,
};
use steel_registry::data_components::vanilla_components::FOOD;
use steel_registry::item_stack::ItemStack;
// use steel_registry::items::item::ItemUseAnimation;
// use steel_registry::sound_events;
// use steel_utils::BlockPos;
// use steel_utils::random::{Random, RandomSource};
use steel_utils::types::InteractionHand;

#[derive(Clone)]
pub struct ConsumableImpl;

impl ConsumableBehavior for ConsumableImpl {}

impl ConsumableImpl {
    pub const fn new() -> Self {
        Self {}
    }

    pub fn start_consuming(
        &self,
        _data: &ConsumableData,
        user: &dyn LivingEntity,
        inv: &InventoryAccess,
        hand: InteractionHand,
    ) -> InteractionResult {
        let Some(player) = user.as_any().downcast_ref::<Player>() else {
            return InteractionResult::Pass;
        };

        let mut binding = match player.inventory.try_lock() {
            Some(x) => x,
            None => {
                println!("Deadlock!");
                return InteractionResult::Fail;
            }
        };

        if !Self::can_consume(player, binding.get_selected_item_mut()) {
            return InteractionResult::Fail;
        }

        drop(binding);

        player.start_using_item(inv, hand);

        println!("after on_consume");

        InteractionResult::Success
    }

    fn can_consume(user: &Player, stack: &ItemStack) -> bool {
        let mut can_always_eat = true;

        if let Some(food_properties) = stack.get(FOOD) {
            can_always_eat = food_properties.can_always_eat;
        }

        user.can_eat(can_always_eat)
    }

    // pub fn emit_particles_and_sounds(
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
