use std::fs;

use heck::ToShoutySnakeCase;
use proc_macro2::{Ident, Span, TokenStream};
use quote::quote;
use serde::Deserialize;

use std::boxed::Box;

fn ambient_default() -> bool {
    false
}

fn visible_default() -> bool {
    true
}

fn hidden_effect_default() -> Option<Box<Effect>> {
    None
}

#[derive(Deserialize, Debug)]
pub struct Effect {
    name: String,
    duration: Option<i32>,
    amplifier: i32,

    #[serde(default = "ambient_default")]
    ambient: bool,

    #[serde(default = "visible_default")]
    visible: bool,

    show_icon: bool,

    #[serde(default = "hidden_effect_default")]
    hidden_effect: Option<Box<Effect>>,
}

#[derive(Deserialize, Debug)]
pub struct PotionJson {
    id: i32,
    name: String,
    effects: Vec<Effect>,
}

fn load_effect(effect_data: &Effect) -> TokenStream {
    let name = &effect_data.name;
    let amplifier = &effect_data.amplifier;
    let ambient = &effect_data.ambient;
    let visible = &effect_data.visible;
    let show_icon = &effect_data.show_icon;

    let duration = match &effect_data.duration {
        Some(val) => quote! { Some(#val) },
        None => quote! { None },
    };

    if let Some(effect) = &effect_data.hidden_effect {
        let hidden_effect = load_effect(effect);

        quote! {
            MobEffectInstance::new(
                LazyMobEffect::Raw(Identifier::vanilla_static(#name)),
                #amplifier
            )
            .with_duration(#duration)
            .with_ambient(#ambient)
            .with_visible(#visible)
            .with_show_icon(#show_icon)
            .with_hidden_effect(Some(Box::new( #hidden_effect ))),
        }
    } else {
        quote! {
            MobEffectInstance::new(
                LazyMobEffect::Raw(Identifier::vanilla_static(#name)),
                #amplifier
            )
            .with_duration(#duration)
            .with_ambient(#ambient)
            .with_visible(#visible)
            .with_show_icon(#show_icon),
        }
    }
}

pub(crate) fn build() -> TokenStream {
    println!("cargo:rerun-if-changed=build_assets/potions.json");

    let potion_jsons: Vec<PotionJson> = serde_json::from_str(
        fs::read_to_string("build_assets/potions.json")
            .unwrap()
            .as_str(),
    )
    .expect("Failed to parse potions.json");

    let mut stream = TokenStream::new();

    stream.extend(quote! {
        use crate::items::potion::{LazyEffects, Potion};
        use crate::PotionRegistry;
        use crate::mob_effect::MobEffectInstance;
        use crate::mob_effect::LazyMobEffect;
        use steel_utils::Identifier;
        use steel_utils::codec::VarInt;
        use steel_utils::serial::{ReadFrom, WriteTo, PrefixedRead, PrefixedWrite};
    });

    // Generate static potion definitions
    let mut register_stream = TokenStream::new();
    for potion_json in &potion_jsons {
        let potion_name = &potion_json.name;

        let potion_ident = if potion_name.chars().next().unwrap().is_ascii_digit() {
            Ident::new(
                &format!("POTION_{}", potion_name.to_shouty_snake_case()),
                Span::call_site(),
            )
        } else {
            Ident::new(&potion_name.to_shouty_snake_case(), Span::call_site())
        };

        let potion_effects_init_ident = Ident::new(
            format!("{}{}", potion_name, "_effects_init").as_str(),
            Span::call_site(),
        );

        let mut mob_effect_instance_inits = Vec::new();

        for effect in &potion_json.effects {
            mob_effect_instance_inits.extend(load_effect(effect));
        }

        let potion_id = potion_json.id;

        stream.extend(quote! {
            fn #potion_effects_init_ident () -> Vec<MobEffectInstance> {
                vec! [ #( #mob_effect_instance_inits )* ]
            }

            pub static #potion_ident: Potion = Potion {
                id: #potion_id,
                key: Identifier::vanilla_static(#potion_name),
                effects: LazyEffects::new(#potion_effects_init_ident)
            };
        });

        register_stream.extend(quote! {
            registry.register(&#potion_ident);
        });
    }

    stream.extend(quote! {
        pub fn register_potions(registry: &mut PotionRegistry) {
            #register_stream
        }
    });

    stream
}
