use bevy_asset::Handle;
use bevy_elf_macros::FromDef;
use bevy_image::TextureAtlasLayout;

#[derive(FromDef)]
#[allow(unused)]
struct Foo {
    #[elf(implicit, from_default)]
    x: Handle<TextureAtlasLayout>,
}

fn main() {}
