use std::collections::HashMap;

use bevy_asset::{Asset, AssetPath, Handle};
use bevy_elf::{AssetResolver, ResolveError};
use bevy_elf_macros::FromDef;
use bevy_reflect::TypePath;
use serde::{self, Deserialize, Serialize};

#[derive(FromDef, Asset, TypePath)]
#[allow(unused)]
struct Foo {
    #[elf(default)]
    a: usize,
    #[elf(with_spec(base_path = "base/path"))]
    b: Handle<MyAsset>,
    #[elf(from_default)]
    c: Vec<Option<String>>,
}

#[derive(FromDef, Asset, TypePath)]
#[allow(unused)]
struct MyAsset {
    a: f32,
    #[elf(with_spec(sub_path = "b"))]
    b: HashMap<String, Handle<AnotherAsset>>,
    #[elf(implicit, with_resolver(Resolver), expose_resolver)]
    c: Handle<AnotherAsset>,
}

#[derive(FromDef, Asset, TypePath)]
#[elf(on_def(
    #[derive(Debug, Serialize, Deserialize)]
))]
#[allow(unused)]
struct AnotherAsset {
    #[elf(on_def(
        #[serde(default, skip_serializing_if = "Option::is_none")]
    ))]
    a: Option<String>,
    b: i16,
}

#[derive(Debug)]
struct Resolver;
impl AssetResolver for Resolver {
    fn resolve(&self, _: &str) -> Result<AssetPath<'static>, ResolveError> {
        Ok(AssetPath::parse("foo"))
    }
}

fn main() -> Result<(), ResolveError> {
    let another_def = AnotherDef { a: None, b: 4i16 };
    let _ = format!("{another_def:?}");
    let resolver = MyAsset::c_resolver();
    let _ = resolver.resolve("")?;
    Ok(())
}
