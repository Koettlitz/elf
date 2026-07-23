use std::collections::HashMap;

use bevy_asset::{Asset, Handle};
use bevy_elf_macros::FromDef;
use bevy_reflect::TypePath;
use serde;

#[derive(FromDef, Asset, TypePath)]
#[allow(unused)]
struct Foo {
    a: usize,
    #[elf(with_spec(base_path = "base/path"))]
    b: Handle<MyAsset>,
    c: Vec<Option<String>>,
}

#[derive(FromDef, Asset, TypePath)]
#[allow(unused)]
struct MyAsset {
    a: f32,
    #[elf(with_spec(sub_path = "b"))]
    b: HashMap<String, Handle<AnotherAsset>>,
}

#[derive(FromDef, Asset, TypePath)]
#[allow(unused)]
struct AnotherAsset {
    #[elf(on_def(
        #[serde(default, skip_serializing_if = "Option::is_none")]
    ))]
    a: Option<String>,
    b: i16,
}

fn main() {}
