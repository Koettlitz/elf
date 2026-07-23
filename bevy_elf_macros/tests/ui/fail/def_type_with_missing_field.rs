use bevy_elf_macros::FromDef;
use serde::{Deserialize, Serialize};

#[derive(FromDef)]
#[elf(def_type(FooDef))]
#[allow(unused)]
struct Foo {
    a: usize,
    b: String,
}

#[derive(Serialize, Deserialize)]
struct FooDef {
    a: usize,
}

fn main() {}
