use rustc_hash::FxHashMap;
use std::io::{self, Cursor, Write};
use std::sync::OnceLock;

use steel_utils::Identifier;
use steel_utils::codec::var_int::VarInt;
use steel_utils::serial::{PrefixedRead, PrefixedWrite, ReadFrom, WriteTo};

use crate::mob_effect::MobEffectInstance;

#[derive(Debug, Clone)]
pub struct LazyEffects {
    init: fn() -> Vec<MobEffectInstance>,
    cache: OnceLock<Vec<MobEffectInstance>>,
}

impl LazyEffects {
    pub const fn new(init: fn() -> Vec<MobEffectInstance>) -> Self {
        Self {
            init,
            cache: OnceLock::new(),
        }
    }

    pub fn with_baked(effects: Vec<MobEffectInstance>) -> Self {
        let cache = OnceLock::new();
        let _ = cache.set(effects);

        Self {
            init: || Vec::new(),
            cache,
        }
    }

    pub fn resolve(&self) -> &Vec<MobEffectInstance> {
        self.cache.get_or_init(self.init)
    }
}

impl PartialEq for LazyEffects {
    fn eq(&self, other: &Self) -> bool {
        self.resolve().eq(other.resolve())
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct Potion {
    pub id: i32,
    pub key: Identifier,
    pub effects: LazyEffects,
}

impl Potion {
    pub fn new(id: i32, name: &'static str, effects: fn() -> Vec<MobEffectInstance>) -> Self {
        Self {
            id,
            key: Identifier::vanilla_static(name),
            effects: LazyEffects::new(effects),
        }
    }

    pub fn has_instant_effects(&self) -> bool {
        for effect in self.effects.resolve() {
            if effect.is_instantaneous() {
                return true;
            }
        }

        false
    }
}

impl ReadFrom for Potion {
    fn read(reader: &mut Cursor<&[u8]>) -> io::Result<Self> {
        Ok(Self {
            id: VarInt::read(reader)?.0,
            key: Identifier::vanilla(String::read_prefixed::<VarInt>(reader)?),
            effects: {
                let len = VarInt::read(reader)?.0 as usize;
                let mut effects = Vec::with_capacity(len);

                for _ in 0..len {
                    effects.push(MobEffectInstance::read(reader)?);
                }

                LazyEffects::with_baked(effects)
            },
        })
    }
}

impl WriteTo for Potion {
    fn write(&self, writer: &mut impl Write) -> io::Result<()> {
        VarInt::from(self.id).write(writer)?;

        self.key.to_string().write_prefixed::<VarInt>(writer)?;

        let effects = self.effects.resolve();

        VarInt::from(effects.len() as i32).write(writer)?;

        for effect in effects {
            effect.write(writer)?;
        }

        Ok(())
    }
}

pub type PotionRef = &'static Potion;

pub struct PotionRegistry {
    potions_by_id: Vec<PotionRef>,
    potions_by_key: FxHashMap<Identifier, usize>,
    allows_registering: bool,
}

impl Default for PotionRegistry {
    fn default() -> Self {
        Self::new()
    }
}

impl PotionRegistry {
    #[must_use]
    pub fn new() -> Self {
        Self {
            potions_by_id: Vec::new(),
            potions_by_key: FxHashMap::default(),
            allows_registering: true,
        }
    }

    pub fn register(&mut self, potion: PotionRef) {
        assert!(
            self.allows_registering,
            "Cannot register mob effects after the registry has been frozen"
        );
        let idx = self.potions_by_id.len();
        self.potions_by_key.insert(potion.key.clone(), idx);
        self.potions_by_id.push(potion);
    }

    pub fn iter(&self) -> impl Iterator<Item = (usize, PotionRef)> + '_ {
        self.potions_by_id
            .iter()
            .enumerate()
            .map(|(id, &potion)| (id, potion))
    }

    pub fn potion_by_key(&self, identifier: &Identifier) -> PotionRef {
        let idx = self.potions_by_key.get(identifier).unwrap_or_else(|| {
            panic!(
                "Tried to get PotionRef by '{}', but was not found PotionRegistry!",
                identifier
            )
        });

        self.potions_by_id[*idx]
    }

    pub fn potion_by_id(&self, id: i32) -> PotionRef {
        self.potions_by_id[id as usize]
    }
}

crate::impl_registry!(
    PotionRegistry,
    Potion,
    potions_by_id,
    potions_by_key,
    potions
);
