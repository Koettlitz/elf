use bevy_elf_macros::FromDef;
use serde::{Deserialize, Serialize};

#[derive(FromDef)]
#[elf(def_type(FooDef))]
#[allow(unused)]
struct Foo {
    foo: usize,
}

#[derive(Serialize, Deserialize)]
struct FooDef {
    bar: usize,
}

fn main() {}
