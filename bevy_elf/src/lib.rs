//! This crate loads and resolves assets that reference other assets by name.
//! It builds on [`serde`] and integrates into `bevy`'s asset ecosystem.
//!
//! Hand-written asset types use [`Handle`]s, which aren't serializable. This
//! crate generates a serializable "Def" counterpart for each type, using plain
//! strings in place of [`Handle`]s, along with a [`trait@FromDef`] impl that
//! converts a Def back into its runtime type — resolving each string into a
//! [`Handle`] along the way.
//!
//! ## Basic usage
//! Let's say you have an animation asset `water_animation.ron`, which references its spritesheet
//! by name:
//! ```ron
//! (
//!     frames: [1, 2, 3,],
//!     frame_duration: (
//!         secs: 0,
//!         nanos: 128000000,
//!     ),
//!     spritesheet: "water",
//! )
//! ```
//! The corresponding struct would look something like this:
//! ```ignore
//! # use bevy_asset::prelude::*;
//! # use std::time::Duration;
//! # use bevy_elf::FromDef;
//! #[derive(FromDef, Asset, TypePath)]
//! struct AnimationAsset {
//!     frames: Vec<usize>,
//!     frame_duration: Duration,
//!     spritesheet: Handle<Spritesheet>,
//! }
//! # #[derive(Asset, TypePath)]
//! # struct Spritesheet;
//! ```
//! The [`derive@FromDef`] derive macro generates a (de)serializable version of the struct as well as an
//! implementation of the [`trait@FromDef`] trait, which converts it into your struct. The generated
//! struct looks something like this:
//! ```ignore
//! # use std::time::Duration;
//! # use serde::{Serialize, Deserialize};
//! #[derive(Serialize, Deserialize)]
//! struct AnimationDef {
//!     frames: Vec<usize>,
//!     frame_duration: Duration,
//!     spritesheet: String,
//! }
//! ```
//! Assets, that implement [`trait@FromDef`] can be loaded with the [`RonAssetLoader`], which calls
//! [`FromDef::from_def()`] to convert the raw deserialized structure into the runtime
//! structure. You can register the asset and the [`RonAssetLoader`] manually or just add the
//! [`RonAssetPlugin`]:
//! ```ignore
//! # use bevy_app::prelude::*;
//! app.add_plugins((
//!     RonAssetPlugin::<AnimationAsset>::default(),
//!     RonAssetPlugin::<Spritesheet>::default(),
//! ));
//! ```
//! To resolve the string names into handles some metadata needs to be provided. Let's
//! take the Spritesheet asset as an example:
//! ```ignore
//! # use bevy_asset::prelude::*;
//! # use bevy_elf::{FromDef, asset_spec};
//! #[derive(FromDef, Asset, TypePath)]
//! #[asset_spec(base_path = "spritesheets", extension = "ron")]
//! struct Spritesheet { /* fields omitted */ }
//! ```
//! With the [`asset_spec`] provided the spritesheet handles inside the `AnimationAsset` can now be
//! resolved, e.g. the "water" spritesheet gets resolved into "spritesheets/water.ron".
//!
//! ## Resolving foreign types with `elf`
//! Asset types you don't own cannot be annotated with attributes. Let's take a closer look at the
//! Spritesheet asset:
//! ```ignore
//! # use bevy_asset::prelude::*;
//! # use bevy_elf::{FromDef, asset_spec};
//! #[derive(FromDef, Asset, TypePath)]
//! #[asset_spec(base_path = "spritesheets", extension = "ron")]
//! struct Spritesheet {
//!     #[elf(with_spec(base_path = "spritesheets/images", extension = "png"))]
//!     image: Handle<Image>,
//!
//!     #[elf(with_spec(base_path = "spritesheets/layouts", extension = "ron"))]
//!     layout: Handle<TextureAtlasLayout>,
//! }
//! ```
//! Since `Image` and `TextureAtlasLayout` are not defined by you they cannot be resolved
//! the same way, because they don't have an [`asset_spec`].
//! For these you can use the `elf` field attribute to tell `bevy_elf` how to resolve them as shown above.
//!
//! ## Implicit fields
//! It is also possible to make asset references implicit by their name.
//! Imagine your assets are organized in the file system like this:
//! ```text
//! assets/
//! ├── animations/
//! │   ├── water.ron
//! │   └── grass.ron
//! └── spritesheets/
//!     ├── water.ron
//!     ├── grass.ron
//!     ├── images/
//!     │   ├── water.png
//!     │   └── grass.png
//!     └── layouts/
//!         ├── water.ron
//!         └── grass.ron
//! ```
//! Explicitly mentioning e.g. "water" everywhere is cumbersome. Make it implicit instead. The
//! `implicit` flag goes along well with `sub_path`:
//! ```ignore
//! # use bevy_asset::prelude::*;
//! # use bevy_elf::{FromDef, asset_spec};
//! #[derive(FromDef, Asset, TypePath)]
//! #[asset_spec(base_path = "spritesheets", extension = "ron")]
//! struct Spritesheet {
//!     #[elf(implicit, with_spec(sub_path = "images", extension = "png"))]
//!     image: Handle<Image>,
//!
//!     #[elf(implicit, with_spec(sub_path = "layouts", extension = "ron"))]
//!     layout: Handle<TextureAtlasLayout>,
//! }
//! ```
//! That way the `image` and `layout` fields are omitted in the generated def type and don't show up
//! in the ron file at all. They are resolved with the same string name as their parent.
//!
//! ## Omitting empty def files
//! With the fields being implicit the spritesheet ron files are now empty. Having to put an empty
//! file there for it all to work isn't nice at all! To omit the whole file tell `bevy_elf` to omit
//! the def type entirely and use `()` instead. Just tell the referencing `AnimationAsset` to not
//! load any file but put a default value into [`FromDef::from_def()`] instead:
//! ```ignore
//! # use bevy_asset::prelude::*;
//! # use bevy_app::prelude::*;
//! # use std::time::Duration;
//! # use bevy_elf::{FromDef, asset_spec};
//! #[derive(FromDef, Asset, TypePath)]
//! struct AnimationAsset {
//!     frames: Vec<usize>,
//!     frame_duration: Duration,
//!
//!     #[elf(from_default)]
//!     spritesheet: Handle<Spritesheet>,
//! }
//!
//! #[derive(FromDef)]
//! #[def_type(())]
//! struct Spritesheet {
//!     #[elf(implicit, with_spec(base_path = "images", extension = "png"))]
//!     image: Handle<Image>,
//!
//!     #[elf(implicit, with_spec(base_path = "layouts", extension = "ron"))]
//!     layout: Handle<TextureAtlasLayout>,
//! }
//! ```
//! That way your directory structure, as well as the ron files themselves become leaner and
//! the spritesheet doesn't appear in the file system at all anymore.
//! But the image and its layout are still neatly stored in their own `Spritesheet` struct.
//! ```text
//! assets/
//! ├── animations/
//! │   ├── water.ron
//! │   └── grass.ron
//! ├── images/
//! │   ├── water.png
//! │   └── grass.png
//! └── layouts/
//!     ├── water.ron
//!     └── grass.ron
//! ```
//! Note that since the spritesheets directory doesn't exist anymore, `images/` and `layouts/`
//! move up to become top-level asset folders, so we changed from `sub_path` back to `base_path`.
//! Also note, that Spritesheet is no `Asset` anymore, since it doesn't get loaded from a file,
//! so the registration via `app.add_plugins(RonAssetPlugin::<Spritesheet>::default());` disappears as well.
//!
//! For more attributes and options see [`derive@FromDef`].

#[cfg(feature = "macros")]
pub use bevy_elf_macros::{FromDef, asset_spec};

use std::borrow::Cow;
use std::collections::HashMap;
use std::fmt::Debug;
use std::hash::Hash;
use std::io;
use std::marker::PhantomData;

use bevy_app::prelude::*;

use bevy_asset::io::Reader;
use bevy_asset::prelude::*;
use bevy_asset::{AssetLoader, AssetPath, LoadContext, ParseAssetPathError};
use bevy_reflect::TypePath;
use ron::de::SpannedError;
use serde::de::DeserializeOwned;
use thiserror::Error;

mod from_def_impls;

type Phantom<L> = PhantomData<fn() -> L>;

/// Resolves the [`AssetPath`] for a given string id.
pub trait AssetResolver {
    fn resolve(&self, asset_id: &str) -> Result<AssetPath<'static>, ResolveError>;
}

/// Resolves the [`AssetPath`] for a given string asset id from a static context, meaning to self
/// object is nessecary.
pub trait StaticAssetResolver {
    fn resolve(asset_id: &str) -> Result<AssetPath<'static>, ResolveError>;
}

/// An asset resolver, that assumes the given string id is the complete asset path and returns it
/// as an [`AssetPath`](`bevy_asset::AssetPath`) unchanged.
pub struct PathResolver;
impl AssetResolver for PathResolver {
    fn resolve(&self, asset_path: &str) -> Result<AssetPath<'static>, ResolveError> {
        Ok(AssetPath::from(asset_path.to_string()))
    }
}

/// Extracts the string id of the asset from its [`AssetPath`].
pub fn extract_id_from(asset_path: AssetPath) -> Result<String, ResolveError> {
    asset_path
        .path()
        .file_prefix()
        .ok_or_else(|| ResolveError::MissingFileName(asset_path.clone_owned()))
        .map(|v| v.to_string_lossy().to_string())
}

#[derive(Error, Debug)]
pub enum FromDefError {
    #[error("{0}")]
    Resolve(#[from] ResolveError),

    #[error("{0}")]
    InvalidDef(String),
}

impl From<ParseAssetPathError> for FromDefError {
    fn from(value: ParseAssetPathError) -> Self {
        Self::from(ResolveError::from(value))
    }
}

#[derive(Error, Debug)]
pub enum ResolveError {
    #[error("{0}")]
    Parse(#[from] ParseAssetPathError),

    #[error("missing file name in asset path {0}")]
    MissingFileName(AssetPath<'static>),

    #[error("invalid asset link \"{0}\"")]
    InvalidAssetLink(String),
}

/// An adapter type, that implements [`AssetResolver`] by delegating to a
/// [`StaticAssetResolver`] (`S`)
pub struct StaticResolverAdapter<S>(Phantom<S>);

impl<S> Default for StaticResolverAdapter<S> {
    fn default() -> Self {
        Self(PhantomData)
    }
}

impl<S: StaticAssetResolver> AssetResolver for StaticResolverAdapter<S> {
    fn resolve(&self, asset_id: &str) -> Result<AssetPath<'static>, ResolveError> {
        S::resolve(asset_id)
    }
}

pub trait HasResolver {
    type Resolver: AssetResolver;

    fn resolver() -> Self::Resolver;
}

impl<T: AssetResolver + Default> HasResolver for T {
    type Resolver = Self;

    fn resolver() -> Self::Resolver {
        Self::default()
    }
}

pub trait AssetPathSpec {
    const BASE_PATH: &'static str;
    const EXTENSION: Option<&'static str> = None;
}

/// An adapter type, that implements [`AssetResolver`] by using an [`AssetPathSpec`] (`S`).
pub struct ResolverSpec<S>(Phantom<S>);

impl<S> Default for ResolverSpec<S> {
    fn default() -> Self {
        Self(PhantomData)
    }
}

impl<S: AssetPathSpec> AssetPathSpecProvider for ResolverSpec<S> {
    fn base_path(&self) -> Cow<'static, str> {
        Cow::Borrowed(S::BASE_PATH)
    }

    fn extension(&self) -> Option<&'static str> {
        S::EXTENSION
    }
}

pub struct DynamicPathResolver {
    pub base_path: String,
    pub extension: Option<&'static str>,
}

impl DynamicPathResolver {
    pub fn resolve_sub_path(
        load_context: &mut LoadContext,
        sub_path: &str,
        extension: Option<&'static str>,
    ) -> Result<Self, ParseAssetPathError> {
        let sub_path = AssetPath::parse(sub_path);
        let base_path = load_context
            .path()
            .parent()
            .map(|p| p.resolve(&sub_path))
            .unwrap_or_else(|| sub_path)
            .to_string();
        Ok(Self {
            base_path,
            extension,
        })
    }
}

impl AssetPathSpecProvider for DynamicPathResolver {
    fn base_path(&self) -> Cow<'static, str> {
        Cow::Owned(self.base_path.clone())
    }

    fn extension(&self) -> Option<&'static str> {
        self.extension
    }
}

pub struct SpecResolver<S>(Phantom<S>);

impl<S> Default for SpecResolver<S> {
    fn default() -> Self {
        Self(PhantomData)
    }
}

impl<S: HasSpecProvider> AssetPathSpecProvider for SpecResolver<S> {
    fn base_path(&self) -> Cow<'static, str> {
        S::provider().base_path()
    }

    fn extension(&self) -> Option<&'static str> {
        S::provider().extension()
    }
}

/// An [`AssetResolver`] using a `base_path` and an optional file `extension`.
pub trait AssetPathSpecProvider {
    fn base_path(&self) -> Cow<'static, str>;
    fn extension(&self) -> Option<&'static str> {
        None
    }
}

pub trait HasSpecProvider {
    type Provider: AssetPathSpecProvider;

    fn provider() -> Self::Provider;
}

impl<T> HasSpecProvider for T
where
    T: AssetPathSpecProvider + Default,
{
    type Provider = Self;

    fn provider() -> Self::Provider {
        Self::default()
    }
}

impl<T> AssetResolver for T
where
    T: AssetPathSpecProvider,
{
    fn resolve(&self, asset_id: &str) -> Result<AssetPath<'static>, ResolveError> {
        let file_name = if let Some(ext) = self.extension() {
            Cow::Owned(asset_id.to_string() + "." + ext)
        } else {
            Cow::Borrowed(asset_id)
        };
        Ok(AssetPath::from(self.base_path().to_string())
            .resolve(&AssetPath::parse(file_name.as_ref())))
    }
}

/// Represents a type, that can be constructed from a deserializable def type.
/// An asset type implementing [`trait@FromDef`] can be loaded by the [`RonAssetLoader`].
/// It deserializes the asset bytes into the [`FromDef::Def`] type and then turns it into
/// the runtime asset type which implements [`trait@FromDef`] by passing it to [`FromDef::from_def()`].
/// This trait can be implemented manually or by using the derive macro [`derive@FromDef`].
/// To enable loading ron assets implementing [`trait@FromDef`] just add the [`RonAssetPlugin`].
pub trait FromDef {
    type Def: DeserializeOwned;

    fn from_def(def: Self::Def, ctx: &mut LoadContext) -> Result<Self, FromDefError>
    where
        Self: Sized;
}

/// Like [`trait@FromDef`], but uses an explicitly passed [`AssetResolver`] to resolve its [`AssetPath`].
pub trait FromDefWithResolver {
    type Def: DeserializeOwned;
    type Error;

    fn from_def_with_resolver<R: AssetResolver>(
        def: Self::Def,
        resolver: &R,
        ctx: &mut LoadContext,
    ) -> Result<Self, Self::Error>
    where
        Self: Sized;
}

/// A [`Handle`] with the assets string id preserved.
#[derive(Debug, Eq)]
pub struct AssetRef<A: Asset> {
    id: String,
    handle: Handle<A>,
}

impl<A: Asset> AssetRef<A> {
    pub fn new(id: String, handle: Handle<A>) -> Self {
        Self { id, handle }
    }

    pub fn id(&self) -> &str {
        &self.id
    }

    pub fn handle(&self) -> &Handle<A> {
        &self.handle
    }
}

impl<A: Asset + HasResolver> FromDef for AssetRef<A> {
    type Def = String;

    fn from_def(def: Self::Def, ctx: &mut LoadContext) -> Result<Self, FromDefError>
    where
        Self: Sized,
    {
        let handle = ctx.load(A::resolver().resolve(&def)?);
        Ok(Self { id: def, handle })
    }
}

impl<A: Asset> FromDefWithResolver for AssetRef<A> {
    type Def = String;
    type Error = ResolveError;

    fn from_def_with_resolver<R: AssetResolver>(
        def: Self::Def,
        resolver: &R,
        ctx: &mut LoadContext,
    ) -> Result<Self, Self::Error> {
        Ok(Self {
            handle: ctx.load(resolver.resolve(&def)?),
            id: def,
        })
    }
}

impl<A: Asset + HasResolver> FromDef for Handle<A> {
    type Def = String;

    fn from_def(def: Self::Def, ctx: &mut LoadContext) -> Result<Self, FromDefError> {
        Ok(ctx.load(A::resolver().resolve(&def)?))
    }
}

impl<A: Asset> FromDefWithResolver for Handle<A> {
    type Def = String;
    type Error = FromDefError;

    fn from_def_with_resolver<R: AssetResolver>(
        def: Self::Def,
        resolver: &R,
        ctx: &mut LoadContext,
    ) -> Result<Self, Self::Error> {
        Ok(ctx.load(resolver.resolve(&def)?))
    }
}

impl<T, D> FromDef for Option<T>
where
    T: FromDef<Def = D>,
    D: DeserializeOwned,
{
    type Def = Option<D>;

    fn from_def(def: Self::Def, ctx: &mut LoadContext) -> Result<Self, FromDefError> {
        def.map(|d| T::from_def(d, ctx)).transpose()
    }
}

impl<T, D> FromDefWithResolver for Option<T>
where
    T: FromDefWithResolver<Def = D>,
    D: DeserializeOwned,
{
    type Def = Option<D>;
    type Error = T::Error;

    fn from_def_with_resolver<R: AssetResolver>(
        def: Self::Def,
        resolver: &R,
        ctx: &mut LoadContext,
    ) -> Result<Self, Self::Error> {
        def.map(|d| T::from_def_with_resolver(d, resolver, ctx))
            .transpose()
    }
}

impl<T, D> FromDef for Vec<T>
where
    T: FromDef<Def = D>,
    D: DeserializeOwned,
{
    type Def = Vec<D>;

    fn from_def(def: Self::Def, ctx: &mut LoadContext) -> Result<Self, FromDefError> {
        def.into_iter().map(|d| T::from_def(d, ctx)).collect()
    }
}

impl<T, D> FromDefWithResolver for Vec<T>
where
    T: FromDefWithResolver<Def = D>,
    D: DeserializeOwned,
{
    type Def = Vec<D>;
    type Error = T::Error;

    fn from_def_with_resolver<R: AssetResolver>(
        def: Self::Def,
        resolver: &R,
        ctx: &mut LoadContext,
    ) -> Result<Self, Self::Error> {
        def.into_iter()
            .map(|d| T::from_def_with_resolver(d, resolver, ctx))
            .collect()
    }
}

impl<A, K, D> FromDef for HashMap<K, A>
where
    A: FromDef<Def = D>,
    K: DeserializeOwned + Eq + Hash,
    D: DeserializeOwned,
{
    type Def = HashMap<K, D>;

    fn from_def(def: Self::Def, ctx: &mut LoadContext) -> Result<Self, FromDefError> {
        def.into_iter()
            .map(|(k, d)| Ok((k, A::from_def(d, ctx)?)))
            .collect()
    }
}

impl<A, K, D> FromDefWithResolver for HashMap<K, A>
where
    A: FromDefWithResolver<Def = D>,
    K: DeserializeOwned + Eq + Hash,
    D: DeserializeOwned,
{
    type Def = HashMap<K, D>;
    type Error = A::Error;

    fn from_def_with_resolver<R: AssetResolver>(
        def: Self::Def,
        resolver: &R,
        ctx: &mut LoadContext,
    ) -> Result<Self, Self::Error> {
        def.into_iter()
            .map(|(k, d)| Ok((k, A::from_def_with_resolver(d, resolver, ctx)?)))
            .collect()
    }
}

/// Registers the asset type `A` and a [`RonAssetLoader<A>`]
/// This is equivalent to calling
/// `app.init_asset::<A>().init_asset_loader::<RonAssetLoader<A>>();`
/// Note that the asset type `A` has to implement [`Asset`] and [`FromDef`].
pub struct RonAssetPlugin<A>(Phantom<A>);
impl<A> Default for RonAssetPlugin<A> {
    fn default() -> Self {
        Self(PhantomData)
    }
}

impl<A> Plugin for RonAssetPlugin<A>
where
    A: Asset + FromDef + 'static,
{
    fn build(&self, app: &mut App) {
        app.init_asset::<A>()
            .init_asset_loader::<RonAssetLoader<A>>();
    }
}

/// Loads assets, which implement [`trait@FromDef`] from ron files passing the deserialized
/// [`FromDef::Def`] value into the assets [`FromDef::from_def`] method.
#[derive(TypePath)]
pub struct RonAssetLoader<A>(Phantom<A>);
impl<A> AssetLoader for RonAssetLoader<A>
where
    A: FromDef + Asset,
{
    type Asset = A;
    type Error = RonAssetLoadError;
    type Settings = ();

    async fn load(
        &self,
        reader: &mut dyn Reader,
        _: &Self::Settings,
        load_context: &mut LoadContext<'_>,
    ) -> Result<Self::Asset, Self::Error> {
        let mut bytes = Vec::new();
        reader.read_to_end(&mut bytes).await?;
        let def: A::Def = ron::de::from_bytes(&bytes)?;
        Ok(A::from_def(def, load_context)?)
    }
}

impl<A> Default for RonAssetLoader<A> {
    fn default() -> Self {
        Self(PhantomData)
    }
}

#[derive(Error, Debug)]
pub enum RonAssetLoadError {
    #[error("{0}")]
    Io(#[from] io::Error),
    #[error("{0}")]
    Spanned(#[from] SpannedError),
    #[error("{0}")]
    FromDef(#[from] FromDefError),
}

impl<A: Asset> PartialEq for AssetRef<A> {
    fn eq(&self, other: &Self) -> bool {
        self.id == other.id && self.handle.id() == other.handle.id()
    }
}

impl<A: Asset> PartialEq<Handle<A>> for AssetRef<A> {
    fn eq(&self, other: &Handle<A>) -> bool {
        self.handle.id() == other.id()
    }
}

impl<A: Asset> Clone for AssetRef<A> {
    fn clone(&self) -> Self {
        Self {
            id: self.id.clone(),
            handle: self.handle.clone(),
        }
    }
}
