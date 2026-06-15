//! Clientbound remove mob effect packet - sent to remove entity effect.

use steel_macros::{ClientPacket, WriteTo};
use steel_registry::packets::play::C_REMOVE_MOB_EFFECT;
use steel_utils::Identifier;

/// Clientbound packet sent to remove entity mob effect.
///
/// Used for mob effects.
/// Vanilla: `ClientboundRemoveMobEffectPacket`
#[derive(ClientPacket, WriteTo, Clone, Debug)]
#[packet_id(Play = C_REMOVE_MOB_EFFECT)]
pub struct CRemoveMobEffect {
    /// The entity ID whose mob effect is being removed.
    #[write(as = VarInt)]
    pub entity_id: i32,
    /// The mob effect to remove.
    pub identifier: Identifier,
}

impl CRemoveMobEffect {
    /// Creates a new update attributes packet.
    #[must_use]
    pub fn new(entity_id: i32, identifier: Identifier) -> Self {
        Self {
            entity_id,
            identifier,
        }
    }
}
