/// Generates the TokenStream for a ConsumeEffect from JSON data.
use std::{collections::BTreeMap, fs};

use heck::ToShoutySnakeCase;
use proc_macro2::{Ident, Span, TokenStream};
use quote::quote;
use serde::Deserialize;
use serde_json::Value;

#[derive(Deserialize, Clone, Debug, Default)]
#[serde(rename_all = "camelCase")]
#[expect(dead_code)]
pub struct Item {
    pub id: u16,
    pub name: String,
    #[serde(default)]
    pub components: BTreeMap<String, Value>,
    #[serde(default)]
    pub block_item: Option<String>,
    #[serde(default)]
    pub is_double: bool,
    #[serde(default)]
    pub is_scaffolding: bool,
    #[serde(default)]
    pub is_water_placable: bool,
}

#[derive(Deserialize, Clone, Debug)]
pub struct Items {
    pub items: Vec<Item>,
}

fn get_component_ident(name: &str) -> Option<Ident> {
    let name = name.strip_prefix("minecraft:").unwrap_or(name);
    let shouty_name = name.to_shouty_snake_case();
    Some(Ident::new(&shouty_name, Span::call_site()))
}

/// Generates the TokenStream for a Tool component from JSON data.
fn generate_tool_component(value: &Value) -> TokenStream {
    let rules = value
        .get("rules")
        .and_then(|r| r.as_array())
        .map(|rules_arr| rules_arr.iter().map(generate_tool_rule).collect::<Vec<_>>())
        .unwrap_or_default();

    let default_mining_speed = value
        .get("default_mining_speed")
        .and_then(|v| v.as_f64())
        .unwrap_or(1.0) as f32;

    let damage_per_block = value
        .get("damage_per_block")
        .and_then(|v| v.as_i64())
        .unwrap_or(1) as i32;

    let can_destroy_blocks_in_creative = value
        .get("can_destroy_blocks_in_creative")
        .and_then(|v| v.as_bool())
        .unwrap_or(true);

    quote! {
        vanilla_components::Tool {
            rules: vec![#(#rules),*],
            default_mining_speed: #default_mining_speed,
            damage_per_block: #damage_per_block,
            can_destroy_blocks_in_creative: #can_destroy_blocks_in_creative,
        }
    }
}

/// Generates the TokenStream for a Consumable component from JSON data.
fn generate_consumable_component(value: &Value) -> TokenStream {
    let consume_seconds = value
        .get("consume_seconds")
        .and_then(|v| v.as_f64())
        .unwrap_or(1.6) as f32;

    let anim = value
        .get("animation")
        .and_then(|a| a.as_str())
        .unwrap_or("eat");

    let has_consume_particles = value
        .get("has_consume_particles")
        .and_then(|v| v.as_bool())
        .unwrap_or(false);

    let on_consume_effects_tokens = match value.get("on_consume_effects") {
        Some(effects_value) => {
            if let Some(effects_array) = effects_value.as_array() {
                let effect_tokens: Vec<TokenStream> = effects_array
                    .iter()
                    .map(generate_consume_effect_tokens)
                    .collect();
                quote! { vec![ #(#effect_tokens),* ] }
            } else {
                let effect_tokens = generate_consume_effect_tokens(effects_value);
                quote! { vec![ #effect_tokens ] }
            }
        }
        None => quote! { vec![] },
    };

    quote! {
        vanilla_components::Consumable {
            consume_seconds: #consume_seconds,
            animation: ItemUseAnimation::from_str(#anim).unwrap(),
            has_consume_particles: #has_consume_particles,
            on_consume_effects: #on_consume_effects_tokens,
        }
    }
}

/// Generates the TokenStream for a FoodProperties component from JSON data.
fn generate_food_properties_component(value: &Value) -> TokenStream {
    let nutrition = value.get("nutrition").and_then(|n| n.as_i64()).unwrap() as i32;

    let saturation = value.get("saturation").and_then(|v| v.as_f64()).unwrap() as f32;

    let can_always_eat = value
        .get("can_always_eat")
        .and_then(|v| v.as_bool())
        .unwrap_or(false);

    quote! {
        vanilla_components::FoodProperties {
            nutrition: #nutrition,
            saturation: #saturation,
            can_always_eat: #can_always_eat
        }
    }
}

/// Generates the TokenStream for a UseCooldown component from JSON data
fn generate_use_cooldown_component(value: &Value) -> TokenStream {
    let seconds = value.get("seconds").and_then(|v| v.as_f64()).unwrap() as f32;

    let opt_cooldown_group = value.get("cooldown_group");

    if let Some(cooldown_group_val) = opt_cooldown_group {
        let cooldown_group = cooldown_group_val.as_str();

        quote! {
            vanilla_components::UseCooldown {
                seconds: #seconds,
                cooldown_group: Some(Identifier::vanilla_static(#cooldown_group)),
            }
        }
    } else {
        quote! {
            vanilla_components::UseCooldown {
                seconds: #seconds,
                cooldown_group: None,
            }
        }
    }
}

/// Generates the TokenStream for a UseRemainder from JSON data
fn generate_use_remainder_component(value: &Value) -> TokenStream {
    let identifier = parse_identifier(value.get("id").and_then(|v| v.as_str()).unwrap());

    quote! {
        vanilla_components::UseRemainder::new(
            #identifier
        )
    }
}

/// Generates the TokenStream for a UseEffects component from JSON data.
fn generate_use_effects_component(value: &Value) -> TokenStream {
    let can_sprint = value
        .get("can_sprint")
        .and_then(|n| n.as_bool())
        .unwrap_or(false);

    let interaction_vibrations = value
        .get("interaction_vibrations")
        .and_then(|v| v.as_bool())
        .unwrap_or(true);

    let speed_multiplier = value
        .get("speed_multiplier")
        .and_then(|v| v.as_f64())
        .unwrap_or(0.2) as f32;

    quote! {
        vanilla_components::UseEffects {
            can_sprint: #can_sprint,
            interaction_vibrations: #interaction_vibrations,
            speed_multiplier: #speed_multiplier
        }
    }
}

fn parse_identifier(name: &str) -> TokenStream {
    if let Some((namespace, path)) = name.split_once(':') {
        quote! {
            Identifier::new_static(#namespace, #path)
        }
    } else {
        quote! {
            Identifier::vanilla_static(#name)
        }
    }
}

/// Generates the TokenStream for a MobEffectInstance from JSON data.
fn generate_mob_effect_instance_tokens(value: &Value) -> TokenStream {
    let id_str = value
        .get("id")
        .and_then(|v| v.as_str())
        .expect("effect id must be string");

    let amplifier = value.get("amplifier").and_then(|v| v.as_u64()).unwrap_or(0) as i32;

    let duration = value.get("duration").and_then(|v| v.as_u64()).unwrap_or(0) as i32;

    let ambient = value
        .get("ambient")
        .and_then(|v| v.as_bool())
        .unwrap_or(false);

    let visible = value
        .get("visible")
        .and_then(|v| v.as_bool())
        .unwrap_or(true);

    let show_icon = value
        .get("show_icon")
        .and_then(|v| v.as_bool())
        .unwrap_or(true);

    let duration = if duration == -1 {
        quote! { Some(#duration) }
    } else {
        quote! { None }
    };

    let identifier = parse_identifier(id_str);

    let hidden_effect_opt = value.get("hidden_effect");

    if let Some(hidden_effect) = hidden_effect_opt {
        let effect = generate_mob_effect_instance_tokens(hidden_effect);

        quote! {
            MobEffectInstance::new(
                LazyMobEffect::Raw(#identifier),
                #amplifier
            )
            .with_duration(#duration)
            .with_ambient(#ambient)
            .with_visible(#visible)
            .with_show_icon(#show_icon)
            .with_hidden_effect(Some(Box::new( #effect ))),
        }
    } else {
        quote! {
            MobEffectInstance::new(
                LazyMobEffect::Raw(#identifier),
                #amplifier
            )
            .with_duration(#duration)
            .with_ambient(#ambient)
            .with_visible(#visible)
            .with_show_icon(#show_icon),
        }
    }
}

/// Generates the TokenStream for a ConsumeEffect from JSON data.
fn generate_consume_effect_tokens(value: &Value) -> TokenStream {
    let obj = value.as_object().expect("ConsumeEffect must be an object");
    let type_str = obj
        .get("type")
        .and_then(|v| v.as_str())
        .expect("Missing 'type' field in ConsumeEffect");

    match type_str {
        "minecraft:apply_effects" => {
            let effects_array = obj
                .get("effects")
                .expect("Missing 'effects' for apply_effects")
                .as_array()
                .expect("effects must be an array");

            let effect_instances: Vec<TokenStream> = effects_array
                .iter()
                .map(generate_mob_effect_instance_tokens)
                .collect();

            quote! {
                ConsumeEffect::ApplyEffects {
                    effects: vec![ #(#effect_instances)* ]
                }
            }
        }

        "minecraft:remove_effects" => {
            let effects_val = obj
                .get("effects")
                .expect("Missing 'effects' for remove_effects");
            let effect_ids: Vec<TokenStream> = if let Some(arr) = effects_val.as_array() {
                arr.iter()
                    .map(|v| {
                        let identifier =
                            parse_identifier(v.as_str().expect("effect must be string"));

                        quote! { LazyMobEffect::Raw(#identifier) }
                    })
                    .collect()
            } else if effects_val.as_str().is_some() {
                let identifier = parse_identifier(
                    effects_val
                        .as_str()
                        .expect("Invalid effects field for remove_effects"),
                );

                vec![quote! { LazyMobEffect::Raw(#identifier) }]
            } else {
                panic!("Invalid effects field for remove_effects");
            };

            quote! {
                ConsumeEffect::RemoveEffects {
                    effects: vec![ #(#effect_ids),* ]
                }
            }
        }

        "minecraft:clear_all_effects" => {
            quote! { ConsumeEffect::ClearAllEffects }
        }

        "minecraft:teleport_randomly" => {
            let diameter = obj.get("diameter").and_then(|v| v.as_f64()).unwrap_or(0.0) as f32;
            quote! {
                ConsumeEffect::TeleportRandomly {
                    diameter: #diameter
                }
            }
        }

        "minecraft:play_sound" => {
            let identifier = parse_identifier(
                obj.get("sound")
                    .and_then(|v| v.as_str())
                    .expect("Missing sound for play_sound"),
            );

            quote! {
                ConsumeEffect::PlaySound {
                    sound_event: #identifier
                }
            }
        }

        _ => panic!("Unknown ConsumeEffect type: {}", type_str),
    }
}

/// Parses a block or tag reference string into an Identifier TokenStream.
/// For tags like "#minecraft:mineable/pickaxe", creates Identifier { namespace: "#minecraft", path: "mineable/pickaxe" }
/// For blocks like "minecraft:stone", creates Identifier { namespace: "minecraft", path: "stone" }
fn parse_block_or_tag(s: &str) -> TokenStream {
    let (is_tag, rest) = if let Some(stripped) = s.strip_prefix('#') {
        (true, stripped)
    } else {
        (false, s)
    };

    // Split namespace:path
    let parts: Vec<&str> = rest.splitn(2, ':').collect();
    let (namespace, path) = if parts.len() == 2 {
        (parts[0], parts[1])
    } else {
        // Default to minecraft namespace
        ("minecraft", rest)
    };

    if is_tag {
        // Prefix namespace with # for tags
        let tag_namespace = format!("#{namespace}");
        quote! { Identifier::new(#tag_namespace, #path) }
    } else {
        quote! { Identifier::new(#namespace, #path) }
    }
}

/// Generates the TokenStream for a single ToolRule from JSON data.
fn generate_tool_rule(rule: &Value) -> TokenStream {
    // Parse blocks - can be a string (single block or tag), or an array of strings
    let blocks_value = rule.get("blocks");
    let blocks_tokens: Vec<TokenStream> = match blocks_value {
        Some(Value::String(s)) => {
            vec![parse_block_or_tag(s)]
        }
        Some(Value::Array(arr)) => arr
            .iter()
            .filter_map(|v| v.as_str())
            .map(parse_block_or_tag)
            .collect(),
        _ => vec![],
    };

    // Parse optional speed
    let speed_token = match rule.get("speed").and_then(|v| v.as_f64()) {
        Some(speed) => {
            let speed = speed as f32;
            quote! { Some(#speed) }
        }
        None => quote! { None },
    };

    // Parse optional correct_for_drops
    let correct_for_drops_token = match rule.get("correct_for_drops").and_then(|v| v.as_bool()) {
        Some(correct) => quote! { Some(#correct) },
        None => quote! { None },
    };

    quote! {
        vanilla_components::ToolRule {
            blocks: vec![#(#blocks_tokens),*],
            speed: #speed_token,
            correct_for_drops: #correct_for_drops_token,
        }
    }
}

/// Returns the crafting remainder item key for a given item, if any.
/// Based on vanilla Minecraft's Item.Properties.craftRemainder() calls.
fn get_craft_remainder(item_name: &str) -> Option<&'static str> {
    match item_name {
        // Buckets return empty bucket
        "water_bucket"
        | "lava_bucket"
        | "milk_bucket"
        | "powder_snow_bucket"
        | "pufferfish_bucket"
        | "salmon_bucket"
        | "cod_bucket"
        | "tropical_fish_bucket"
        | "axolotl_bucket"
        | "tadpole_bucket" => Some("bucket"),
        // Bottles return empty glass bottle
        "dragon_breath" | "honey_bottle" => Some("glass_bottle"),
        // Potions also return glass bottles when used in crafting
        "potion" => Some("glass_bottle"),
        _ => None,
    }
}

fn generate_builder_calls(item: &Item) -> Vec<TokenStream> {
    let mut builder_calls = Vec::new();

    for (key, value) in &item.components {
        let component_ident = if let Some(ident) = get_component_ident(key) {
            ident
        } else {
            continue;
        };

        match key.as_str() {
            "minecraft:max_stack_size" => {
                let val = value.as_i64().unwrap() as i32;
                if val != 64 {
                    builder_calls.push(
                        quote! { .builder_set(vanilla_components::#component_ident, Some(#val)) },
                    );
                }
            }
            "minecraft:max_damage" => {
                let val = value.as_i64().unwrap() as i32;
                builder_calls.push(
                    quote! { .builder_set(vanilla_components::#component_ident, Some(#val)) },
                );
            }
            "minecraft:damage" => {
                let val = value.as_i64().unwrap() as i32;
                builder_calls.push(
                    quote! { .builder_set(vanilla_components::#component_ident, Some(#val)) },
                );
            }
            "minecraft:repair_cost" => {
                let val = value.as_i64().unwrap() as i32;
                if val != 0 {
                    builder_calls.push(
                        quote! { .builder_set(vanilla_components::#component_ident, Some(#val)) },
                    );
                }
            }
            "minecraft:unbreakable" => {
                builder_calls
                    .push(quote! { .builder_set(vanilla_components::#component_ident, Some(())) });
            }
            "minecraft:glider" => {
                builder_calls
                    .push(quote! { .builder_set(vanilla_components::#component_ident, Some(())) });
            }
            "minecraft:enchantment_glint_override" => {
                let val = value.as_bool().unwrap();
                builder_calls.push(
                    quote! { .builder_set(vanilla_components::#component_ident, Some(#val)) },
                );
            }
            "minecraft:equippable" => {
                // Parse the equippable component to get the slot
                if let Some(slot_str) = value.get("slot").and_then(|s| s.as_str()) {
                    let slot_variant = match slot_str {
                        "head" => quote! { vanilla_components::EquippableSlot::Head },
                        "chest" => quote! { vanilla_components::EquippableSlot::Chest },
                        "legs" => quote! { vanilla_components::EquippableSlot::Legs },
                        "feet" => quote! { vanilla_components::EquippableSlot::Feet },
                        "body" => quote! { vanilla_components::EquippableSlot::Body },
                        "mainhand" => quote! { vanilla_components::EquippableSlot::Mainhand },
                        "offhand" => quote! { vanilla_components::EquippableSlot::Offhand },
                        "saddle" => quote! { vanilla_components::EquippableSlot::Saddle },
                        _ => continue,
                    };
                    builder_calls.push(
                        quote! { .builder_set(vanilla_components::EQUIPPABLE, Some(vanilla_components::Equippable { slot: #slot_variant })) },
                    );
                }
            }
            "minecraft:tool" => {
                let tool_token = generate_tool_component(value);
                builder_calls
                    .push(quote! { .builder_set(vanilla_components::TOOL, Some(#tool_token)) });
            }
            "minecraft:food" => {
                let food_props_token = generate_food_properties_component(value);
                builder_calls.push(
                    quote! { .builder_set(vanilla_components::FOOD, Some(#food_props_token)) },
                );
            }
            "minecraft:consumable" => {
                let consumable_token = generate_consumable_component(value);
                builder_calls
                    .push(quote! { .builder_set(vanilla_components::CONSUMABLE, Some(#consumable_token)) });
            }
            "minecraft:use_cooldown" => {
                let use_cooldown_token = generate_use_cooldown_component(value);
                builder_calls
                    .push(quote! { .builder_set(vanilla_components::USE_COOLDOWN, Some(#use_cooldown_token)) });
            }
            "minecraft:use_remainder" => {
                let use_remainder_token = generate_use_remainder_component(value);
                builder_calls
                    .push(quote! { .builder_set(vanilla_components::USE_REMAINDER, Some(#use_remainder_token)) });
            }
            "minecraft:use_effects" => {
                let use_effects_token = generate_use_effects_component(value);
                builder_calls
                    .push(quote! { .builder_set(vanilla_components::USE_EFFECTS, Some(#use_effects_token)) });
            }
            _ => {
                // TODO: Implement more
            }
        }
    }

    builder_calls
}

pub(crate) fn build() -> TokenStream {
    println!("cargo:rerun-if-changed=build_assets/items.json");
    let item_assets: Items =
        serde_json::from_str(&fs::read_to_string("build_assets/items.json").unwrap()).unwrap();

    let mut item_definitions = TokenStream::new();
    let mut item_construction = TokenStream::new();

    let mut register_stream = TokenStream::new();
    for item in &item_assets.items {
        let item_ident = Ident::new(&item.name, Span::call_site());
        let item_name_str = item.name.clone();

        item_definitions.extend(quote! {
           pub #item_ident: Item,
        });

        if let Some(block_name) = &item.block_item {
            let block_ident = Ident::new(&block_name.to_shouty_snake_case(), Span::call_site());
            let builder_calls = generate_builder_calls(item);

            if builder_calls.is_empty() {
                if block_name != &item.name {
                    item_construction.extend(quote! {
                        #item_ident: Item::from_block_custom_name(&vanilla_blocks::#block_ident, #item_name_str),
                    });
                } else {
                    item_construction.extend(quote! {
                        #item_ident: Item::from_block(&vanilla_blocks::#block_ident),
                    });
                }
            } else {
                // Block item with custom components
                if block_name != &item.name {
                    item_construction.extend(quote! {
                        #item_ident: Item::from_block_custom_name(&vanilla_blocks::#block_ident, #item_name_str)
                            #(#builder_calls)*,
                    });
                } else {
                    item_construction.extend(quote! {
                        #item_ident: Item::from_block(&vanilla_blocks::#block_ident)
                            #(#builder_calls)*,
                    });
                }
            }
        } else {
            let builder_calls = generate_builder_calls(item);

            let craft_remainder_value = if let Some(remainder) = get_craft_remainder(&item.name) {
                quote! { Some(Identifier::vanilla_static(#remainder)) }
            } else {
                quote! { None }
            };

            item_construction.extend(quote! {
                #item_ident: Item {
                    key: Identifier::vanilla_static(#item_name_str),
                    components: DataComponentMap::common_item_components()
                        #(#builder_calls)*,
                    craft_remainder: #craft_remainder_value,
                    id: OnceLock::new(),
                },
            });
        }

        register_stream.extend(quote! {
            registry.register(&ITEMS.#item_ident);
        });
    }

    quote! {
        use crate::{
            data_components::{vanilla_components, DataComponentMap, components::ConsumeEffect},
            vanilla_blocks,
            items::{Item, ItemRegistry},

        };

        use crate::items::item::ItemUseAnimation;
        use crate::mob_effect::{MobEffectInstance, LazyMobEffect};
        use crate::item_stack::ItemStackTemplate;
        use crate::REGISTRY;

        use steel_utils::Identifier;
        use steel_utils::codec::VarInt;

        use std::sync::{LazyLock, OnceLock};
        use std::str::FromStr;

        pub static ITEMS: LazyLock<Items> = LazyLock::new(Items::init);

        pub struct Items {
            #item_definitions
        }

        impl Items {
            fn init() -> Self {
                Self {
                    #item_construction
                }
            }
        }

        pub fn register_items(registry: &mut ItemRegistry) {
            #register_stream
        }
    }
}
