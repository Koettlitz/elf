use bevy_elf_macros::FromDef;

#[derive(FromDef)]
#[allow(unused)]
struct Foo {
    #[elf()]
    x: bool,
}

fn main() {}
