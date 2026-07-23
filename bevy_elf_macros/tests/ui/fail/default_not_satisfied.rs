use bevy_elf_macros::FromDef;

#[derive(FromDef)]
#[allow(unused)]
struct Foo {
    #[elf(default)]
    x: NonDefault,
}

#[derive(FromDef)]
#[allow(unused)]
struct NonDefault {
    x: u8,
}

fn main() {}
