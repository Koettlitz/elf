use bevy_elf_macros::FromDef;

#[derive(FromDef)]
#[elf()]
#[allow(unused)]
struct Foo {
    x: bool,
}

fn main() {}
