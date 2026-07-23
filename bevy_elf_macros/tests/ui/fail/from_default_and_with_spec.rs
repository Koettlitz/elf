use bevy_elf_macros::FromDef;

#[derive(FromDef)]
#[allow(unused)]
struct Foo {
    #[elf(from_default, with_spec(base_path = "foo"))]
    x: bool,
}

fn main() {}
