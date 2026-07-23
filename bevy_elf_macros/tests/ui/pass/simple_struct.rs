use bevy_elf_macros::FromDef;

#[derive(FromDef)]
#[allow(unused)]
struct Foo {
    a: usize,
    b: String,
}

fn main() {}
