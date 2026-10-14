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
#[cfg(feature = "alloc")]
#[repr(transparent)]
#[derive(Debug, Clone, PartialEq, Default)]
pub struct HeapList<T, const N: usize>(alloc::vec::Vec<T>, core::marker::PhantomData<[T; N]>);

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
        Ok(Self(s.to_vec(), core::marker::PhantomData))
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
        Self(alloc::vec::Vec::new(), core::marker::PhantomData)
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

        impl<T: Copy + PartialEq, const N: usize> PartialEq<&[T]> for $storage<T, N> {
            fn eq(&self, other: &&[T]) -> bool {
                self.as_slice() == *other
            }
        }

        impl<T: Copy + PartialEq, const N: usize, const M: usize> PartialEq<&[T; M]>
            for $storage<T, N>
        {
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
/// [`InlineStr`] of the matching capacity.
#[cfg(feature = "alloc")]
#[repr(transparent)]
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct HeapStr<const N: usize>(alloc::string::String, core::marker::PhantomData<[u8; N]>);

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

/// The compile-time judgement that text of at most `FROM` bytes fits a buffer
/// of `TO`: naming `Fits::<FROM, TO>::OK` makes a copy that could overflow a
/// build error where it is written, which is what lets [`SceString::widened`]
/// and [`sce_text!`] be total functions.
#[doc(hidden)]
pub struct Fits<const FROM: usize, const TO: usize>;

impl<const FROM: usize, const TO: usize> Fits<FROM, TO> {
    #[doc(hidden)]
    pub const OK: () = assert!(
        FROM <= TO,
        "a text is copied into a buffer smaller than the one it was held in"
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
        Ok(Self(
            alloc::string::String::from(s),
            core::marker::PhantomData,
        ))
    }

    /// Borrow the owned text back as `&str`.
    pub fn as_str(&self) -> &str {
        self.0.as_str()
    }

    /// The text of `s`, which the caller has already held to `N` bytes.
    ///
    /// Total: a text past `N` is cut at the last character boundary that fits
    /// rather than reported, because the callers are the ones that proved the
    /// fit — a generated body whose every write was judged against this bound,
    /// and a machine that holds the text within it. Use [`Self::from_text`]
    /// for text nothing has judged.
    pub fn held(s: &str) -> Self {
        Self(
            alloc::string::String::from(prefix_within(s, N)),
            core::marker::PhantomData,
        )
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
    };
}

#[cfg(feature = "alloc")]
str_view_parity!(HeapStr);
str_view_parity!(InlineStr);

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
