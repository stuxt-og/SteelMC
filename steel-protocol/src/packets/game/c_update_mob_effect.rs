//! Clientbound update mob effect packet - sent to sync entity effect.

use steel_macros::{ClientPacket, WriteTo};
use steel_registry::packets::play::C_UPDATE_MOB_EFFECT;

/// Clientbound packet sent to update entity mob effect.
///
/// Used for mob effects.
/// Vanilla: `ClientboundUpdateMobEffectPacket`
#[derive(ClientPacket, WriteTo, Clone, Debug)]
#[packet_id(Play = C_UPDATE_MOB_EFFECT)]
pub struct CUpdateMobEffect {
    /// The entity ID whose mob effect are being updated.
    #[write(as = VarInt)]
    pub entity_id: i32,
    /// The mob effect to sync.
    #[write(as = VarInt)]
    pub effect_id: i32,
    /// The mob effect amplifier to sync.
    #[write(as = VarInt)]
    pub amplfier: i32,
    /// The mob effect duration to sync.
    #[write(as = VarInt)]
    pub duration: i32,
    // The mob effect flags to sync
    pub flags: i8,
}

impl CUpdateMobEffect {
    /// Creates a new update mob effect packet.
    #[must_use]
    pub fn new(
        entity_id: i32,
        effect_id: i32,
        amplifier: i32,
        duration: Option<i32>,
        flags: i8,
    ) -> Self {
        Self {
            entity_id,
            effect_id,
            amplfier: amplifier,
            duration: duration.unwrap_or(-1),
            flags,
        }
    }

    #[must_use]
    pub const fn make_flags(ambient: bool, visible: bool, show_icon: bool, blend: bool) -> i8 {
        let mut flags: i8 = 0;
        if ambient {
            flags |= 1;
        }
        if visible {
            flags |= 2;
        }
        if show_icon {
            flags |= 4;
        }
        if blend {
            flags |= 8;
        }
        flags
    }
}
