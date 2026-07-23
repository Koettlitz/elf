use bevy_elf_macros::FromDef;

#[derive(FromDef)]
#[elf(def_type(Bar))]
#[allow(unused)]
struct Foo;

fn main() {}
