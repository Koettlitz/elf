use bevy_elf_macros::FromDef;

#[derive(FromDef)]
#[allow(unused)]
struct Foo {
    #[elf(implicit)]
    x: bool,
}

fn main() {}
