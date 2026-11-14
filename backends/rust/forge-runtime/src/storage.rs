// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael

//! Storage policy for the owned mirror of a codec (`{Codec}Owned`).
//!
//! A decoded wire value has one schema but more than one legitimate resting
//! place. An application-processor path wants growable containers and no
//! ceiling; a real-time or MCU path wants every byte inline with a hard bound
//! and no allocator. The generated owned type is therefore parameterised by a
//! storage profile — `{Codec}Owned<Heap>` and `{Codec}Owned<Inline>` are the
//! same generated type at two profiles, and both exist in the same binary.
//!
//! # What a profile answers
//!
//! A profile says what it holds a field in, field by field: each variable
//! field of a codec has a generated tag type, and a profile answers
//! [`HoldsList`], [`HoldsStr`] or [`HoldsBytes`] for it. A capacity is therefore
//! a fact about a profile and not a number in the generated tree, so one tree
//! serves every deployment in a build (docs/adr/0077 §7). [`Heap`] answers for
//! every tag. [`Inline`] answers in the generated tree, from the bound the
//! document states. A deployment is a type of its own that answers the same
//! way, from a file of its own.
//!
//! Each codec has a `{Codec}Storage` trait, whose supertraits are what it and
//! the codecs it holds ask of a profile. A generated type names that one
//! trait: a struct's `where` clause is not implied where the struct is used,
//! and a supertrait is.
//!
//! # Why a policy parameter rather than a second generated type
//!
//! The alternative — emitting a separate `{Codec}Inline` beside
//! `{Codec}Owned` — doubles the generated surface, doubles the conversion
//! matrix, and lets the two mirrors of one wire type drift apart. The policy
//! keeps one type, one `try_into_owned`, one `as_borrowed`, one test vector
//! set, and makes the storage choice a caller's decision at the use site
//! rather than the code generator's decision at build-configuration time.
//!
//! The profile is a *type*, so:
//!
//! - a heap-capable program can still pin a value to non-allocating storage
//!   (`{Codec}Owned<Inline>`), which a `cfg`-switched container cannot express
//!   — under `alloc` the switched form is growable for everyone;
//! - the owned mirror is no longer gated behind `#[cfg(feature = "alloc")]`,
//!   so an MCU build gets owned values for list- and embed-bearing codecs
//!   instead of only for flat scalar ones;
//! - moving a value between profiles is a checked projection
//!   (`{Codec}Owned::transcode_in`), not a re-decode.
//!
//! Naming follows the standard library's allocator parameterisation
//! (`Box::new_in`, `Vec::with_capacity_in`): the plain method uses
//! [`DefaultStorage`], the `_in` suffixed method takes the profile explicitly.
//!
//! A capacity is not a parameter of these traits: a tag cannot give a `const N`
//! without `generic_const_exprs`. A container carries its capacity in its own
//! type, and the impl for a tag names that type.
//!
//! # Container contract
//!
//! [`SceList`], [`SceStr`] and [`SceByteBuf`] are the three container shapes a
//! codec field can need. They are deliberately minimal — construct, append or
//! copy-in, and read back as a borrowed view — because that is the whole of
//! what the generated owned↔borrowed projections do. Every method is fallible
//! so one call shape serves both profiles; on the growable profile the error
//! is unreachable rather than absent, which keeps the generated code free of
//! per-profile branches.

use crate::codec::CodecError;

#[cfg(feature = "alloc")]
pub use crate::capped::{CappedBytes, CappedStr, CappedVec};
#[cfg(feature = "alloc")]
pub use sce_portable_bytes::HeapBytes;
pub use sce_portable_bytes::{InlineBytes, SceBytes};
// The language's list, hard-bounded on every profile: what a CBOR map holds a
// list of records in (docs/adr/0086, section 7.4), and no codec container.
pub use sce_portable_bytes::BoundedList;

/// A bounded or growable sequence of `T`, as a codec's owned list field.
///
/// `N` lives on the *type* (`heapless::Vec<T, N>`) rather than on the trait,
/// so the growable implementation is free of a capacity it does not enforce.
pub trait SceList<T>: Sized {
    /// An empty list. The fixed-capacity implementation reserves `N` inline.
    fn empty() -> Self;

    /// Append one element, raising [`CodecError::CapacityExceeded`] when a
    /// fixed-capacity list is full. The growable implementation never fails.
    fn try_push(&mut self, value: T) -> Result<(), CodecError>;

    /// Empty the list, keeping what it has reserved.
    fn clear(&mut self);

    /// Read the elements back as a slice — the input to the owned→borrowed
    /// projection, which re-bounds them into the borrowed view's inline list.
    fn as_slice(&self) -> &[T];

    /// The last element, where a decode builds an entry in place.
    fn last_mut(&mut self) -> Option<&mut T>;

    /// Take out the element at `at`, closing the gap, which a chain's named
    /// entry leaves. `at` is within the list.
    fn remove_at(&mut self, at: usize);

    /// Whether [`Self::try_push`] would refuse the next element: a decode reads
    /// an entry past the capacity and does not hold it (docs/adr/0077 §7). A
    /// growable list is never full.
    fn is_full(&self) -> bool {
        false
    }
}

/// A [`SceList`] that says what it holds, so a bound can reach the entries from
/// the list alone: a chain's membership asks every list a profile can hold
/// through this one trait (docs/adr/0065 §6 R6).
pub trait HeldList: SceList<<Self as HeldList>::Entry> {
    /// What the list holds.
    type Entry;
}

/// Owned text storage for a codec's `string` field.
pub trait SceStr: Sized {
    /// Copy a borrowed decode view into owned storage, raising
    /// [`CodecError::CapacityExceeded`] past a fixed capacity.
    fn from_view(s: &str) -> Result<Self, CodecError>;

    /// Borrow the text back — the owned→borrowed projection's read side.
    fn as_str(&self) -> &str;

    /// Append `s`, raising [`CodecError::CapacityExceeded`] past a fixed
    /// capacity, the text then as it was: what a reader that builds a text as
    /// it unescapes it writes through, with no buffer of its own
    /// (docs/adr/0077 §7).
    fn try_push_str(&mut self, s: &str) -> Result<(), CodecError>;
}

/// Owned byte storage for a codec's `bytes` field.
pub trait SceByteBuf: Sized {
    /// Copy a borrowed decode view into owned storage, raising
    /// [`CodecError::CapacityExceeded`] past a fixed capacity.
    fn from_slice(b: &[u8]) -> Result<Self, CodecError>;

    /// Borrow the bytes back — the owned→borrowed projection's read side.
    fn as_slice(&self) -> &[u8];
}

/// What a `bytes` field is held in and read from: a buffer a profile owns, or
/// the frame's own bytes on [`Lent`], which no slice can be copied into.
pub trait HeldBytes {
    /// The bytes.
    fn held_bytes(&self) -> &[u8];
}

impl<B: SceByteBuf> HeldBytes for B {
    fn held_bytes(&self) -> &[u8] {
        self.as_slice()
    }
}

impl HeldBytes for &[u8] {
    fn held_bytes(&self) -> &[u8] {
        self
    }
}

/// What a `string` field is held in and read from, as [`HeldBytes`] is for a
/// `bytes` field.
pub trait HeldStr {
    /// The text.
    fn held_str(&self) -> &str;
}

impl<T: SceStr> HeldStr for T {
    fn held_str(&self) -> &str {
        self.as_str()
    }
}

impl HeldStr for &str {
    fn held_str(&self) -> &str {
        self
    }
}

/// What a profile holds the `<sce:repeat>` or `<sce:tlv-chain>` field `Tag` in.
///
/// It reads as [`crate::bounded::Entries`], so a list held as a
/// [`crate::bounded::Bounded`] presents its entries with their count, and its
/// default is the fewest entries its bounds admit
/// ([`crate::bounded::Fewest`]). The bounds are what the generated
/// `#[derive(Debug, Clone, PartialEq)]` on a value needs: a derived impl
/// reasons about the field types.
pub trait HoldsList<Tag> {
    /// The list of entries `T`.
    type List<T>: HeldList<Entry = T>
        + SceList<T>
        + crate::bounded::Entries
        + crate::bounded::Fewest
        + Default
        + IntoIterator<Item = T>
        + core::fmt::Debug
        + Clone
        + PartialEq
    where
        T: Default + core::fmt::Debug + Clone + PartialEq;
}

/// What a profile holds the `string` field `Tag` in. The text is a value with
/// a default, the empty text, so a list of texts and a reader that builds one
/// in place can start from it (docs/adr/0077 §7).
pub trait HoldsStr<Tag> {
    /// The text.
    type Str: HeldStr + crate::bounded::Fewest + Default + core::fmt::Debug + Clone + PartialEq;
}

/// What a profile holds the `bytes` field `Tag` in. A profile may hold bytes in
/// a container of its own, a view of the frame it shares, with no empty value
/// to start from; no reader builds bytes in place.
pub trait HoldsBytes<Tag> {
    /// The bytes.
    type Bytes: HeldBytes + crate::bounded::Fewest + core::fmt::Debug + Clone + PartialEq;
}

/// How a profile holds the `bytes` field `Tag` when it reads a frame of the
/// lifetime `'f`. A profile that copies takes a frame of any lifetime, and
/// [`Lent`] one that outlives it.
pub trait StoresBytes<'f, Tag>: HoldsBytes<Tag> {
    /// `view`, as the field holds it.
    fn store_bytes(view: &'f [u8]) -> Result<Self::Bytes, CodecError>;
}

/// How a profile holds the `string` field `Tag` when it reads a frame of the
/// lifetime `'f`, as [`StoresBytes`] is for a `bytes` field.
pub trait StoresStr<'f, Tag>: HoldsStr<Tag> {
    /// `view`, as the field holds it.
    fn store_str(view: &'f str) -> Result<Self::Str, CodecError>;
}

/// A profile that copies a frame's bytes and text into its own containers, so
/// it reads a frame of any lifetime. A deployment's file says this of its type.
pub trait Copying {}

impl<'f, Tag, S> StoresBytes<'f, Tag> for S
where
    S: Copying + HoldsBytes<Tag>,
    S::Bytes: SceByteBuf,
{
    fn store_bytes(view: &'f [u8]) -> Result<Self::Bytes, CodecError> {
        <S::Bytes as SceByteBuf>::from_slice(view)
    }
}

impl<'f, Tag, S> StoresStr<'f, Tag> for S
where
    S: Copying + HoldsStr<Tag>,
    S::Str: SceStr,
{
    fn store_str(view: &'f str) -> Result<Self::Str, CodecError> {
        <S::Str as SceStr>::from_view(view)
    }
}

/// The eager form's profile: bytes and text are the frame's own, and lists are
/// held as the profile `D` holds them.
///
/// A field that borrows stores nothing, so it asks for no capacity (docs/adr/0077
/// §7). What holds a list is the only thing a deployment decides here.
///
/// A value on this profile is invariant in `'a`, unlike a view: its fields are
/// associated types of the profile, and Rust does not see through one. A
/// function that takes it takes it at its own lifetime.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Lent<'a, D = Inline>(core::marker::PhantomData<(&'a (), D)>);

impl<'a, D, Tag> HoldsBytes<Tag> for Lent<'a, D> {
    type Bytes = &'a [u8];
}

impl<'a, D, Tag> HoldsStr<Tag> for Lent<'a, D> {
    type Str = &'a str;
}

impl<'a, D: HoldsList<Tag>, Tag> HoldsList<Tag> for Lent<'a, D> {
    type List<T>
        = D::List<T>
    where
        T: Default + core::fmt::Debug + Clone + PartialEq;
}

impl<'f: 'a, 'a, D, Tag> StoresBytes<'f, Tag> for Lent<'a, D> {
    fn store_bytes(view: &'f [u8]) -> Result<&'a [u8], CodecError> {
        Ok(view)
    }
}

impl<'f: 'a, 'a, D, Tag> StoresStr<'f, Tag> for Lent<'a, D> {
    fn store_str(view: &'f str) -> Result<&'a str, CodecError> {
        Ok(view)
    }
}

/// A storage profile whose byte and text containers are made WITH knowledge of
/// where the decoded bytes came from.
///
/// [`SceByteBuf::from_slice`] receives a borrowed slice and nothing else, so a
/// container it builds can only copy: it cannot tell which buffer the slice is
/// part of, and so cannot hold a reference to that buffer instead. A host that
/// receives a frame into reference-counted storage and wants the decoded
/// message to share the frame — not copy out of it — has no place to say so.
/// This trait is that place. The projection `{Codec}::try_into_owned_in_origin`
/// (generated only on request, `--owned-origin`) hands the profile an
/// [`Self::Origin`] and the slice, and the container the profile holds the
/// field in refers back to the origin however it chooses.
///
/// What the origin IS belongs to the profile, not to this crate: this crate is
/// `no_std`, takes no position on reference counting, and ships no profile that
/// uses an origin. A profile implements this trait beside its [`HoldsBytes`] and
/// [`HoldsStr`] impls; [`Heap`] and [`Inline`] do not — an inline profile has no
/// origin to refer to, which is the point of it — and neither impl, nor the
/// generated output of a document that does not ask, changes.
///
/// The trait says only what the origin is. Whether a field shares it or copies
/// out of it is decided by the container the profile holds that field in, tag
/// by tag ([`HoldsBytes`], [`HoldsStr`]): the container builds itself from the
/// origin ([`FromOrigin`], [`FromOriginStr`]), and [`OriginBytes`] and
/// [`OriginStr`] are what the projection asks of the profile for each field
/// (docs/adr/0077 §7). One profile may so share a frame-sized payload and copy
/// a short piece of metadata beside it.
///
/// There is no copying default. A default that copied would let a profile
/// that forgot to say how a container is built from its origin build exactly
/// what [`SceByteBuf::from_slice`] already builds, and report success: the
/// sharing this trait exists for would silently not happen. A field whose
/// container has no constructor from the profile's origin is a compile error at
/// the projection; a profile that wants to copy says so, with
/// [`copy_from_origin`] or [`copy_str_from_origin`].
pub trait OriginStorage {
    /// What the decoded bytes were taken from — a handle to a frame, an arena,
    /// a pool slot. Unsized so that a profile can take a slice or a trait
    /// object by reference as readily as a sized handle.
    type Origin: ?Sized;
}

/// A byte container built from a slice AND the origin `O` the slice was
/// decoded from. The container decides: hold a reference to the origin, or
/// copy the slice.
///
/// `view` is a slice of the buffer `origin` stands for when it was decoded
/// from it, but a borrowed view can be built by hand (a default field is the
/// empty slice; a test passes a literal). A container decides what to do with
/// a slice that is not part of its origin — see [`subrange_of`], which answers
/// the question — and answers [`CodecError`] or copies; it must not assume.
///
/// A container that copies is built by [`copy_from_origin`]. Nothing here
/// implements this trait for every [`SceByteBuf`]: a sharing container is one
/// too, and a profile that forgot to choose would copy without a word.
pub trait FromOrigin<O: ?Sized>: Sized {
    /// The container of `view`, which is part of `origin`.
    fn from_origin(origin: &O, view: &[u8]) -> Result<Self, CodecError>;
}

/// A text container built from a text AND the origin `O` it was decoded from;
/// the same contract as [`FromOrigin`]. `view` is valid UTF-8 by type.
pub trait FromOriginStr<O: ?Sized>: Sized {
    /// The container of `view`, which is part of `origin`.
    fn from_origin_str(origin: &O, view: &str) -> Result<Self, CodecError>;
}

/// How an origin profile holds the `bytes` field `Tag`, as [`StoresBytes`] is
/// for a copying read: through the container it holds the field in, built from
/// the origin ([`FromOrigin`]). Every profile whose container for `Tag` has
/// such a constructor answers it, and no other; a profile does not implement
/// it by hand.
pub trait OriginBytes<Tag>: OriginStorage + HoldsBytes<Tag> {
    /// `view`, which is part of `origin`, as the field holds it.
    fn bytes_from(origin: &Self::Origin, view: &[u8]) -> Result<Self::Bytes, CodecError>;
}

impl<Tag, S> OriginBytes<Tag> for S
where
    S: OriginStorage + HoldsBytes<Tag>,
    <S as HoldsBytes<Tag>>::Bytes: FromOrigin<<S as OriginStorage>::Origin>,
{
    fn bytes_from(
        origin: &<S as OriginStorage>::Origin,
        view: &[u8],
    ) -> Result<<S as HoldsBytes<Tag>>::Bytes, CodecError> {
        <<S as HoldsBytes<Tag>>::Bytes as FromOrigin<<S as OriginStorage>::Origin>>::from_origin(
            origin, view,
        )
    }
}

/// How an origin profile holds the `string` field `Tag`, as [`OriginBytes`] is
/// for a `bytes` field ([`FromOriginStr`]).
pub trait OriginStr<Tag>: OriginStorage + HoldsStr<Tag> {
    /// `view`, which is part of `origin`, as the field holds it.
    fn str_from(origin: &Self::Origin, view: &str) -> Result<Self::Str, CodecError>;
}

impl<Tag, S> OriginStr<Tag> for S
where
    S: OriginStorage + HoldsStr<Tag>,
    <S as HoldsStr<Tag>>::Str: FromOriginStr<<S as OriginStorage>::Origin>,
{
    fn str_from(
        origin: &<S as OriginStorage>::Origin,
        view: &str,
    ) -> Result<<S as HoldsStr<Tag>>::Str, CodecError> {
        <<S as HoldsStr<Tag>>::Str as FromOriginStr<<S as OriginStorage>::Origin>>::from_origin_str(
            origin, view,
        )
    }
}

/// [`FromOrigin::from_origin`] for a container that copies: `view` copied in,
/// the origin not used. A profile whose origin type is its own writes the
/// copying container's impl with it, which the orphan rule admits:
///
/// ```ignore
/// impl FromOrigin<MyFrame> for GrowBytes {
///     fn from_origin(origin: &MyFrame, view: &[u8]) -> Result<Self, CodecError> {
///         copy_from_origin(origin, view)
///     }
/// }
/// ```
pub fn copy_from_origin<B: SceByteBuf, O: ?Sized>(
    _origin: &O,
    view: &[u8],
) -> Result<B, CodecError> {
    B::from_slice(view)
}

/// [`FromOriginStr::from_origin_str`] for a text container that copies, as
/// [`copy_from_origin`] is for bytes.
pub fn copy_str_from_origin<T: SceStr, O: ?Sized>(
    _origin: &O,
    view: &str,
) -> Result<T, CodecError> {
    T::from_view(view)
}

/// Where `sub` lies inside `base`, as a byte range of `base`; `None` when it
/// does not lie wholly inside.
///
/// The check an [`OriginStorage`] profile makes to turn a borrowed slice back
/// into a position in the buffer it was decoded from. It compares addresses,
/// so it answers "is this memory part of that buffer", which is what sharing
/// the buffer needs, and not "does it hold the same bytes", which a copy would
/// satisfy. An empty `sub` is inside `base` only if its address is: a literal
/// `&[]` has a dangling address that may happen to equal one, so a caller that
/// cares about the difference tests emptiness first.
#[must_use]
pub fn subrange_of(base: &[u8], sub: &[u8]) -> Option<core::ops::Range<usize>> {
    let base_start = base.as_ptr() as usize;
    let sub_start = sub.as_ptr() as usize;
    let start = sub_start.checked_sub(base_start)?;
    let end = start.checked_add(sub.len())?;
    (end <= base.len()).then_some(start..end)
}

/// Growable storage: heap containers, for every field.
///
/// The profile for hosts with an allocator, where the on-wire protocol places
/// no ceiling on a payload and the application must not invent one.
#[cfg(feature = "alloc")]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Heap;

/// Non-allocating storage: every field held inline at its bound.
///
/// The profile for MCU tiers and for real-time paths inside otherwise
/// heap-capable programs. It answers for a field in the generated tree, from
/// the bound the document states, and exceeding what it holds is
/// [`CodecError::CapacityExceeded`] rather than an allocation.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Inline;

#[cfg(feature = "alloc")]
impl Copying for Heap {}
impl Copying for Inline {}

/// The bytes [`Heap`] holds: growable, for a field of any bound.
#[cfg(feature = "alloc")]
pub type GrowBytes = HeapBytes<{ usize::MAX }>;

/// The text [`Heap`] holds: growable, for a field of any bound.
#[cfg(feature = "alloc")]
pub type GrowStr = HeapStr<{ usize::MAX }>;

/// The list [`Heap`] holds: growable, for a field of any bound. A deployment's
/// file on a heap node names it for a field it gives no capacity, so the file
/// needs no `alloc` of the crate it is compiled in (docs/adr/0077 §7).
#[cfg(feature = "alloc")]
pub type GrowVec<T> = alloc::vec::Vec<T>;

/// The storage profile a build treats as the default: [`Heap`] where an
/// allocator is available, [`Inline`] on the heap-free tier.
///
/// `{Codec}Owned` defaults to it, so code with no reason to pin a profile
/// keeps spelling the bare type and follows its target.
#[cfg(feature = "alloc")]
pub type DefaultStorage = Heap;
/// no-alloc build: the default profile is the non-allocating one.
#[cfg(not(feature = "alloc"))]
pub type DefaultStorage = Inline;

// The tag is unused here: every field of this profile grows, whatever its bound.
#[cfg(feature = "alloc")]
impl<Tag> HoldsList<Tag> for Heap {
    type List<T>
        = alloc::vec::Vec<T>
    where
        T: Default + core::fmt::Debug + Clone + PartialEq;
}

#[cfg(feature = "alloc")]
impl<Tag> HoldsStr<Tag> for Heap {
    type Str = GrowStr;
}

#[cfg(feature = "alloc")]
impl<Tag> HoldsBytes<Tag> for Heap {
    type Bytes = GrowBytes;
}

// ── list containers ──────────────────────────────────────────────

#[cfg(feature = "alloc")]
impl<T> SceList<T> for alloc::vec::Vec<T> {
    fn empty() -> Self {
        alloc::vec::Vec::new()
    }

    fn try_push(&mut self, value: T) -> Result<(), CodecError> {
        self.push(value);
        Ok(())
    }

    fn clear(&mut self) {
        self.clear();
    }

    fn as_slice(&self) -> &[T] {
        self.as_slice()
    }

    fn last_mut(&mut self) -> Option<&mut T> {
        self.as_mut_slice().last_mut()
    }

    fn remove_at(&mut self, at: usize) {
        self.remove(at);
    }
}

#[cfg(feature = "alloc")]
impl<T> HeldList for alloc::vec::Vec<T> {
    type Entry = T;
}

impl<T, const N: usize> SceList<T> for heapless::Vec<T, N> {
    fn empty() -> Self {
        heapless::Vec::new()
    }

    fn try_push(&mut self, value: T) -> Result<(), CodecError> {
        self.push(value).map_err(|_| CodecError::CapacityExceeded)
    }

    fn clear(&mut self) {
        self.clear();
    }

    fn as_slice(&self) -> &[T] {
        self.as_slice()
    }

    fn last_mut(&mut self) -> Option<&mut T> {
        self.as_mut_slice().last_mut()
    }

    fn remove_at(&mut self, at: usize) {
        self.remove(at);
    }

    fn is_full(&self) -> bool {
        heapless::Vec::is_full(self)
    }
}

impl<T, const N: usize> HeldList for heapless::Vec<T, N> {
    type Entry = T;
}

// ── text containers ──────────────────────────────────────────────

// The containers are `sce-portable-bytes`'; only the codec's reading of them as
// a [`SceStr`] is this crate's. `SceString` beside them is the language's text
// type (an algorithm's record field), hard-bounded on every profile and no
// codec container (docs/adr/0086, section 7.4).
#[cfg(feature = "alloc")]
pub use sce_portable_bytes::HeapStr;
pub use sce_portable_bytes::{InlineStr, SceString};

/// The codec's reading of a text container: the copy-in is the container's own
/// fallible `from_text`, its capacity error relabelled as the codec's.
macro_rules! sce_str_impl {
    ($storage:ident) => {
        impl<const N: usize> SceStr for $storage<N> {
            fn from_view(s: &str) -> Result<Self, CodecError> {
                Self::from_text(s).map_err(CodecError::from)
            }

            fn as_str(&self) -> &str {
                Self::as_str(self)
            }

            fn try_push_str(&mut self, s: &str) -> Result<(), CodecError> {
                Self::try_push_str(self, s).map_err(CodecError::from)
            }
        }
    };
}

#[cfg(feature = "alloc")]
sce_str_impl!(HeapStr);
sce_str_impl!(InlineStr);

// ── byte containers ──────────────────────────────────────────────

/// `SceByteBuf` over the two `sce-portable-bytes` profiles. The construction
/// logic lives in that crate (shared with the statechart-emit runtime); this
/// only re-labels its capacity error as the codec error a generated
/// `try_into_owned` already threads.
macro_rules! byte_buf_impl {
    ($storage:ident) => {
        impl<const N: usize> SceByteBuf for $storage<N> {
            fn from_slice(b: &[u8]) -> Result<Self, CodecError> {
                Self::from_slice(b).map_err(CodecError::from)
            }

            fn as_slice(&self) -> &[u8] {
                Self::as_slice(self)
            }
        }
    };
}

#[cfg(feature = "alloc")]
byte_buf_impl!(HeapBytes);
byte_buf_impl!(InlineBytes);

// ── projections ──────────────────────────────────────────────────

/// Build an owned list by projecting each element of a decoded borrowed list.
///
/// The SSOT for the borrowed→owned list step: every generated
/// `try_into_owned` calls this rather than open-coding the loop, so the
/// capacity semantics live in one place. The destination profile is inferred
/// from the field the result is being placed in, which is what lets one
/// generated body serve every storage profile: on [`Inline`] an over-long
/// source raises [`CodecError::CapacityExceeded`], on [`Heap`] it cannot fail.
///
/// The inverse direction (owned→borrowed) is
/// [`crate::codec::try_project_bounded`], which always targets the borrowed
/// view's inline list and so is capacity-checked on every profile.
pub fn try_collect_list<L, T, U>(
    src: impl IntoIterator<Item = T>,
    mut f: impl FnMut(T) -> Result<U, CodecError>,
) -> Result<L, CodecError>
where
    L: SceList<U>,
{
    let mut out = L::empty();
    for item in src {
        out.try_push(f(item)?)?;
    }
    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn inline_list_is_bounded_at_its_declared_capacity() {
        let full: Result<heapless::Vec<u8, 2>, _> = try_collect_list([1u8, 2], Ok);
        assert!(full.is_ok());
        let over: Result<heapless::Vec<u8, 2>, _> = try_collect_list([1u8, 2, 3], Ok);
        assert_eq!(over, Err(CodecError::CapacityExceeded));
    }

    #[test]
    fn projection_error_propagates_before_capacity() {
        let out: Result<heapless::Vec<u8, 8>, _> =
            try_collect_list([1u8, 2], |_| Err(CodecError::InvalidUtf8));
        assert_eq!(out, Err(CodecError::InvalidUtf8));
    }

    #[cfg(feature = "alloc")]
    #[test]
    fn heap_list_accepts_more_than_the_declared_capacity() {
        // The same call shape on the growable profile: `N` is advisory, so a
        // run longer than any inline bound still collects.
        let out: Result<alloc::vec::Vec<u8>, _> = try_collect_list(0..64u8, Ok);
        assert_eq!(out.unwrap().len(), 64);
    }

    #[test]
    fn inline_str_is_bounded_even_where_heap_exists() {
        assert!(InlineStr::<4>::from_view("abcd").is_ok());
        assert_eq!(
            InlineStr::<4>::from_view("abcde"),
            Err(CodecError::CapacityExceeded)
        );
    }

    #[cfg(feature = "alloc")]
    #[test]
    fn heap_str_is_unbounded() {
        let s = HeapStr::<4>::from_view("far past four").unwrap();
        assert_eq!(s.as_str(), "far past four");
        assert_eq!(s, "far past four");
    }

    // The property the whole policy exists for: both profiles are usable from
    // one build, so a storage choice is made per value rather than per binary.
    #[cfg(feature = "alloc")]
    #[test]
    fn both_profiles_resolve_in_one_binary() {
        struct Note;
        impl HoldsStr<Note> for Inline {
            type Str = InlineStr<8>;
        }
        fn text<'f, S: StoresStr<'f, Note>>(src: &'f str) -> Result<S::Str, CodecError> {
            S::store_str(src)
        }
        assert_eq!(text::<Heap>("ok").unwrap().as_str(), "ok");
        assert_eq!(text::<Inline>("ok").unwrap().as_str(), "ok");
        // ...and only the inline profile enforces the declared capacity.
        assert!(text::<Heap>("nine char").is_ok());
        assert_eq!(
            text::<Inline>("nine char").err(),
            Some(CodecError::CapacityExceeded)
        );
    }

    #[test]
    fn a_slice_of_the_buffer_is_found_at_its_position() {
        let frame = [10u8, 11, 12, 13, 14, 15];
        assert_eq!(subrange_of(&frame, &frame[..]), Some(0..6));
        assert_eq!(subrange_of(&frame, &frame[2..5]), Some(2..5));
        // The last byte is inside; a slice ending exactly at the end is too.
        assert_eq!(subrange_of(&frame, &frame[5..]), Some(5..6));
    }

    #[test]
    fn memory_outside_the_buffer_is_not_found_even_when_equal() {
        let frame = [1u8, 2, 3, 4];
        // Same bytes, different memory: sharing a buffer is about where the
        // bytes live, and a copy lives elsewhere.
        let copy = frame;
        assert_eq!(subrange_of(&frame, &copy[1..3]), None);
        // A slice that starts inside but runs past the end is not inside.
        let longer = [1u8, 2, 3, 4, 5, 6];
        assert_eq!(subrange_of(&frame, &longer[2..]), None);
        // Before the buffer: the offset would be negative.
        assert_eq!(subrange_of(&longer[2..], &longer[..2]), None);
    }

    #[test]
    fn an_empty_slice_is_found_only_where_its_address_is() {
        let frame = [1u8, 2, 3];
        assert_eq!(subrange_of(&frame, &frame[1..1]), Some(1..1));
        assert_eq!(subrange_of(&frame, &frame[3..3]), Some(3..3));
        // An empty slice in other memory has an address of its own. A static
        // and a position inside it: one past the end of a stack array could
        // coincide with the start of its neighbour.
        static ELSEWHERE: [u8; 4] = [0; 4];
        assert_eq!(subrange_of(&frame, &ELSEWHERE[2..2]), None);
    }

    // An origin profile holds each field in the container it chose for the
    // field's tag, and that container decides whether the field shares the
    // origin or copies out of it: one profile shares one field and copies the
    // one beside it.
    #[cfg(feature = "alloc")]
    #[test]
    fn each_field_is_built_from_the_origin_by_the_container_its_profile_holds_it_in() {
        use alloc::rc::Rc;

        // The origin: a frame this test owns.
        struct Frame(Rc<[u8]>);

        // A range of the frame, shared.
        #[derive(Debug, Clone, PartialEq)]
        struct Window(Rc<[u8]>, core::ops::Range<usize>);
        impl SceByteBuf for Window {
            fn from_slice(b: &[u8]) -> Result<Self, CodecError> {
                Ok(Window(Rc::from(b), 0..b.len()))
            }
            fn as_slice(&self) -> &[u8] {
                &self.0[self.1.clone()]
            }
        }
        impl crate::bounded::Fewest for Window {
            fn fewest<const MIN: usize>() -> Self {
                Window(Rc::from(alloc::vec![0u8; MIN]), 0..MIN)
            }
        }
        impl FromOrigin<Frame> for Window {
            fn from_origin(origin: &Frame, view: &[u8]) -> Result<Self, CodecError> {
                match subrange_of(&origin.0, view) {
                    Some(range) => Ok(Window(Rc::clone(&origin.0), range)),
                    None => Window::from_slice(view),
                }
            }
        }
        // The growable containers copy, and say so.
        impl FromOrigin<Frame> for GrowBytes {
            fn from_origin(origin: &Frame, view: &[u8]) -> Result<Self, CodecError> {
                copy_from_origin(origin, view)
            }
        }
        impl FromOriginStr<Frame> for GrowStr {
            fn from_origin_str(origin: &Frame, view: &str) -> Result<Self, CodecError> {
                copy_str_from_origin(origin, view)
            }
        }

        struct Payload;
        struct Schema;
        struct Note;
        struct Mixed;
        impl OriginStorage for Mixed {
            type Origin = Frame;
        }
        impl HoldsBytes<Payload> for Mixed {
            type Bytes = Window;
        }
        impl HoldsBytes<Schema> for Mixed {
            type Bytes = GrowBytes;
        }
        impl HoldsStr<Note> for Mixed {
            type Str = GrowStr;
        }

        let frame = Frame(Rc::from(&[1u8, 2, 3, 4, 5][..]));
        let payload = <Mixed as OriginBytes<Payload>>::bytes_from(&frame, &frame.0[1..3]).unwrap();
        assert_eq!(payload.as_slice(), &[2, 3]);
        assert_eq!(
            payload.as_slice().as_ptr(),
            frame.0[1..].as_ptr(),
            "the payload is the frame's memory"
        );
        let schema = <Mixed as OriginBytes<Schema>>::bytes_from(&frame, &frame.0[3..5]).unwrap();
        assert_eq!(schema.as_slice(), &[4, 5]);
        assert_eq!(
            subrange_of(&frame.0, schema.as_slice()),
            None,
            "the schema is a copy"
        );
        let note = <Mixed as OriginStr<Note>>::str_from(&frame, "hi").unwrap();
        assert_eq!(note.as_str(), "hi");
        // The frame, and the one field that shares it.
        assert_eq!(Rc::strong_count(&frame.0), 2);
        drop(payload);
        assert_eq!(Rc::strong_count(&frame.0), 1);
    }

    #[test]
    fn a_copying_container_copies_whatever_the_origin() {
        let origin = [9u8, 8, 7];
        let bytes: InlineBytes<4> = copy_from_origin(&origin, &origin[..2]).unwrap();
        assert_eq!(bytes.as_slice(), &[9, 8]);
        assert_eq!(subrange_of(&origin, bytes.as_slice()), None);
        let over: Result<InlineBytes<1>, _> = copy_from_origin(&origin, &origin[..2]);
        assert_eq!(over.err(), Some(CodecError::CapacityExceeded));
        let text: InlineStr<4> = copy_str_from_origin(&origin, "abc").unwrap();
        assert_eq!(text.as_str(), "abc");
        let over: Result<InlineStr<2>, _> = copy_str_from_origin(&origin, "abc");
        assert_eq!(over.err(), Some(CodecError::CapacityExceeded));
    }
}
