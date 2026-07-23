use bevy_elf_macros::FromDef;

#[derive(FromDef)]
#[allow(unused)]
struct Foo {
    #[elf(from_default, with_resolver(Bar))]
    x: bool,
}

struct Bar;

fn main() {}
