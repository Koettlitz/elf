# elf

Workspace for [`bevy_elf`](bevy_elf), a crate for loading and resolving [Bevy](https://bevyengine.org/)
assets that reference other assets by name.

## Crates

| Crate                                  | Description                                                        |
|-----------------------------------------|----------------------------------------------------------------------|
| [`bevy_elf`](bevy_elf)                   | The public crate. See its [README](bevy_elf/README.md) for usage.   |
| [`bevy_elf_macros`](bevy_elf_macros)     | Implements the `FromDef` derive macro and `asset_spec` attribute. Not meant to be depended on directly — re-exported through `bevy_elf`'s `macros` feature. |

## License

Dual-licensed under either [MIT](LICENSE-MIT) or [Apache License, Version 2.0](LICENSE-APACHE)
at your option.
