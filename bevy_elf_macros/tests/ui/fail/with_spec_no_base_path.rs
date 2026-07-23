use bevy_elf_macros::FromDef;
use bevy_image::TextureAtlasLayout;

#[derive(FromDef)]
#[allow(unused)]
struct Foo {
    #[elf(with_spec(extension = "ron"))]
    x: Handle<TextureAtlasLayout>,
}

fn main() {}
