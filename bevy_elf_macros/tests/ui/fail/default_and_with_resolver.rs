use bevy_elf_macros::FromDef;

#[derive(FromDef)]
#[allow(unused)]
struct Foo {
    #[elf(default, with_resolver(Bar))]
    x: bool,
}

struct Bar;

fn main() {}
