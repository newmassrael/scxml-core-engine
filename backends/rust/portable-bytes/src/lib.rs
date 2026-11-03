// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael

//! Profile-portable owned `bytes` storage shared by the codec runtime
//! (`sce-forge-runtime` — the `{Codec}Owned` byte fields) and the
//! statechart-emit runtime (`sce-rust-runtime` — typed `_event.data` byte
//! payloads). Both are the same concept: an owned byte sequence with a
//! per-field capacity `N` (the SCXML `sce:max-size`) that must compile on
//! both the std/alloc and the heap-free no_std profiles. This crate holds
//! the single definition; both runtimes `pub use` it, so the construction
//! logic (the fallible no-alloc copy, the advisory-`N` alloc copy) lives in
//! exactly one place and cannot drift between the two consumers.
//!
//! The storage is generic over its element: [`HeapList`] / [`InlineList`]
//! hold any `T: Copy`, and the byte types are their `u8` instances. The
//! algorithm kind's `list<T>` buffers (SCE_FORGE.md §4.12) use the same two
//! types, so a byte buffer and a list of `int64` grow, bound and report
//! [`CapacityExceeded`] by one definition.
//!
//! `N` rides on the newtype rather than being erased to a bare `Vec<u8>`
//! alias under `alloc`: a hand-assembled owner (a codec encode-side builder,
//! or a typed-event inject caller) then infers the cap from the destination
//! field's type — `SceBytes::from_slice(&v)?` — instead of hardcoding it at
//! every call site (which would duplicate the `sce:max-size` source of
//! truth into consumer code).
//!
//! # The profile is a type, not a compilation mode
//!
//! [`HeapBytes`] (growable, `N` advisory) and [`InlineBytes`] (fixed `N`,
//! heap-free) are two *types*, both compiled whenever `alloc` is on, rather
//! than one name whose meaning flips with a `cfg`. A single binary therefore
//! holds both storage profiles at once: an application-processor path can
//! keep growable owned values while a real-time path in the same program
//! stores its values inline with a hard capacity. [`SceBytes`] is the alias
//! naming whichever profile is the *default* for the current build, so the
//! plain spelling stays correct on both tiers, and the no-alloc tier — where
//! `HeapBytes` does not exist at all — cannot accidentally name the growable
//! form.

#![no_std]

#[cfg(feature = "alloc")]
extern crate alloc;

/// Returned by `from_slice` / `push` / `extend_from_slice` when a copy would
/// exceed a fixed capacity `N`. The growable profile has no such bound and
/// never returns it. Downstream runtimes map it into their own error enum
/// (e.g. the codec runtime's `CodecError::TooManyElements`) via a `From` impl,
/// so a generated `try_into_owned` keeps threading one `?`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CapacityExceeded;

/// Growable owned sequence of `T`: wraps `Vec<T>`, so `N` is advisory — the
/// on-wire protocol caps no payload, and the application-processor profile
/// must not either. `N` still rides on the type so a downstream owned-builder
/// infers the cap from the field rather than hardcoding it, and so a value can
/// be transcoded into [`InlineList`] of the matching capacity.
///
/// Bytes are the `T = u8` case ([`HeapBytes`]); the algorithm kind's
/// `list<T>` locals and returns are every other fixed-width scalar. One
/// definition serves both, so the byte buffer and the list cannot drift in
/// how they grow, bound or report overflow.
///
/// No field names `N`: a const parameter needs none, and a marker of `[T; N]`
/// would be an array the compiler has to lay out to write debug info for the
/// type. `sce-forge-runtime`'s growable profile names `{ usize::MAX }`, which
/// no such array fits, so a build at full debug info would refuse it.
#[cfg(feature = "alloc")]
#[repr(transparent)]
#[derive(Debug, Clone, PartialEq, Default)]
pub struct HeapList<T, const N: usize>(alloc::vec::Vec<T>);

/// Fixed-capacity owned sequence of `T`: wraps `heapless::Vec<T, N>` (the C11
/// `T[N]` analog), so the value never allocates and `N` is a hard bound.
///
/// Compiled on every profile — including alongside [`HeapList`] under `alloc`
/// — so a heap-capable program can still hold values that are guaranteed not
/// to allocate.
#[repr(transparent)]
#[derive(Debug, Clone, PartialEq, Default)]
pub struct InlineList<T, const N: usize>(heapless::Vec<T, N>);

/// Growable owned byte storage — [`HeapList`] of `u8`.
#[cfg(feature = "alloc")]
pub type HeapBytes<const N: usize> = HeapList<u8, N>;

/// Fixed-capacity owned byte storage — [`InlineList`] of `u8`.
pub type InlineBytes<const N: usize> = InlineList<u8, N>;

/// The list storage profile this build treats as the default, as
/// [`SceBytes`] is for bytes.
///
/// ⚠ Named `SceOwnedList`, not `SceList`: `sce-forge-runtime` re-exports this
/// crate's types and already defines a `SceList` TRAIT (its codec list
/// containers), so the shorter name would collide there.
#[cfg(feature = "alloc")]
pub type SceOwnedList<T, const N: usize> = HeapList<T, N>;
/// no-alloc build: the default list profile is the fixed-capacity form.
#[cfg(not(feature = "alloc"))]
pub type SceOwnedList<T, const N: usize> = InlineList<T, N>;

/// The byte storage profile this build treats as the default: [`HeapBytes`]
/// where an allocator is available, [`InlineBytes`] on the heap-free tier.
///
/// Code that has no reason to pin a profile spells `SceBytes<N>` and follows
/// the target; code that must not allocate names [`InlineBytes`] explicitly
/// and keeps that guarantee even in a build where the default is growable.
#[cfg(feature = "alloc")]
pub type SceBytes<const N: usize> = HeapBytes<N>;
/// no-alloc build: the default profile is the fixed-capacity form.
#[cfg(not(feature = "alloc"))]
pub type SceBytes<const N: usize> = InlineBytes<N>;

#[cfg(feature = "alloc")]
impl<T: Clone, const N: usize> HeapList<T, N> {
    /// Copy a borrowed `&[T]` view into the owned form. The heap copy is
    /// unbounded (`N` advisory), but the return stays a `Result` so a
    /// generated `try_into_owned` threads one `?` on every profile and the
    /// two profiles keep one call shape.
    pub fn from_slice(s: &[T]) -> Result<Self, CapacityExceeded> {
        Ok(Self(s.to_vec()))
    }

    /// Borrow the owned elements back as a slice — the projection an owned
    /// value's `as_borrowed` re-uses.
    pub fn as_slice(&self) -> &[T] {
        self.0.as_slice()
    }

    /// An empty buffer that grows on demand. Used by the algorithm kind's
    /// `<sce:var type="bytes" capacity="N">` and `type="list<T>"` locals to
    /// seed a buffer that `push` / `extend_from_slice` then fill.
    pub fn new() -> Self {
        Self(alloc::vec::Vec::new())
    }

    /// Empty the buffer and keep what it allocated. An algorithm that returns a
    /// list or bytes writes into a destination its caller holds
    /// (docs/adr/0082), and starts by emptying it, so a caller may hand the
    /// same destination to every call and allocate once.
    pub fn clear(&mut self) {
        self.0.clear();
    }

    /// Append one element. The heap form grows, so this never reports
    /// overflow; the `Result` matches [`InlineList::push`] so a generated
    /// buffer-building algorithm threads one `?` per append rather than
    /// branching on the allocator.
    pub fn push(&mut self, v: T) -> Result<(), CapacityExceeded> {
        self.0.push(v);
        Ok(())
    }

    /// Append a borrowed slice. Same contract as [`HeapList::push`].
    pub fn extend_from_slice(&mut self, s: &[T]) -> Result<(), CapacityExceeded> {
        self.0.extend_from_slice(s);
        Ok(())
    }

    /// The elements of a sequence of at most `M` elements, in one of `N`. The
    /// fit is a build error where it does not hold ([`Fits`]), so nothing is
    /// refused: the copy of a byte string into a place of a larger bound, which
    /// the build proved it fits.
    pub fn widened<const M: usize>(from: &HeapList<T, M>) -> Self {
        let () = Fits::<M, N>::OK;
        Self(from.0.clone())
    }
}

impl<T: Clone, const N: usize> InlineList<T, N> {
    /// Copy a borrowed `&[T]` view into the fixed-capacity form, raising
    /// [`CapacityExceeded`] past `N`.
    pub fn from_slice(s: &[T]) -> Result<Self, CapacityExceeded> {
        heapless::Vec::from_slice(s)
            .map(Self)
            .map_err(|_| CapacityExceeded)
    }

    /// Borrow the owned elements back as a slice — the projection an owned
    /// value's `as_borrowed` re-uses.
    pub fn as_slice(&self) -> &[T] {
        self.0.as_slice()
    }

    /// An empty buffer holding the fixed capacity `N` inline.
    pub fn new() -> Self {
        Self(heapless::Vec::new())
    }

    /// Empty the buffer. An algorithm that returns a list or bytes writes into a
    /// destination its caller holds (docs/adr/0082), and starts by emptying it:
    /// the `N` elements live where the caller put them, never in the
    /// algorithm's frame or in its return.
    pub fn clear(&mut self) {
        self.0.clear();
    }

    /// Append one element, raising [`CapacityExceeded`] once the fixed `N` is
    /// full — never silent truncation, never a panic.
    pub fn push(&mut self, v: T) -> Result<(), CapacityExceeded> {
        self.0.push(v).map_err(|_| CapacityExceeded)
    }

    /// Append a borrowed slice. Either the whole slice is appended or none is
    /// — the bound is checked before copying.
    pub fn extend_from_slice(&mut self, s: &[T]) -> Result<(), CapacityExceeded> {
        self.0.extend_from_slice(s).map_err(|_| CapacityExceeded)
    }

    /// The elements of a sequence of at most `M` elements, in one of `N`. See
    /// [`HeapList::widened`].
    pub fn widened<const M: usize>(from: &InlineList<T, M>) -> Self {
        let () = Fits::<M, N>::OK;
        // At most `M <= N` elements, so the copy cannot refuse them; the empty
        // sequence is the answer for a refusal that cannot occur.
        Self(heapless::Vec::from_slice(from.as_slice()).unwrap_or_default())
    }
}

/// Byte-view parity surface, written once for both profiles so the growable
/// and fixed forms cannot drift apart in how they are read or compared.
///
/// `Deref<Target = [u8]>` gives `len()` / iteration / slicing; the `PartialEq`
/// arms let a consumer (or a generated round-trip sidecar) write
/// `assert_eq!(owned.payload, b"...")` and `owned.payload == &slice` without
/// an explicit `.as_slice()`. Only the reference forms are implemented —
/// those are what the sidecar (`b"..."` is `&[u8; M]`) and the owned
/// round-trip tests (`&[u8]`) actually consume.
macro_rules! byte_view_parity {
    ($storage:ident) => {
        impl<T, const N: usize> core::ops::Deref for $storage<T, N> {
            type Target = [T];
            fn deref(&self) -> &[T] {
                self.0.as_slice()
            }
        }

        impl<T: PartialEq, const N: usize> PartialEq<&[T]> for $storage<T, N> {
            fn eq(&self, other: &&[T]) -> bool {
                self.as_slice() == *other
            }
        }

        impl<T: PartialEq, const N: usize, const M: usize> PartialEq<&[T; M]> for $storage<T, N> {
            fn eq(&self, other: &&[T; M]) -> bool {
                self.as_slice() == other.as_slice()
            }
        }
    };
}

#[cfg(feature = "alloc")]
byte_view_parity!(HeapList);
byte_view_parity!(InlineList);

// ── text containers ──────────────────────────────────────────────

/// Growable owned text storage: wraps `String`, so `N` is advisory. `N` still
/// rides on the type so a hand-assembled owned value infers the cap from the
/// field it is being placed in, and so the value can be transcoded into
/// [`InlineStr`] of the matching capacity. No field names `N`, for the reason
/// [`HeapList`] gives.
#[cfg(feature = "alloc")]
#[repr(transparent)]
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct HeapStr<const N: usize>(alloc::string::String);

/// Fixed-capacity owned text storage: wraps `heapless::String<N>`, so the
/// value never allocates and `N` is a hard bound. Compiled on every profile,
/// including alongside [`HeapStr`].
#[repr(transparent)]
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct InlineStr<const N: usize>(heapless::String<N>);

/// The text storage profile this build treats as the default — the sibling of
/// [`SceBytes`] for `string` fields.
#[cfg(feature = "alloc")]
pub type SceString<const N: usize> = HeapStr<N>;
/// no-alloc build: the default text profile is the fixed-capacity form.
#[cfg(not(feature = "alloc"))]
pub type SceString<const N: usize> = InlineStr<N>;

/// The compile-time judgement that a value of at most `FROM` (bytes of a text
/// or a byte string, elements of a list) fits a place of `TO`: naming
/// `Fits::<FROM, TO>::OK` makes a copy that could overflow a build error where
/// it is written, which is what lets [`SceString::widened`],
/// [`BoundedList::widened`] and [`sce_text!`] be total functions. A generated
/// machine copies a value into a place of a larger bound only through one of
/// them, so a copy its build did not prove to fit does not compile
/// (docs/adr/0086, section 7.4).
#[doc(hidden)]
pub struct Fits<const FROM: usize, const TO: usize>;

impl<const FROM: usize, const TO: usize> Fits<FROM, TO> {
    #[doc(hidden)]
    pub const OK: () = assert!(
        FROM <= TO,
        "a value is copied into a place whose bound is smaller than the one it was held in"
    );
}

/// The longest prefix of `s` that is at most `n` bytes and ends on a character
/// boundary — what a text past its bound is cut to by [`SceString::held`].
fn prefix_within(s: &str, n: usize) -> &str {
    if s.len() <= n {
        return s;
    }
    let mut end = n;
    while !s.is_char_boundary(end) {
        end -= 1;
    }
    &s[..end]
}

#[cfg(feature = "alloc")]
impl<const N: usize> HeapStr<N> {
    /// Copy a borrowed `&str` into owned storage. The heap copy is unbounded
    /// (`N` advisory); the `Result` matches [`InlineStr::from_text`] so one
    /// call shape serves both profiles.
    pub fn from_text(s: &str) -> Result<Self, CapacityExceeded> {
        Ok(Self(alloc::string::String::from(s)))
    }

    /// Borrow the owned text back as `&str`.
    pub fn as_str(&self) -> &str {
        self.0.as_str()
    }

    /// Append `s`. The heap text grows (`N` advisory); the `Result` matches
    /// [`InlineStr::try_push_str`] so one call shape serves both profiles.
    pub fn try_push_str(&mut self, s: &str) -> Result<(), CapacityExceeded> {
        self.0.push_str(s);
        Ok(())
    }

    /// The text of `s`, which the caller has already held to `N` bytes.
    ///
    /// Total: a text past `N` is cut at the last character boundary that fits
    /// rather than reported, because the callers are the ones that proved the
    /// fit — a generated body whose every write was judged against this bound,
    /// and a machine that holds the text within it. Use [`Self::from_text`]
    /// for text nothing has judged.
    pub fn held(s: &str) -> Self {
        Self(alloc::string::String::from(prefix_within(s, N)))
    }

    /// The text of a buffer of at most `N` bytes, in one of `N`. The fit is a
    /// build error where it does not hold ([`Fits`]), so nothing is cut.
    pub fn widened<const M: usize>(from: &HeapStr<M>) -> Self {
        let () = Fits::<M, N>::OK;
        Self::held(from.as_str())
    }
}

impl<const N: usize> InlineStr<N> {
    /// Copy a borrowed `&str` into fixed-capacity storage, raising
    /// [`CapacityExceeded`] past `N` bytes.
    pub fn from_text(s: &str) -> Result<Self, CapacityExceeded> {
        heapless::String::try_from(s)
            .map(Self)
            .map_err(|_| CapacityExceeded)
    }

    /// Borrow the owned text back as `&str`.
    pub fn as_str(&self) -> &str {
        self.0.as_str()
    }

    /// Append `s`, raising [`CapacityExceeded`] where the text would pass `N`
    /// bytes; the text is then as it was. What a reader that builds a text as
    /// it unescapes it writes through.
    pub fn try_push_str(&mut self, s: &str) -> Result<(), CapacityExceeded> {
        self.0.push_str(s).map_err(|_| CapacityExceeded)
    }

    /// The text of `s`, which the caller has already held to `N` bytes. See
    /// [`HeapStr::held`].
    pub fn held(s: &str) -> Self {
        // The prefix is at most `N` bytes, so the conversion cannot refuse it;
        // the empty text is the answer for a refusal that cannot occur.
        Self(heapless::String::try_from(prefix_within(s, N)).unwrap_or_default())
    }

    /// The text of a buffer of at most `N` bytes, in one of `N`. See
    /// [`HeapStr::widened`].
    pub fn widened<const M: usize>(from: &InlineStr<M>) -> Self {
        let () = Fits::<M, N>::OK;
        Self::held(from.as_str())
    }
}

/// Text-view parity surface, written once for both profiles so the growable
/// and fixed forms cannot drift apart in how they are read or compared.
///
/// `Deref<Target = str>` gives the whole `&str` surface; the `PartialEq<&str>`
/// arm lets a consumer write `assert_eq!(owned.locator, "abc")` without an
/// explicit `.as_str()`. Only the `&str` form is implemented — string literals
/// are `&str`, and the unsized right-hand side has no consumer.
macro_rules! str_view_parity {
    ($storage:ident) => {
        impl<const N: usize> core::ops::Deref for $storage<N> {
            type Target = str;
            fn deref(&self) -> &str {
                self.0.as_str()
            }
        }

        impl<const N: usize> PartialEq<&str> for $storage<N> {
            fn eq(&self, other: &&str) -> bool {
                self.as_str() == *other
            }
        }

        /// The text itself, so a text is formatted (and concatenated by
        /// `format!`) as the `&str` it holds.
        impl<const N: usize> core::fmt::Display for $storage<N> {
            fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
                f.write_str(self.as_str())
            }
        }
    };
}

#[cfg(feature = "alloc")]
str_view_parity!(HeapStr);
str_view_parity!(InlineStr);

// ── bounded values: the bound is part of the type on every profile ──────────

/// Where a [`BoundedList`] keeps its elements: a `Vec<T>` where an allocator is
/// available, and `heapless::Vec<T, N>`, `N` inline, where none is. The bound
/// is held by the list's constructors on both, so the storage only decides
/// where the elements live, never how many there may be.
#[cfg(feature = "alloc")]
type BoundedListStorage<T, const N: usize> = alloc::vec::Vec<T>;
#[cfg(not(feature = "alloc"))]
type BoundedListStorage<T, const N: usize> = heapless::Vec<T, N>;

/// A list of at most `N` elements of any `T`, `N` a hard bound on every profile
/// (docs/adr/0086, sections 5.4 and 7.4): the value of a `list<T>[N]` that a
/// host builds and hands to a typed send, and the place a Rust machine holds a
/// list in, a variable, a record's field and a payload's alike.
///
/// [`SceOwnedList`]'s `N` is advisory under `alloc`. Every constructor and every
/// append of this type refuses more than `N` elements instead, so a value that
/// exists is one of its type, and a copy into a place of a larger bound is a
/// conversion that cannot fail ([`Self::widened`]). A nested type is the
/// composition, `BoundedList<BoundedList<BoundedText<8>, 4>, 3>`, each level
/// holding its own bound.
#[repr(transparent)]
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BoundedList<T, const N: usize>(BoundedListStorage<T, N>);

/// A byte string of at most `N` bytes on every profile: [`BoundedList`] of `u8`.
pub type BoundedBytes<const N: usize> = BoundedList<u8, N>;

impl<T, const N: usize> BoundedList<T, N> {
    /// The bound the type declares.
    pub const CAPACITY: usize = N;

    /// The empty list.
    pub fn new() -> Self {
        Self(BoundedListStorage::<T, N>::new())
    }

    /// Append one element, refusing it once the list holds `N`; the list is then
    /// as it was.
    pub fn push(&mut self, value: T) -> Result<(), CapacityExceeded> {
        if self.0.len() >= N {
            return Err(CapacityExceeded);
        }
        #[cfg(feature = "alloc")]
        {
            self.0.push(value);
            Ok(())
        }
        #[cfg(not(feature = "alloc"))]
        {
            self.0.push(value).map_err(|_| CapacityExceeded)
        }
    }

    /// The elements, in order.
    pub fn as_slice(&self) -> &[T] {
        self.0.as_slice()
    }

    /// Empty the list. What a statechart's `<sce:clear>` does to a list
    /// variable, which then takes up to `N` elements again.
    pub fn clear(&mut self) {
        self.0.clear();
    }

    /// The list of the elements `items` yields, in order, refused whole when it
    /// yields more than `N`: the elements past the bound are never taken.
    pub fn try_from_iter(items: impl IntoIterator<Item = T>) -> Result<Self, CapacityExceeded> {
        let mut list = Self::new();
        for item in items {
            list.push(item)?;
        }
        Ok(list)
    }

    /// The list of `f` of each element of a list of at most `M` elements, in
    /// one of `N`: the copy of a list into a place of a larger bound whose
    /// element is copied by a conversion of its own (a text into a larger
    /// bound). The fit is a build error where it does not hold ([`Fits`]), so
    /// nothing is refused.
    pub fn widened_map<U, const M: usize>(
        from: &BoundedList<U, M>,
        mut f: impl FnMut(&U) -> T,
    ) -> Self {
        let () = Fits::<M, N>::OK;
        let mut list = Self::new();
        for item in from {
            // At most `M <= N` elements, so no element is refused.
            if list.push(f(item)).is_err() {
                break;
            }
        }
        list
    }
}

impl<T: Clone, const N: usize> BoundedList<T, N> {
    /// Copy `items` into a list, refused whole past `N`.
    pub fn from_slice(items: &[T]) -> Result<Self, CapacityExceeded> {
        if items.len() > N {
            return Err(CapacityExceeded);
        }
        Self::try_from_iter(items.iter().cloned())
    }

    /// The elements of a list of at most `M` elements, in one of `N`: the copy
    /// a machine makes of a list into a place of a larger bound, which its
    /// build proved the list fits (docs/adr/0086, sections 5.1 and 7.4). The
    /// fit is a build error where it does not hold ([`Fits`]).
    pub fn widened<const M: usize>(from: &BoundedList<T, M>) -> Self {
        Self::widened_map(from, T::clone)
    }
}

/// The elements, taken by value, in order: what a `<foreach>` walks, over a
/// copy of the list, so its body may change the list it walks.
#[cfg(feature = "alloc")]
impl<T, const N: usize> IntoIterator for BoundedList<T, N> {
    type Item = T;
    type IntoIter = <alloc::vec::Vec<T> as IntoIterator>::IntoIter;

    fn into_iter(self) -> Self::IntoIter {
        self.0.into_iter()
    }
}

/// The elements, taken by value, in order, as under `alloc`.
#[cfg(not(feature = "alloc"))]
impl<T, const N: usize> IntoIterator for BoundedList<T, N> {
    type Item = T;
    type IntoIter = <heapless::Vec<T, N> as IntoIterator>::IntoIter;

    fn into_iter(self) -> Self::IntoIter {
        self.0.into_iter()
    }
}

#[cfg(feature = "alloc")]
impl<T, const N: usize> BoundedList<T, N> {
    /// Take `items` as the list, refused whole past `N` and without copying them.
    pub fn try_from_vec(items: alloc::vec::Vec<T>) -> Result<Self, CapacityExceeded> {
        if items.len() > N {
            return Err(CapacityExceeded);
        }
        Ok(Self(items))
    }

    /// The elements, as a `Vec`, without copying them.
    pub fn into_vec(self) -> alloc::vec::Vec<T> {
        self.0
    }
}

impl<T, const N: usize> Default for BoundedList<T, N> {
    fn default() -> Self {
        Self::new()
    }
}

impl<'a, T, const N: usize> IntoIterator for &'a BoundedList<T, N> {
    type Item = &'a T;
    type IntoIter = core::slice::Iter<'a, T>;

    fn into_iter(self) -> Self::IntoIter {
        self.as_slice().iter()
    }
}

byte_view_parity!(BoundedList);

/// Where a [`BoundedText`] keeps its bytes, as [`BoundedListStorage`] does.
#[cfg(feature = "alloc")]
type BoundedTextStorage<const N: usize> = alloc::string::String;
#[cfg(not(feature = "alloc"))]
type BoundedTextStorage<const N: usize> = heapless::String<N>;

/// A text of at most `N` UTF-8 bytes, `N` a hard bound on every profile
/// (docs/adr/0086, section 5.4): the value of a `string[N]` a host builds. The
/// sibling of [`BoundedList`] for text; [`SceString`]'s `N` is advisory under
/// `alloc`, this one's never is.
#[repr(transparent)]
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BoundedText<const N: usize>(BoundedTextStorage<N>);

impl<const N: usize> BoundedText<N> {
    /// The bound the type declares, in UTF-8 bytes.
    pub const CAPACITY: usize = N;

    /// The empty text.
    pub fn new() -> Self {
        Self(BoundedTextStorage::<N>::new())
    }

    /// Copy `text`, refused when it is more than `N` bytes; nothing is cut.
    pub fn from_text(text: &str) -> Result<Self, CapacityExceeded> {
        if text.len() > N {
            return Err(CapacityExceeded);
        }
        let mut held = Self::new();
        held.try_push_str(text)?;
        Ok(held)
    }

    /// Append `text`, refused where the whole would pass `N` bytes; the text is
    /// then as it was.
    pub fn try_push_str(&mut self, text: &str) -> Result<(), CapacityExceeded> {
        if self.0.len() + text.len() > N {
            return Err(CapacityExceeded);
        }
        #[cfg(feature = "alloc")]
        {
            self.0.push_str(text);
            Ok(())
        }
        #[cfg(not(feature = "alloc"))]
        {
            self.0.push_str(text).map_err(|_| CapacityExceeded)
        }
    }

    /// Borrow the text.
    pub fn as_str(&self) -> &str {
        self.0.as_str()
    }
}

impl<const N: usize> Default for BoundedText<N> {
    fn default() -> Self {
        Self::new()
    }
}

str_view_parity!(BoundedText);

/// A text of the default profile ([`SceString`]) written as a literal, in a
/// buffer of `$n` bytes. The literal's length against `$n` is judged at build
/// time, so a literal that does not fit is a compile error where it is
/// written and the value needs no `Result`.
#[macro_export]
macro_rules! sce_text {
    ($n:expr, $lit:literal) => {{
        const _: () = $crate::Fits::<{ $lit.len() }, { $n }>::OK;
        $crate::SceString::<{ $n }>::held($lit)
    }};
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn new_is_empty_then_push_fills() {
        let mut b = SceBytes::<4>::new();
        assert_eq!(b.as_slice(), &[] as &[u8]);
        assert!(b.push(1).is_ok());
        assert!(b.push(2).is_ok());
        assert_eq!(b.as_slice(), &[1u8, 2]);
        assert_eq!(b.len(), 2);
    }

    #[test]
    fn extend_from_slice_appends_in_order() {
        let mut b = SceBytes::<8>::new();
        assert!(b.extend_from_slice(&[1, 2, 3]).is_ok());
        assert!(b.push(4).is_ok());
        assert!(b.extend_from_slice(&[5, 6]).is_ok());
        assert_eq!(b.as_slice(), &[1u8, 2, 3, 4, 5, 6]);
    }

    // The fixed-capacity profile is a hard bound on EVERY build, not only on
    // the heap-free tier: naming `InlineBytes` under `alloc` must still refuse
    // to grow. This is the guarantee a cfg-switched single type cannot give.
    #[test]
    fn inline_profile_is_bounded_even_where_heap_exists() {
        let mut b = InlineBytes::<2>::new();
        assert!(b.push(1).is_ok());
        assert!(b.push(2).is_ok());
        assert_eq!(b.push(3), Err(CapacityExceeded));
        assert_eq!(b.as_slice(), &[1u8, 2]);
    }

    #[test]
    fn inline_extend_past_capacity_is_atomic() {
        let mut b = InlineBytes::<3>::new();
        assert!(b.push(9).is_ok());
        // Three more would exceed N=3 — heapless rejects without partial copy.
        assert_eq!(b.extend_from_slice(&[1, 2, 3]), Err(CapacityExceeded));
        assert_eq!(b.as_slice(), &[9u8]);
    }

    #[test]
    fn inline_from_slice_rejects_past_capacity() {
        assert!(InlineBytes::<3>::from_slice(&[1, 2, 3]).is_ok());
        assert_eq!(
            InlineBytes::<3>::from_slice(&[1, 2, 3, 4]),
            Err(CapacityExceeded)
        );
    }

    // alloc profile: `N` is advisory; the growable form passes it without error.
    #[cfg(feature = "alloc")]
    #[test]
    fn heap_profile_is_unbounded() {
        let mut b = HeapBytes::<2>::new();
        for i in 0..10u8 {
            assert!(b.push(i).is_ok());
        }
        assert_eq!(b.len(), 10);
        assert!(HeapBytes::<2>::from_slice(&[0; 64]).is_ok());
    }

    // A list of a wider element obeys the same bound as bytes: the fixed
    // profile refuses past `N` and keeps what it had, the default profile
    // holds the values in order. Bytes are one instance of this type, so the
    // two cannot come to disagree about overflow.
    #[test]
    fn a_list_of_i64_is_bounded_like_bytes() {
        let mut inline = InlineList::<i64, 2>::new();
        assert!(inline.push(-5).is_ok());
        assert!(inline.push(5_000_000_001).is_ok());
        assert_eq!(inline.push(0), Err(CapacityExceeded));
        assert_eq!(inline.as_slice(), &[-5i64, 5_000_000_001]);

        let mut default = SceOwnedList::<i64, 4>::new();
        assert!(default.extend_from_slice(&[1, -2, 3]).is_ok());
        assert_eq!(default, &[1i64, -2, 3]);
    }

    // The fixed text profile refuses past its bound on every build and keeps
    // nothing; the default profile holds what it is given.
    #[test]
    fn inline_text_is_bounded_and_from_text_reports_it() {
        assert_eq!(InlineStr::<3>::from_text("abc").unwrap().as_str(), "abc");
        assert_eq!(InlineStr::<3>::from_text("abcd"), Err(CapacityExceeded));
        assert_eq!(SceString::<3>::from_text("ab").unwrap(), "ab");
    }

    // `held` is total: a text past the bound is cut on a character boundary,
    // never in the middle of one.
    #[test]
    fn held_cuts_at_a_character_boundary() {
        assert_eq!(InlineStr::<4>::held("ab").as_str(), "ab");
        // `é` is two bytes, so three bytes of room keep `aé` and drop the rest.
        assert_eq!(InlineStr::<3>::held("aéb").as_str(), "aé");
        assert_eq!(InlineStr::<2>::held("aé").as_str(), "a");
    }

    // A text widens into a buffer at least as large, by type; the literal macro
    // judges its own length against the bound before the program runs.
    #[test]
    fn a_text_widens_and_a_literal_is_judged_at_build_time() {
        let small = sce_text!(4, "abcd");
        let wide = SceString::<9>::widened(&small);
        assert_eq!(wide, "abcd");
        assert_eq!(sce_text!(2, "é"), "é");
    }

    // A bounded list refuses past its bound through every constructor, on the
    // profile this test runs under whichever it is: the bound is the type's, not
    // the storage's.
    #[test]
    fn a_bounded_list_refuses_past_its_bound_through_every_constructor() {
        assert_eq!(BoundedList::<u8, 2>::CAPACITY, 2);
        let full = BoundedList::<u8, 2>::from_slice(&[1, 2]).expect("two fit two");
        assert_eq!(full.as_slice(), &[1u8, 2]);
        assert_eq!(
            BoundedList::<u8, 2>::from_slice(&[1, 2, 3]),
            Err(CapacityExceeded)
        );
        assert_eq!(
            BoundedList::<u8, 2>::try_from_iter([1u8, 2, 3]),
            Err(CapacityExceeded)
        );
        let mut grown = BoundedList::<u8, 2>::new();
        assert!(grown.push(7).is_ok());
        assert!(grown.push(8).is_ok());
        assert_eq!(grown.push(9), Err(CapacityExceeded));
        assert_eq!(grown, &[7u8, 8]);
        let mut one = BoundedList::<u8, 1>::default();
        assert!(one.is_empty());
        assert!(one.push(1).is_ok());
        assert_eq!(one.push(2), Err(CapacityExceeded));
    }

    // Under `alloc` a list is taken from a `Vec` without a copy and handed back
    // as one, the bound held on the way in.
    #[cfg(feature = "alloc")]
    #[test]
    fn a_bounded_list_takes_a_vec_within_its_bound_and_gives_it_back() {
        let list =
            BoundedList::<i64, 3>::try_from_vec(alloc::vec![-1, 0, 1]).expect("three fit three");
        assert_eq!(list.into_vec(), alloc::vec![-1i64, 0, 1]);
        assert_eq!(
            BoundedList::<i64, 3>::try_from_vec(alloc::vec![0; 4]),
            Err(CapacityExceeded)
        );
    }

    // A bounded text counts UTF-8 bytes, refuses past its bound whole and keeps
    // what it held when an append would pass it.
    #[test]
    fn a_bounded_text_refuses_past_its_bound_in_bytes() {
        assert_eq!(BoundedText::<4>::from_text("abcd").expect("fits"), "abcd");
        assert_eq!(BoundedText::<4>::from_text("abcde"), Err(CapacityExceeded));
        // `é` is two bytes: three characters, five bytes.
        assert_eq!(BoundedText::<4>::from_text("aéé"), Err(CapacityExceeded));
        let mut text = BoundedText::<3>::new();
        assert!(text.try_push_str("ab").is_ok());
        assert_eq!(text.try_push_str("cd"), Err(CapacityExceeded));
        assert_eq!(text.as_str(), "ab");
        assert_eq!(BoundedText::<3>::CAPACITY, 3);
        assert!(BoundedText::<1>::default().is_empty());
    }

    // A nested type is the composition, and each level holds its own bound: an
    // inner list past its bound cannot be built, so an outer list of them holds
    // none.
    #[test]
    fn a_nested_bounded_type_holds_each_bound_at_its_own_level() {
        type Tags = BoundedList<BoundedText<8>, 4>;
        let tag = BoundedText::<8>::from_text("eight888").expect("eight bytes fit eight");
        assert_eq!(
            BoundedText::<8>::from_text("nine99999"),
            Err(CapacityExceeded)
        );
        let tags = Tags::from_slice(&[tag.clone(), tag.clone()]).expect("two fit four");
        assert_eq!(
            Tags::try_from_iter((0..5).map(|_| tag.clone())),
            Err(CapacityExceeded)
        );
        let mut items = BoundedList::<Tags, 3>::new();
        for _ in 0..3 {
            assert!(items.push(tags.clone()).is_ok());
        }
        assert_eq!(items.push(tags), Err(CapacityExceeded));
        let mut held = 0;
        for each in &items {
            held += each.len();
        }
        assert_eq!(held, 6);
        assert_eq!(items[2][1], "eight888");
    }

    // A list widens into a list of a larger bound by type, whole or element by
    // element, on whichever profile this test runs under: the copy a machine
    // makes of a value its build proved to fit.
    #[test]
    fn a_list_widens_into_a_larger_bound_whole_or_element_by_element() {
        let small = BoundedList::<u8, 2>::from_slice(&[1, 2]).expect("two fit two");
        let wide = BoundedList::<u8, 4>::widened(&small);
        assert_eq!(wide, &[1u8, 2]);
        assert_eq!(wide.len(), 2);
        let mut room = wide.clone();
        assert!(room.push(3).is_ok());
        assert!(room.push(4).is_ok());
        assert_eq!(room.push(5), Err(CapacityExceeded));

        let words = BoundedList::<SceString<4>, 2>::try_from_iter([
            SceString::<4>::from_text("ab").expect("fits"),
            SceString::<4>::from_text("abcd").expect("fits"),
        ])
        .expect("two fit two");
        let wider = BoundedList::<SceString<8>, 3>::widened_map(&words, SceString::<8>::widened);
        assert_eq!(wider.len(), 2);
        assert_eq!(wider[0], "ab");
        assert_eq!(wider[1], "abcd");
    }

    // A list is walked by value, in order, and emptied whole; emptied, it takes
    // its bound again.
    #[test]
    fn a_list_is_walked_by_value_and_cleared() {
        let mut list = BoundedList::<i64, 3>::from_slice(&[-1, 0, 1]).expect("three fit three");
        let mut walked = BoundedList::<i64, 3>::new();
        for each in list.clone() {
            assert!(walked.push(each * 2).is_ok());
        }
        assert_eq!(walked, &[-2i64, 0, 2]);
        assert_eq!(list.clone().into_iter().enumerate().last(), Some((2, 1)));
        list.clear();
        assert!(list.is_empty());
        for each in [7, 8, 9] {
            assert!(list.push(each).is_ok());
        }
        assert_eq!(list.push(10), Err(CapacityExceeded));
    }

    // A byte string widens into a larger bound by type on both storage forms.
    #[test]
    fn a_byte_string_widens_into_a_larger_bound() {
        let small = SceBytes::<2>::from_slice(&[0, 255]).expect("two fit two");
        assert_eq!(SceBytes::<4>::widened(&small), &[0u8, 255]);
        let inline = InlineBytes::<2>::from_slice(&[9]).expect("one fits two");
        let mut wide = InlineBytes::<3>::widened(&inline);
        assert_eq!(wide, &[9u8]);
        assert!(wide.extend_from_slice(&[1, 2]).is_ok());
        assert_eq!(wide.push(3), Err(CapacityExceeded));
    }

    // A list of values that are not `Copy` compares with a slice and an array
    // of them, as a list of numbers does.
    #[test]
    fn a_list_of_texts_compares_with_a_slice() {
        let tag = BoundedText::<4>::from_text("ab").expect("fits");
        let list = BoundedList::<BoundedText<4>, 2>::from_slice(core::slice::from_ref(&tag))
            .expect("one fits two");
        assert_eq!(list, &[tag.clone()]);
        assert!(list == &[tag][..]);
    }

    // A text is formatted as the text it holds, on every text type.
    #[test]
    fn a_text_is_formatted_as_what_it_holds() {
        use core::fmt::Write;
        let mut out = heapless::String::<16>::new();
        let text = SceString::<4>::from_text("ab").expect("fits");
        let inline = InlineStr::<4>::from_text("cd").expect("fits");
        let bounded = BoundedText::<4>::from_text("é").expect("fits");
        assert!(write!(out, "{text}{inline}{bounded}").is_ok());
        assert_eq!(out.as_str(), "abcdé");
    }

    // Both profiles coexist in one binary and carry the same bytes — the
    // property that makes a per-use-site storage choice possible at all.
    #[cfg(feature = "alloc")]
    #[test]
    fn both_profiles_coexist_over_the_same_payload() {
        let payload: &[u8] = b"same wire bytes";
        let heap = HeapBytes::<32>::from_slice(payload).unwrap();
        let inline = InlineBytes::<32>::from_slice(payload).unwrap();
        assert_eq!(heap.as_slice(), inline.as_slice());
        assert_eq!(heap, payload);
        assert_eq!(inline, payload);
    }
}
