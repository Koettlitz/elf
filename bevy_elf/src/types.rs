#[cfg(feature = "math")]
mod math {
    use bevy_elf_macros::from_def_self;
    use bevy_math::{IRect, IVec2, IVec3, URect, UVec2, UVec3, Vec2, Vec3};
    from_def_self![IRect, URect, IVec2, IVec3, UVec2, UVec3, Vec2, Vec3];
}

#[cfg(feature = "image")]
mod image {
    use bevy_elf_macros::from_def_self;
    use bevy_image::TextureAtlasLayout;
    from_def_self![TextureAtlasLayout];
}
