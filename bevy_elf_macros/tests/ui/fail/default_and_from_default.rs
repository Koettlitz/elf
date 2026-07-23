use bevy_elf_macros::FromDef;

#[derive(FromDef)]
#[allow(unused)]
struct Foo {
    #[elf(default, from_default)]
    x: bool,
}

fn main() {}
