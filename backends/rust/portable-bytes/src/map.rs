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
//!   clears once they outnumber the live entries.
//!
//! The hash is SipHash-1-3 under a 128-bit key each map is built with
//! ([`MapSeed`]), over the canonical bytes of the key ([`MapKey`]), so a peer
//! that chooses the keys cannot steer them into one chain of the index
//! without the seed. Nothing a document or a host sees depends on the hash: the
//! order is the links', and a map restored under another seed is the same map.
//!
//! Where an allocator is available the slots and the index grow as the map is
//! filled, up to what the bound needs and never past it; where none is, the map
//! holds them inline, the bound reserved (D4, "Storage").

use crate::{BoundedList, SceString};

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

/// The end of a list of slots: no slot.
const NIL: u32 = u32::MAX;
/// An index cell no key ever took.
const EMPTY: u32 = u32::MAX;
/// An index cell whose key was removed: a probe goes on past it, and a put
/// may take it.
const TOMBSTONE: u32 = u32::MAX - 1;

/// One slot of a map: its entry, or none for a slot on the free list; the hash
/// of its key, which a rebuild of the index reads instead of hashing again; and
/// its neighbours, in the map's order for a slot in use and on the free list
/// (`next` alone) for one that is not.
#[derive(Clone)]
struct Slot<K, V> {
    entry: Option<(K, V)>,
    hash: u64,
    prev: u32,
    next: u32,
}

/// Where a map keeps its slots: a `Vec` that grows as slots are first taken
/// where an allocator is available, and `N` inline where none is.
#[cfg(feature = "alloc")]
type Slots<K, V, const N: usize> = alloc::vec::Vec<Slot<K, V>>;
#[cfg(not(feature = "alloc"))]
type Slots<K, V, const N: usize> = heapless::Vec<Slot<K, V>, N>;

/// A map's index: cells that each hold the slot of a key, [`EMPTY`] or
/// [`TOMBSTONE`]. Twice the bound at most, so a full map leaves it half
/// empty; grown to that as the map fills where an allocator is available, and
/// that long inline, two rows of `N`, where none is.
#[derive(Clone)]
struct Index<const N: usize> {
    #[cfg(feature = "alloc")]
    cells: alloc::vec::Vec<u32>,
    #[cfg(not(feature = "alloc"))]
    cells: [[u32; N]; 2],
}

impl<const N: usize> Index<N> {
    /// The most cells an index of a map of `N` entries holds.
    const MOST: usize = 2 * N;

    #[cfg(feature = "alloc")]
    fn new() -> Self {
        Self {
            cells: alloc::vec::Vec::new(),
        }
    }

    #[cfg(not(feature = "alloc"))]
    fn new() -> Self {
        Self {
            cells: [[EMPTY; N]; 2],
        }
    }

    #[cfg(feature = "alloc")]
    fn len(&self) -> usize {
        self.cells.len()
    }

    #[cfg(not(feature = "alloc"))]
    fn len(&self) -> usize {
        Self::MOST
    }

    #[cfg(feature = "alloc")]
    fn get(&self, cell: usize) -> u32 {
        self.cells[cell]
    }

    #[cfg(not(feature = "alloc"))]
    fn get(&self, cell: usize) -> u32 {
        self.cells[cell / N][cell % N]
    }

    #[cfg(feature = "alloc")]
    fn set(&mut self, cell: usize, value: u32) {
        self.cells[cell] = value;
    }

    #[cfg(not(feature = "alloc"))]
    fn set(&mut self, cell: usize, value: u32) {
        self.cells[cell / N][cell % N] = value;
    }

    /// Every cell empty, `len` of them where the index grows.
    #[cfg(feature = "alloc")]
    fn reset(&mut self, len: usize) {
        self.cells.clear();
        self.cells.resize(len, EMPTY);
    }

    #[cfg(not(feature = "alloc"))]
    fn reset(&mut self, _len: usize) {
        self.cells = [[EMPTY; N]; 2];
    }

    /// How long the index must be for `occupied` cells to fill at most half of
    /// it: its own length while that does, and otherwise doubled from it, from
    /// eight for an index never grown, never past [`Self::MOST`].
    fn length_for(&self, occupied: usize) -> usize {
        let mut len = self.len();
        if len == 0 {
            len = Self::MOST.min(8);
        }
        while occupied.saturating_mul(2) > len && len < Self::MOST {
            len = len.saturating_mul(2).min(Self::MOST);
        }
        len
    }
}

/// What a probe of the index found for a key.
enum Probe {
    /// The key, at this cell of the index and this slot.
    Found { cell: usize, slot: u32 },
    /// Not the key; a put takes this cell for it.
    Vacant(usize),
    /// Not the key, and no cell for it: an index not yet grown.
    NoRoom,
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
    /// The key was absent and the map holds its bound: the map is as it was.
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

/// A position in a map's order, which a walk of the map steps through
/// ([`BoundedMap::first`], [`BoundedMap::next_after`]) and reads the entry at
/// ([`BoundedMap::key_at`], [`BoundedMap::value_at`]) without holding a borrow
/// of the map between two reads. It stays where it is while the map is not
/// written; a generated machine walks a copy of a map its loop writes
/// (docs/adr/0108, D6 item 3).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct MapCursor(u32);

/// A map of at most `N` entries, each a key of `K` and a value of `V`, no two
/// keys alike, in the order each key was first put (docs/adr/0108): the value
/// of a `map<K, V>[N]`. See the [module](self) for how it holds them.
#[derive(Clone)]
pub struct BoundedMap<K, V, const N: usize> {
    seed: MapSeed,
    slots: Slots<K, V, N>,
    index: Index<N>,
    head: u32,
    tail: u32,
    free: u32,
    live: u32,
    tombstones: u32,
}

impl<K, V, const N: usize> BoundedMap<K, V, N> {
    /// The bound the type declares.
    pub const CAPACITY: usize = N;

    /// A bound a slot's number holds, with [`NIL`] and [`TOMBSTONE`] apart:
    /// named where a map is built, so a bound past it does not compile.
    const NUMBERED: () = assert!(N < (u32::MAX - 1) as usize, "a map's bound is a uint32");

    /// The empty map, its keys hashed under `seed`.
    pub fn with_seed(seed: MapSeed) -> Self {
        let () = Self::NUMBERED;
        Self {
            seed,
            slots: Slots::<K, V, N>::new(),
            index: Index::new(),
            head: NIL,
            tail: NIL,
            free: NIL,
            live: 0,
            tombstones: 0,
        }
    }

    /// The empty map of the seed this one was built with: what a variable is
    /// declared with again before its machine starts.
    pub fn empty_like(&self) -> Self {
        Self::with_seed(self.seed)
    }

    /// How many entries the map holds.
    pub fn len(&self) -> usize {
        self.live as usize
    }

    /// Whether the map holds no entry.
    pub fn is_empty(&self) -> bool {
        self.live == 0
    }

    /// Empty the map, which then takes up to `N` entries again. Its storage is
    /// kept for the entries it takes next.
    pub fn clear(&mut self) {
        self.slots.clear();
        let len = self.index.len();
        self.index.reset(len);
        self.head = NIL;
        self.tail = NIL;
        self.free = NIL;
        self.live = 0;
        self.tombstones = 0;
    }

    /// The entries, in the map's order.
    pub fn iter(&self) -> Iter<'_, K, V, N> {
        Iter {
            map: self,
            at: self.head,
            left: self.len(),
        }
    }

    /// The position of the first entry, or `None` for an empty map.
    pub fn first(&self) -> Option<MapCursor> {
        (self.head != NIL).then_some(MapCursor(self.head))
    }

    /// The position of the entry after `at`, or `None` after the last.
    pub fn next_after(&self, at: MapCursor) -> Option<MapCursor> {
        let next = self.slots.get(at.0 as usize)?.next;
        (next != NIL).then_some(MapCursor(next))
    }

    /// The key of the entry at `at`.
    ///
    /// # Panics
    ///
    /// When no entry is at `at`: the map was written since the position was
    /// read, which a walk of a map it writes does on a copy instead.
    pub fn key_at(&self, at: MapCursor) -> &K {
        &self.entry_at(at).0
    }

    /// The value of the entry at `at`.
    ///
    /// # Panics
    ///
    /// As [`Self::key_at`].
    pub fn value_at(&self, at: MapCursor) -> &V {
        &self.entry_at(at).1
    }

    fn entry_at(&self, at: MapCursor) -> &(K, V) {
        match self
            .slots
            .get(at.0 as usize)
            .and_then(|slot| slot.entry.as_ref())
        {
            Some(entry) => entry,
            None => panic!("a map position names no entry: the map was written during its walk"),
        }
    }

    fn slot(&self, at: u32) -> &Slot<K, V> {
        &self.slots[at as usize]
    }

    fn slot_mut(&mut self, at: u32) -> &mut Slot<K, V> {
        &mut self.slots[at as usize]
    }

    /// Take a slot for an entry, from the free list or past the slots taken so
    /// far, and link it last. `None` when no slot is left, which a map that
    /// holds fewer than `N` entries always has.
    fn store(&mut self, key: K, value: V, hash: u64) -> Option<u32> {
        let slot = Slot {
            entry: Some((key, value)),
            hash,
            prev: self.tail,
            next: NIL,
        };
        let at = if self.free != NIL {
            let at = self.free;
            self.free = self.slot(at).next;
            *self.slot_mut(at) = slot;
            at
        } else {
            let at = self.slots.len();
            if at >= N {
                return None;
            }
            #[cfg(feature = "alloc")]
            {
                self.slots.push(slot);
            }
            #[cfg(not(feature = "alloc"))]
            {
                if self.slots.push(slot).is_err() {
                    return None;
                }
            }
            at as u32
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
    /// its entry dropped.
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
        slot.prev = NIL;
        slot.next = free;
        self.free = at;
    }

    /// The index again, `len` cells long, with every entry at the cell its
    /// stored hash first probes free and no tombstone left.
    fn rebuild(&mut self, len: usize) {
        self.index.reset(len);
        self.tombstones = 0;
        if len == 0 {
            return;
        }
        let mut at = self.head;
        while at != NIL {
            let mut cell = home(self.slot(at).hash, len);
            while self.index.get(cell) != EMPTY {
                cell = if cell + 1 == len { 0 } else { cell + 1 };
            }
            self.index.set(cell, at);
            at = self.slot(at).next;
        }
    }
}

impl<K: MapKey, V, const N: usize> BoundedMap<K, V, N> {
    /// Whether `key` is a key of the map.
    pub fn contains_key<Q: MapKey + ?Sized>(&self, key: &Q) -> bool {
        matches!(
            self.probe(key, hash_of(&self.seed, key)),
            Probe::Found { .. }
        )
    }

    /// The value at `key`, or `None` where it is absent.
    pub fn get<Q: MapKey + ?Sized>(&self, key: &Q) -> Option<&V> {
        match self.probe(key, hash_of(&self.seed, key)) {
            Probe::Found { slot, .. } => self.slot(slot).entry.as_ref().map(|(_, value)| value),
            Probe::Vacant(_) | Probe::NoRoom => None,
        }
    }

    /// Set the value at `key` to `value`: a key that is present keeps its place
    /// and takes the value, and one that is absent goes last while the map
    /// holds fewer than `N` entries. Into a full map an absent key is not put,
    /// and the map is as it was ([`PutOutcome::Full`]).
    pub fn put(&mut self, key: K, value: V) -> PutOutcome {
        let hash = hash_of(&self.seed, &key);
        if let Probe::Found { slot, .. } = self.probe(&key, hash) {
            if let Some(entry) = self.slot_mut(slot).entry.as_mut() {
                entry.1 = value;
            }
            return PutOutcome::Replaced;
        }
        if self.len() >= N {
            return PutOutcome::Full;
        }
        // Room in the index for one more occupied cell, at most half of it
        // occupied while it can still grow.
        let occupied = (self.live + self.tombstones) as usize + 1;
        let len = self.index.length_for(occupied);
        if len != self.index.len() {
            self.rebuild(len);
        }
        let Probe::Vacant(cell) = self.probe(&key, hash) else {
            return PutOutcome::Full;
        };
        let Some(at) = self.store(key, value, hash) else {
            return PutOutcome::Full;
        };
        if self.index.get(cell) == TOMBSTONE {
            self.tombstones -= 1;
        }
        self.index.set(cell, at);
        self.live += 1;
        PutOutcome::Inserted
    }

    /// The entries of a map of at most `M` entries, in its order, each key and
    /// value made by `key` and `value` of the one it holds, in a map of `N`:
    /// the copy a machine makes of a map into a place of a larger bound, or of
    /// texts of larger bounds, which its build proved the map fits
    /// (docs/adr/0108, D2). The fit of the bounds is a build error where it
    /// does not hold ([`crate::Fits`]), and a copy of a key is the same key, so
    /// no entry is refused. The copy hashes under the seed of the map it copies.
    pub fn widened_map<K2, V2, const M: usize>(
        from: &BoundedMap<K2, V2, M>,
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

    /// Take the entry at `key` out of the map, the others keeping their order
    /// and their slots; a key that is absent leaves the map as it was.
    pub fn remove<Q: MapKey + ?Sized>(&mut self, key: &Q) -> RemoveOutcome {
        let Probe::Found { cell, slot } = self.probe(key, hash_of(&self.seed, key)) else {
            return RemoveOutcome::Absent;
        };
        self.index.set(cell, TOMBSTONE);
        self.tombstones += 1;
        self.release(slot);
        self.live -= 1;
        if self.tombstones > self.live {
            let len = self.index.len();
            self.rebuild(len);
        }
        RemoveOutcome::Removed
    }

    /// Where `key`, of hash `hash`, is in the index, or the cell a put of it
    /// takes: the first tombstone the probe passed, or the empty cell it ended
    /// at.
    fn probe<Q: MapKey + ?Sized>(&self, key: &Q, hash: u64) -> Probe {
        let len = self.index.len();
        if len == 0 {
            return Probe::NoRoom;
        }
        let mut cell = home(hash, len);
        let mut vacant = None;
        for _ in 0..len {
            match self.index.get(cell) {
                EMPTY => return Probe::Vacant(vacant.unwrap_or(cell)),
                TOMBSTONE => {
                    vacant.get_or_insert(cell);
                }
                at => {
                    let slot = self.slot(at);
                    let found = slot.hash == hash
                        && slot
                            .entry
                            .as_ref()
                            .is_some_and(|(held, _)| same_key(held, key));
                    if found {
                        return Probe::Found { cell, slot: at };
                    }
                }
            }
            cell = if cell + 1 == len { 0 } else { cell + 1 };
        }
        vacant.map_or(Probe::NoRoom, Probe::Vacant)
    }
}

impl<K, V, const N: usize> FromSeed for BoundedMap<K, V, N> {
    fn from_seed(seed: MapSeed) -> Self {
        Self::with_seed(seed)
    }
}

/// Formatted as its entries in order, `{7: 5, 9: -2}`.
impl<K: core::fmt::Debug, V: core::fmt::Debug, const N: usize> core::fmt::Debug
    for BoundedMap<K, V, N>
{
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.debug_map().entries(self.iter()).finish()
    }
}

/// Two maps are equal when they hold equal entries in one order, whatever
/// their seeds: what a host compares two snapshots by. A document compares no
/// two maps (docs/adr/0108, D2).
impl<K: PartialEq, V: PartialEq, const N: usize> PartialEq for BoundedMap<K, V, N> {
    fn eq(&self, other: &Self) -> bool {
        self.len() == other.len()
            && self
                .iter()
                .zip(other.iter())
                .all(|((k1, v1), (k2, v2))| k1 == k2 && v1 == v2)
    }
}

impl<K: Eq, V: Eq, const N: usize> Eq for BoundedMap<K, V, N> {}

/// The entries of a map in its order, each its key and its value.
pub struct Iter<'m, K, V, const N: usize> {
    map: &'m BoundedMap<K, V, N>,
    at: u32,
    left: usize,
}

impl<'m, K, V, const N: usize> Iterator for Iter<'m, K, V, N> {
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

impl<K, V, const N: usize> ExactSizeIterator for Iter<'_, K, V, N> {}

impl<'m, K, V, const N: usize> IntoIterator for &'m BoundedMap<K, V, N> {
    type Item = (&'m K, &'m V);
    type IntoIter = Iter<'m, K, V, N>;

    fn into_iter(self) -> Self::IntoIter {
        self.iter()
    }
}

// ── the set ─────────────────────────────────────────────────────────────────

/// A set of at most `N` keys of `K`, no two alike, in the order each was first
/// put (docs/adr/0108, D1): the value of a `set<K>[N]`, the map whose entries
/// carry no value.
#[derive(Clone, PartialEq, Eq)]
pub struct BoundedSet<K, const N: usize>(BoundedMap<K, (), N>);

impl<K, const N: usize> BoundedSet<K, N> {
    /// The bound the type declares.
    pub const CAPACITY: usize = N;

    /// The empty set, its keys hashed under `seed`.
    pub fn with_seed(seed: MapSeed) -> Self {
        Self(BoundedMap::with_seed(seed))
    }

    /// The empty set of the seed this one was built with.
    pub fn empty_like(&self) -> Self {
        Self(self.0.empty_like())
    }

    /// How many keys the set holds.
    pub fn len(&self) -> usize {
        self.0.len()
    }

    /// Whether the set holds no key.
    pub fn is_empty(&self) -> bool {
        self.0.is_empty()
    }

    /// Empty the set.
    pub fn clear(&mut self) {
        self.0.clear();
    }

    /// The keys, in the set's order.
    pub fn iter(&self) -> SetIter<'_, K, N> {
        SetIter(self.0.iter())
    }

    /// The position of the first key, or `None` for an empty set.
    pub fn first(&self) -> Option<MapCursor> {
        self.0.first()
    }

    /// The position of the key after `at`, or `None` after the last.
    pub fn next_after(&self, at: MapCursor) -> Option<MapCursor> {
        self.0.next_after(at)
    }

    /// The key at `at`.
    ///
    /// # Panics
    ///
    /// As [`BoundedMap::key_at`].
    pub fn key_at(&self, at: MapCursor) -> &K {
        self.0.key_at(at)
    }
}

impl<K: MapKey, const N: usize> BoundedSet<K, N> {
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

    /// The keys of a set of at most `M`, in its order, each made by `key` of
    /// the one it holds, in a set of `N` ([`BoundedMap::widened_map`]).
    pub fn widened_map<K2, const M: usize>(
        from: &BoundedSet<K2, M>,
        key: impl FnMut(&K2) -> K,
    ) -> Self {
        Self(BoundedMap::widened_map(&from.0, key, |_| ()))
    }
}

impl<K, const N: usize> FromSeed for BoundedSet<K, N> {
    fn from_seed(seed: MapSeed) -> Self {
        Self::with_seed(seed)
    }
}

/// Formatted as its keys in order, `{"red", "blue"}`.
impl<K: core::fmt::Debug, const N: usize> core::fmt::Debug for BoundedSet<K, N> {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.debug_set().entries(self.iter()).finish()
    }
}

/// The keys of a set in its order.
pub struct SetIter<'s, K, const N: usize>(Iter<'s, K, (), N>);

impl<'s, K, const N: usize> Iterator for SetIter<'s, K, N> {
    type Item = &'s K;

    fn next(&mut self) -> Option<Self::Item> {
        self.0.next().map(|(key, ())| key)
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        self.0.size_hint()
    }
}

impl<K, const N: usize> ExactSizeIterator for SetIter<'_, K, N> {}

impl<'s, K, const N: usize> IntoIterator for &'s BoundedSet<K, N> {
    type Item = &'s K;
    type IntoIter = SetIter<'s, K, N>;

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

impl<K: MapKey, V, const N: usize> Keyed for BoundedMap<K, V, N> {
    fn count(&self) -> u32 {
        self.live
    }
    fn holds<Q: MapKey + ?Sized>(&self, key: &Q) -> bool {
        self.contains_key(key)
    }
}

impl<K: MapKey, const N: usize> Keyed for BoundedSet<K, N> {
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
pub fn value<'m, K: MapKey, V, Q: MapKey + ?Sized, const N: usize>(
    map: &'m BoundedMap<K, V, N>,
    key: &Q,
) -> Option<&'m V> {
    map.get(key)
}

/// `m[k]`: the value at `key`, a copy the place it is written to takes; `None`
/// where the key is absent.
pub fn value_cloned<K: MapKey, V: Clone, Q: MapKey + ?Sized, const N: usize>(
    map: &BoundedMap<K, V, N>,
    key: &Q,
) -> Option<V> {
    map.get(key).cloned()
}

#[cfg(test)]
mod tests {
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
    /// removes and clears. Many operations, drawn over a few keys so each is
    /// put, replaced and removed again and again, run on a map and on a list of
    /// its entries in order; after each the two hold the same entries in the
    /// same order, and each key the list holds is found and each it does not is
    /// not. Over a small bound, so the map is full often and its tombstones are
    /// rebuilt away often.
    #[test]
    fn order_survives_any_sequence_of_puts_removes_and_clears() {
        const BOUND: usize = 12;
        let mut map = BoundedMap::<u16, u32, BOUND>::with_seed(SEED);
        let mut model: heapless::Vec<(u16, u32), BOUND> = heapless::Vec::new();
        let mut random = Xorshift(0x9e37_79b9_7f4a_7c15);
        for step in 0..20_000u32 {
            let draw = random.draw();
            let key = (draw % 24) as u16;
            let value = (draw >> 32) as u32;
            match (draw >> 8) % 16 {
                0..=8 => {
                    let expected = match model.iter_mut().find(|(k, _)| *k == key) {
                        Some(entry) => {
                            entry.1 = value;
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
            assert!(map.tombstones <= map.live, "step {step}");
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
                assert!(map.tombstones <= map.live);
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
}
