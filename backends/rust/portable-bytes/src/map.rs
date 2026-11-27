// SPDX-License-Identifier: AGPL-3.0-only WITH AdditionRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael

//! The value of a `map<K, V>[N]` and of a `set<K>[N]` (docs/adr/0108, D1 to D8):
//! [`BoundedMap`] and [`BoundedSet`], one algorithm for both, the set being the
//! map whose entries carry no value.
//!
//! A map holds at most `N` entries in the order their keys were first put
//! (D2), and each type is (D8):
//!
//! * **an array of `N` entry slots** with a free list of the unused ones, so a
//!   put takes a free slot and a remove gives it back, and no operation moves
//!   an entry;
//! * **a doubly linked order through the slots**, which is the order a walk,
//!   the saved form and the snapshot follow: a put links its slot last, a
//!   replace leaves the links alone, and a remove unlinks its slot;
//! * **an open-addressing index** from the hash of a key to its slot, of twice
//!   the bound, probed linearly, with tombstones a put reuses and a rebuild
//!   clears once they outnumber the live entries; or, in [`Linear`] storage,
//!   no index, a lookup comparing the key with each key the map holds.
//!
//! The hash is SipHash-1-3 under a 128-bit key each map is built with
//! ([`MapSeed`]), over the canonical bytes of the key ([`MapKey`]), so a peer
//! that chooses the keys cannot steer them into one chain of the index
//! without the seed. Nothing a document or a host sees depends on the hash: the
//! order is the links', and a map restored under another seed is the same map.
//!
//! Where a map keeps its slots and its index is a parameter of its type, its
//! storage `S` ([`MapStorage`], D4 "Storage"), not a feature of the build.
//! [`Inline`] holds them in the map, the bound reserved, and never allocates;
//! [`Heap`], under the crate's `alloc` feature, grows them as the map is
//! filled, up to what the bound needs and never past it; [`Linear`] holds
//! the slots in the map and keeps no index, for a small bound, where an index
//! buys nothing a scan of a few keys does not: such a map hashes no key and
//! reads no seed to find one, so a program whose maps are all of it carries no
//! hash function, and a lookup makes at most `N` comparisons whatever keys a
//! peer chose. [`Lent`] and [`LentIndexed`] keep the slots, and the second
//! its index too, in a run of memory the host lends for the map's life, sized
//! when the map is built (docs/adr/0154, D1): how many entries such a map may
//! hold, its limit ([`BoundedMap::limit`]), is a value of the map no greater
//! than `N`, and the map value holds no slot, so a map lent nothing costs a few
//! words whatever its bound. Cargo turns a feature
//! on for every crate of a build that depends on this one once any of them asks
//! for it, so a storage chosen by the feature would move every map of the build
//! to the heap; chosen by type, a map that must never allocate stays inline
//! beside one that grows, in one program, as a list does in [`crate::InlineList`]
//! beside [`crate::HeapList`]. [`DefaultMapStorage`] is the storage of a map
//! whose type names none, as [`crate::SceOwnedList`] is a list's: [`Heap`]
//! under `alloc` and [`Inline`] without it. The map's algorithm is written once,
//! over what any storage gives it.
//!
//! An entry stays in the slot a put took for it until a remove takes it out.
//! In [`Lent`] and [`LentIndexed`] storage each slot keeps a generation,
//! which moves each time the slot is freed, and a host reads the slot of an
//! entry as a [`MapSlot`], its number below the map's limit and its
//! generation, so a number kept for an entry that was removed finds nothing
//! once another entry takes the slot (docs/adr/0154, D2). Every other storage
//! keeps no generation and names no slot, and its slots are as large as they
//! would be without it.
//!
//! A link between slots and a cell of the index name a slot by its number and
//! one, so the link to no slot and the cell of no key are both 0. A map built
//! in a const context ([`BoundedMap::with_seed`] is a `const fn`) with
//! `MapSeed::from_halves(0, 0)` in [`Inline`] storage is then all zero bytes,
//! whatever features the build turns on, the inline slots left uninitialised,
//! and a `static` holding it is placed in `.bss`, not `.data`. Such a map
//! hashes under a key anyone knows until [`BoundedMap::reseed`] gives it a
//! drawn one; because the map is bounded, the worst a chosen set of keys does to
//! it until then is make each lookup a scan of its `N` entries. A map in
//! [`Linear`] storage, built the same way or by `BoundedMap::new`, is all
//! zero bytes too, and holds nothing for an index. A map in [`Heap`] storage
//! is not all zero when empty: a `Vec` that holds nothing holds a dangling
//! pointer, which is not zero.

use crate::{BoundedList, SceString};
use core::marker::PhantomData;
use storage::{Generation as _, KeyIndex, Lookup, Slot, SlotStore};

// ── SipHash ─────────────────────────────────────────────────────────────────

/// SipHash with `C` compression rounds and `D` finalization rounds over a
/// 128-bit key (Aumasson and Bernstein, "SipHash: a fast short-input PRF",
/// 2012). A map hashes with SipHash-1-3 ([`SipHash13`]); the rounds are
/// parameters so the one implementation is held to the paper's SipHash-2-4
/// vectors in the tests.
#[derive(Clone)]
struct SipHash<const C: usize, const D: usize> {
    v0: u64,
    v1: u64,
    v2: u64,
    v3: u64,
    /// The bytes of the message not yet compressed, little-endian, at most 7.
    tail: u64,
    /// How many bytes `tail` holds.
    held: u32,
    /// How many bytes were written in all; its low byte ends the message.
    length: u64,
}

/// The hash of a map's index (docs/adr/0108, D8).
type SipHash13 = SipHash<1, 3>;

impl<const C: usize, const D: usize> SipHash<C, D> {
    fn new(seed: &MapSeed) -> Self {
        Self {
            v0: seed.k0 ^ 0x736f_6d65_7073_6575,
            v1: seed.k1 ^ 0x646f_7261_6e64_6f6d,
            v2: seed.k0 ^ 0x6c79_6765_6e65_7261,
            v3: seed.k1 ^ 0x7465_6462_7974_6573,
            tail: 0,
            held: 0,
            length: 0,
        }
    }

    fn round(&mut self) {
        self.v0 = self.v0.wrapping_add(self.v1);
        self.v1 = self.v1.rotate_left(13);
        self.v1 ^= self.v0;
        self.v0 = self.v0.rotate_left(32);
        self.v2 = self.v2.wrapping_add(self.v3);
        self.v3 = self.v3.rotate_left(16);
        self.v3 ^= self.v2;
        self.v0 = self.v0.wrapping_add(self.v3);
        self.v3 = self.v3.rotate_left(21);
        self.v3 ^= self.v0;
        self.v2 = self.v2.wrapping_add(self.v1);
        self.v1 = self.v1.rotate_left(17);
        self.v1 ^= self.v2;
        self.v2 = self.v2.rotate_left(32);
    }

    fn compress(&mut self, word: u64) {
        self.v3 ^= word;
        for _ in 0..C {
            self.round();
        }
        self.v0 ^= word;
    }

    fn write(&mut self, bytes: &[u8]) {
        self.length = self.length.wrapping_add(bytes.len() as u64);
        for &byte in bytes {
            self.tail |= u64::from(byte) << (8 * self.held);
            self.held += 1;
            if self.held == 8 {
                let word = self.tail;
                self.compress(word);
                self.tail = 0;
                self.held = 0;
            }
        }
    }

    fn finish(mut self) -> u64 {
        let last = ((self.length & 0xff) << 56) | self.tail;
        self.compress(last);
        self.v2 ^= 0xff;
        for _ in 0..D {
            self.round();
        }
        self.v0 ^ self.v1 ^ self.v2 ^ self.v3
    }
}

impl<const C: usize, const D: usize> KeyFeed for SipHash<C, D> {
    fn feed(&mut self, bytes: &[u8]) {
        self.write(bytes);
    }
}

// ── the seed ────────────────────────────────────────────────────────────────

/// The 128-bit key a map hashes its keys under (docs/adr/0108, D8), drawn for
/// each map when it is built. Where the program has an entropy source the
/// runtime draws it (`sce_rust_runtime::map_seed` with `std`); where it has
/// none the host hands one to the machine it builds, and a host that has no
/// entropy either hands a fixed one, which leaves a map that a peer knowing the
/// seed can flood. Formatted without its bits, so a log never shows it.
#[derive(Clone, Copy, PartialEq, Eq)]
pub struct MapSeed {
    k0: u64,
    k1: u64,
}

impl MapSeed {
    /// The seed of the 16 bytes `bytes`: the first eight its low half and the
    /// last eight its high half, each little-endian, as SipHash reads a key.
    pub const fn new(bytes: [u8; 16]) -> Self {
        let mut k0 = 0u64;
        let mut k1 = 0u64;
        let mut i = 0;
        while i < 8 {
            k0 |= (bytes[i] as u64) << (8 * i);
            k1 |= (bytes[i + 8] as u64) << (8 * i);
            i += 1;
        }
        Self { k0, k1 }
    }

    /// The seed of the two halves of a 128-bit key, low first.
    pub const fn from_halves(low: u64, high: u64) -> Self {
        Self { k0: low, k1: high }
    }

    /// The seed of the map numbered `ordinal` among those one machine holds:
    /// two SipHash-1-3 values of the number under this seed, so each map of a
    /// machine is keyed apart from the others and none of them from the seed
    /// itself.
    pub fn derive(&self, ordinal: u32) -> Self {
        let half = |side: u8| {
            let mut sip = SipHash13::new(self);
            sip.write(&[side]);
            sip.write(&ordinal.to_le_bytes());
            sip.finish()
        };
        Self::from_halves(half(0), half(1))
    }
}

impl core::fmt::Debug for MapSeed {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.write_str("MapSeed(..)")
    }
}

/// The ordinal the seed of a map or a set held as a map's value is drawn
/// under, from the seed of the map that holds it (docs/adr/0108, D12 item 7):
/// past every ordinal a machine numbers its own maps with, so no inner map is
/// keyed as one of the machine's maps is, and the one the C11 lane draws an
/// inner map's seed under (`SCE_MAP_SEED_INNER_ORDINAL`).
pub const INNER_SEED_ORDINAL: u32 = u32::MAX;

/// The seeds of the maps one machine holds, each drawn from one seed in the
/// order the machine declares its maps ([`MapSeed::derive`]): what a generated
/// machine builds its maps from, given the seed its host or its runtime drew.
#[derive(Clone, Debug)]
pub struct MapSeeds {
    seed: MapSeed,
    drawn: u32,
}

impl MapSeeds {
    /// The seeds drawn from `seed`.
    pub const fn new(seed: MapSeed) -> Self {
        Self { seed, drawn: 0 }
    }

    /// The seed of the next map.
    pub fn draw(&mut self) -> MapSeed {
        let seed = self.seed.derive(self.drawn);
        self.drawn = self.drawn.wrapping_add(1);
        seed
    }
}

/// A map or a set built empty under a seed: what a generated machine builds
/// each of its maps with, whichever of the two it is, the type named where the
/// variable is declared.
pub trait FromSeed {
    /// The empty value keyed by `seed`.
    fn from_seed(seed: MapSeed) -> Self;
}

// ── keys ────────────────────────────────────────────────────────────────────

/// Where a key writes its canonical bytes ([`MapKey`]): the hash, or a
/// comparison with another key.
pub trait KeyFeed {
    /// The next bytes of the key.
    fn feed(&mut self, bytes: &[u8]);
}

/// A type whose values are keys of a map (docs/adr/0108, D1), by the bytes
/// that say which key a value is: two values are one key exactly when they
/// write the same bytes. A fixed-width integer writes its value as the 16
/// little-endian bytes of an `i128`, whatever its own width, so a literal, a
/// narrower value or a wider one probes the key of the same number; `bool` one
/// byte; a value of an enum its declared number, as an integer; a text and a
/// byte string their length as eight little-endian bytes and then their bytes;
/// a record its fields in the order its schema declares them.
///
/// A map compares keys by these bytes and hashes them, so a key may be looked up
/// by any value of another type that writes them alike: a text slice for a
/// bounded text, a payload for the record it is read into.
pub trait MapKey {
    /// Write the canonical bytes of the key into `into`.
    fn feed_key(&self, into: &mut dyn KeyFeed);
}

macro_rules! integer_key {
    ($($t:ty),*) => {$(
        impl MapKey for $t {
            fn feed_key(&self, into: &mut dyn KeyFeed) {
                into.feed(&i128::from(*self).to_le_bytes());
            }
        }
    )*};
}
integer_key!(u8, u16, u32, u64, i8, i16, i32, i64);

impl MapKey for bool {
    fn feed_key(&self, into: &mut dyn KeyFeed) {
        into.feed(&[u8::from(*self)]);
    }
}

impl MapKey for [u8] {
    fn feed_key(&self, into: &mut dyn KeyFeed) {
        into.feed(&(self.len() as u64).to_le_bytes());
        into.feed(self);
    }
}

/// A byte-string literal (`b"ab"`), as the slice of its bytes.
impl<const L: usize> MapKey for [u8; L] {
    fn feed_key(&self, into: &mut dyn KeyFeed) {
        self.as_slice().feed_key(into);
    }
}

impl MapKey for str {
    fn feed_key(&self, into: &mut dyn KeyFeed) {
        self.as_bytes().feed_key(into);
    }
}

impl<const N: usize> MapKey for SceString<N> {
    fn feed_key(&self, into: &mut dyn KeyFeed) {
        self.as_str().feed_key(into);
    }
}

impl<const N: usize> MapKey for BoundedList<u8, N> {
    fn feed_key(&self, into: &mut dyn KeyFeed) {
        self.as_slice().feed_key(into);
    }
}

impl<const N: usize> MapKey for heapless::String<N> {
    fn feed_key(&self, into: &mut dyn KeyFeed) {
        self.as_str().feed_key(into);
    }
}

#[cfg(feature = "alloc")]
impl MapKey for alloc::string::String {
    fn feed_key(&self, into: &mut dyn KeyFeed) {
        self.as_str().feed_key(into);
    }
}

impl<T: MapKey + ?Sized> MapKey for &T {
    fn feed_key(&self, into: &mut dyn KeyFeed) {
        (**self).feed_key(into);
    }
}

/// How many bytes a key writes.
struct Count(usize);

impl KeyFeed for Count {
    fn feed(&mut self, bytes: &[u8]) {
        self.0 += bytes.len();
    }
}

/// The bytes `expected` held against the bytes a key writes from position
/// `start` on: whether every one of them is equal, and how many were met.
struct Window<'e> {
    start: usize,
    expected: &'e [u8],
    at: usize,
    met: usize,
    equal: bool,
}

impl KeyFeed for Window<'_> {
    fn feed(&mut self, bytes: &[u8]) {
        let begin = self.at;
        let end = begin + bytes.len();
        self.at = end;
        let low = begin.max(self.start);
        let high = end.min(self.start + self.expected.len());
        if low < high {
            if bytes[low - begin..high - begin]
                != self.expected[low - self.start..high - self.start]
            {
                self.equal = false;
            }
            self.met += high - low;
        }
    }
}

/// The bytes one key writes, each run held against the same run of the bytes
/// `other` writes, which is written again for each run: no key's bytes are
/// held anywhere, so two keys of any bound compare with nothing allocated.
struct Matcher<'o, O: ?Sized> {
    other: &'o O,
    at: usize,
    equal: bool,
}

impl<O: MapKey + ?Sized> KeyFeed for Matcher<'_, O> {
    fn feed(&mut self, bytes: &[u8]) {
        if self.equal && !bytes.is_empty() {
            let mut window = Window {
                start: self.at,
                expected: bytes,
                at: 0,
                met: 0,
                equal: true,
            };
            self.other.feed_key(&mut window);
            self.equal = window.equal && window.met == bytes.len();
        }
        self.at += bytes.len();
    }
}

/// Whether `a` and `b` are one key: they write the same bytes ([`MapKey`]).
fn same_key<A: MapKey + ?Sized, B: MapKey + ?Sized>(a: &A, b: &B) -> bool {
    let mut length = Count(0);
    b.feed_key(&mut length);
    let mut matcher = Matcher {
        other: b,
        at: 0,
        equal: true,
    };
    a.feed_key(&mut matcher);
    matcher.equal && matcher.at == length.0
}

/// The hash of `key` under `seed`.
fn hash_of<Q: MapKey + ?Sized>(seed: &MapSeed, key: &Q) -> u64 {
    let mut sip = SipHash13::new(seed);
    key.feed_key(&mut sip);
    sip.finish()
}

/// The cell of an index of `len` cells a hash starts its probe at: the high
/// bits of the product, which spreads a hash over any length.
fn home(hash: u64, len: usize) -> usize {
    ((u128::from(hash) * len as u128) >> 64) as usize
}

// ── the map ─────────────────────────────────────────────────────────────────

/// The end of a list of slots: the link to no slot ([`link_to`]).
const NIL: u32 = 0;
/// An index cell no key ever took.
const EMPTY: u32 = 0;
/// An index cell whose key was removed: a probe goes on past it, and a put
/// may take it. No slot's link is this, which the bound of a map keeps apart
/// (`BoundedMap::NUMBERED`).
const TOMBSTONE: u32 = u32::MAX;

/// The link to the slot numbered `slot`, as a list of slots and a cell of the
/// index hold it: its number and one, so that [`NIL`] and [`EMPTY`] are 0 and
/// an empty map is all zero bytes.
fn link_to(slot: usize) -> u32 {
    slot as u32 + 1
}

/// The number of the slot `link` names, which is not [`NIL`] ([`link_to`]).
fn slot_number(link: u32) -> usize {
    (link - 1) as usize
}

// ── storage ─────────────────────────────────────────────────────────────────

/// Where a map keeps its slots and how it finds the slot of a key
/// (docs/adr/0108, D4 "Storage"), a parameter of its type: [`Inline`], the
/// bound and a hash index reserved in the map itself; [`Heap`], the slots and
/// the index grown as the map is filled; [`Linear`], the bound reserved and
/// no index at all; or, in a run of memory the host lends for the map's life
/// (docs/adr/0154, D1), [`Lent`], the slots and no index, and
/// [`LentIndexed`], the slots and a hash index. Sealed: the map's algorithm
/// is written once over what the five give it, and no other storage can break
/// what it relies on.
pub trait MapStorage: storage::Sealed {
    /// What a slot keeps of its key beside the key for the map's lookups: the
    /// key's hash under the map's seed where the map keeps an index, nothing
    /// where it compares keys one by one.
    #[doc(hidden)]
    type Stamp: Copy;
    /// The slots of a map of `N` entries, each a key of `K` and a value of
    /// `V`.
    #[doc(hidden)]
    type Slots<K, V, const N: usize>: storage::SlotStore<K, V, Self::Stamp, Self::Generation>;
    /// How a map of `N` entries finds the slot of a key: the cells of its
    /// index, or nothing where it compares keys one by one.
    #[doc(hidden)]
    type Cells<const N: usize>: storage::KeyIndex<Self::Stamp>;
    /// The storage a copy of a map of this storage is kept in
    /// ([`BoundedMap::to_owned_storage`]), and a map a generated algorithm
    /// builds beside one it is lent (docs/adr/0154, D3): the storage itself
    /// where the map holds its own slots, and where its host lends them, the
    /// storage that holds them in the value and finds a key the same way,
    /// [`Linear`] for [`Lent`] and [`Inline`] for [`LentIndexed`], since a
    /// host lends a run to one map.
    type Owned: MapStorage<Stamp = Self::Stamp>;
    /// What each slot keeps of how many times it was freed (docs/adr/0154,
    /// D2): `u32` where the map names its slots to its host as [`MapSlot`]s,
    /// in [`Lent`] and [`LentIndexed`] storage, and `()` in every other, whose
    /// slots and positions ([`MapCursor`]) are then as large as they would be
    /// with no generation.
    type Generation: storage::Generation;

    /// The slots and the index of a copy of a map of this storage, `slots` and
    /// `index`, its order starting at `head` and holding `live` entries, in
    /// its owned storage: a copy of each where the storage is its own owned
    /// storage, as a map's clone has always been.
    #[doc(hidden)]
    fn owned_parts<K: Clone, V: Clone, const N: usize>(
        slots: &Self::Slots<K, V, N>,
        index: &Self::Cells<N>,
        head: u32,
        live: usize,
    ) -> storage::OwnedParts<Self, K, V, N>;
}

/// A map's storage held in the map, the bound reserved: `N` slots and an index
/// of `2 * N` cells, so the map never allocates, in every configuration of the
/// crate's features. An empty map of it built under the zero seed is all zero
/// bytes (see the [module](self)).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Inline;

impl storage::Sealed for Inline {}

impl MapStorage for Inline {
    type Stamp = u64;
    type Slots<K, V, const N: usize> = heapless::Vec<storage::Slot<K, V, u64, ()>, N>;
    type Cells<const N: usize> = storage::Hashed<[[u32; N]; 2]>;
    type Owned = Inline;
    type Generation = ();

    fn owned_parts<K: Clone, V: Clone, const N: usize>(
        slots: &Self::Slots<K, V, N>,
        index: &Self::Cells<N>,
        _head: u32,
        _live: usize,
    ) -> storage::OwnedParts<Self, K, V, N> {
        (slots.clone(), index.clone())
    }
}

/// A map's storage on the heap: the slots and the index grow as the map is
/// filled, up to what the bound needs and never past it, so a map of a large
/// bound pays for the entries it holds.
#[cfg(feature = "alloc")]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Heap;

#[cfg(feature = "alloc")]
impl storage::Sealed for Heap {}

#[cfg(feature = "alloc")]
impl MapStorage for Heap {
    type Stamp = u64;
    type Slots<K, V, const N: usize> = alloc::vec::Vec<storage::Slot<K, V, u64, ()>>;
    type Cells<const N: usize> = storage::Hashed<alloc::vec::Vec<u32>>;
    type Owned = Heap;
    type Generation = ();

    fn owned_parts<K: Clone, V: Clone, const N: usize>(
        slots: &Self::Slots<K, V, N>,
        index: &Self::Cells<N>,
        _head: u32,
        _live: usize,
    ) -> storage::OwnedParts<Self, K, V, N> {
        (slots.clone(), index.clone())
    }
}

/// A map's storage held in the map, the bound reserved, with no index: `N`
/// slots, and a lookup that compares the key with each key the map holds, in
/// the map's order, so it makes at most `N` comparisons (docs/adr/0108, D4
/// "Storage"). For a small bound, where an index buys nothing a scan of a few
/// keys does not: the map hashes no key and no lookup of it reads its seed, so
/// a program whose maps are all of it carries no hash function, and a peer that
/// chooses the keys can make a lookup no worse than the `N` comparisons it
/// always is. What a document or a host sees of the map, its order, what each
/// operation answers, its saved form and its equality with a map of another
/// storage, is what it sees of an [`Inline`] map given the same operations.
///
/// The map keeps the seed it was built with only to hand it on, to the maps
/// drawn from it ([`BoundedMap::derived_seed`]) and to a copy in another
/// storage, so `BoundedMap::<K, V, N, Linear>::new()` builds it with none, all
/// zero bytes, and it never allocates, in every configuration of the crate's
/// features.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Linear;

impl storage::Sealed for Linear {}

impl MapStorage for Linear {
    type Stamp = ();
    type Slots<K, V, const N: usize> = heapless::Vec<storage::Slot<K, V, (), ()>, N>;
    type Cells<const N: usize> = storage::Scan;
    type Owned = Linear;
    type Generation = ();

    fn owned_parts<K: Clone, V: Clone, const N: usize>(
        slots: &Self::Slots<K, V, N>,
        index: &Self::Cells<N>,
        _head: u32,
        _live: usize,
    ) -> storage::OwnedParts<Self, K, V, N> {
        (slots.clone(), *index)
    }
}

/// A map's storage in a run of [`LentSlot`]s its host lends for the life
/// `'a` of the map, with no index (docs/adr/0154, D1): the map may hold as many
/// entries as the run has slots, its limit ([`BoundedMap::limit`]), at most
/// `N`, and a lookup compares the key with each key the map holds, as in
/// [`Linear`] storage, so the map hashes no key. The map value holds its seed,
/// the ends of its order, its free list, its count and the run, never a slot:
/// a map of any bound lent no slot is a few words, and one lent `c` slots
/// borrows `c` slots' bytes and no more. What a document or a host sees of
/// the map, its order, what each operation answers, its saved form and its
/// equality with a map of another storage, is what it sees of an [`Inline`]
/// map given the same operations, once the map is lent `N` slots; lent fewer,
/// a put of an absent key answers [`PutOutcome::Full`] once the map holds its
/// limit, as an inline map does once it holds `N`.
///
/// Built by `BoundedMap::over`. A map built any other way
/// ([`BoundedMap::with_seed`], [`FromSeed`]) is lent no slot, its limit 0. A
/// copy of the map, which no run is lent to, is kept in [`Linear`] storage
/// ([`MapStorage::Owned`]).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Lent<'a>(PhantomData<&'a mut ()>);

impl storage::Sealed for Lent<'_> {}

impl<'a> MapStorage for Lent<'a> {
    type Stamp = ();
    type Slots<K, V, const N: usize> = storage::LentSlots<'a, K, V, (), u32>;
    type Cells<const N: usize> = storage::Scan;
    type Owned = Linear;
    type Generation = u32;

    /// Each slot taken copied into the value, at its number, without its
    /// generation, which a copy, naming no slot, does not keep.
    fn owned_parts<K: Clone, V: Clone, const N: usize>(
        slots: &Self::Slots<K, V, N>,
        _index: &Self::Cells<N>,
        _head: u32,
        _live: usize,
    ) -> storage::OwnedParts<Self, K, V, N> {
        let mut owned = heapless::Vec::new();
        for slot in slots.taken() {
            // At most the run's slots, which are at most `N`.
            if owned.push(slot.regenerated(())).is_err() {
                break;
            }
        }
        (owned, storage::Scan)
    }
}

/// A map's storage in runs its host lends for the life `'a` of the map, its
/// slots ([`LentIndexedSlot`]) and the cells of its hash index
/// ([`LentCell`]), twice as many cells as slots, so a full map leaves the
/// index half empty, as [`Inline`] storage's is (docs/adr/0154, D1). The map
/// may hold as many entries as the run has slots, its limit, at most `N`; it
/// hashes and probes as an inline map does, under its seed. What a document or
/// a host sees of it is what [`Lent`] storage says. A copy of the map is kept
/// in [`Inline`] storage ([`MapStorage::Owned`]).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct LentIndexed<'a>(PhantomData<&'a mut ()>);

impl storage::Sealed for LentIndexed<'_> {}

impl<'a> MapStorage for LentIndexed<'a> {
    type Stamp = u64;
    type Slots<K, V, const N: usize> = storage::LentSlots<'a, K, V, u64, u32>;
    type Cells<const N: usize> = storage::Hashed<storage::LentCells<'a>>;
    type Owned = Inline;
    type Generation = u32;

    /// Each slot taken copied into the value, at its number, without its
    /// generation, and the index built again at twice the bound from the
    /// slots' stamps, no key hashed again.
    fn owned_parts<K: Clone, V: Clone, const N: usize>(
        slots: &Self::Slots<K, V, N>,
        _index: &Self::Cells<N>,
        head: u32,
        live: usize,
    ) -> storage::OwnedParts<Self, K, V, N> {
        let mut owned: heapless::Vec<storage::Slot<K, V, u64, ()>, N> = heapless::Vec::new();
        for slot in slots.taken() {
            // At most the run's slots, which are at most `N`.
            if owned.push(slot.regenerated(())).is_err() {
                break;
            }
        }
        let mut index = <storage::Hashed<[[u32; N]; 2]> as KeyIndex<u64>>::VACANT;
        index.refill(owned.as_slice(), head, live, N);
        (owned, index)
    }
}

/// One slot of a map in [`Lent`] storage, held in the run its host lends: an
/// entry of the map or none, and the links of the map's order, as an
/// [`Inline`] map holds a slot in itself. The host builds each empty
/// ([`Self::EMPTY`]), lends the run to `BoundedMap::over`, and reaches what
/// it holds only through the map while the map lives; a map dropped drops the
/// entries it left in the run.
#[repr(transparent)]
pub struct LentSlot<K, V>(Slot<K, V, (), u32>);

impl<K, V> LentSlot<K, V> {
    /// A slot that holds no entry, which a host fills the run it lends with.
    pub const EMPTY: Self = Self(Slot::vacant((), 0));

    /// [`Self::EMPTY`], for a run built by a function
    /// (`core::array::from_fn`).
    pub const fn empty() -> Self {
        Self::EMPTY
    }

    /// Whether the slot holds an entry: none, once the map it was lent to is
    /// dropped.
    pub fn holds_entry(&self) -> bool {
        self.0.entry.is_some()
    }
}

/// One slot of a map in [`LentIndexed`] storage, held in the run of slots its
/// host lends, as [`LentSlot`] is in [`Lent`] storage, with the hash of its
/// key, which a rebuild of the index reads instead of hashing again.
#[repr(transparent)]
pub struct LentIndexedSlot<K, V>(Slot<K, V, u64, u32>);

impl<K, V> LentIndexedSlot<K, V> {
    /// A slot that holds no entry, which a host fills the run it lends with.
    pub const EMPTY: Self = Self(Slot::vacant(0, 0));

    /// [`Self::EMPTY`], for a run built by a function
    /// (`core::array::from_fn`).
    pub const fn empty() -> Self {
        Self::EMPTY
    }

    /// Whether the slot holds an entry: none, once the map it was lent to is
    /// dropped.
    pub fn holds_entry(&self) -> bool {
        self.0.entry.is_some()
    }
}

/// One cell of the hash index of a map in [`LentIndexed`] storage, held in the
/// run of cells its host lends, twice as many as the slots: the link to the
/// slot of a key, or none. Built empty ([`Self::EMPTY`]); a map built over a
/// run empties every cell of it first, so a run lent again holds nothing of
/// the map it was lent to before.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(transparent)]
pub struct LentCell(u32);

impl LentCell {
    /// A cell that names no slot.
    pub const EMPTY: Self = Self(EMPTY);

    /// [`Self::EMPTY`].
    pub const fn empty() -> Self {
        Self::EMPTY
    }

    /// Whether the cell names a slot: a cell the map it was lent to left
    /// naming one is emptied by the next map built over the run.
    pub const fn names_slot(self) -> bool {
        self.0 != EMPTY
    }
}

/// Why the runs a host lends a map were refused where the map is built
/// (`BoundedMap::over`): the type holds the bound, the run the limit, and
/// the storage never grows (docs/adr/0154, D1).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LendRefused {
    /// More slots were lent than the bound of the map's type.
    PastBound {
        /// How many slots were lent.
        lent: usize,
        /// The bound `N` of the map's type.
        bound: usize,
    },
    /// Fewer index cells were lent than twice the slots lent.
    IndexShort {
        /// How many cells were lent.
        cells: usize,
        /// How many the index of the slots lent needs.
        needed: usize,
    },
}

impl core::fmt::Display for LendRefused {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            Self::PastBound { lent, bound } => write!(
                f,
                "{lent} slots lent to a map bounded by {bound}, past its bound"
            ),
            Self::IndexShort { cells, needed } => write!(
                f,
                "{cells} index cells lent where the slots lent need {needed}"
            ),
        }
    }
}

/// The storage a map takes where its type names none: [`Heap`] where the
/// crate's `alloc` feature is on, as a generated machine's map variable grows
/// on a lane with a heap (docs/adr/0108, D4).
#[cfg(feature = "alloc")]
pub type DefaultMapStorage = Heap;
/// Without `alloc`, the storage a map takes where its type names none is
/// [`Inline`], the one hashed storage there is.
#[cfg(not(feature = "alloc"))]
pub type DefaultMapStorage = Inline;

/// What [`MapStorage`] is built from, out of reach outside this module: the
/// slot a map holds an entry in, the operations on the slots, and how a map
/// finds the slot of a key, by an index of a key's hash or by comparing keys
/// one by one.
mod storage {
    use core::marker::PhantomData;
    use core::ptr::NonNull;

    use super::{hash_of, home, same_key, slot_number, MapKey, MapSeed, EMPTY, NIL, TOMBSTONE};

    /// Implemented by [`super::Inline`], [`super::Heap`], [`super::Linear`],
    /// [`super::Lent`] and [`super::LentIndexed`] alone.
    pub trait Sealed {}

    /// One slot of a map: its entry, or none for a slot on the free list; what
    /// the map's lookups keep of its key, its stamp `H` (the key's hash, which a
    /// rebuild of the index reads instead of hashing again, or nothing for a
    /// map with no index); and the links to its neighbours, in the map's order
    /// for a slot in use and on the free list (`next` alone) for one that is
    /// not; and its generation `G` ([`Generation`], docs/adr/0154, D2):
    /// nothing in a storage that names no slot to its host, so its slot is as
    /// large as a slot with no generation, and in one that does, how many times
    /// the slot was freed, which a host's `MapSlot` carries.
    #[derive(Clone)]
    pub struct Slot<K, V, H, G> {
        pub(super) entry: Option<(K, V)>,
        pub(super) stamp: H,
        pub(super) prev: u32,
        pub(super) next: u32,
        pub(super) generation: G,
    }

    impl<K, V, H, G> Slot<K, V, H, G> {
        /// The slot that holds no entry, stamped `stamp`, of the generation
        /// `generation`.
        pub(super) const fn vacant(stamp: H, generation: G) -> Self {
            Self {
                entry: None,
                stamp,
                prev: NIL,
                next: NIL,
                generation,
            }
        }

        /// A copy of the slot of the generation `generation`: what a copy of a
        /// map in storage of another generation holds.
        pub(super) fn regenerated<F>(&self, generation: F) -> Slot<K, V, H, F>
        where
            K: Clone,
            V: Clone,
            H: Copy,
        {
            Slot {
                entry: self.entry.clone(),
                stamp: self.stamp,
                prev: self.prev,
                next: self.next,
                generation,
            }
        }
    }

    /// What a slot keeps of how many times it was freed (docs/adr/0154, D2):
    /// `u32` in a storage whose map names its slots to its host
    /// ([`super::Lent`], [`super::LentIndexed`]), moved each time the slot is
    /// freed and 0 for one never freed, so a map built empty in a const context
    /// is still all zero bytes; and `()` in every other, which costs a slot and
    /// a position nothing.
    pub trait Generation: Copy + Eq + core::fmt::Debug {
        /// The generation of a slot never freed.
        const FIRST: Self;
        /// Whether a slot keeps one: a clear then frees every slot it took,
        /// moving each one's generation, where a storage that keeps none
        /// gives its slots back.
        const KEPT: bool;
        /// The generation of the slot once it is freed.
        fn moved(self) -> Self;
    }

    impl Generation for () {
        const FIRST: Self = ();
        const KEPT: bool = false;
        fn moved(self) -> Self {}
    }

    impl Generation for u32 {
        const FIRST: Self = 0;
        const KEPT: bool = true;
        fn moved(self) -> Self {
            self.wrapping_add(1)
        }
    }

    /// The slots and the index of a copy of a map of the storage `S` in its
    /// owned storage (`MapStorage::owned_parts`).
    pub type OwnedParts<S, K, V, const N: usize> = (
        <<S as super::MapStorage>::Owned as super::MapStorage>::Slots<K, V, N>,
        <<S as super::MapStorage>::Owned as super::MapStorage>::Cells<N>,
    );

    /// The slots of a map, numbered from 0 in the order they were first taken.
    pub trait SlotStore<K, V, H, G>: Sized {
        /// The store of a map that has taken no slot.
        const VACANT: Self;
        /// The slots taken so far.
        fn taken(&self) -> &[Slot<K, V, H, G>];
        /// The slots taken so far, to write.
        fn taken_mut(&mut self) -> &mut [Slot<K, V, H, G>];
        /// Take the slot after the last one taken, holding `slot`: whether
        /// there was one to take.
        fn take_slot(&mut self, slot: Slot<K, V, H, G>) -> bool;
        /// How many slots the store can take for a map bounded by `bound`:
        /// the bound, or the slots its host lent (docs/adr/0154, D1).
        fn room(&self, bound: usize) -> usize;
        /// Give back every slot, the storage kept for the slots taken next.
        fn give_back(&mut self);
    }

    /// The cells of a map's index, each the link to the slot of a key
    /// (`link_to`), `EMPTY` or `TOMBSTONE`.
    pub trait CellStore: Sized {
        /// The cells of a map that holds nothing, every one `EMPTY`.
        const VACANT: Self;
        /// How many cells there are.
        fn cell_count(&self) -> usize;
        /// The cell `at`.
        fn cell(&self, at: usize) -> u32;
        /// Write `value` into the cell `at`.
        fn set_cell(&mut self, at: usize, value: u32);
        /// Every cell `EMPTY`, `len` of them where the index grows.
        fn reset(&mut self, len: usize);
    }

    /// What a lookup of a key found ([`KeyIndex::find`]).
    pub enum Lookup<P, H> {
        /// The key, in the slot `slot` links to, held at `place` by the index.
        Found { slot: u32, place: P },
        /// Not the key; the key's stamp, which a put of it keeps in its slot.
        Absent(H),
    }

    /// How a map whose slots are stamped `H` finds the slot of a key. Each
    /// operation is handed the map's slots and the head of its order; the
    /// slots and the links are the map's to write, and the index reads them.
    pub trait KeyIndex<H>: Sized + PartialEq {
        /// The index of a map that holds nothing.
        const VACANT: Self;
        /// Where the index holds a key it found, which a remove of it reads.
        type Place: Copy;
        /// Where `key` is among `slots`, whose order starts at `head`, the
        /// map keyed by `seed`; or, where the map does not hold it, its stamp.
        fn find<K: MapKey, V, G, Q: MapKey + ?Sized>(
            &self,
            seed: &MapSeed,
            slots: &[Slot<K, V, H, G>],
            head: u32,
            key: &Q,
        ) -> Lookup<Self::Place, H>;
        /// Take in the entry the map has just linked last at `at`, stamped as
        /// [`Self::find`] answered, the map holding `live` entries before it
        /// and limited to `limit` (`BoundedMap::limit`).
        fn enter<K, V, G>(
            &mut self,
            slots: &[Slot<K, V, H, G>],
            head: u32,
            at: u32,
            live: usize,
            limit: usize,
        );
        /// Let go of the key found at `place`, whose slot the map has taken out
        /// of its order, the map now holding `live` entries.
        fn leave<K, V, G>(
            &mut self,
            slots: &[Slot<K, V, H, G>],
            head: u32,
            place: Self::Place,
            live: usize,
        );
        /// Hold no key, the storage kept.
        fn empty(&mut self);
        /// Stamp each entry again under `seed` and take them all in again.
        fn restamp<K: MapKey, V, G>(
            &mut self,
            seed: &MapSeed,
            slots: &mut [Slot<K, V, H, G>],
            head: u32,
        );
        /// Take in every entry of `slots` linked from `head`, each stamped as
        /// it is, into an index that holds no key, the map holding `live`
        /// entries and limited to `limit`: the index of a copy of a map.
        fn refill<K, V, G>(
            &mut self,
            slots: &[Slot<K, V, H, G>],
            head: u32,
            live: usize,
            limit: usize,
        );
        /// How many cells a removed key left that a rebuild has not cleared.
        #[cfg(test)]
        fn tombstones(&self) -> u32;
    }

    /// An open-addressing index (docs/adr/0108, D8): the cells `C`, twice the
    /// bound at most, so a full map leaves it half empty, probed linearly from
    /// the cell a key's hash starts at, and how many of them are tombstones,
    /// which a put reuses and a rebuild clears once they outnumber the live
    /// entries. Each slot's stamp is its key's hash under the map's seed.
    #[derive(Debug, Clone, PartialEq, Eq)]
    pub struct Hashed<C> {
        pub(super) cells: C,
        pub(super) tombstones: u32,
    }

    impl<C: CellStore> Hashed<C> {
        /// The index again, `len` cells long, with every entry linked from
        /// `head` at the cell its stamp first probes free and no tombstone left.
        fn rebuild<K, V, G>(&mut self, slots: &[Slot<K, V, u64, G>], head: u32, len: usize) {
            self.cells.reset(len);
            self.tombstones = 0;
            if len == 0 {
                return;
            }
            let mut at = head;
            while at != NIL {
                let slot = &slots[slot_number(at)];
                let mut cell = home(slot.stamp, len);
                while self.cells.cell(cell) != EMPTY {
                    cell = if cell + 1 == len { 0 } else { cell + 1 };
                }
                self.cells.set_cell(cell, at);
                at = slot.next;
            }
        }

        /// How long the index must be for `occupied` cells to fill at most half
        /// of it: its own length while that does, and otherwise doubled from it,
        /// from eight for an index never grown, never past `most`. An inline
        /// index is `most` long from the start, and never grows.
        fn length_for(&self, occupied: usize, most: usize) -> usize {
            let mut len = self.cells.cell_count();
            if len == 0 {
                len = most.min(8);
            }
            while occupied.saturating_mul(2) > len && len < most {
                len = len.saturating_mul(2).min(most);
            }
            len
        }
    }

    impl<C: CellStore + PartialEq> KeyIndex<u64> for Hashed<C> {
        const VACANT: Self = Self {
            cells: <C as CellStore>::VACANT,
            tombstones: 0,
        };
        type Place = usize;

        /// The probe from the cell the key's hash starts at, past tombstones
        /// and the cells of other keys, to the key or to an empty cell.
        fn find<K: MapKey, V, G, Q: MapKey + ?Sized>(
            &self,
            seed: &MapSeed,
            slots: &[Slot<K, V, u64, G>],
            _head: u32,
            key: &Q,
        ) -> Lookup<usize, u64> {
            let hash = hash_of(seed, key);
            let len = self.cells.cell_count();
            let mut cell = home(hash, len);
            for _ in 0..len {
                match self.cells.cell(cell) {
                    EMPTY => break,
                    TOMBSTONE => {}
                    at => {
                        let slot = &slots[slot_number(at)];
                        let found = slot.stamp == hash
                            && slot
                                .entry
                                .as_ref()
                                .is_some_and(|(held, _)| same_key(held, key));
                        if found {
                            return Lookup::Found {
                                slot: at,
                                place: cell,
                            };
                        }
                    }
                }
                cell = if cell + 1 == len { 0 } else { cell + 1 };
            }
            Lookup::Absent(hash)
        }

        /// Room for one more occupied cell, at most half of the index occupied
        /// while it can still grow: an index that grows is built again with
        /// every entry, this one last; otherwise the entry takes the first
        /// tombstone or empty cell its probe meets.
        fn enter<K, V, G>(
            &mut self,
            slots: &[Slot<K, V, u64, G>],
            head: u32,
            at: u32,
            live: usize,
            limit: usize,
        ) {
            let occupied = live + self.tombstones as usize + 1;
            let len = self.length_for(occupied, limit.saturating_mul(2));
            if len != self.cells.cell_count() {
                self.rebuild(slots, head, len);
                return;
            }
            let mut cell = home(slots[slot_number(at)].stamp, len);
            for _ in 0..len {
                let held = self.cells.cell(cell);
                if held == EMPTY || held == TOMBSTONE {
                    if held == TOMBSTONE {
                        self.tombstones -= 1;
                    }
                    self.cells.set_cell(cell, at);
                    return;
                }
                cell = if cell + 1 == len { 0 } else { cell + 1 };
            }
            // Every cell holds a key, which the length above rules out; a
            // rebuild at the same length takes the entry in all the same.
            self.rebuild(slots, head, len);
        }

        fn leave<K, V, G>(
            &mut self,
            slots: &[Slot<K, V, u64, G>],
            head: u32,
            place: usize,
            live: usize,
        ) {
            self.cells.set_cell(place, TOMBSTONE);
            self.tombstones += 1;
            if self.tombstones as usize > live {
                let len = self.cells.cell_count();
                self.rebuild(slots, head, len);
            }
        }

        fn empty(&mut self) {
            let len = self.cells.cell_count();
            self.cells.reset(len);
            self.tombstones = 0;
        }

        /// Each entry's hash taken again under `seed`, and the index built
        /// again at its length, no tombstone left.
        fn restamp<K: MapKey, V, G>(
            &mut self,
            seed: &MapSeed,
            slots: &mut [Slot<K, V, u64, G>],
            head: u32,
        ) {
            let mut at = head;
            while at != NIL {
                let slot = &mut slots[slot_number(at)];
                if let Some((key, _)) = &slot.entry {
                    slot.stamp = hash_of(seed, key);
                }
                at = slot.next;
            }
            let len = self.cells.cell_count();
            self.rebuild(slots, head, len);
        }

        /// The index at the length the map's entries need, each at the cell
        /// its stamp first probes free: an index that grows holds no cell for a
        /// map that holds nothing.
        fn refill<K, V, G>(
            &mut self,
            slots: &[Slot<K, V, u64, G>],
            head: u32,
            live: usize,
            limit: usize,
        ) {
            if live == 0 {
                return;
            }
            let len = self.length_for(live, limit.saturating_mul(2));
            self.rebuild(slots, head, len);
        }

        #[cfg(test)]
        fn tombstones(&self) -> u32 {
            self.tombstones
        }
    }

    /// No index ([`super::Linear`]): a lookup compares the key with each key
    /// the map holds, in the map's order, so it makes at most as many
    /// comparisons as the map's bound whatever keys a peer chose. It hashes
    /// nothing, reads no seed and holds nothing.
    #[derive(Debug, Clone, Copy, PartialEq, Eq)]
    pub struct Scan;

    impl KeyIndex<()> for Scan {
        const VACANT: Self = Scan;
        type Place = ();

        fn find<K: MapKey, V, G, Q: MapKey + ?Sized>(
            &self,
            _seed: &MapSeed,
            slots: &[Slot<K, V, (), G>],
            head: u32,
            key: &Q,
        ) -> Lookup<(), ()> {
            let mut at = head;
            while at != NIL {
                let slot = &slots[slot_number(at)];
                if slot
                    .entry
                    .as_ref()
                    .is_some_and(|(held, _)| same_key(held, key))
                {
                    return Lookup::Found {
                        slot: at,
                        place: (),
                    };
                }
                at = slot.next;
            }
            Lookup::Absent(())
        }

        fn enter<K, V, G>(
            &mut self,
            _slots: &[Slot<K, V, (), G>],
            _head: u32,
            _at: u32,
            _live: usize,
            _limit: usize,
        ) {
        }

        fn leave<K, V, G>(
            &mut self,
            _slots: &[Slot<K, V, (), G>],
            _head: u32,
            _place: (),
            _live: usize,
        ) {
        }

        fn empty(&mut self) {}

        fn restamp<K: MapKey, V, G>(
            &mut self,
            _seed: &MapSeed,
            _slots: &mut [Slot<K, V, (), G>],
            _head: u32,
        ) {
        }

        fn refill<K, V, G>(
            &mut self,
            _slots: &[Slot<K, V, (), G>],
            _head: u32,
            _live: usize,
            _limit: usize,
        ) {
        }

        #[cfg(test)]
        fn tombstones(&self) -> u32 {
            0
        }
    }

    impl<K, V, H: Copy, G: Copy, const N: usize> SlotStore<K, V, H, G>
        for heapless::Vec<Slot<K, V, H, G>, N>
    {
        const VACANT: Self = heapless::Vec::new();

        fn taken(&self) -> &[Slot<K, V, H, G>] {
            self.as_slice()
        }

        fn taken_mut(&mut self) -> &mut [Slot<K, V, H, G>] {
            self.as_mut_slice()
        }

        fn take_slot(&mut self, slot: Slot<K, V, H, G>) -> bool {
            self.push(slot).is_ok()
        }

        fn room(&self, _bound: usize) -> usize {
            N
        }

        fn give_back(&mut self) {
            self.clear();
        }
    }

    /// Two rows of `N`, twice the bound, so a full map leaves the index half
    /// empty; held whole from the start.
    impl<const N: usize> CellStore for [[u32; N]; 2] {
        const VACANT: Self = [[EMPTY; N]; 2];

        fn cell_count(&self) -> usize {
            2 * N
        }

        fn cell(&self, at: usize) -> u32 {
            self[at / N][at % N]
        }

        fn set_cell(&mut self, at: usize, value: u32) {
            self[at / N][at % N] = value;
        }

        fn reset(&mut self, _len: usize) {
            *self = Self::VACANT;
        }
    }

    #[cfg(feature = "alloc")]
    impl<K, V, H: Copy, G: Copy> SlotStore<K, V, H, G> for alloc::vec::Vec<Slot<K, V, H, G>> {
        const VACANT: Self = alloc::vec::Vec::new();

        fn taken(&self) -> &[Slot<K, V, H, G>] {
            self.as_slice()
        }

        fn taken_mut(&mut self) -> &mut [Slot<K, V, H, G>] {
            self.as_mut_slice()
        }

        fn take_slot(&mut self, slot: Slot<K, V, H, G>) -> bool {
            self.push(slot);
            true
        }

        fn room(&self, bound: usize) -> usize {
            bound
        }

        fn give_back(&mut self) {
            self.clear();
        }
    }

    /// Grown by the map as it fills, to twice the bound at most.
    #[cfg(feature = "alloc")]
    impl CellStore for alloc::vec::Vec<u32> {
        const VACANT: Self = alloc::vec::Vec::new();

        fn cell_count(&self) -> usize {
            self.len()
        }

        fn cell(&self, at: usize) -> u32 {
            self[at]
        }

        fn set_cell(&mut self, at: usize, value: u32) {
            self[at] = value;
        }

        fn reset(&mut self, len: usize) {
            self.clear();
            self.resize(len, EMPTY);
        }
    }

    /// A run of `T`s a host lent for `'a` (docs/adr/0154, D1), held by its
    /// start and its length: the borrow a map was built over, its lifetime kept
    /// by the marker so the run outlives the region, and its elements' type in
    /// no lifetime bound, since a storage names its slots for keys and values of
    /// any type (`MapStorage::Slots`) and could not state that they
    /// outlive `'a`, which the borrow the region was made from already proved.
    /// The start is a raw pointer, so the region is invariant in `T` as the
    /// borrow is.
    pub struct Region<'a, T> {
        base: *mut T,
        len: usize,
        lent: PhantomData<&'a mut ()>,
    }

    impl<'a, T> Region<'a, T> {
        /// The empty run, at an address no element is read at.
        const VACANT: Self = Self {
            base: NonNull::dangling().as_ptr(),
            len: 0,
            lent: PhantomData,
        };

        /// The run `lent`, each of its elements read as the `T` it is laid
        /// out as.
        fn over<U: LaidOutAs<T>>(lent: &'a mut [U]) -> Self {
            Self {
                base: lent.as_mut_ptr().cast::<T>(),
                len: lent.len(),
                lent: PhantomData,
            }
        }

        /// How many elements the run holds.
        pub(super) fn length(&self) -> usize {
            self.len
        }

        /// Where the run starts, which a test holds to the address the host
        /// lent.
        #[cfg(test)]
        pub(super) fn start(&self) -> *const T {
            self.base
        }

        /// The run, to read.
        pub(super) fn whole(&self) -> &[T] {
            // SAFETY: `base` and `len` are those of the `&'a mut [U]` the
            // region was made from (`Self::over`), `U` laid out as `T`
            // (`LaidOutAs`), or the empty run at a dangling, aligned address
            // (`Self::VACANT`): `len` initialised `T`s, aligned, in one
            // allocation. The region took that borrow by value and holds it for
            // `'a`, which it cannot outlive (`lent`), so nothing else reaches
            // the run while it lives, and this view lasts no longer than the
            // borrow of `self`.
            unsafe { core::slice::from_raw_parts(self.base, self.len) }
        }

        /// The run, to write.
        pub(super) fn whole_mut(&mut self) -> &mut [T] {
            // SAFETY: as `Self::whole`; the region holds the borrow it was
            // made from exclusively, and `&mut self` makes this view the only
            // one while it lasts.
            unsafe { core::slice::from_raw_parts_mut(self.base, self.len) }
        }
    }

    // SAFETY: a region is the `&'a mut [T]` it was made from, held by its
    // parts, which is `Send` where `T` is: moving the region moves that
    // exclusive borrow, as moving the reference does.
    unsafe impl<T: Send> Send for Region<'_, T> {}

    // SAFETY: a `&Region` gives out only `&[T]` (`Region::whole`), so sharing
    // it shares the elements as sharing `&mut [T]` does, which is `Sync` where
    // `T` is.
    unsafe impl<T: Sync> Sync for Region<'_, T> {}

    /// A type a host's run holds that is laid out as `T`, so the run is read as
    /// a run of `T` (`Region::over`).
    ///
    /// # Safety
    ///
    /// `Self` is `#[repr(transparent)]` over `T`.
    pub unsafe trait LaidOutAs<T> {}

    // SAFETY: `LentSlot<K, V>` is `#[repr(transparent)]` over `Slot<K, V, (), u32>`.
    unsafe impl<K, V> LaidOutAs<Slot<K, V, (), u32>> for super::LentSlot<K, V> {}

    // SAFETY: `LentIndexedSlot<K, V>` is `#[repr(transparent)]` over
    // `Slot<K, V, u64, u32>`.
    unsafe impl<K, V> LaidOutAs<Slot<K, V, u64, u32>> for super::LentIndexedSlot<K, V> {}

    // SAFETY: `LentCell` is `#[repr(transparent)]` over `u32`.
    unsafe impl LaidOutAs<u32> for super::LentCell {}

    /// The slots of a map in lent storage ([`super::Lent`],
    /// [`super::LentIndexed`]): the run its host lent, of which the first
    /// `taken` are the slots the map has taken, never more than the run holds.
    pub struct LentSlots<'a, K, V, H, G> {
        pub(super) region: Region<'a, Slot<K, V, H, G>>,
        pub(super) taken: usize,
    }

    impl<'a, K, V, H, G> LentSlots<'a, K, V, H, G> {
        /// The run `lent`, no slot of it taken: whatever an earlier map left
        /// in it is overwritten as the map takes each slot.
        pub(super) fn over<U: LaidOutAs<Slot<K, V, H, G>>>(lent: &'a mut [U]) -> Self {
            Self {
                region: Region::over(lent),
                taken: 0,
            }
        }
    }

    impl<K, V, H: Copy, G: Copy> SlotStore<K, V, H, G> for LentSlots<'_, K, V, H, G> {
        const VACANT: Self = Self {
            region: Region::VACANT,
            taken: 0,
        };

        fn taken(&self) -> &[Slot<K, V, H, G>] {
            &self.region.whole()[..self.taken]
        }

        fn taken_mut(&mut self) -> &mut [Slot<K, V, H, G>] {
            let taken = self.taken;
            &mut self.region.whole_mut()[..taken]
        }

        fn take_slot(&mut self, slot: Slot<K, V, H, G>) -> bool {
            let at = self.taken;
            match self.region.whole_mut().get_mut(at) {
                Some(place) => {
                    *place = slot;
                    self.taken = at + 1;
                    true
                }
                None => false,
            }
        }

        fn room(&self, _bound: usize) -> usize {
            self.region.length()
        }

        /// Every slot taken dropped of its entry, the run kept whole.
        fn give_back(&mut self) {
            let taken = self.taken;
            for slot in &mut self.region.whole_mut()[..taken] {
                slot.entry = None;
            }
            self.taken = 0;
        }
    }

    /// A map dropped drops the entries it left in its host's run; the slots
    /// stay, holding none, the host's to lend again.
    impl<K, V, H, G> Drop for LentSlots<'_, K, V, H, G> {
        fn drop(&mut self) {
            let taken = self.taken;
            for slot in &mut self.region.whole_mut()[..taken] {
                slot.entry = None;
            }
        }
    }

    /// The cells of a map's index in [`super::LentIndexed`] storage: the run
    /// its host lent, twice as many cells as the slots, never grown, since the
    /// map takes no more entries than the run of slots holds.
    pub struct LentCells<'a>(pub(super) Region<'a, u32>);

    impl<'a> LentCells<'a> {
        /// The run `lent`, read as it holds: a map built over it empties it.
        pub(super) fn over(lent: &'a mut [super::LentCell]) -> Self {
            Self(Region::over(lent))
        }
    }

    impl CellStore for LentCells<'_> {
        const VACANT: Self = Self(Region::VACANT);

        fn cell_count(&self) -> usize {
            self.0.length()
        }

        fn cell(&self, at: usize) -> u32 {
            self.0.whole()[at]
        }

        fn set_cell(&mut self, at: usize, value: u32) {
            self.0.whole_mut()[at] = value;
        }

        /// Every cell `EMPTY`. The run keeps its length: an index is only ever
        /// asked for the length it holds, twice the slots lent, which is twice
        /// the map's limit.
        fn reset(&mut self, len: usize) {
            debug_assert_eq!(len, self.0.length(), "a lent index is never resized");
            self.0.whole_mut().fill(EMPTY);
        }
    }

    impl PartialEq for LentCells<'_> {
        fn eq(&self, other: &Self) -> bool {
            self.0.whole() == other.0.whole()
        }
    }
}

/// What [`BoundedMap::put`] did (docs/adr/0108, D4), which a statechart's
/// `<sce:put outcome>` writes as the variant of the same name of the standard
/// `put_outcome` enum.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PutOutcome {
    /// The key was absent, and its entry is now the last in the map's order.
    Inserted,
    /// The key was present, and its value is replaced where it stands.
    Replaced,
    /// The key was absent and the map holds its limit, its bound or the slots
    /// its host lent it (docs/adr/0154, D1): the map is as it was.
    Full,
}

/// What [`BoundedMap::remove`] did (docs/adr/0108, D4), which a statechart's
/// `<sce:remove outcome>` writes as the variant of the same name of the
/// standard `remove_outcome` enum.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RemoveOutcome {
    /// The key was present, and its entry is gone; the others keep their order.
    Removed,
    /// The key was absent: the map is as it was.
    Absent,
}

/// The slot an entry of a map in [`Lent`] or [`LentIndexed`] storage is held
/// in (docs/adr/0154, D2): its number, from 0 and
/// below the map's limit ([`BoundedMap::limit`]), and the generation of the
/// slot, which moves each time the slot is freed. An entry keeps its slot from
/// the put that inserts it to the remove that takes it out (no operation of
/// the map moves an entry, docs/adr/0108, D8), so a host may index a table of
/// its own by the number; the generation tells a number kept for an entry that
/// was removed from the entry that took the slot after it, which
/// [`BoundedMap::at_slot`] does not answer for it. Laid out as two `uint32`s,
/// the number first, as a pool's slot index is (docs/adr/0111, D4).
///
/// A number names a slot of the map that answered it. A copy of the map
/// ([`BoundedMap::to_owned_storage`]) holds each entry at the same number, in
/// storage that keeps no generation and names no slot; a map built again from
/// a saved form holds them at slots of its own. A generation counts a slot's
/// frees modulo 2^32, so a number kept across 2^32 frees of its slot is read
/// as current again.
#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct MapSlot {
    /// The slot's number, from 0.
    pub index: u32,
    /// How many times the slot was freed before the entry was put in it.
    pub generation: u32,
}

/// A position in a map's order, which a walk of the map steps through
/// ([`BoundedMap::first`], [`BoundedMap::next_after`]) and reads the entry at
/// ([`BoundedMap::key_at`], [`BoundedMap::value_at`]) without holding a borrow
/// of the map between two reads. It stays where it is while the map is not
/// written; a generated machine walks a copy of a map its loop writes
/// (docs/adr/0108, D6 item 3).
///
/// `G` is the generation its map's slots keep (`MapStorage::Generation`):
/// `()` in every storage but the two a host lends, so a position is one
/// `u32`, and `u32` in those, where a position read before its slot was
/// freed and taken again names no entry and answers its [`MapSlot`]
/// (docs/adr/0154, D2).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct MapCursor<G = ()> {
    /// The link to the slot of the entry (`link_to`), never `NIL`.
    link: u32,
    /// The generation of that slot when the position was read.
    generation: G,
}

impl MapCursor<u32> {
    /// The slot of the entry at this position (docs/adr/0154, D2).
    pub fn slot(self) -> MapSlot {
        MapSlot {
            index: self.link - 1,
            generation: self.generation,
        }
    }
}

/// A map of at most `N` entries, each a key of `K` and a value of `V`, no two
/// keys alike, in the order each key was first put (docs/adr/0108): the value
/// of a `map<K, V>[N]`, its slots and its index kept in the storage `S`
/// ([`Inline`], [`Heap`], [`Linear`], [`Lent`] or [`LentIndexed`],
/// [`DefaultMapStorage`] where the type names none). See the [module](self)
/// for how it holds them.
pub struct BoundedMap<K, V, const N: usize, S: MapStorage = DefaultMapStorage> {
    seed: MapSeed,
    slots: S::Slots<K, V, N>,
    /// How the map finds the slot of a key: an index of twice the bound at
    /// most, so a full map leaves it half empty, or nothing in [`Linear`]
    /// storage.
    index: S::Cells<N>,
    head: u32,
    tail: u32,
    free: u32,
    live: u32,
}

impl<K, V, const N: usize, S: MapStorage> BoundedMap<K, V, N, S> {
    /// The bound the type declares.
    pub const CAPACITY: usize = N;

    /// A bound the link to each slot holds ([`link_to`]), with [`NIL`] and
    /// [`TOMBSTONE`] apart: named where a map is built, so a bound past it does
    /// not compile.
    const NUMBERED: () = assert!(N < (u32::MAX - 1) as usize, "a map's bound is a uint32");

    /// The empty map, its keys hashed under `seed`.
    ///
    /// A `const fn` for every storage, so a `static` can hold a map. Built
    /// with `MapSeed::from_halves(0, 0)` in [`Inline`] storage, the map is all
    /// zero bytes, its inline slots uninitialised, and the `static` is placed
    /// in `.bss`, not `.data` (see the [module](self)). Its keys then hash
    /// under a key anyone knows until [`Self::reseed`] gives it a drawn one, and
    /// a peer that chooses them can make each lookup a scan of the `N` entries,
    /// no worse, the map being bounded (docs/adr/0108, D8). In [`Heap`] storage
    /// the empty map is not all zero: its `Vec`s hold a dangling pointer, which
    /// is not zero. In [`Linear`] storage no lookup reads the seed: the map
    /// keeps it only to hand on ([`Self::derived_seed`]). In [`Lent`] and
    /// [`LentIndexed`] storage the map built so is lent no slot, its limit 0;
    /// such a map is built over the run its host lends by `over`.
    pub const fn with_seed(seed: MapSeed) -> Self {
        let () = Self::NUMBERED;
        Self {
            seed,
            slots: <S::Slots<K, V, N> as SlotStore<K, V, S::Stamp, S::Generation>>::VACANT,
            index: <S::Cells<N> as KeyIndex<S::Stamp>>::VACANT,
            head: NIL,
            tail: NIL,
            free: NIL,
            live: 0,
        }
    }

    /// The empty map of the seed this one was built with: what a variable is
    /// declared with again before its machine starts.
    pub fn empty_like(&self) -> Self {
        Self::with_seed(self.seed)
    }

    /// The seed numbered `ordinal` drawn from the one this map was built with
    /// ([`MapSeed::derive`]): what a map an algorithm builds for itself is
    /// keyed by, from the map its caller lent it (docs/adr/0108, D8). The
    /// seed itself is not handed out.
    pub fn derived_seed(&self, ordinal: u32) -> MapSeed {
        self.seed.derive(ordinal)
    }

    /// The seed of each map or set this map holds as a value (docs/adr/0108,
    /// D12 item 7): drawn from this map's own under [`INNER_SEED_ORDINAL`],
    /// one ordinal at every depth, each depth's seed drawn from the one above
    /// it, as the C11 lane draws it (`sce_map_seed_of_inner`). What an empty
    /// value put at a key is built under ([`FromSeed`]).
    pub fn inner_seed(&self) -> MapSeed {
        self.seed.derive(INNER_SEED_ORDINAL)
    }

    /// How many entries the map holds.
    pub fn len(&self) -> usize {
        self.live as usize
    }

    /// Whether the map holds no entry.
    pub fn is_empty(&self) -> bool {
        self.live == 0
    }

    /// How many entries the map may hold (docs/adr/0154, D1): the bound `N`,
    /// or, in [`Lent`] and [`LentIndexed`] storage, the slots its host lent
    /// it, at most `N`. A put of an absent key into a map that holds its limit
    /// answers [`PutOutcome::Full`].
    pub fn limit(&self) -> usize {
        self.slots.room(N)
    }

    /// Empty the map, which then takes up to its limit of entries again. Its
    /// storage is kept for the entries it takes next. In a storage whose slots
    /// keep a generation ([`Lent`], [`LentIndexed`]) every slot it took is
    /// freed, so a [`MapSlot`] read for an entry it held names no entry
    /// (docs/adr/0154, D2), and the puts after it take the slots from the
    /// first; in every other its slots are given back.
    pub fn clear(&mut self) {
        let mut free = NIL;
        if <S::Generation as storage::Generation>::KEPT {
            for (number, slot) in self.slots.taken_mut().iter_mut().enumerate().rev() {
                if slot.entry.take().is_some() {
                    slot.generation = slot.generation.moved();
                }
                slot.prev = NIL;
                slot.next = free;
                free = link_to(number);
            }
        } else {
            self.slots.give_back();
        }
        self.index.empty();
        self.head = NIL;
        self.tail = NIL;
        self.free = free;
        self.live = 0;
    }

    /// The entries, in the map's order.
    pub fn iter(&self) -> Iter<'_, K, V, N, S> {
        Iter {
            map: self,
            at: self.head,
            left: self.len(),
        }
    }

    /// The position of the first entry, or `None` for an empty map.
    pub fn first(&self) -> Option<MapCursor<S::Generation>> {
        (self.head != NIL).then(|| self.cursor(self.head))
    }

    /// The position of the entry after `at`, or `None` after the last.
    pub fn next_after(&self, at: MapCursor<S::Generation>) -> Option<MapCursor<S::Generation>> {
        let next = self.slots.taken().get(slot_number(at.link))?.next;
        (next != NIL).then(|| self.cursor(next))
    }

    /// A copy of the map in the storage that holds its own slots
    /// ([`MapStorage::Owned`], docs/adr/0154, D3): each entry in the slot of
    /// the same number, in the same order, under the same seed, so the copy
    /// answers every operation as the map does. What a map whose host lent its
    /// slots is copied by, the run being lent to one map, its slots' generations
    /// left behind, since the copy names no slot; what a walk of a generated
    /// algorithm copies its map by, whatever storage the map is in. For a map
    /// that holds its own slots it is its [`Clone`], the slots and the index
    /// copied as they stand.
    pub fn to_owned_storage(&self) -> BoundedMap<K, V, N, S::Owned>
    where
        K: Clone,
        V: Clone,
    {
        let (slots, index) = S::owned_parts(&self.slots, &self.index, self.head, self.len());
        BoundedMap {
            seed: self.seed,
            slots,
            index,
            head: self.head,
            tail: self.tail,
            free: self.free,
            live: self.live,
        }
    }

    /// The key of the entry at `at`.
    ///
    /// # Panics
    ///
    /// When no entry is at `at`: the map was written since the position was
    /// read, which a walk of a map it writes does on a copy instead.
    pub fn key_at(&self, at: MapCursor<S::Generation>) -> &K {
        &self.entry_at(at).0
    }

    /// The value of the entry at `at`.
    ///
    /// # Panics
    ///
    /// As [`Self::key_at`].
    pub fn value_at(&self, at: MapCursor<S::Generation>) -> &V {
        &self.entry_at(at).1
    }

    fn entry_at(&self, at: MapCursor<S::Generation>) -> &(K, V) {
        match self
            .slots
            .taken()
            .get(slot_number(at.link))
            .filter(|slot| slot.generation == at.generation)
            .and_then(|slot| slot.entry.as_ref())
        {
            Some(entry) => entry,
            None => panic!("a map position names no entry: the map was written during its walk"),
        }
    }

    /// The slot `at` links to, which is not [`NIL`].
    fn slot(&self, at: u32) -> &Slot<K, V, S::Stamp, S::Generation> {
        &self.slots.taken()[slot_number(at)]
    }

    /// The position of the entry in the slot `at` links to.
    fn cursor(&self, at: u32) -> MapCursor<S::Generation> {
        MapCursor {
            link: at,
            generation: self.slot(at).generation,
        }
    }

    /// The slot `at` links to, to write.
    fn slot_mut(&mut self, at: u32) -> &mut Slot<K, V, S::Stamp, S::Generation> {
        &mut self.slots.taken_mut()[slot_number(at)]
    }

    /// Take a slot for an entry, from the free list or past the slots taken so
    /// far, link it last, and give the link to it. `None` when no slot is
    /// left, which a map that holds fewer than its limit always has. A freed
    /// slot keeps its generation.
    fn store(&mut self, key: K, value: V, stamp: S::Stamp) -> Option<u32> {
        let tail = self.tail;
        let at = if self.free != NIL {
            let at = self.free;
            let slot = self.slot_mut(at);
            let free = slot.next;
            slot.entry = Some((key, value));
            slot.stamp = stamp;
            slot.prev = tail;
            slot.next = NIL;
            self.free = free;
            at
        } else {
            let at = self.slots.taken().len();
            let slot = Slot {
                entry: Some((key, value)),
                stamp,
                prev: tail,
                next: NIL,
                generation: <S::Generation as storage::Generation>::FIRST,
            };
            if at >= N || !self.slots.take_slot(slot) {
                return None;
            }
            link_to(at)
        };
        if self.tail == NIL {
            self.head = at;
        } else {
            let tail = self.tail;
            self.slot_mut(tail).next = at;
        }
        self.tail = at;
        Some(at)
    }

    /// Take the slot `at` out of the map's order and put it on the free list,
    /// its entry dropped and its generation moved on (docs/adr/0154, D2).
    fn release(&mut self, at: u32) {
        let (prev, next) = (self.slot(at).prev, self.slot(at).next);
        if prev == NIL {
            self.head = next;
        } else {
            self.slot_mut(prev).next = next;
        }
        if next == NIL {
            self.tail = prev;
        } else {
            self.slot_mut(next).prev = prev;
        }
        let free = self.free;
        let slot = self.slot_mut(at);
        slot.entry = None;
        slot.generation = slot.generation.moved();
        slot.prev = NIL;
        slot.next = free;
        self.free = at;
    }
}

impl<K: MapKey, V, const N: usize, S: MapStorage> BoundedMap<K, V, N, S> {
    /// Whether `key` is a key of the map.
    pub fn contains_key<Q: MapKey + ?Sized>(&self, key: &Q) -> bool {
        self.link_of(key).is_some()
    }

    /// The value at `key`, or `None` where it is absent.
    pub fn get<Q: MapKey + ?Sized>(&self, key: &Q) -> Option<&V> {
        let at = self.link_of(key)?;
        self.slot(at).entry.as_ref().map(|(_, value)| value)
    }

    /// The value at `key`, lent to be changed where the map holds it, or
    /// `None` where it is absent: what a write into a value the map holds goes
    /// through (docs/adr/0108, D12 item 2). The key, its slot, its place in the
    /// map's order, the index and the free slots are all left as they are, so a
    /// value that is itself a map, a set or a list is written without being
    /// taken out and put back.
    pub fn get_mut<Q: MapKey + ?Sized>(&mut self, key: &Q) -> Option<&mut V> {
        let at = self.link_of(key)?;
        self.slot_mut(at).entry.as_mut().map(|(_, value)| value)
    }

    /// Set the value at `key` to `value`: a key that is present keeps its place
    /// and takes the value, and one that is absent goes last while the map
    /// holds fewer entries than its limit ([`Self::limit`]). Into a full map
    /// an absent key is not put, and the map is as it was
    /// ([`PutOutcome::Full`]).
    pub fn put(&mut self, key: K, value: V) -> PutOutcome {
        self.put_link(key, value).0
    }

    /// [`Self::put`], answering beside its outcome the link to the slot the
    /// key's entry is held in after it, `None` with [`PutOutcome::Full`].
    fn put_link(&mut self, key: K, value: V) -> (PutOutcome, Option<u32>) {
        let stamp = match self
            .index
            .find(&self.seed, self.slots.taken(), self.head, &key)
        {
            Lookup::Found { slot, .. } => {
                if let Some(entry) = self.slot_mut(slot).entry.as_mut() {
                    entry.1 = value;
                }
                return (PutOutcome::Replaced, Some(slot));
            }
            Lookup::Absent(stamp) => stamp,
        };
        let live = self.len();
        let limit = self.limit();
        if live >= limit {
            return (PutOutcome::Full, None);
        }
        let Some(at) = self.store(key, value, stamp) else {
            return (PutOutcome::Full, None);
        };
        self.index
            .enter(self.slots.taken(), self.head, at, live, limit);
        self.live += 1;
        (PutOutcome::Inserted, Some(at))
    }

    /// Take the entry at `key` out of the map, the others keeping their order
    /// and their slots; a key that is absent leaves the map as it was.
    pub fn remove<Q: MapKey + ?Sized>(&mut self, key: &Q) -> RemoveOutcome {
        let Lookup::Found { slot, place } =
            self.index
                .find(&self.seed, self.slots.taken(), self.head, key)
        else {
            return RemoveOutcome::Absent;
        };
        self.release(slot);
        self.live -= 1;
        let live = self.len();
        self.index.leave(self.slots.taken(), self.head, place, live);
        RemoveOutcome::Removed
    }

    /// Key the map by `seed` from now on, its entries and their order as they
    /// were: each entry's hash is taken again under `seed` and the index is
    /// built again at its length, no tombstone left; a map in [`Linear`]
    /// storage, which hashes no key, keeps the seed to hand on. What a map
    /// built in a const context under a known seed (`with_seed`) is given a
    /// drawn seed by, once the program has one (docs/adr/0108, D8).
    pub fn reseed(&mut self, seed: MapSeed) {
        self.seed = seed;
        self.index
            .restamp(&self.seed, self.slots.taken_mut(), self.head);
    }

    /// The link to the slot of `key`, or `None` where the map does not hold
    /// it.
    fn link_of<Q: MapKey + ?Sized>(&self, key: &Q) -> Option<u32> {
        match self
            .index
            .find(&self.seed, self.slots.taken(), self.head, key)
        {
            Lookup::Found { slot, .. } => Some(slot),
            Lookup::Absent(_) => None,
        }
    }
}

/// The slots of a map in storage whose slots keep a generation, [`Lent`] and
/// [`LentIndexed`] (docs/adr/0154, D2): what a host reads of where each entry
/// is held. A storage that keeps none names no slot, and costs nothing for it.
impl<K, V, const N: usize, S: MapStorage<Generation = u32>> BoundedMap<K, V, N, S> {
    /// The entry held in `slot`, or `None` where the slot holds none, where
    /// it holds an entry put after the one `slot` was read for (its
    /// generation has moved since), or where the number names no slot the map
    /// took.
    pub fn at_slot(&self, slot: MapSlot) -> Option<(&K, &V)> {
        let held = self.slots.taken().get(slot.index as usize)?;
        if held.generation != slot.generation {
            return None;
        }
        held.entry.as_ref().map(|(key, value)| (key, value))
    }

    /// The [`MapSlot`] of the slot `at` links to.
    fn map_slot(&self, at: u32) -> MapSlot {
        MapSlot {
            index: at - 1,
            generation: self.slot(at).generation,
        }
    }
}

impl<K: MapKey, V, const N: usize, S: MapStorage<Generation = u32>> BoundedMap<K, V, N, S> {
    /// [`Self::put`], answering beside its outcome the slot the key's entry is
    /// held in after it (docs/adr/0154, D2): the slot it was put in, or the
    /// one it stands in where its value was replaced; `None` with
    /// [`PutOutcome::Full`].
    pub fn put_slot(&mut self, key: K, value: V) -> (PutOutcome, Option<MapSlot>) {
        let (outcome, at) = self.put_link(key, value);
        (outcome, at.map(|at| self.map_slot(at)))
    }

    /// The slot of the entry at `key` (docs/adr/0154, D2), or `None` where
    /// the map does not hold it.
    pub fn slot_of<Q: MapKey + ?Sized>(&self, key: &Q) -> Option<MapSlot> {
        self.link_of(key).map(|at| self.map_slot(at))
    }
}

impl<K: MapKey, V, const N: usize, S: MapStorage<Owned = S>> BoundedMap<K, V, N, S> {
    /// The entries of a map of at most `M` entries, in its order, each key and
    /// value made by `key` and `value` of the one it holds, in a map of `N`:
    /// the copy a machine makes of a map into a place of a larger bound, or of
    /// texts of larger bounds, which its build proved the map fits
    /// (docs/adr/0108, D2). The fit of the bounds is a build error where it
    /// does not hold ([`crate::Fits`]), and a copy of a key is the same key, so
    /// no entry is refused. The copy hashes under the seed of the map it copies,
    /// whose storage may be another than its own; its own holds its slots, a
    /// run lent to one map being no place to copy into (docs/adr/0154, D3).
    pub fn widened_map<K2, V2, const M: usize, S2: MapStorage>(
        from: &BoundedMap<K2, V2, M, S2>,
        mut key: impl FnMut(&K2) -> K,
        mut value: impl FnMut(&V2) -> V,
    ) -> Self {
        let () = crate::Fits::<M, N>::OK;
        let mut map = Self::with_seed(from.seed);
        for (k, v) in from.iter() {
            // At most `M <= N` keys, none of them alike.
            if map.put(key(k), value(v)) != PutOutcome::Inserted {
                break;
            }
        }
        map
    }
}

impl<'a, K, V, const N: usize> BoundedMap<K, V, N, Lent<'a>> {
    /// The empty map over the run of slots `slots` its host lends for the
    /// map's life (docs/adr/0154, D1): its limit is the run's length, which may
    /// be 0, and a run longer than the bound `N` is refused
    /// ([`LendRefused::PastBound`]). The map hashes no key: `seed` is kept
    /// only to hand on, to the maps drawn from it ([`Self::derived_seed`]) and
    /// to a copy ([`Self::to_owned_storage`]). Whatever an earlier map left in
    /// the run is overwritten as this one takes each slot.
    pub fn over(seed: MapSeed, slots: &'a mut [LentSlot<K, V>]) -> Result<Self, LendRefused> {
        let () = Self::NUMBERED;
        if slots.len() > N {
            return Err(LendRefused::PastBound {
                lent: slots.len(),
                bound: N,
            });
        }
        Ok(Self {
            seed,
            slots: storage::LentSlots::over(slots),
            index: storage::Scan,
            head: NIL,
            tail: NIL,
            free: NIL,
            live: 0,
        })
    }
}

impl<'a, K, V, const N: usize> BoundedMap<K, V, N, LentIndexed<'a>> {
    /// The empty map over the run of slots `slots` and the run of index cells
    /// `cells` its host lends for the map's life, its keys hashed under `seed`
    /// (docs/adr/0154, D1): its limit is the length of `slots`, which may be
    /// 0, and its index the first twice as many cells of `cells`, each emptied.
    /// A run of slots longer than the bound `N` is refused
    /// ([`LendRefused::PastBound`]), and a run of cells shorter than twice the
    /// slots ([`LendRefused::IndexShort`]).
    pub fn over(
        seed: MapSeed,
        slots: &'a mut [LentIndexedSlot<K, V>],
        cells: &'a mut [LentCell],
    ) -> Result<Self, LendRefused> {
        let () = Self::NUMBERED;
        if slots.len() > N {
            return Err(LendRefused::PastBound {
                lent: slots.len(),
                bound: N,
            });
        }
        let needed = slots.len().saturating_mul(2);
        if cells.len() < needed {
            return Err(LendRefused::IndexShort {
                cells: cells.len(),
                needed,
            });
        }
        let (cells, _) = cells.split_at_mut(needed);
        let mut index = storage::Hashed {
            cells: storage::LentCells::over(cells),
            tombstones: 0,
        };
        index.empty();
        Ok(Self {
            seed,
            slots: storage::LentSlots::over(slots),
            index,
            head: NIL,
            tail: NIL,
            free: NIL,
            live: 0,
        })
    }
}

impl<K, V, const N: usize, S: MapStorage> FromSeed for BoundedMap<K, V, N, S> {
    fn from_seed(seed: MapSeed) -> Self {
        Self::with_seed(seed)
    }
}

/// A list hashes nothing, so its empty value is the one whatever the seed: what
/// a map whose value is a list puts at a key that names no value
/// (docs/adr/0108, D12 item 4), as it puts the empty map or set of a map of
/// those.
impl<T, const N: usize> FromSeed for BoundedList<T, N> {
    fn from_seed(_seed: MapSeed) -> Self {
        Self::new()
    }
}

impl<K, V, const N: usize> BoundedMap<K, V, N, Linear> {
    /// The empty map in [`Linear`] storage, which needs no seed, since none of
    /// its lookups reads one: a `const fn`, all zero bytes, so a `static`
    /// holding it is placed in `.bss`. The seed it hands on
    /// ([`Self::derived_seed`]) is then the zero seed, which anyone knows,
    /// until [`Self::reseed`] gives it one.
    pub const fn new() -> Self {
        Self::with_seed(MapSeed::from_halves(0, 0))
    }
}

/// The empty map in [`Linear`] storage ([`BoundedMap::new`]).
impl<K, V, const N: usize> Default for BoundedMap<K, V, N, Linear> {
    fn default() -> Self {
        Self::new()
    }
}

/// A copy holds the entries in the same slots and the same order, under the
/// same seed, in storage of the same kind ([`BoundedMap::to_owned_storage`]).
/// A map in storage its host lends is copied into the storage that holds its
/// own slots, the run being lent to one map (docs/adr/0154, D3).
impl<K: Clone, V: Clone, const N: usize, S: MapStorage<Owned = S>> Clone
    for BoundedMap<K, V, N, S>
{
    fn clone(&self) -> Self {
        self.to_owned_storage()
    }
}

/// Formatted as its entries in order, `{7: 5, 9: -2}`.
impl<K: core::fmt::Debug, V: core::fmt::Debug, const N: usize, S: MapStorage> core::fmt::Debug
    for BoundedMap<K, V, N, S>
{
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.debug_map().entries(self.iter()).finish()
    }
}

/// Two maps are equal when they hold equal entries in one order, whatever
/// their seeds and whatever storage each keeps them in: what a host compares
/// two snapshots by. A document compares no two maps (docs/adr/0108, D2).
impl<K: PartialEq, V: PartialEq, const N: usize, S: MapStorage, S2: MapStorage>
    PartialEq<BoundedMap<K, V, N, S2>> for BoundedMap<K, V, N, S>
{
    fn eq(&self, other: &BoundedMap<K, V, N, S2>) -> bool {
        self.len() == other.len()
            && self
                .iter()
                .zip(other.iter())
                .all(|((k1, v1), (k2, v2))| k1 == k2 && v1 == v2)
    }
}

impl<K: Eq, V: Eq, const N: usize, S: MapStorage> Eq for BoundedMap<K, V, N, S> {}

/// The entries of a map in its order, each its key and its value.
pub struct Iter<'m, K, V, const N: usize, S: MapStorage = DefaultMapStorage> {
    map: &'m BoundedMap<K, V, N, S>,
    at: u32,
    left: usize,
}

impl<'m, K, V, const N: usize, S: MapStorage> Iterator for Iter<'m, K, V, N, S> {
    type Item = (&'m K, &'m V);

    fn next(&mut self) -> Option<Self::Item> {
        while self.at != NIL {
            let slot = self.map.slot(self.at);
            self.at = slot.next;
            if let Some((key, value)) = &slot.entry {
                self.left = self.left.saturating_sub(1);
                return Some((key, value));
            }
        }
        None
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        (self.left, Some(self.left))
    }
}

impl<K, V, const N: usize, S: MapStorage> ExactSizeIterator for Iter<'_, K, V, N, S> {}

impl<'m, K, V, const N: usize, S: MapStorage> IntoIterator for &'m BoundedMap<K, V, N, S> {
    type Item = (&'m K, &'m V);
    type IntoIter = Iter<'m, K, V, N, S>;

    fn into_iter(self) -> Self::IntoIter {
        self.iter()
    }
}

// ── the set ─────────────────────────────────────────────────────────────────

/// A set of at most `N` keys of `K`, no two alike, in the order each was first
/// put (docs/adr/0108, D1): the value of a `set<K>[N]`, the map whose entries
/// carry no value, kept in the storage `S` as a map is.
pub struct BoundedSet<K, const N: usize, S: MapStorage = DefaultMapStorage>(
    BoundedMap<K, (), N, S>,
);

impl<K, const N: usize, S: MapStorage> BoundedSet<K, N, S> {
    /// The bound the type declares.
    pub const CAPACITY: usize = N;

    /// The empty set, its keys hashed under `seed`: a `const fn`, all zero
    /// bytes under `MapSeed::from_halves(0, 0)` in [`Inline`] storage, as
    /// [`BoundedMap::with_seed`] says.
    pub const fn with_seed(seed: MapSeed) -> Self {
        Self(BoundedMap::with_seed(seed))
    }

    /// The empty set of the seed this one was built with.
    pub fn empty_like(&self) -> Self {
        Self(self.0.empty_like())
    }

    /// The seed numbered `ordinal` drawn from the one this set was built with
    /// ([`BoundedMap::derived_seed`]).
    pub fn derived_seed(&self, ordinal: u32) -> MapSeed {
        self.0.derived_seed(ordinal)
    }

    /// How many keys the set holds.
    pub fn len(&self) -> usize {
        self.0.len()
    }

    /// Whether the set holds no key.
    pub fn is_empty(&self) -> bool {
        self.0.is_empty()
    }

    /// How many keys the set may hold: its bound, or the slots its host lent
    /// it ([`BoundedMap::limit`]).
    pub fn limit(&self) -> usize {
        self.0.limit()
    }

    /// Empty the set ([`BoundedMap::clear`]).
    pub fn clear(&mut self) {
        self.0.clear();
    }

    /// A copy of the set in the storage that holds its own slots, each key in
    /// the slot it holds here ([`BoundedMap::to_owned_storage`]).
    pub fn to_owned_storage(&self) -> BoundedSet<K, N, S::Owned>
    where
        K: Clone,
    {
        BoundedSet(self.0.to_owned_storage())
    }

    /// The keys, in the set's order.
    pub fn iter(&self) -> SetIter<'_, K, N, S> {
        SetIter(self.0.iter())
    }

    /// The position of the first key, or `None` for an empty set.
    pub fn first(&self) -> Option<MapCursor<S::Generation>> {
        self.0.first()
    }

    /// The position of the key after `at`, or `None` after the last.
    pub fn next_after(&self, at: MapCursor<S::Generation>) -> Option<MapCursor<S::Generation>> {
        self.0.next_after(at)
    }

    /// The key at `at`.
    ///
    /// # Panics
    ///
    /// As [`BoundedMap::key_at`].
    pub fn key_at(&self, at: MapCursor<S::Generation>) -> &K {
        self.0.key_at(at)
    }
}

impl<K: MapKey, const N: usize, S: MapStorage> BoundedSet<K, N, S> {
    /// Whether `key` is in the set.
    pub fn contains_key<Q: MapKey + ?Sized>(&self, key: &Q) -> bool {
        self.0.contains_key(key)
    }

    /// Put `key` in the set: a key it holds stays where it is
    /// ([`PutOutcome::Replaced`]), and one it does not goes last while the set
    /// holds fewer than `N` ([`PutOutcome::Full`] otherwise, the set as it was).
    pub fn put(&mut self, key: K) -> PutOutcome {
        self.0.put(key, ())
    }

    /// Take `key` out of the set, the others keeping their order.
    pub fn remove<Q: MapKey + ?Sized>(&mut self, key: &Q) -> RemoveOutcome {
        self.0.remove(key)
    }

    /// Key the set by `seed` from now on, its keys and their order as they
    /// were ([`BoundedMap::reseed`]).
    pub fn reseed(&mut self, seed: MapSeed) {
        self.0.reseed(seed);
    }
}

/// The slots of a set in storage whose slots keep a generation, as a map's
/// (docs/adr/0154, D2).
impl<K, const N: usize, S: MapStorage<Generation = u32>> BoundedSet<K, N, S> {
    /// The key held in `slot`, or `None` where the slot holds none or a key
    /// put after the one `slot` was read for ([`BoundedMap::at_slot`]).
    pub fn at_slot(&self, slot: MapSlot) -> Option<&K> {
        self.0.at_slot(slot).map(|(key, ())| key)
    }
}

impl<K: MapKey, const N: usize, S: MapStorage<Generation = u32>> BoundedSet<K, N, S> {
    /// [`Self::put`], answering beside its outcome the slot the key is held in
    /// after it ([`BoundedMap::put_slot`]).
    pub fn put_slot(&mut self, key: K) -> (PutOutcome, Option<MapSlot>) {
        self.0.put_slot(key, ())
    }

    /// The slot `key` is held in, or `None` where the set does not hold it
    /// ([`BoundedMap::slot_of`]).
    pub fn slot_of<Q: MapKey + ?Sized>(&self, key: &Q) -> Option<MapSlot> {
        self.0.slot_of(key)
    }
}

impl<K: MapKey, const N: usize, S: MapStorage<Owned = S>> BoundedSet<K, N, S> {
    /// The keys of a set of at most `M`, in its order, each made by `key` of
    /// the one it holds, in a set of `N` ([`BoundedMap::widened_map`]), the
    /// set copied kept in any storage.
    pub fn widened_map<K2, const M: usize, S2: MapStorage>(
        from: &BoundedSet<K2, M, S2>,
        key: impl FnMut(&K2) -> K,
    ) -> Self {
        Self(BoundedMap::widened_map(&from.0, key, |_| ()))
    }
}

impl<'a, K, const N: usize> BoundedSet<K, N, Lent<'a>> {
    /// The empty set over the run of slots `slots` its host lends for the
    /// set's life (`BoundedMap::over` in [`Lent`] storage).
    pub fn over(seed: MapSeed, slots: &'a mut [LentSlot<K, ()>]) -> Result<Self, LendRefused> {
        BoundedMap::<K, (), N, Lent<'a>>::over(seed, slots).map(Self)
    }
}

impl<'a, K, const N: usize> BoundedSet<K, N, LentIndexed<'a>> {
    /// The empty set over the runs of slots and index cells its host lends
    /// for the set's life (`BoundedMap::over` in [`LentIndexed`] storage).
    pub fn over(
        seed: MapSeed,
        slots: &'a mut [LentIndexedSlot<K, ()>],
        cells: &'a mut [LentCell],
    ) -> Result<Self, LendRefused> {
        BoundedMap::<K, (), N, LentIndexed<'a>>::over(seed, slots, cells).map(Self)
    }
}

impl<K, const N: usize, S: MapStorage> FromSeed for BoundedSet<K, N, S> {
    fn from_seed(seed: MapSeed) -> Self {
        Self::with_seed(seed)
    }
}

impl<K, const N: usize> BoundedSet<K, N, Linear> {
    /// The empty set in [`Linear`] storage, all zero bytes, as
    /// [`BoundedMap::new`] says.
    pub const fn new() -> Self {
        Self(BoundedMap::<K, (), N, Linear>::new())
    }
}

/// The empty set in [`Linear`] storage ([`BoundedSet::new`]).
impl<K, const N: usize> Default for BoundedSet<K, N, Linear> {
    fn default() -> Self {
        Self::new()
    }
}

/// A copy holds the keys in the same slots and the same order, in storage
/// that holds its own slots, as a map's copy does.
impl<K: Clone, const N: usize, S: MapStorage<Owned = S>> Clone for BoundedSet<K, N, S> {
    fn clone(&self) -> Self {
        Self(self.0.clone())
    }
}

/// Two sets are equal when they hold equal keys in one order, whatever their
/// seeds and storage, as two maps are.
impl<K: PartialEq, const N: usize, S: MapStorage, S2: MapStorage> PartialEq<BoundedSet<K, N, S2>>
    for BoundedSet<K, N, S>
{
    fn eq(&self, other: &BoundedSet<K, N, S2>) -> bool {
        self.0 == other.0
    }
}

impl<K: Eq, const N: usize, S: MapStorage> Eq for BoundedSet<K, N, S> {}

/// Formatted as its keys in order, `{"red", "blue"}`.
impl<K: core::fmt::Debug, const N: usize, S: MapStorage> core::fmt::Debug for BoundedSet<K, N, S> {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.debug_set().entries(self.iter()).finish()
    }
}

/// The keys of a set in its order.
pub struct SetIter<'s, K, const N: usize, S: MapStorage = DefaultMapStorage>(Iter<'s, K, (), N, S>);

impl<'s, K, const N: usize, S: MapStorage> Iterator for SetIter<'s, K, N, S> {
    type Item = &'s K;

    fn next(&mut self) -> Option<Self::Item> {
        self.0.next().map(|(key, ())| key)
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        self.0.size_hint()
    }
}

impl<K, const N: usize, S: MapStorage> ExactSizeIterator for SetIter<'_, K, N, S> {}

impl<'s, K, const N: usize, S: MapStorage> IntoIterator for &'s BoundedSet<K, N, S> {
    type Item = &'s K;
    type IntoIter = SetIter<'s, K, N, S>;

    fn into_iter(self) -> Self::IntoIter {
        self.iter()
    }
}

// ── what an expression reads ────────────────────────────────────────────────

/// A map or a set, as the reads of a statechart's expression take either
/// (docs/adr/0108, D3): how many entries it holds and whether a key is one of
/// them.
pub trait Keyed {
    /// How many entries it holds, a `uint32` as a list's length is.
    fn count(&self) -> u32;
    /// Whether `key` is one of its keys.
    fn holds<Q: MapKey + ?Sized>(&self, key: &Q) -> bool;
}

impl<K: MapKey, V, const N: usize, S: MapStorage> Keyed for BoundedMap<K, V, N, S> {
    fn count(&self) -> u32 {
        self.live
    }
    fn holds<Q: MapKey + ?Sized>(&self, key: &Q) -> bool {
        self.contains_key(key)
    }
}

impl<K: MapKey, const N: usize, S: MapStorage> Keyed for BoundedSet<K, N, S> {
    fn count(&self) -> u32 {
        self.0.count()
    }
    fn holds<Q: MapKey + ?Sized>(&self, key: &Q) -> bool {
        self.0.contains_key(key)
    }
}

/// `len(m)`: how many entries `map` holds.
pub fn count<M: Keyed + ?Sized>(map: &M) -> u32 {
    map.count()
}

/// `has(m, k)`: whether `key` is a key of `map`.
pub fn holds<M: Keyed + ?Sized, Q: MapKey + ?Sized>(map: &M, key: &Q) -> bool {
    map.holds(key)
}

/// `m[k].field`: the value at `key`, lent, for a read of a part of it; `None`
/// where the key is absent, which the read fails on as an index outside a list
/// does.
pub fn value<'m, K: MapKey, V, Q: MapKey + ?Sized, const N: usize, S: MapStorage>(
    map: &'m BoundedMap<K, V, N, S>,
    key: &Q,
) -> Option<&'m V> {
    map.get(key)
}

/// `m[k]`: the value at `key`, a copy the place it is written to takes; `None`
/// where the key is absent.
pub fn value_cloned<K: MapKey, V: Clone, Q: MapKey + ?Sized, const N: usize, S: MapStorage>(
    map: &BoundedMap<K, V, N, S>,
    key: &Q,
) -> Option<V> {
    map.get(key).cloned()
}

/// `m[k]` as the place a nested write changes, the `target` of
/// `<sce:put target="m[k]">` and its kin (docs/adr/0108, D12): the value at
/// `key`, lent to be written where the map holds it ([`BoundedMap::get_mut`]);
/// `None` where the key is absent, which the write fails on as a read of
/// [`value`] does (D12 item 3), no entry being made for it.
pub fn value_mut<'m, K: MapKey, V, Q: MapKey + ?Sized, const N: usize, S: MapStorage>(
    map: &'m mut BoundedMap<K, V, N, S>,
    key: &Q,
) -> Option<&'m mut V> {
    map.get_mut(key)
}

/// `m[k].field` in an algorithm that does not declare `may-fail`, where the
/// build proved `key` held (a guard of `has(m, k)` with nothing between that
/// writes the map or the key, docs/adr/0108, D6 item 2): the value at `key`,
/// lent, read as a proven index of a list is read.
///
/// # Panics
///
/// Where `key` is absent, which the proof the build made rules out.
pub fn held<'m, K: MapKey, V, Q: MapKey + ?Sized, const N: usize, S: MapStorage>(
    map: &'m BoundedMap<K, V, N, S>,
    key: &Q,
) -> &'m V {
    match map.get(key) {
        Some(value) => value,
        None => panic!("a key the build proved held is absent from the map"),
    }
}

/// `m[k]` where the build proved `key` held ([`held`]): a copy of the value
/// at `key`.
///
/// # Panics
///
/// As [`held`].
pub fn held_cloned<K: MapKey, V: Clone, Q: MapKey + ?Sized, const N: usize, S: MapStorage>(
    map: &BoundedMap<K, V, N, S>,
    key: &Q,
) -> V {
    held(map, key).clone()
}

#[cfg(test)]
mod tests {
    use super::storage::{CellStore, Scan};
    use super::*;

    const SEED: MapSeed = MapSeed::from_halves(0x0123_4567_89ab_cdef, 0xfedc_ba98_7654_3210);
    const OTHER_SEED: MapSeed = MapSeed::from_halves(0x0f1e_2d3c_4b5a_6978, 0x8796_a5b4_c3d2_e1f0);

    /// The key of the paper's vectors: the bytes 0 to 15.
    fn paper_key() -> MapSeed {
        let mut bytes = [0u8; 16];
        for (i, byte) in bytes.iter_mut().enumerate() {
            *byte = i as u8;
        }
        MapSeed::new(bytes)
    }

    fn sip24(seed: &MapSeed, message: &[u8]) -> u64 {
        let mut sip = SipHash::<2, 4>::new(seed);
        sip.write(message);
        sip.finish()
    }

    /// The one implementation of the rounds, held to the SipHash-2-4 vectors of
    /// the reference: the paper's own (Appendix A, a message of the bytes 0 to
    /// 14) and the empty message of its reference implementation's table, both
    /// under the key of the bytes 0 to 15. SipHash-1-3 is the same code with
    /// other round counts.
    #[test]
    fn siphash_meets_the_reference_vectors() {
        let key = paper_key();
        assert_eq!(sip24(&key, &[]), 0x726f_db47_dd0e_0e31);
        let message: [u8; 15] = core::array::from_fn(|i| i as u8);
        assert_eq!(sip24(&key, &message), 0xa129_ca61_49be_45e5);
        // A message written in pieces hashes as the same message written whole.
        let mut pieces = SipHash::<2, 4>::new(&key);
        pieces.write(&message[..3]);
        pieces.write(&message[3..11]);
        pieces.write(&message[11..]);
        assert_eq!(pieces.finish(), 0xa129_ca61_49be_45e5);
        // SipHash-1-3 is another function of the same key and message.
        let mut sip13 = SipHash13::new(&key);
        sip13.write(&message);
        assert_ne!(sip13.finish(), 0xa129_ca61_49be_45e5);
    }

    /// Two values are one key exactly when they write the same bytes: an
    /// integer whatever its width, a text whatever holds it, and two texts
    /// whose bytes run together alike are not one key.
    #[test]
    fn keys_are_equal_by_their_canonical_bytes() {
        assert!(same_key(&7u8, &7i32));
        assert!(same_key(&7u64, &7u16));
        assert!(!same_key(&7u32, &8u32));
        assert!(!same_key(&-1i32, &u32::MAX));
        let held = SceString::<8>::from_text("red").expect("fits");
        assert!(same_key(&held, "red"));
        assert!(!same_key(&held, "re"));
        assert!(!same_key(&held, "redd"));
        // A byte string, held or written as a literal, is the key of its bytes.
        let bytes = BoundedList::<u8, 4>::from_slice(&[0x00, 0xff]).expect("fits");
        assert!(same_key(&bytes, b"\x00\xff"));
        assert!(same_key(&bytes, &[0x00u8, 0xff][..]));
        assert!(!same_key(&bytes, b"\x00\xff\x00"));
        assert!(same_key(&true, &true));
        assert!(!same_key(&true, &false));
        assert_eq!(hash_of(&SEED, &held), hash_of(&SEED, "red"));
        assert_eq!(hash_of(&SEED, &7u8), hash_of(&SEED, &7i64));
    }

    /// A record of two keys, written field by field.
    #[derive(Debug, Clone, Copy, PartialEq, Eq)]
    struct Pair {
        low: u64,
        high: u64,
    }

    impl MapKey for Pair {
        fn feed_key(&self, into: &mut dyn KeyFeed) {
            self.low.feed_key(into);
            self.high.feed_key(into);
        }
    }

    /// A record key is its fields in order: the halves swapped are another key.
    #[test]
    fn a_record_key_is_its_fields_in_order() {
        let mut seen = BoundedMap::<Pair, u32, 4>::with_seed(SEED);
        let one = Pair { low: 1, high: 0 };
        let swapped = Pair { low: 0, high: 1 };
        assert_eq!(seen.put(one, 1), PutOutcome::Inserted);
        assert_eq!(seen.put(swapped, 1), PutOutcome::Inserted);
        assert_eq!(seen.put(Pair { low: 11, high: 1 }, 1), PutOutcome::Inserted);
        assert_eq!(seen.put(Pair { low: 1, high: 11 }, 1), PutOutcome::Inserted);
        assert_eq!(seen.put(one, 2), PutOutcome::Replaced);
        assert_eq!(seen.get(&one), Some(&2));
        assert_eq!(seen.get(&swapped), Some(&1));
        assert_eq!(seen.len(), 4);
    }

    /// docs/adr/0108, D2 and D4: a put of a key that is present replaces its
    /// value where it stands, a removed key put again goes last, a full map
    /// answers `Full` and is as it was, and a key it holds is replaced however
    /// full it is.
    #[test]
    fn a_map_keeps_the_order_its_keys_were_first_put() {
        let mut map = BoundedMap::<u8, u8, 3>::with_seed(SEED);
        assert_eq!(map.put(1, 10), PutOutcome::Inserted);
        assert_eq!(map.put(2, 20), PutOutcome::Inserted);
        assert_eq!(map.put(3, 30), PutOutcome::Inserted);
        assert_eq!(map.put(1, 11), PutOutcome::Replaced);
        let entries = |map: &BoundedMap<u8, u8, 3>| -> heapless::Vec<(u8, u8), 3> {
            map.iter().map(|(k, v)| (*k, *v)).collect()
        };
        assert_eq!(entries(&map).as_slice(), &[(1, 11), (2, 20), (3, 30)]);
        let before = map.clone();
        assert_eq!(map.put(4, 40), PutOutcome::Full);
        assert_eq!(map, before);
        assert_eq!(map.put(2, 21), PutOutcome::Replaced);
        assert_eq!(map.remove(&1), RemoveOutcome::Removed);
        assert_eq!(map.remove(&1), RemoveOutcome::Absent);
        assert_eq!(map.put(1, 12), PutOutcome::Inserted);
        assert_eq!(entries(&map).as_slice(), &[(2, 21), (3, 30), (1, 12)]);
        map.clear();
        assert!(map.is_empty());
        assert_eq!(map.first(), None);
        assert_eq!(map.put(5, 50), PutOutcome::Inserted);
        assert_eq!(entries(&map).as_slice(), &[(5, 50)]);
    }

    /// docs/adr/0108, D8: a remove moves no entry. Each other entry stays in
    /// the slot it was put in, and a walk by positions meets them in order.
    #[test]
    fn a_remove_moves_no_entry() {
        let mut map = BoundedMap::<u16, u16, 8>::with_seed(SEED);
        for key in 0..8u16 {
            assert_eq!(map.put(key, key * 10), PutOutcome::Inserted);
        }
        let slot_of = |map: &BoundedMap<u16, u16, 8>, key: u16| {
            let mut at = map.first();
            while let Some(cursor) = at {
                if *map.key_at(cursor) == key {
                    return Some(cursor);
                }
                at = map.next_after(cursor);
            }
            None
        };
        let before: heapless::Vec<Option<MapCursor>, 8> =
            (0..8u16).map(|key| slot_of(&map, key)).collect();
        assert_eq!(map.remove(&3), RemoveOutcome::Removed);
        assert_eq!(map.remove(&6), RemoveOutcome::Removed);
        for key in (0..8u16).filter(|key| *key != 3 && *key != 6) {
            assert_eq!(slot_of(&map, key), before[key as usize], "key {key}");
            let cursor = slot_of(&map, key).expect("held");
            assert_eq!(*map.value_at(cursor), key * 10);
        }
        // The slot a remove gave back is the one the next put takes.
        assert_eq!(map.put(100, 1), PutOutcome::Inserted);
        assert_eq!(slot_of(&map, 100), before[6]);
    }

    /// A deterministic source of operations: xorshift64.
    struct Xorshift(u64);

    impl Xorshift {
        fn draw(&mut self) -> u64 {
            let mut x = self.0;
            x ^= x << 13;
            x ^= x >> 7;
            x ^= x << 17;
            self.0 = x;
            x
        }
    }

    /// docs/adr/0108, D2 and D8: the order survives any sequence of puts,
    /// removes and clears, in every storage. Many operations, drawn over a few
    /// keys so each is put, replaced and removed again and again, run on a map
    /// and on a list of its entries in order; after each the two hold the same
    /// entries in the same order, and each key the list holds is found and each
    /// it does not is not. Over a small bound, so the map is full often and its
    /// tombstones are rebuilt away often.
    #[test]
    fn order_survives_any_sequence_of_puts_removes_and_clears() {
        any_sequence_keeps_the_order(
            BoundedMap::<u16, u32, ORDER_BOUND, DefaultMapStorage>::with_seed(SEED),
        );
        any_sequence_keeps_the_order(BoundedMap::<u16, u32, ORDER_BOUND, Inline>::with_seed(SEED));
        #[cfg(feature = "alloc")]
        any_sequence_keeps_the_order(BoundedMap::<u16, u32, ORDER_BOUND, Heap>::with_seed(SEED));
        any_sequence_keeps_the_order(BoundedMap::<u16, u32, ORDER_BOUND, Linear>::with_seed(SEED));
        let mut slots: [LentSlot<u16, u32>; ORDER_BOUND] = lent_run();
        any_sequence_keeps_the_order(
            BoundedMap::<u16, u32, ORDER_BOUND, Lent<'_>>::over(SEED, &mut slots)
                .expect("as many slots as the bound"),
        );
        let mut slots: [LentIndexedSlot<u16, u32>; ORDER_BOUND] = indexed_run();
        let mut cells = [LentCell::EMPTY; 2 * ORDER_BOUND];
        any_sequence_keeps_the_order(
            BoundedMap::<u16, u32, ORDER_BOUND, LentIndexed<'_>>::over(
                SEED, &mut slots, &mut cells,
            )
            .expect("as many slots as the bound"),
        );
    }

    /// The bound of the maps `any_sequence_keeps_the_order` runs on.
    const ORDER_BOUND: usize = 12;

    /// A run of `C` empty slots, which a host lends a map in [`Lent`] storage.
    fn lent_run<K, V, const C: usize>() -> [LentSlot<K, V>; C] {
        core::array::from_fn(|_| LentSlot::empty())
    }

    /// A run of `C` empty slots, which a host lends a map in [`LentIndexed`]
    /// storage.
    fn indexed_run<K, V, const C: usize>() -> [LentIndexedSlot<K, V>; C] {
        core::array::from_fn(|_| LentIndexedSlot::empty())
    }

    /// The sequence of `order_survives_any_sequence_of_puts_removes_and_clears`
    /// on `map`, empty and kept in the storage `S`. After each operation each
    /// key the model holds is held in a slot below the map's limit, which reads
    /// back its entry (docs/adr/0154, D2).
    fn any_sequence_keeps_the_order<S: MapStorage>(mut map: BoundedMap<u16, u32, ORDER_BOUND, S>) {
        const BOUND: usize = ORDER_BOUND;
        let mut model: heapless::Vec<(u16, u32), BOUND> = heapless::Vec::new();
        let mut random = Xorshift(0x9e37_79b9_7f4a_7c15);
        for step in 0..20_000u32 {
            let draw = random.draw();
            let key = (draw % 24) as u16;
            let value = (draw >> 32) as u32;
            match (draw >> 8) % 16 {
                0..=8 => {
                    let expected = match model.iter().position(|(k, _)| *k == key) {
                        Some(at) => {
                            model[at].1 = value;
                            PutOutcome::Replaced
                        }
                        None if model.len() == BOUND => PutOutcome::Full,
                        None => {
                            model.push((key, value)).expect("below the bound");
                            PutOutcome::Inserted
                        }
                    };
                    assert_eq!(map.put(key, value), expected, "step {step}");
                }
                9..=14 => {
                    let expected = match model.iter().position(|(k, _)| *k == key) {
                        Some(at) => {
                            model.remove(at);
                            RemoveOutcome::Removed
                        }
                        None => RemoveOutcome::Absent,
                    };
                    assert_eq!(map.remove(&key), expected, "step {step}");
                }
                _ => {
                    model.clear();
                    map.clear();
                }
            }
            assert_eq!(map.len(), model.len(), "step {step}");
            assert!(
                map.iter().map(|(k, v)| (*k, *v)).eq(model.iter().copied()),
                "step {step}"
            );
            for probe in 0..24u16 {
                let held = model.iter().find(|(k, _)| *k == probe).map(|(_, v)| v);
                assert_eq!(map.get(&probe), held, "step {step}, key {probe}");
            }
            for (key, value) in &model {
                let at = map.link_of(key).expect("a key the model holds");
                assert!(slot_number(at) < map.limit(), "step {step}");
                let held = map.slot(at).entry.as_ref().map(|(k, v)| (k, v));
                assert_eq!(held, Some((key, value)), "step {step}");
            }
            assert!(map.index.tombstones() <= map.live, "step {step}");
        }
    }

    /// The cell of the index each key starts its probe at, for an index of
    /// `len` cells, under `seed`.
    fn home_of(seed: &MapSeed, key: u32, len: usize) -> usize {
        home(hash_of(seed, &key), len)
    }

    /// docs/adr/0108, D8: the hash is keyed. Keys chosen so that all of them
    /// start their probe at one cell under one seed are found all the same, and
    /// under another seed they start at many cells: a peer that does not know a
    /// map's seed cannot choose keys that make every lookup a scan.
    #[test]
    fn keys_chosen_to_collide_under_one_seed_spread_under_another() {
        const BOUND: usize = 16;
        let len = 2 * BOUND;
        let mut colliding: heapless::Vec<u32, 8> = heapless::Vec::new();
        let mut key = 0u32;
        while !colliding.is_full() {
            if home_of(&SEED, key, len) == 0 {
                colliding.push(key).expect("not full");
            }
            key += 1;
        }
        // Under the seed they were chosen for, one chain, and every key found.
        let mut flooded = BoundedMap::<u32, u32, BOUND>::with_seed(SEED);
        for (i, key) in colliding.iter().enumerate() {
            assert_eq!(flooded.put(*key, i as u32), PutOutcome::Inserted);
        }
        for (i, key) in colliding.iter().enumerate() {
            assert_eq!(flooded.get(key), Some(&(i as u32)));
        }
        // Under another seed, they spread.
        let mut cells: heapless::Vec<usize, 8> = colliding
            .iter()
            .map(|key| home_of(&OTHER_SEED, *key, len))
            .collect();
        cells.sort_unstable();
        let distinct = cells.windows(2).filter(|pair| pair[0] != pair[1]).count() + 1;
        assert!(distinct >= 3, "{cells:?}");
    }

    /// docs/adr/0108, D8: a rebuild of the index keeps every lookup right.
    /// Filling and emptying the map again and again leaves tombstones that a
    /// rebuild clears once they outnumber the live entries; each key is found
    /// where it is held, under the seed, after every such rebuild.
    #[test]
    fn a_rebuild_of_the_tombstones_keeps_every_lookup_right() {
        const BOUND: usize = 6;
        let mut map = BoundedMap::<u32, u32, BOUND>::with_seed(OTHER_SEED);
        for round in 0..50u32 {
            for key in 0..BOUND as u32 {
                assert_eq!(map.put(round * 100 + key, key), PutOutcome::Inserted);
            }
            for key in 0..BOUND as u32 - 1 {
                assert_eq!(map.remove(&(round * 100 + key)), RemoveOutcome::Removed);
                assert!(map.index.tombstones <= map.live);
                for later in key + 1..BOUND as u32 {
                    assert_eq!(map.get(&(round * 100 + later)), Some(&later));
                }
            }
            let last = round * 100 + BOUND as u32 - 1;
            assert_eq!(map.remove(&last), RemoveOutcome::Removed);
            assert!(map.is_empty());
        }
    }

    /// A set is the map whose entries carry no value: a key it holds is
    /// `Replaced` and stays where it was, one past the bound is `Full`, and a
    /// text set is probed by a text slice.
    #[test]
    fn a_set_holds_keys_in_the_order_they_were_first_put() {
        type Tag = SceString<8>;
        let tag = |text: &str| Tag::from_text(text).expect("fits");
        let mut tags = BoundedSet::<Tag, 3>::with_seed(SEED);
        assert_eq!(tags.put(tag("red")), PutOutcome::Inserted);
        assert_eq!(tags.put(tag("blue")), PutOutcome::Inserted);
        assert_eq!(tags.put(tag("red")), PutOutcome::Replaced);
        assert!(holds(&tags, "red"));
        assert_eq!(count(&tags), 2);
        assert_eq!(tags.remove("red"), RemoveOutcome::Removed);
        assert_eq!(tags.put(tag("red")), PutOutcome::Inserted);
        assert_eq!(tags.put(tag("green")), PutOutcome::Inserted);
        assert_eq!(tags.put(tag("pink")), PutOutcome::Full);
        assert!(tags.iter().map(Tag::as_str).eq(["blue", "red", "green"]));
        let mut at = tags.first();
        let mut walked = 0;
        while let Some(cursor) = at {
            assert!(holds(&tags, tags.key_at(cursor)));
            walked += 1;
            at = tags.next_after(cursor);
        }
        assert_eq!(walked, 3);
    }

    /// The reads an expression lowers to: `len`, `has`, `m[k]` and `m[k].f`.
    #[test]
    fn the_reads_of_an_expression_answer_what_the_map_holds() {
        let mut amounts = BoundedMap::<u32, i32, 3>::with_seed(SEED);
        assert_eq!(count(&amounts), 0);
        assert!(!holds(&amounts, &7));
        assert_eq!(value_cloned(&amounts, &7), None);
        assert_eq!(amounts.put(7, 5), PutOutcome::Inserted);
        assert_eq!(count(&amounts), 1);
        assert!(holds(&amounts, &7));
        assert!(holds(&amounts, &7u8));
        assert_eq!(value_cloned(&amounts, &7), Some(5));
        assert_eq!(value(&amounts, &7), Some(&5));
    }

    /// docs/adr/0108, D2: a map taken whole into a place of a larger bound, its
    /// texts into larger bounds, holds the same entries in the same order; a set
    /// likewise. A text lent where a map holds it compares with one held.
    #[test]
    fn a_map_copied_into_a_larger_bound_keeps_its_entries_in_order() {
        let word = |text: &str| SceString::<4>::from_text(text).expect("fits");
        let mut small = BoundedMap::<SceString<4>, u8, 2>::with_seed(SEED);
        assert_eq!(small.put(word("b"), 2), PutOutcome::Inserted);
        assert_eq!(small.put(word("a"), 1), PutOutcome::Inserted);
        let wide = BoundedMap::<SceString<8>, u8, 4>::widened_map(
            &small,
            SceString::<8>::widened,
            Clone::clone,
        );
        assert!(wide
            .iter()
            .map(|(k, v)| (k.as_str(), *v))
            .eq([("b", 2), ("a", 1)]));
        assert_eq!(wide.get("a"), Some(&1));
        let mut tags = BoundedSet::<u8, 2>::with_seed(SEED);
        assert_eq!(tags.put(9), PutOutcome::Inserted);
        assert_eq!(tags.put(3), PutOutcome::Inserted);
        let wider = BoundedSet::<u8, 5>::widened_map(&tags, Clone::clone);
        assert!(wider.iter().copied().eq([9, 3]));
        let lent = small.key_at(small.first().expect("an entry"));
        assert!(lent == word("b"));
        assert!(word("b") == lent);
        assert!(lent != SceString::<8>::from_text("a").expect("fits"));
    }

    /// The seeds of one machine's maps are drawn apart from one another and
    /// from the seed they are drawn from, and a seed is never formatted.
    #[test]
    fn the_seeds_of_a_machine_s_maps_are_drawn_apart() {
        let mut seeds = MapSeeds::new(SEED);
        let first = seeds.draw();
        let second = seeds.draw();
        assert_ne!(first, second);
        assert_ne!(first, SEED);
        assert_eq!(first, SEED.derive(0));
        let mut text: heapless::String<32> = heapless::String::new();
        core::fmt::write(&mut text, format_args!("{first:?}")).expect("fits");
        assert_eq!(text.as_str(), "MapSeed(..)");
    }

    /// docs/adr/0108, D6 item 2 and D8: a read the build proved held reads
    /// the value at its key, and a map an algorithm builds for itself is keyed
    /// by a seed drawn from the one it was lent, as a machine's maps are drawn
    /// from its seed.
    #[test]
    fn a_proven_read_reads_its_key_and_a_derived_seed_is_drawn_apart() {
        let mut map = BoundedMap::<u32, u64, 4>::with_seed(SEED);
        assert_eq!(map.put(7, 70), PutOutcome::Inserted);
        assert_eq!(*held(&map, &7u32), 70);
        assert_eq!(held_cloned(&map, &7u32), 70);
        assert_eq!(map.derived_seed(0), SEED.derive(0));
        assert_ne!(map.derived_seed(0), map.derived_seed(1));
        let set = BoundedSet::<u8, 2>::with_seed(OTHER_SEED);
        assert_eq!(set.derived_seed(3), OTHER_SEED.derive(3));
    }

    /// docs/adr/0108, D12 items 4 and 7: the empty value a map of maps puts at a
    /// key is keyed by the seed the map draws under the ordinal no machine's own
    /// map is numbered by, and a map held at that depth draws the next depth's
    /// the same way; the empty list a map of lists puts is the empty list
    /// whatever the seed.
    #[test]
    fn an_empty_value_put_at_a_key_is_keyed_by_the_inner_seed() {
        let mut regions =
            BoundedMap::<u32, BoundedMap<u8, BoundedSet<u8, 2>, 2>, 3>::with_seed(SEED);
        assert_eq!(regions.inner_seed(), SEED.derive(INNER_SEED_ORDINAL));
        assert_eq!(regions.inner_seed(), SEED.derive(u32::MAX));
        assert_ne!(regions.inner_seed(), SEED.derive(0));
        let inner_seed = regions.inner_seed();
        assert_eq!(
            regions.put(7, FromSeed::from_seed(inner_seed)),
            PutOutcome::Inserted
        );
        let faces = value_mut(&mut regions, &7u32).expect("held");
        assert_eq!(faces.derived_seed(0), inner_seed.derive(0));
        assert_eq!(faces.inner_seed(), inner_seed.derive(INNER_SEED_ORDINAL));
        let deeper = faces.inner_seed();
        assert_eq!(
            faces.put(1, FromSeed::from_seed(deeper)),
            PutOutcome::Inserted
        );
        assert_eq!(
            value(faces, &1u8).map(|kinds| kinds.derived_seed(0)),
            Some(deeper.derive(0))
        );
        let mut samples = BoundedMap::<u32, BoundedList<u16, 3>, 2>::with_seed(SEED);
        assert_eq!(
            samples.put(1, FromSeed::from_seed(samples.inner_seed())),
            PutOutcome::Inserted
        );
        assert_eq!(value(&samples, &1u32).map(|list| list.len()), Some(0));
    }

    /// A proven read of a key that is absent is no value: it panics, as a
    /// proven index past a list's end does.
    #[test]
    #[should_panic(expected = "a key the build proved held is absent from the map")]
    fn a_proven_read_of_an_absent_key_panics() {
        let map = BoundedMap::<u32, u64, 4>::with_seed(SEED);
        let _ = held(&map, &7u32);
    }

    /// An empty map built in a const context under the zero seed, as a
    /// firmware `static` is built.
    const ZEROED: BoundedMap<u32, u32, 4> = BoundedMap::with_seed(MapSeed::from_halves(0, 0));

    /// A `static` holding the map built in a const context.
    static HELD: BoundedMap<u32, u32, 4> = ZEROED;

    /// A `static` holding a set built in a const context.
    static HELD_SET: BoundedSet<u8, 4> = BoundedSet::with_seed(MapSeed::from_halves(0, 0));

    /// A `static` holding an inline map built in a const context, which is
    /// inline whatever features the build turns on.
    static HELD_INLINE: BoundedMap<u32, u32, 4, Inline> =
        BoundedMap::with_seed(MapSeed::from_halves(0, 0));

    /// A `static` holding an inline set built in a const context.
    static HELD_INLINE_SET: BoundedSet<u8, 4, Inline> =
        BoundedSet::with_seed(MapSeed::from_halves(0, 0));

    /// The empty map a const context builds under the zero seed is zero in
    /// every field: no slot linked, none free, no entry or tombstone counted,
    /// and every cell of its index empty, the link to no slot and the empty
    /// cell both being 0. In inline storage, under every configuration of the
    /// crate's features, its index is held whole and zero, so a `static`
    /// holding the map is all zero bytes, its inline slots uninitialised; the
    /// map of the default storage is checked as it is.
    #[test]
    fn an_empty_map_built_in_a_const_context_is_zero() {
        let map = &HELD;
        assert_eq!((map.head, map.tail, map.free), (0, 0, 0));
        assert_eq!((map.live, map.index.tombstones), (0, 0));
        assert_eq!(map.seed, MapSeed::from_halves(0, 0));
        assert!(map.slots.is_empty());
        assert!((0..map.index.cells.cell_count()).all(|cell| map.index.cells.cell(cell) == 0));
        #[cfg(not(feature = "alloc"))]
        {
            assert_eq!(map.index.cells.cell_count(), 8);
            assert_eq!(map.index.cells, [[0u32; 4]; 2]);
            assert_eq!(HELD_SET.0.index.cells, [[0u32; 4]; 2]);
        }
        let set = &HELD_SET.0;
        assert_eq!((set.head, set.tail, set.free), (0, 0, 0));
        assert_eq!((set.live, set.index.tombstones), (0, 0));
        assert!(map.first().is_none() && map.iter().next().is_none());

        let inline = &HELD_INLINE;
        assert_eq!((inline.head, inline.tail, inline.free), (0, 0, 0));
        assert_eq!((inline.live, inline.index.tombstones), (0, 0));
        assert_eq!(inline.seed, MapSeed::from_halves(0, 0));
        assert!(inline.slots.is_empty());
        assert_eq!(inline.index.cells.cell_count(), 8);
        assert_eq!(inline.index.cells, [[0u32; 4]; 2]);
        let inline_set = &HELD_INLINE_SET.0;
        assert_eq!(
            (inline_set.head, inline_set.tail, inline_set.free),
            (0, 0, 0)
        );
        assert_eq!((inline_set.live, inline_set.index.tombstones), (0, 0));
        assert_eq!(inline_set.index.cells, [[0u32; 4]; 2]);
        assert!(inline.first().is_none() && HELD_INLINE_SET.iter().next().is_none());
    }

    /// docs/adr/0108, D4 "Storage": an inline map holds its bound in itself,
    /// its index whole from the start and never grown, the slots reserved in
    /// the value, whatever features the build turns on; a heap map holds no
    /// cell until its first put and grows its index as it fills, to twice the
    /// bound and no further.
    #[test]
    fn an_inline_map_holds_its_bound_in_itself_and_a_heap_map_grows() {
        const BOUND: usize = 16;
        let mut inline = BoundedMap::<u64, u64, BOUND, Inline>::with_seed(SEED);
        assert_eq!(inline.index.cells.cell_count(), 2 * BOUND);
        assert!(
            core::mem::size_of::<BoundedMap<u64, u64, BOUND, Inline>>()
                >= BOUND * core::mem::size_of::<Slot<u64, u64, u64>>() + 2 * BOUND * 4
        );
        for key in 0..BOUND as u64 {
            assert_eq!(inline.put(key, key), PutOutcome::Inserted);
            assert_eq!(inline.index.cells.cell_count(), 2 * BOUND);
        }
        assert_eq!(inline.put(99, 0), PutOutcome::Full);
        #[cfg(feature = "alloc")]
        {
            let mut heap = BoundedMap::<u64, u64, BOUND, Heap>::with_seed(SEED);
            assert_eq!(heap.index.cells.cell_count(), 0);
            assert_eq!(heap.put(0, 0), PutOutcome::Inserted);
            assert_eq!(heap.index.cells.cell_count(), 8);
            for key in 1..BOUND as u64 {
                assert_eq!(heap.put(key, key), PutOutcome::Inserted);
            }
            assert_eq!(heap.index.cells.cell_count(), 2 * BOUND);
            assert_eq!(heap.put(99, 0), PutOutcome::Full);
            assert_eq!(heap.index.cells.cell_count(), 2 * BOUND);
            assert_eq!(heap, inline);
        }
    }

    /// docs/adr/0108, D2: a map is copied whole from one storage into another,
    /// its entries in its order, and two maps of different storages holding the
    /// same entries in one order are equal; a set likewise. A copy of an inline
    /// map is an inline map holding the same entries.
    #[test]
    fn a_map_crosses_between_storages_with_its_entries_in_order() {
        let mut inline = BoundedMap::<u32, u8, 4, Inline>::with_seed(SEED);
        assert_eq!(inline.put(9, 1), PutOutcome::Inserted);
        assert_eq!(inline.put(3, 2), PutOutcome::Inserted);
        assert_eq!(inline.put(9, 3), PutOutcome::Replaced);
        let default = BoundedMap::<u32, u8, 4>::widened_map(&inline, Clone::clone, Clone::clone);
        assert!(default.iter().map(|(k, v)| (*k, *v)).eq([(9, 3), (3, 2)]));
        assert!(default == inline && inline == default);
        assert_eq!(default.derived_seed(0), SEED.derive(0));
        let back =
            BoundedMap::<u32, u8, 4, Inline>::widened_map(&default, Clone::clone, Clone::clone);
        assert!(back == inline);
        let wide = BoundedMap::<u32, u8, 6>::widened_map(&inline, Clone::clone, Clone::clone);
        assert!(wide.iter().map(|(k, v)| (*k, *v)).eq([(9, 3), (3, 2)]));
        let mut other = BoundedMap::<u32, u8, 4>::with_seed(OTHER_SEED);
        assert_eq!(other.put(3, 2), PutOutcome::Inserted);
        assert_eq!(other.put(9, 3), PutOutcome::Inserted);
        assert!(other != inline, "the same entries in another order");
        let copy = inline.clone();
        assert_eq!(copy.index, inline.index);
        assert!(copy == inline);

        let mut tags = BoundedSet::<u8, 3, Inline>::with_seed(SEED);
        assert_eq!(tags.put(5), PutOutcome::Inserted);
        assert_eq!(tags.put(1), PutOutcome::Inserted);
        let wider = BoundedSet::<u8, 4>::widened_map(&tags, Clone::clone);
        assert!(wider.iter().copied().eq([5, 1]));
        let same = BoundedSet::<u8, 3>::widened_map(&tags, Clone::clone);
        assert!(same == tags && tags == same);
        let copied = tags.clone();
        assert!(copied.iter().copied().eq([5, 1]));
    }

    /// docs/adr/0108, D8: a map given another seed holds what it held. After
    /// `reseed` every key it held is found with its value and every other is
    /// absent, its order is the one before, each entry's hash is its key's
    /// under the new seed, and the tombstone a remove left is gone; puts and
    /// removes go on as before. A full map likewise, which still refuses one
    /// more key and still replaces one it holds; and a set.
    #[test]
    fn a_reseeded_map_holds_what_it_held_in_its_order() {
        let mut map = BoundedMap::<u32, u32, 8>::with_seed(SEED);
        for key in 0..6u32 {
            assert_eq!(map.put(key * 7, key), PutOutcome::Inserted);
        }
        assert_eq!(map.remove(&14), RemoveOutcome::Removed);
        assert_eq!(map.index.tombstones, 1);
        let before: heapless::Vec<(u32, u32), 8> = map.iter().map(|(k, v)| (*k, *v)).collect();
        map.reseed(OTHER_SEED);
        assert_eq!(map.seed, OTHER_SEED);
        assert_eq!(map.index.tombstones, 0);
        assert!(map.iter().map(|(k, v)| (*k, *v)).eq(before.iter().copied()));
        for (key, value) in &before {
            assert_eq!(map.get(key), Some(value), "key {key}");
        }
        for absent in [14u32, 1, 100] {
            assert!(!map.contains_key(&absent), "key {absent}");
        }
        let mut at = map.head;
        while at != NIL {
            let slot = map.slot(at);
            let (key, _) = slot.entry.as_ref().expect("a linked slot holds an entry");
            assert_eq!(slot.stamp, hash_of(&OTHER_SEED, key));
            at = slot.next;
        }
        assert_eq!(map.put(14, 9), PutOutcome::Inserted);
        assert_eq!(map.remove(&0), RemoveOutcome::Removed);
        assert!(map
            .iter()
            .map(|(k, v)| (*k, *v))
            .eq([(7, 1), (21, 3), (28, 4), (35, 5), (14, 9)]));

        let mut full = BoundedMap::<u32, u32, 4>::with_seed(SEED);
        for key in 0..4u32 {
            assert_eq!(full.put(key, key), PutOutcome::Inserted);
        }
        full.reseed(OTHER_SEED);
        for key in 0..4u32 {
            assert_eq!(full.get(&key), Some(&key), "key {key}");
        }
        assert_eq!(full.put(9, 9), PutOutcome::Full);
        assert_eq!(full.put(2, 20), PutOutcome::Replaced);
        assert!(full
            .iter()
            .map(|(k, v)| (*k, *v))
            .eq([(0, 0), (1, 1), (2, 20), (3, 3)]));

        let mut tags = BoundedSet::<u8, 4>::with_seed(SEED);
        assert_eq!(tags.put(3), PutOutcome::Inserted);
        assert_eq!(tags.put(1), PutOutcome::Inserted);
        tags.reseed(OTHER_SEED);
        assert!(tags.contains_key(&3u8) && tags.contains_key(&1u8));
        assert!(!tags.contains_key(&2u8));
        assert!(tags.iter().copied().eq([3, 1]));
    }

    /// A map built in a const context under the zero seed and given `SEED`
    /// before its first put is the map built under `SEED`: each operation
    /// answers alike, the two hold the same entries in the same order, each key
    /// sits at the same cell of the index, and the seeds drawn from them are
    /// the same. A set likewise.
    #[test]
    fn an_empty_map_reseeded_is_the_map_built_under_that_seed() {
        let mut reseeded = ZEROED;
        reseeded.reseed(SEED);
        let mut built = BoundedMap::<u32, u32, 4>::with_seed(SEED);
        for key in [5u32, 9, 2] {
            assert_eq!(reseeded.put(key, key + 1), built.put(key, key + 1));
        }
        assert_eq!(reseeded.remove(&9), built.remove(&9));
        assert_eq!(reseeded.put(11, 0), built.put(11, 0));
        assert_eq!(reseeded, built);
        assert_eq!(reseeded.derived_seed(0), built.derived_seed(0));
        assert_eq!(
            reseeded.index.cells.cell_count(),
            built.index.cells.cell_count()
        );
        assert!((0..built.index.cells.cell_count())
            .all(|cell| reseeded.index.cells.cell(cell) == built.index.cells.cell(cell)));
        for key in [5u32, 9, 2, 11, 0] {
            assert_eq!(reseeded.get(&key), built.get(&key), "key {key}");
        }

        let mut tags = HELD_SET.clone();
        tags.reseed(SEED);
        assert_eq!(tags.put(4), PutOutcome::Inserted);
        assert!(tags.contains_key(&4u8));
        assert_eq!(tags.derived_seed(0), SEED.derive(0));
    }

    /// docs/adr/0108, D12 item 2: a value changed where the map holds it keeps
    /// its key's place. After writes through `get_mut` every entry is in the
    /// slot it was in and in the order it was in, the index, the links, the
    /// free slot and the counts are as they were, and only the values written
    /// are new; a key that is absent lends nothing. The slot a remove freed is
    /// still the one the next put takes. In every storage.
    #[test]
    fn a_value_changed_in_place_keeps_its_slot_and_its_place() {
        a_value_changed_in_place_keeps_its_slot(
            BoundedMap::<u16, u32, 6, DefaultMapStorage>::with_seed(SEED),
        );
        a_value_changed_in_place_keeps_its_slot(BoundedMap::<u16, u32, 6, Inline>::with_seed(SEED));
        #[cfg(feature = "alloc")]
        a_value_changed_in_place_keeps_its_slot(BoundedMap::<u16, u32, 6, Heap>::with_seed(SEED));
        a_value_changed_in_place_keeps_its_slot(BoundedMap::<u16, u32, 6, Linear>::with_seed(SEED));
        let mut slots: [LentSlot<u16, u32>; 6] = lent_run();
        a_value_changed_in_place_keeps_its_slot(
            BoundedMap::<u16, u32, 6, Lent<'_>>::over(SEED, &mut slots)
                .expect("as many slots as the bound"),
        );
    }

    /// The writes of `a_value_changed_in_place_keeps_its_slot_and_its_place`
    /// on `map`, empty and kept in the storage `S`, whose index is copied to
    /// be compared.
    fn a_value_changed_in_place_keeps_its_slot<S: MapStorage>(mut map: BoundedMap<u16, u32, 6, S>)
    where
        <S as MapStorage>::Cells<6>: Clone,
    {
        const BOUND: usize = 6;
        let placed = |map: &BoundedMap<u16, u32, BOUND, S>| {
            let mut entries: heapless::Vec<(u16, u32, MapCursor<S::Generation>), BOUND> =
                heapless::Vec::new();
            let mut at = map.first();
            while let Some(cursor) = at {
                entries
                    .push((*map.key_at(cursor), *map.value_at(cursor), cursor))
                    .expect("within the bound");
                at = map.next_after(cursor);
            }
            entries
        };
        for key in 0..5u16 {
            assert_eq!(map.put(key, u32::from(key) * 10), PutOutcome::Inserted);
        }
        let freed = placed(&map)
            .iter()
            .find(|(key, _, _)| *key == 1)
            .map(|(_, _, cursor)| *cursor);
        assert_eq!(map.remove(&1u16), RemoveOutcome::Removed);
        let before = placed(&map);
        let links = (map.head, map.tail, map.free, map.live);
        let index = map.index.clone();

        *map.get_mut(&3u16).expect("held") = 333;
        // A key of another width that writes the same bytes is the same key.
        *map.get_mut(&0u8).expect("held by its bytes") += 1;
        assert!(map.get_mut(&1u16).is_none(), "a removed key lends nothing");
        assert!(map.get_mut(&9u16).is_none(), "an absent key lends nothing");

        let after = placed(&map);
        assert_eq!(after.len(), before.len());
        for ((key, old, cursor), (key_after, new, cursor_after)) in before.iter().zip(after.iter())
        {
            assert_eq!((key, cursor), (key_after, cursor_after), "key {key}");
            let expected = match *key {
                3 => 333,
                0 => *old + 1,
                _ => *old,
            };
            assert_eq!(*new, expected, "key {key}");
        }
        assert_eq!((map.head, map.tail, map.free, map.live), links);
        assert!(map.index == index, "the index is as it was");
        assert_eq!(map.get(&3u16), Some(&333));
        assert_eq!(map.put(7, 70), PutOutcome::Inserted);
        let last = placed(&map)
            .last()
            .map(|(key, _, cursor)| (*key, cursor.link));
        assert_eq!(last, freed.map(|cursor| (7, cursor.link)));
    }

    /// docs/adr/0108, D12 items 2 and 3: a place at a key the map does not hold
    /// is none, as the value at it is: `value_mut` answers `None` where
    /// `value` does, before the key is put and after it is removed, and leaves
    /// the map as it was; at a key the map holds, what is written through it
    /// is what `value` reads next.
    #[test]
    fn a_place_at_an_absent_key_is_none_as_its_value_is() {
        let mut amounts = BoundedMap::<u32, i32, 3>::with_seed(SEED);
        assert_eq!(value(&amounts, &7u32), None);
        assert_eq!(value_mut(&mut amounts, &7u32), None);
        assert!(amounts.is_empty());
        assert_eq!(amounts.put(7, 5), PutOutcome::Inserted);
        assert_eq!(amounts.put(9, 6), PutOutcome::Inserted);
        assert_eq!(amounts.remove(&9u32), RemoveOutcome::Removed);
        assert_eq!(value(&amounts, &9u32), None);
        assert_eq!(value_mut(&mut amounts, &9u32), None);
        assert_eq!(count(&amounts), 1);
        *value_mut(&mut amounts, &7u32).expect("held") = -3;
        assert_eq!(value(&amounts, &7u32), Some(&-3));
        assert_eq!(value_cloned(&amounts, &7u8), Some(-3));
        assert!(amounts.iter().map(|(k, v)| (*k, *v)).eq([(7, -3)]));
    }

    /// docs/adr/0108, D12: a map whose value is a map is a value like any
    /// other. A put into an inner map answers the inner map's outcome, `Full`
    /// when the inner map is full (item 5); a write at an outer key that is
    /// absent finds no place and makes no entry (item 3). A copy holds equal
    /// inner maps in their order and compares equal, and a write into the
    /// copy's inner map leaves the original's as it was; the same inner
    /// entries in another order are another value. An inner map keyed by the
    /// outer map's seed derived (item 7) draws its own seeds from that one. In
    /// every storage.
    #[test]
    fn a_map_of_maps_is_copied_whole_and_written_in_place() {
        a_map_of_maps_in::<DefaultMapStorage>();
        a_map_of_maps_in::<Inline>();
        #[cfg(feature = "alloc")]
        a_map_of_maps_in::<Heap>();
        a_map_of_maps_in::<Linear>();
    }

    /// The map of maps of `a_map_of_maps_is_copied_whole_and_written_in_place`,
    /// both levels kept in the storage `S`.
    fn a_map_of_maps_in<S: MapStorage<Owned = S>>() {
        let faces_of = |regions: &BoundedMap<u32, BoundedMap<u8, u16, 2, S>, 3, S>, region: u32| {
            let mut faces: heapless::Vec<(u8, u16), 2> = heapless::Vec::new();
            for (face, weight) in regions.get(&region).into_iter().flatten() {
                faces
                    .push((*face, *weight))
                    .expect("within the inner bound");
            }
            faces
        };
        let mut regions = BoundedMap::<u32, BoundedMap<u8, u16, 2, S>, 3, S>::with_seed(SEED);
        let inner_seed = regions.derived_seed(1);
        assert_eq!(
            regions.put(7, BoundedMap::with_seed(inner_seed)),
            PutOutcome::Inserted
        );
        assert_eq!(
            regions.put(4, BoundedMap::with_seed(inner_seed)),
            PutOutcome::Inserted
        );
        let mut put = |region: u32, face: u8, weight: u16| {
            value_mut(&mut regions, &region).map(|faces| faces.put(face, weight))
        };
        assert_eq!(put(7, 9, 90), Some(PutOutcome::Inserted));
        assert_eq!(put(7, 2, 20), Some(PutOutcome::Inserted));
        assert_eq!(put(7, 5, 50), Some(PutOutcome::Full));
        assert_eq!(put(7, 9, 91), Some(PutOutcome::Replaced));
        assert_eq!(put(4, 9, 1), Some(PutOutcome::Inserted));
        assert_eq!(put(8, 1, 1), None);
        assert_eq!(regions.len(), 2);
        assert!(!regions.contains_key(&8u32));
        assert!(regions.iter().map(|(region, _)| *region).eq([7, 4]));
        assert_eq!(faces_of(&regions, 7).as_slice(), &[(9, 91), (2, 20)]);
        assert_eq!(faces_of(&regions, 4).as_slice(), &[(9, 1)]);

        let copy = regions.clone();
        assert_eq!(copy, regions);
        assert_eq!(faces_of(&copy, 7).as_slice(), &[(9, 91), (2, 20)]);
        let mut changed = regions.clone();
        assert_eq!(
            changed.get_mut(&7u32).map(|faces| faces.remove(&9u8)),
            Some(RemoveOutcome::Removed)
        );
        assert_ne!(changed, regions);
        assert_eq!(faces_of(&changed, 7).as_slice(), &[(2, 20)]);
        assert_eq!(faces_of(&regions, 7).as_slice(), &[(9, 91), (2, 20)]);
        let mut reordered = regions.clone();
        let faces = reordered.get_mut(&7u32).expect("held");
        assert_eq!(faces.remove(&9u8), RemoveOutcome::Removed);
        assert_eq!(faces.put(9, 91), PutOutcome::Inserted);
        assert_ne!(reordered, regions);

        assert_eq!(
            regions.get(&7u32).map(|faces| faces.derived_seed(0)),
            Some(inner_seed.derive(0))
        );
    }

    /// docs/adr/0108, D8 and D12 item 7: a seed is derived as the C11 lane's
    /// `sce_map_seed_derive` derives it, two SipHash-1-3 values under the seed,
    /// of a side byte (0, then 1) followed by the ordinal's four little-endian
    /// bytes, so an inner map is keyed from its outer map's seed alike on every
    /// lane. The halves are the function's values under `SEED`, the seed the
    /// C11 lane's own map tests start their maps with.
    #[test]
    fn a_derived_seed_is_the_one_every_lane_derives() {
        assert_eq!(
            SEED.derive(0),
            MapSeed::from_halves(0xbe0c_97ec_4d02_272d, 0xac1c_1e65_c25b_faa3)
        );
        assert_eq!(
            SEED.derive(1),
            MapSeed::from_halves(0xbbf2_e616_ed4f_55e8, 0xc818_9588_b142_684c)
        );
    }

    /// The bound of the maps `answers_as_inline` runs.
    const ANSWER_BOUND: usize = 5;

    /// One sequence of puts, replaces, removes, writes in place and clears,
    /// run on `other` and on an [`Inline`] map built empty beside it: each
    /// operation answers alike, a put the same slot; after each the two maps
    /// are equal both ways, walk their entries in one order, hold each entry at
    /// the same position, and find, miss and place the same keys. Answers the
    /// inline map as the sequence left it.
    fn answers_as_inline<S: MapStorage>(
        other: &mut BoundedMap<u16, u32, ANSWER_BOUND, S>,
    ) -> BoundedMap<u16, u32, ANSWER_BOUND, Inline> {
        let mut inline = BoundedMap::<u16, u32, ANSWER_BOUND, Inline>::with_seed(SEED);
        let mut random = Xorshift(0x2545_f491_4f6c_dd1d);
        for step in 0..5_000u32 {
            let draw = random.draw();
            let key = (draw % 9) as u16;
            let value = (draw >> 32) as u32;
            match (draw >> 8) % 16 {
                0..=7 => {
                    assert_eq!(other.put(key, value), inline.put(key, value), "step {step}");
                }
                8..=11 => {
                    assert_eq!(other.remove(&key), inline.remove(&key), "step {step}");
                }
                12..=14 => {
                    let write = |held: &mut u32| {
                        *held = held.wrapping_add(value);
                        *held
                    };
                    let written = other.get_mut(&key).map(write);
                    assert_eq!(written, inline.get_mut(&key).map(write), "step {step}");
                }
                _ => {
                    other.clear();
                    inline.clear();
                }
            }
            assert!(*other == inline && inline == *other, "step {step}");
            assert!(other.iter().eq(inline.iter()), "step {step}");
            let mut at = (other.first(), inline.first());
            while let (Some(here), Some(there)) = at {
                assert_eq!(here.link, there.link, "step {step}");
                assert_eq!(other.key_at(here), inline.key_at(there), "step {step}");
                assert_eq!(other.value_at(here), inline.value_at(there), "step {step}");
                at = (other.next_after(here), inline.next_after(there));
            }
            assert!(at.0.is_none() && at.1.is_none(), "step {step}");
            for probe in 0..9u16 {
                assert_eq!(
                    other.get(&probe),
                    inline.get(&probe),
                    "step {step}, key {probe}"
                );
                assert_eq!(
                    other.link_of(&probe),
                    inline.link_of(&probe),
                    "step {step}, key {probe}"
                );
            }
        }
        inline
    }

    /// docs/adr/0108, D4 "Storage": a map with no index answers every
    /// operation as a map with one does. One sequence of puts, replaces,
    /// removes, writes in place and clears, run on a [`Linear`] and an
    /// [`Inline`] map, answers alike at every step ([`answers_as_inline`]).
    /// Each copies into the other's storage and back as the same map, and two
    /// sets likewise.
    #[test]
    fn a_linear_map_answers_as_an_inline_map_does() {
        const BOUND: usize = ANSWER_BOUND;
        let mut linear = BoundedMap::<u16, u32, BOUND, Linear>::new();
        let inline = answers_as_inline(&mut linear);
        let crossed =
            BoundedMap::<u16, u32, BOUND>::widened_map(&linear, Clone::clone, Clone::clone);
        assert!(crossed == inline);
        let back = BoundedMap::<u16, u32, BOUND, Linear>::widened_map(
            &crossed,
            Clone::clone,
            Clone::clone,
        );
        assert!(back == linear);

        let mut linear_tags = BoundedSet::<u8, 3, Linear>::default();
        let mut inline_tags = BoundedSet::<u8, 3, Inline>::with_seed(SEED);
        for key in [4u8, 1, 4, 7, 2, 1] {
            assert_eq!(linear_tags.put(key), inline_tags.put(key), "key {key}");
        }
        assert_eq!(linear_tags.remove(&4u8), inline_tags.remove(&4u8));
        assert_eq!(linear_tags.put(9), inline_tags.put(9));
        assert!(linear_tags == inline_tags && inline_tags == linear_tags);
        assert!(linear_tags.iter().copied().eq([1, 7, 9]));
    }

    /// A `static` holding a map with no index, built by its `new`, which names
    /// no seed.
    static HELD_LINEAR: BoundedMap<u64, u32, 4, Linear> = BoundedMap::<u64, u32, 4, Linear>::new();

    /// A `static` holding a set with no index, built by its `new`.
    static HELD_LINEAR_SET: BoundedSet<u8, 4, Linear> = BoundedSet::<u8, 4, Linear>::new();

    /// docs/adr/0108, D4 "Storage": an empty map with no index is zero in every
    /// field, so a `static` holding it is all zero bytes, its slots
    /// uninitialised, under every configuration of the crate's features; it
    /// holds nothing for an index and its slots keep no hash, so it is smaller
    /// than an inline map of the same bound. Built under a seed, it is the
    /// same empty map but for the seed it hands on.
    #[test]
    fn an_empty_linear_map_is_zero_and_holds_no_index() {
        let map = &HELD_LINEAR;
        assert_eq!((map.head, map.tail, map.free, map.live), (0, 0, 0, 0));
        assert_eq!(map.seed, MapSeed::from_halves(0, 0));
        assert!(map.slots.is_empty());
        assert!(map.first().is_none() && map.iter().next().is_none());
        let set = &HELD_LINEAR_SET.0;
        assert_eq!((set.head, set.tail, set.free, set.live), (0, 0, 0, 0));
        assert_eq!(set.seed, MapSeed::from_halves(0, 0));
        assert!(set.slots.is_empty());

        assert_eq!(core::mem::size_of::<Scan>(), 0);
        assert_eq!(core::mem::size_of_val(&map.index), 0);
        assert!(
            core::mem::size_of::<Slot<u64, u32, (), ()>>()
                < core::mem::size_of::<Slot<u64, u32, u64, ()>>()
        );
        assert!(
            core::mem::size_of::<BoundedMap<u64, u32, 4, Linear>>()
                < core::mem::size_of::<BoundedMap<u64, u32, 4, Inline>>()
        );

        let seeded = BoundedMap::<u64, u32, 4, Linear>::with_seed(SEED);
        assert_eq!(
            (seeded.head, seeded.tail, seeded.free, seeded.live),
            (0, 0, 0, 0)
        );
        assert_eq!(seeded.derived_seed(0), SEED.derive(0));
        assert!(seeded == *map);
        let defaulted = BoundedMap::<u64, u32, 4, Linear>::default();
        assert_eq!(defaulted.seed, MapSeed::from_halves(0, 0));
        assert!(defaulted == *map);
    }

    /// A key that counts how many times it writes its bytes ([`MapKey`]),
    /// whatever it writes them into: a hash, or a comparison with another key.
    struct Watched<'c> {
        key: u32,
        feeds: &'c core::cell::Cell<u32>,
    }

    impl MapKey for Watched<'_> {
        fn feed_key(&self, into: &mut dyn KeyFeed) {
            self.feeds.set(self.feeds.get() + 1);
            self.key.feed_key(into);
        }
    }

    /// docs/adr/0108, D4 "Storage": a map with no index hashes no key. A key
    /// writes its bytes for a [`Linear`] map only to be compared with a key the
    /// map holds, so an empty one is read, written in place, removed from and
    /// put into without a key writing them once, and a reseed of one that
    /// holds keys writes none of them; an [`Inline`] map, which hashes each key
    /// it is handed, writes the key's bytes on each of the same operations.
    #[test]
    fn a_linear_map_hashes_no_key() {
        let feeds = core::cell::Cell::new(0u32);
        let counter = &feeds;
        let watched = move |key: u32| Watched {
            key,
            feeds: counter,
        };

        let mut linear: BoundedMap<Watched<'_>, u8, 3, Linear> = BoundedMap::with_seed(SEED);
        assert!(linear.get(&watched(1)).is_none());
        assert!(!linear.contains_key(&watched(1)));
        assert!(linear.get_mut(&watched(1)).is_none());
        assert_eq!(linear.remove(&watched(1)), RemoveOutcome::Absent);
        assert_eq!(linear.put(watched(1), 10), PutOutcome::Inserted);
        assert_eq!(
            feeds.get(),
            0,
            "an empty map compares no key and hashes none"
        );
        assert_eq!(linear.put(watched(2), 20), PutOutcome::Inserted);
        assert_eq!(linear.put(watched(3), 30), PutOutcome::Inserted);
        feeds.set(0);
        linear.reseed(OTHER_SEED);
        assert_eq!(feeds.get(), 0, "a reseed hashes no key");
        assert_eq!(linear.get(&watched(3)), Some(&30));
        assert!(feeds.get() > 0, "a lookup compares the key with those held");

        let mut inline: BoundedMap<Watched<'_>, u8, 3, Inline> = BoundedMap::with_seed(SEED);
        feeds.set(0);
        assert!(inline.get(&watched(1)).is_none());
        assert_eq!(feeds.get(), 1, "an inline map hashes the key it looks up");
        assert_eq!(inline.put(watched(1), 10), PutOutcome::Inserted);
        assert_eq!(inline.put(watched(2), 20), PutOutcome::Inserted);
        feeds.set(0);
        inline.reseed(OTHER_SEED);
        assert_eq!(
            feeds.get(),
            2,
            "an inline map hashes each key it holds again"
        );
    }

    /// docs/adr/0154, D1: a map its host lends as many slots as its bound
    /// answers every operation as an inline map does, in both lent storages
    /// ([`answers_as_inline`]): alike at every step, each entry at the same
    /// position and the same slot. Copied into its owned storage it is equal to
    /// both, each entry at the slot it held, and a write into the copy leaves
    /// the lent map as it was.
    #[test]
    fn a_lent_map_answers_as_an_inline_map_does() {
        let mut slots: [LentSlot<u16, u32>; ANSWER_BOUND] = lent_run();
        let mut lent = BoundedMap::<u16, u32, ANSWER_BOUND, Lent<'_>>::over(SEED, &mut slots)
            .expect("as many slots as the bound");
        assert_eq!(lent.limit(), ANSWER_BOUND);
        let inline = answers_as_inline(&mut lent);
        let mut copy: BoundedMap<u16, u32, ANSWER_BOUND, Linear> = lent.to_owned_storage();
        assert!(copy == lent && copy == inline);
        for (key, _) in inline.iter() {
            assert_eq!(copy.link_of(key), lent.link_of(key), "key {key}");
        }
        copy.clear();
        assert!(lent == inline);

        let mut slots: [LentIndexedSlot<u16, u32>; ANSWER_BOUND] = indexed_run();
        let mut cells = [LentCell::EMPTY; 2 * ANSWER_BOUND];
        let mut indexed = BoundedMap::<u16, u32, ANSWER_BOUND, LentIndexed<'_>>::over(
            SEED, &mut slots, &mut cells,
        )
        .expect("as many slots as the bound");
        let inline = answers_as_inline(&mut indexed);
        let mut copy: BoundedMap<u16, u32, ANSWER_BOUND, Inline> = indexed.to_owned_storage();
        assert!(copy == indexed && copy == inline);
        for (key, _) in inline.iter() {
            assert_eq!(copy.link_of(key), indexed.link_of(key), "key {key}");
            assert_eq!(copy.get(key), indexed.get(key), "key {key}");
        }
        copy.clear();
        assert!(indexed == inline);
    }

    /// docs/adr/0154, D1: a map lent fewer slots than its bound holds as many
    /// entries as it was lent, its limit. One more absent key is `Full` and
    /// leaves the map as it was, a key it holds is still replaced, and the slot
    /// a remove freed takes the next key; every slot it answers is below the
    /// limit. In both lent storages, the indexed one lent more cells than it
    /// needs.
    #[test]
    fn a_lent_map_holds_as_many_entries_as_it_was_lent() {
        let mut slots: [LentSlot<u32, u32>; 3] = lent_run();
        holds_its_limit(
            BoundedMap::<u32, u32, 8, Lent<'_>>::over(SEED, &mut slots).expect("3 within 8"),
        );
        let mut slots: [LentIndexedSlot<u32, u32>; 3] = indexed_run();
        let mut cells = [LentCell::EMPTY; 7];
        holds_its_limit(
            BoundedMap::<u32, u32, 8, LentIndexed<'_>>::over(SEED, &mut slots, &mut cells)
                .expect("3 within 8"),
        );
    }

    /// The puts of `a_lent_map_holds_as_many_entries_as_it_was_lent` on `map`,
    /// empty, bounded by 8 and lent 3 slots.
    fn holds_its_limit<S: MapStorage<Generation = u32>>(mut map: BoundedMap<u32, u32, 8, S>) {
        assert_eq!(map.limit(), 3);
        assert_eq!(BoundedMap::<u32, u32, 8, S>::CAPACITY, 8);
        for key in 0..3u32 {
            let (outcome, slot) = map.put_slot(key, key * 10);
            assert_eq!(outcome, PutOutcome::Inserted);
            assert_eq!(slot.map(|slot| slot.index), Some(key));
        }
        assert_eq!(map.put_slot(9, 90), (PutOutcome::Full, None));
        assert!(map
            .iter()
            .map(|(k, v)| (*k, *v))
            .eq([(0, 0), (1, 10), (2, 20)]));
        assert_eq!(map.put(1, 11), PutOutcome::Replaced);
        assert_eq!(map.remove(&0u32), RemoveOutcome::Removed);
        let (outcome, slot) = map.put_slot(9, 90);
        assert_eq!(outcome, PutOutcome::Inserted);
        assert_eq!(slot.map(|slot| slot.index), Some(0));
        assert_eq!(map.put(10, 100), PutOutcome::Full);
        assert!(map
            .iter()
            .map(|(k, v)| (*k, *v))
            .eq([(1, 11), (2, 20), (9, 90)]));
        for key in [1u32, 2, 9] {
            let slot = map.slot_of(&key).expect("held");
            assert!((slot.index as usize) < map.limit(), "key {key}");
        }
        assert_eq!(map.get(&10u32), None);
    }

    /// docs/adr/0154, D1: a map lent no slot borrows no byte and holds nothing:
    /// a put of any key is `Full`, every read misses, and a clear leaves it so.
    /// The value of a lent map holds no slot, so it is as large whatever its
    /// bound, and smaller than four inline slots; a map in lent storage built
    /// by `with_seed` is the map lent no slot. In both lent storages.
    #[test]
    fn a_map_lent_no_slot_borrows_no_byte_and_holds_nothing() {
        let mut none: [LentSlot<u64, u64>; 0] = [];
        holds_nothing(
            &mut BoundedMap::<u64, u64, 4096, Lent<'_>>::over(SEED, &mut none)
                .expect("no slot is within any bound"),
        );
        let mut none: [LentIndexedSlot<u64, u64>; 0] = [];
        let mut cells: [LentCell; 0] = [];
        holds_nothing(
            &mut BoundedMap::<u64, u64, 4096, LentIndexed<'_>>::over(SEED, &mut none, &mut cells)
                .expect("no slot is within any bound"),
        );
        holds_nothing(&mut BoundedMap::<u64, u64, 4096, Lent<'static>>::with_seed(
            SEED,
        ));
        holds_nothing(&mut BoundedMap::<u64, u64, 4096, LentIndexed<'static>>::with_seed(SEED));
        assert_eq!(
            core::mem::size_of::<BoundedMap<u64, u64, 4096, Lent<'static>>>(),
            core::mem::size_of::<BoundedMap<u64, u64, 1, Lent<'static>>>()
        );
        assert_eq!(
            core::mem::size_of::<BoundedMap<u64, u64, 4096, LentIndexed<'static>>>(),
            core::mem::size_of::<BoundedMap<u64, u64, 1, LentIndexed<'static>>>()
        );
        assert!(
            core::mem::size_of::<BoundedMap<u64, u64, 4096, LentIndexed<'static>>>()
                < 4 * core::mem::size_of::<Slot<u64, u64, u64>>()
        );
        assert_eq!(core::mem::size_of::<[LentSlot<u64, u64>; 0]>(), 0);
    }

    /// The operations of `a_map_lent_no_slot_borrows_no_byte_and_holds_nothing`
    /// on `map`, lent no slot.
    fn holds_nothing<S: MapStorage<Generation = u32>>(map: &mut BoundedMap<u64, u64, 4096, S>) {
        assert_eq!(map.limit(), 0);
        assert_eq!(map.put_slot(1, 1), (PutOutcome::Full, None));
        assert_eq!(map.get(&1u64), None);
        assert_eq!(map.remove(&1u64), RemoveOutcome::Absent);
        assert_eq!(map.slot_of(&1u64), None);
        assert_eq!(
            map.at_slot(MapSlot {
                index: 0,
                generation: 0
            }),
            None
        );
        assert!(map.is_empty() && map.first().is_none());
        map.clear();
        assert_eq!(map.put(2, 2), PutOutcome::Full);
        assert!(map.is_empty());
    }

    /// docs/adr/0154, D1: the type holds the bound and the run the limit, so a
    /// run of more slots than the bound is refused where the map is built, as
    /// is a run of index cells shorter than twice the slots; a run of as many
    /// slots as the bound is taken, and of more cells than the index needs,
    /// whose first twice the slots it keeps. The refusal names the counts.
    #[test]
    fn a_run_past_the_bound_or_an_index_short_of_its_slots_is_refused() {
        let mut five: [LentSlot<u8, u8>; 5] = lent_run();
        assert_eq!(
            BoundedMap::<u8, u8, 4, Lent<'_>>::over(SEED, &mut five).err(),
            Some(LendRefused::PastBound { lent: 5, bound: 4 })
        );
        let mut four: [LentSlot<u8, u8>; 4] = lent_run();
        let lent = BoundedMap::<u8, u8, 4, Lent<'_>>::over(SEED, &mut four)
            .expect("as many slots as the bound");
        assert_eq!(lent.limit(), 4);

        let mut past: [LentIndexedSlot<u8, u8>; 5] = indexed_run();
        let mut cells = [LentCell::EMPTY; 10];
        assert_eq!(
            BoundedMap::<u8, u8, 4, LentIndexed<'_>>::over(SEED, &mut past, &mut cells).err(),
            Some(LendRefused::PastBound { lent: 5, bound: 4 })
        );
        let mut slots: [LentIndexedSlot<u8, u8>; 3] = indexed_run();
        let mut short = [LentCell::EMPTY; 5];
        assert_eq!(
            BoundedMap::<u8, u8, 4, LentIndexed<'_>>::over(SEED, &mut slots, &mut short).err(),
            Some(LendRefused::IndexShort {
                cells: 5,
                needed: 6
            })
        );
        let mut enough = [LentCell::EMPTY; 9];
        let indexed = BoundedMap::<u8, u8, 4, LentIndexed<'_>>::over(SEED, &mut slots, &mut enough)
            .expect("six of nine cells");
        assert_eq!(indexed.limit(), 3);
        assert_eq!(indexed.index.cells.cell_count(), 6);

        let mut text: heapless::String<64> = heapless::String::new();
        core::fmt::write(
            &mut text,
            format_args!(
                "{}",
                LendRefused::IndexShort {
                    cells: 5,
                    needed: 6
                }
            ),
        )
        .expect("fits");
        assert_eq!(
            text.as_str(),
            "5 index cells lent where the slots lent need 6"
        );
    }

    /// docs/adr/0154, D2, and docs/adr/0108, D8: an entry keeps its slot from
    /// the put that inserts it to the remove that takes it out. Other keys put
    /// and removed, its value replaced and written in place, and a reseed leave
    /// it where it was; the slot reads back the entry, by `at_slot` and by the
    /// position a walk reads it at, and a replace answers the slot the key
    /// stands in. In every storage.
    #[test]
    fn an_entry_keeps_its_slot_until_it_is_removed() {
        let mut slots: [LentSlot<u32, u32>; 6] = lent_run();
        stays_in_its_slot(
            BoundedMap::<u32, u32, 6, Lent<'_>>::over(SEED, &mut slots)
                .expect("as many slots as the bound"),
        );
        let mut slots: [LentIndexedSlot<u32, u32>; 6] = indexed_run();
        let mut cells = [LentCell::EMPTY; 12];
        stays_in_its_slot(
            BoundedMap::<u32, u32, 6, LentIndexed<'_>>::over(SEED, &mut slots, &mut cells)
                .expect("as many slots as the bound"),
        );
    }

    /// The operations of `an_entry_keeps_its_slot_until_it_is_removed` on
    /// `map`, empty.
    fn stays_in_its_slot<S: MapStorage<Generation = u32>>(mut map: BoundedMap<u32, u32, 6, S>) {
        let (outcome, kept) = map.put_slot(7, 70);
        assert_eq!(outcome, PutOutcome::Inserted);
        let kept = kept.expect("an empty map takes a key");
        assert_eq!(
            kept,
            MapSlot {
                index: 0,
                generation: 0
            }
        );
        for key in 0..4u32 {
            assert_eq!(map.put(key, key), PutOutcome::Inserted);
        }
        assert_eq!(map.remove(&1u32), RemoveOutcome::Removed);
        assert_eq!(map.remove(&3u32), RemoveOutcome::Removed);
        assert_eq!(map.put(10, 0), PutOutcome::Inserted);
        assert_eq!(map.put_slot(7, 71), (PutOutcome::Replaced, Some(kept)));
        *map.get_mut(&7u32).expect("held") += 1;
        map.reseed(OTHER_SEED);
        assert_eq!(map.slot_of(&7u32), Some(kept));
        assert_eq!(map.at_slot(kept), Some((&7, &72)));
        let mut at = map.first();
        let mut walked = None;
        while let Some(cursor) = at {
            if *map.key_at(cursor) == 7 {
                walked = Some(cursor.slot());
            }
            at = map.next_after(cursor);
        }
        assert_eq!(walked, Some(kept));
    }

    /// docs/adr/0154, D2: a slot freed moves its generation, so a [`MapSlot`]
    /// read for an entry that was removed reads nothing, though the slot holds
    /// another entry since, whose own number is the old one's with the next
    /// generation. A clear frees every slot, no number of an entry it held
    /// reads anything, and the next put takes the first slot; a number of no
    /// slot the map took reads nothing. In every storage.
    #[test]
    fn a_slot_taken_again_does_not_answer_its_old_number() {
        let mut slots: [LentSlot<u32, u32>; 4] = lent_run();
        answers_none_to_an_old_number(
            BoundedMap::<u32, u32, 4, Lent<'_>>::over(SEED, &mut slots)
                .expect("as many slots as the bound"),
        );
        let mut slots: [LentIndexedSlot<u32, u32>; 4] = indexed_run();
        let mut cells = [LentCell::EMPTY; 8];
        answers_none_to_an_old_number(
            BoundedMap::<u32, u32, 4, LentIndexed<'_>>::over(SEED, &mut slots, &mut cells)
                .expect("as many slots as the bound"),
        );
    }

    /// The operations of `a_slot_taken_again_does_not_answer_its_old_number`
    /// on `map`, empty.
    fn answers_none_to_an_old_number<S: MapStorage<Generation = u32>>(
        mut map: BoundedMap<u32, u32, 4, S>,
    ) {
        let first = map.put_slot(1, 10).1.expect("room");
        let second = map.put_slot(2, 20).1.expect("room");
        assert_eq!(map.remove(&1u32), RemoveOutcome::Removed);
        assert_eq!(map.at_slot(first), None);
        let again = map.put_slot(3, 30).1.expect("room");
        assert_eq!(again.index, first.index);
        assert_eq!(again.generation, first.generation + 1);
        assert_eq!(map.at_slot(first), None);
        assert_eq!(map.at_slot(again), Some((&3, &30)));
        assert_eq!(map.at_slot(second), Some((&2, &20)));
        assert_eq!(map.first().map(MapCursor::<u32>::slot), Some(second));
        map.clear();
        assert_eq!(map.at_slot(second), None);
        assert_eq!(map.at_slot(again), None);
        let after = map.put_slot(4, 40).1.expect("room");
        assert_eq!(after.index, 0);
        assert_ne!(after, again);
        assert_eq!(map.at_slot(after), Some((&4, &40)));
        assert_eq!(
            map.at_slot(MapSlot {
                index: 3,
                generation: 0
            }),
            None
        );
        assert_eq!(
            map.at_slot(MapSlot {
                index: u32::MAX,
                generation: 0
            }),
            None
        );
    }

    /// docs/adr/0154, D2: a position read before its entry was removed names
    /// no entry once another entry takes the slot, as a position read before
    /// any write of a map is no position of the map after it: it panics, where
    /// it would read the other entry, the slot's generation having moved.
    #[test]
    #[should_panic(expected = "a map position names no entry")]
    fn a_position_read_before_its_slot_was_taken_again_names_no_entry() {
        let mut slots: [LentSlot<u32, u32>; 2] = lent_run();
        let mut map =
            BoundedMap::<u32, u32, 2, Lent<'_>>::over(SEED, &mut slots).expect("within the bound");
        assert_eq!(map.put(1, 10), PutOutcome::Inserted);
        let stale = map.first().expect("an entry");
        assert_eq!(map.remove(&1u32), RemoveOutcome::Removed);
        assert_eq!(map.put(2, 20), PutOutcome::Inserted);
        let _ = map.key_at(stale);
    }

    /// docs/adr/0154, D3: a clone of a map that holds its own slots is the
    /// copy it always was, its slots and its index copied as they stand, a
    /// tombstone a remove left included, so a position read from one is a
    /// position of the other and the slot a remove freed is the one the copy's
    /// next put takes; the copy is its own. A map whose host lent its slots is
    /// copied into its owned storage with each entry at the same number, and
    /// the slot its remove freed is the one that copy's next put takes too.
    #[test]
    fn a_copy_holds_each_entry_at_the_slot_it_held() {
        let mut map = BoundedMap::<u32, u32, 4, Inline>::with_seed(SEED);
        for key in 0..4u32 {
            assert_eq!(map.put(key, key * 10), PutOutcome::Inserted);
        }
        assert_eq!(map.remove(&1u32), RemoveOutcome::Removed);
        assert_eq!(map.index.tombstones, 1);
        let mut copy = map.clone();
        assert_eq!(copy.index, map.index, "the index as it stands");
        assert_eq!(copy.slots.len(), map.slots.len());
        assert_eq!(
            (copy.head, copy.tail, copy.free),
            (map.head, map.tail, map.free)
        );
        assert_eq!(copy.first(), map.first());
        assert_eq!(copy.put(9, 90), PutOutcome::Inserted);
        assert_eq!(copy.link_of(&9u32), Some(link_to(1)));
        assert_eq!(map.get(&9u32), None);
        assert_eq!(map.len(), 3);

        let mut slots: [LentIndexedSlot<u32, u32>; 4] = indexed_run();
        let mut cells = [LentCell::EMPTY; 8];
        let mut lent =
            BoundedMap::<u32, u32, 4, LentIndexed<'_>>::over(SEED, &mut slots, &mut cells)
                .expect("as many slots as the bound");
        for key in 0..4u32 {
            assert_eq!(lent.put(key, key * 10), PutOutcome::Inserted);
        }
        assert_eq!(lent.remove(&1u32), RemoveOutcome::Removed);
        let mut owned: BoundedMap<u32, u32, 4, Inline> = lent.to_owned_storage();
        for key in [0u32, 2, 3] {
            let slot = lent.slot_of(&key).expect("held");
            assert_eq!(owned.link_of(&key), Some(slot.index + 1), "key {key}");
            assert_eq!(owned.get(&key), Some(&(key * 10)), "key {key}");
        }
        assert_eq!(owned.put(9, 90), PutOutcome::Inserted);
        assert_eq!(owned.link_of(&9u32), Some(link_to(1)));
        assert_eq!(lent.get(&9u32), None);
    }

    /// The slot of a map in storage that holds its own slots laid out as it
    /// was before any storage kept a generation (docs/adr/0154, D2): its entry,
    /// its stamp and its two links, and nothing more.
    #[allow(dead_code)]
    struct SlotWithoutGeneration<K, V, H> {
        entry: Option<(K, V)>,
        stamp: H,
        prev: u32,
        next: u32,
    }

    /// docs/adr/0154, D2: a storage that names no slot to its host pays
    /// nothing for the slots that do. The slot of an [`Inline`], [`Heap`] and
    /// [`Linear`] map is as large as one with no generation, for keys and
    /// values of several widths, a position of such a map is one `u32`, and
    /// an inline map is as large as its slots and index with no generation
    /// make it; a lent map's slot carries its generation, and its position
    /// carries one too.
    #[test]
    fn a_storage_that_names_no_slot_pays_nothing_for_a_generation() {
        fn same<K, V, H>() {
            assert_eq!(
                core::mem::size_of::<Slot<K, V, H, ()>>(),
                core::mem::size_of::<SlotWithoutGeneration<K, V, H>>()
            );
            assert_eq!(
                core::mem::align_of::<Slot<K, V, H, ()>>(),
                core::mem::align_of::<SlotWithoutGeneration<K, V, H>>()
            );
        }
        same::<u32, u32, u64>();
        same::<u64, u64, u64>();
        same::<u8, (), u64>();
        same::<u16, u32, ()>();
        same::<u64, u64, ()>();
        same::<SceString<8>, u32, u64>();
        assert_eq!(core::mem::size_of::<Slot<u64, u64, u64, ()>>(), 40);
        assert_eq!(core::mem::size_of::<Slot<u32, u32, (), ()>>(), 20);
        assert_eq!(
            core::mem::size_of::<heapless::Vec<Slot<u64, u64, u64, ()>, 16>>(),
            core::mem::size_of::<heapless::Vec<SlotWithoutGeneration<u64, u64, u64>, 16>>()
        );
        assert_eq!(core::mem::size_of::<MapCursor>(), 4);
        assert_eq!(core::mem::size_of::<MapCursor<()>>(), 4);
        assert_eq!(core::mem::size_of::<MapCursor<u32>>(), 8);
        assert_eq!(
            core::mem::size_of::<LentSlot<u32, u32>>(),
            core::mem::size_of::<Slot<u32, u32, (), ()>>() + 4
        );
    }

    /// docs/adr/0154, D1, what the `unsafe` of a lent run rests on: the map
    /// reads and writes its host's run at the run's address and length and no
    /// further, and takes no more slots than the run holds, through a long
    /// sequence of puts, removes and clears; the host's slot and cell are laid
    /// out as the slot and the cell the map reads them as. A map over a run is
    /// sent and shared between threads where its keys and values are, as the
    /// borrow it holds is.
    #[test]
    fn a_lent_map_reaches_only_the_run_it_was_lent() {
        assert_eq!(
            core::mem::size_of::<LentSlot<u64, u32>>(),
            core::mem::size_of::<Slot<u64, u32, (), u32>>()
        );
        assert_eq!(
            core::mem::align_of::<LentSlot<u64, u32>>(),
            core::mem::align_of::<Slot<u64, u32, (), u32>>()
        );
        assert_eq!(
            core::mem::size_of::<LentIndexedSlot<u8, u16>>(),
            core::mem::size_of::<Slot<u8, u16, u64, u32>>()
        );
        assert_eq!(
            core::mem::align_of::<LentIndexedSlot<u8, u16>>(),
            core::mem::align_of::<Slot<u8, u16, u64, u32>>()
        );
        assert_eq!(
            core::mem::size_of::<LentCell>(),
            core::mem::size_of::<u32>()
        );
        assert_eq!(
            core::mem::align_of::<LentCell>(),
            core::mem::align_of::<u32>()
        );

        let mut slots: [LentIndexedSlot<u32, u32>; 3] = indexed_run();
        let mut cells = [LentCell::EMPTY; 6];
        let (slots_at, cells_at) = (slots.as_ptr() as usize, cells.as_ptr() as usize);
        let mut map =
            BoundedMap::<u32, u32, 8, LentIndexed<'_>>::over(SEED, &mut slots, &mut cells)
                .expect("within the bound");
        assert_eq!(map.slots.region.start() as usize, slots_at);
        assert_eq!(map.slots.region.length(), 3);
        assert_eq!(map.index.cells.0.start() as usize, cells_at);
        assert_eq!(map.index.cells.0.length(), 6);
        let mut random = Xorshift(0x51ed_27a3_9e1b_c40d);
        for step in 0..2_000u32 {
            let draw = random.draw();
            let key = (draw % 6) as u32;
            match (draw >> 8) % 8 {
                0..=4 => {
                    let _ = map.put(key, step);
                }
                5 | 6 => {
                    let _ = map.remove(&key);
                }
                _ => map.clear(),
            }
            assert!(map.slots.taken <= map.slots.region.length(), "step {step}");
            assert!(map.len() <= map.limit(), "step {step}");
            assert_eq!(map.slots.region.start() as usize, slots_at, "step {step}");
        }

        fn sendable<T: Send + Sync>() {}
        sendable::<BoundedMap<u32, u32, 8, Lent<'static>>>();
        sendable::<BoundedMap<u32, u32, 8, LentIndexed<'static>>>();
        sendable::<BoundedSet<u32, 8, Lent<'static>>>();
    }

    /// A value that counts the drops of its kind.
    struct Counted<'c>(&'c core::cell::Cell<u32>);

    impl Drop for Counted<'_> {
        fn drop(&mut self) {
            self.0.set(self.0.get() + 1);
        }
    }

    /// docs/adr/0154, D1: the entries of a lent map are the map's. A remove, a
    /// replace and a clear drop the values they take out, and a map dropped
    /// drops the entries it left in its host's run, so the run lent again
    /// holds none of them and the map built over it starts empty.
    #[test]
    fn a_lent_map_drops_the_entries_it_leaves_in_its_run() {
        let drops = core::cell::Cell::new(0u32);
        let mut slots: [LentSlot<u32, Counted<'_>>; 4] = lent_run();
        {
            let mut map = BoundedMap::<u32, Counted<'_>, 4, Lent<'_>>::over(SEED, &mut slots)
                .expect("as many slots as the bound");
            for key in 0..3u32 {
                assert_eq!(map.put(key, Counted(&drops)), PutOutcome::Inserted);
            }
            assert_eq!(map.remove(&1u32), RemoveOutcome::Removed);
            assert_eq!(drops.get(), 1);
            assert_eq!(map.put(0, Counted(&drops)), PutOutcome::Replaced);
            assert_eq!(drops.get(), 2);
        }
        assert_eq!(
            drops.get(),
            4,
            "the map dropped drops the two entries it held"
        );
        assert!(slots.iter().all(|slot| !slot.holds_entry()));
        let mut map = BoundedMap::<u32, Counted<'_>, 4, Lent<'_>>::over(SEED, &mut slots)
            .expect("lent again");
        assert!(map.is_empty() && map.get(&0u32).is_none());
        assert_eq!(map.put(5, Counted(&drops)), PutOutcome::Inserted);
        map.clear();
        assert_eq!(drops.get(), 5);
    }

    /// docs/adr/0154, D1: a run lent again holds nothing of the map it was
    /// lent to before. The cells an indexed map left naming its slots are
    /// emptied by the map built over them next, which finds none of the keys
    /// the first held and takes them again from its first slot.
    #[test]
    fn a_run_lent_again_holds_nothing_of_the_map_before() {
        let mut slots: [LentIndexedSlot<u32, u32>; 3] = indexed_run();
        let mut cells = [LentCell::EMPTY; 6];
        assert!(!LentCell::EMPTY.names_slot());
        {
            let mut first =
                BoundedMap::<u32, u32, 3, LentIndexed<'_>>::over(SEED, &mut slots, &mut cells)
                    .expect("as many slots as the bound");
            for key in [5u32, 6, 7] {
                assert_eq!(first.put(key, key), PutOutcome::Inserted);
            }
        }
        assert!(cells.iter().any(|cell| cell.names_slot()));
        assert!(slots.iter().all(|slot| !slot.holds_entry()));
        let mut again =
            BoundedMap::<u32, u32, 3, LentIndexed<'_>>::over(SEED, &mut slots, &mut cells)
                .expect("lent again");
        for key in [5u32, 6, 7] {
            assert_eq!(again.get(&key), None, "key {key}");
        }
        assert_eq!(again.put_slot(6, 60).1.map(|slot| slot.index), Some(0));
        assert_eq!(again.get(&6u32), Some(&60));
        assert_eq!(again.len(), 1);
    }

    /// docs/adr/0154, D1 and D2: a set lent its slots answers as an inline set
    /// given the same puts and removes, the same slot for each, reads a key
    /// back by its slot, and is copied into its owned storage with each key at
    /// its slot; an indexed set lent two slots holds two keys.
    #[test]
    fn a_lent_set_holds_its_keys_in_the_slots_it_was_lent() {
        let mut slots: [LentSlot<u8, ()>; 3] = lent_run();
        let mut tags = BoundedSet::<u8, 3, Lent<'_>>::over(SEED, &mut slots)
            .expect("as many slots as the bound");
        let mut inline = BoundedSet::<u8, 3, Inline>::with_seed(SEED);
        for key in [4u8, 1, 4, 7, 2, 1] {
            let (outcome, slot) = tags.put_slot(key);
            assert_eq!(outcome, inline.put(key), "key {key}");
            assert_eq!(
                slot.map(|slot| slot.index + 1),
                inline.0.link_of(&key),
                "key {key}"
            );
        }
        assert_eq!(tags.remove(&4u8), inline.remove(&4u8));
        assert_eq!(tags.put(9), inline.put(9));
        assert!(tags == inline && inline == tags);
        assert_eq!(tags.limit(), 3);
        let held = tags.slot_of(&7u8).expect("held");
        assert_eq!(tags.at_slot(held), Some(&7));
        let copy: BoundedSet<u8, 3, Linear> = tags.to_owned_storage();
        assert!(copy == tags);
        assert_eq!(copy.0.link_of(&7u8), Some(held.index + 1));

        let mut pair: [LentIndexedSlot<u8, ()>; 2] = indexed_run();
        let mut cells = [LentCell::EMPTY; 4];
        let mut two = BoundedSet::<u8, 3, LentIndexed<'_>>::over(SEED, &mut pair, &mut cells)
            .expect("two slots within three");
        assert_eq!(two.put(1), PutOutcome::Inserted);
        assert_eq!(two.put(2), PutOutcome::Inserted);
        assert_eq!(two.put(3), PutOutcome::Full);
        assert!(two.contains_key(&2u8) && !two.contains_key(&3u8));
        assert!(two.iter().copied().eq([1, 2]));
    }
}
