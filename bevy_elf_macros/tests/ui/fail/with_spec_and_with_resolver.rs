use bevy_asset::AssetPath;
use bevy_elf::{AssetResolver, ResolveError};
use bevy_elf_macros::FromDef;
use bevy_image::TextureAtlasLayout;

#[derive(FromDef)]
#[allow(unused)]
struct Foo {
    #[elf(with_spec(base_path = "base_path"), with_resolver(Resolver))]
    x: Handle<TextureAtlasLayout>,
}

struct Resolver;
impl AssetResolver for Resolver {
    fn resolve(&self, _: &str) -> Result<AssetPath<'static>, ResolveError> {
        unimplemented!()
    }
}

fn main() {}
