//! Individual component type definitions.

mod consumable;
mod enchantments;
mod equippable;
mod tool;

pub use consumable::{
    Consumable, ConsumableBehavior, ConsumableComponent, ConsumableData, FoodProperties,
};
pub use enchantments::ItemEnchantments;
pub use equippable::{Equippable, EquippableSlot};
pub use tool::{Tool, ToolRule};
