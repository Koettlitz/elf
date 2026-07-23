# bevy_elf

[![Crates.io](https://img.shields.io/crates/v/bevy_elf.svg)](https://crates.io/crates/bevy_elf)
[![docs.rs](https://img.shields.io/docsrs/bevy_elf)](https://docs.rs/bevy_elf)
[![License](https://img.shields.io/badge/license-MIT%2FApache--2.0-blue.svg)](#license)

Load and resolve assets that reference other assets by name, in [Bevy](https://bevyengine.org/).

Hand-written asset types use `Handle<T>`, which isn't serializable. `bevy_elf` generates a
serializable "Def" counterpart of your asset struct — using plain strings in place of
`Handle`s — and a `FromDef` implementation that converts the Def back into your runtime type,
resolving each string into a `Handle` by using bevy's `LoadContext::load()` along the way.

## Example

```ron
// water_animation.ron
(
    frames: [1, 2, 3],
    frame_duration: (secs: 0, nanos: 128000000),
    spritesheet: "water",
)
```

```rust
use bevy_asset::prelude::*;
use bevy_elf::{asset_spec, FromDef};
use bevy_image::{Image, TextureAtlasLayout};
use bevy_reflect::TypePath;
use std::time::Duration;

#[derive(FromDef, Asset, TypePath)]
struct AnimationAsset {
    frames: Vec<usize>,
    frame_duration: Duration,
    spritesheet: Handle<Spritesheet>,
}

#[derive(FromDef, Asset, TypePath)]
#[asset_spec(base_path = "spritesheets", extension = "ron")]
struct Spritesheet {
    #[elf(with_spec(base_path = "spritesheets/images", extension = "png"))]
    image: Handle<Image>,

    #[elf(with_spec(base_path = "spritesheets/layouts", extension = "ron"))]
    layout: Handle<TextureAtlasLayout>,
}

fn main() {}
```

The derive macro generates the `Def` struct, its `Deserialize` impl, and the resolution logic
that turns `"water"` into `Handle<Spritesheet>` by loading `spritesheets/water.ron`. Register
the loader with the `RonAssetPlugin`:

```rust
use bevy_elf::RonAssetPlugin;

app.add_plugins((
    RonAssetPlugin::<AnimationAsset>::default(),
    RonAssetPlugin::<Spritesheet>::default(),
));
```

## Why

Bevy assets that reference other assets naturally want to hold a `Handle<T>`, but `Handle`
isn't something you can put in a `.ron`/`.toml`/`.json` file — there's nothing to point at
until the asset is loaded. The workaround is writing two versions of every asset type by
hand: a serializable "def" version with string IDs, and a runtime version with `Handle`s, plus
the boilerplate to convert between them. `bevy_elf` generates that boilerplate for you, so you
have one type with annotations as the single source of truth.

## Features

| Feature   | Default | Adds                                                                          |
|-----------|:-------:|-------------------------------------------------------------------------------|
| `macros`  |    ✅   | The `FromDef` derive macro and `asset_spec` attribute (via `bevy_elf_macros`) |
| `app`     |    ✅   | Includes the `RonAssetPlugin` and pulls `bevy_app` as a dependency            |
| `math`    |    ✅   | `FromDef` impls for `bevy_math` types (`Vec2`, `Vec3`, `Quat`, `Rect`, ...)   |
| `image`   |    ❌   | `FromDef` impl for `bevy_image::TextureAtlasLayout`                           |

Without `macros`, you can still implement `FromDef`/`FromDefWithResolver` by hand for full
control over the conversion.

## More examples

Implicit fields, omitting empty def files, and resolving foreign types (types you don't own,
like `Handle<Image>`) are covered in the [crate documentation](https://docs.rs/bevy_elf).

## Bevy compatibility

| bevy   | bevy_elf |
|--------|----------|
| 0.19   | 0.1      |

## License

Dual-licensed under either [MIT](https://github.com/Koettlitz/elf/blob/master/LICENSE-MIT) or
[Apache License, Version 2.0](https://github.com/Koettlitz/elf/blob/master/LICENSE-APACHE) at
your option.
