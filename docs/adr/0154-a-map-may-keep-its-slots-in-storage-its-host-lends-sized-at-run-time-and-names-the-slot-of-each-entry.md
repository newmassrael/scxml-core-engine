# ADR 0154 — A map may keep its slots in storage its host lends, sized at run time, and names the slot of each entry

- Status: Proposed (2026-11-28), for the owner's acceptance. Written under the owner's standing rule (the most
  long-term correct structure, textbook in its foundations, cost ignored; decided, not asked) at the request of a
  downstream consumer, whose request named the decisions below; each is decided by the textbook rule, and the
  acceptance may overturn any of them. The Rust step (Order of work, step 2) lands beside this record.
- Date: 2026-11-28
- Amends, once accepted (each amended document gains an "Amended by: ADR 0154" line in the landing that moves it):
  - ADR 0108 D4 "Storage": two storages a host lends, and a limit below the bound that the storage, not the
    document, sets (D1);
  - ADR 0108 D2: a host reads the slot each entry is held in; a document still sees no slot (D2);
  - ADR 0108 D4 "Storage" and D8: a map an algorithm builds for itself, and the copy a walk takes, is kept in the
    owned storage of the map it follows, which is that map's own storage for every storage that holds its slots
    (D3);
  - `docs/SCE_ACCEPTED_SUBSET.md` §2.15, the paragraph "Rust's map";
  - `docs/SCE_CODEC_CONSUMER_MIGRATION.md`, a section for the landing.
- Scope: the Rust leaf crate `sce-portable-bytes` (`map`), the Rust runtime's saved state, the Rust generator's
  algorithm lowering of a map (`sce-build/src/forge/rust_algorithm_map.rs`) and the Rust conformance harness. The C11
  runtime is step 3; no other lane changes (D4).
- Related: ADR 0082 (an algorithm writes into storage its caller holds), ADR 0086 (a bound is part of a type), ADR 0108
  (a map; D2, D4, D8), ADR 0111 D4 (a pool made over storage its caller holds, its count a value under the bound, and
  a slot index of a number and a generation).

## Context

A consumer is moving a session's tables (its subscribers, the replies it waits for, the declarations of its remote,
its queries, its tokens, its tables of ids) from hand-written code onto the map a generated algorithm writes
(`BoundedMap<K, V, N, S>`). Its C ABI is `no_std` and has no heap: when a session opens it borrows, from the
session's memory, as many rows of each table as the session's configuration gives, it allocates nothing after
that, and a table configured empty costs no byte.

What exists, checked at `0386870fe29`:

- **A map's storage is sealed, three ways.** `MapStorage` is `Inline` (`N` slots and `2N` index cells in the map
  value), `Heap` (grown under the crate's `alloc` feature) and `Linear` (`N` slots in the value, no index). A table
  bounded at the most any configuration allows reserves that bound in every session that holds it, however few rows
  the session was configured for, or it needs an allocator.
- **The generated algorithm is generic over the storage already.** It takes each map `&` or `&mut` as
  `BoundedMap<K, V, N0, S0>` with `S0: MapStorage` (ADR 0108 D4, amended 2026-11-21). But it builds each map of
  its own in `S0` and takes the copy a walk needs by `Clone`, both of which assume that a map of `S0` can be made
  from nothing.
- **No operation moves an entry** (ADR 0108 D8): a put takes a free slot, a remove gives it back, a rebuild of the
  index rewrites cells only. But the slot is not visible. A host that keeps state of its own beside each row (a
  handle table of its ABI) has no stable number for the row, and `MapCursor` names a slot with no generation, so a
  number kept past a remove names whatever entry takes the slot next.
- **ADR 0111 D4 has the pattern.** A pool is made over storage its caller holds, `Pool::over(bytes, c, l)`, its
  count `c` a run-time value under the build bound, refused past it; a slot crosses an interface as a `SlotIndex`,
  a number and a generation.

## The textbook structure

- **A container with no allocator is built over memory its caller owns.** Its capacity is the length of what it was
  given, and it never grows (the C idiom `init(buffer, length)`; in Rust, a container that holds a `&'a mut [T]`
  and lives no longer than `'a`). ADR 0082 and ADR 0111 D4 already give SCE this shape for lists and pools.
- **The type holds the bound, the value the limit** (ADR 0111, U16): one saved state means one thing on every build,
  and storage never grows.
- **A handle that crosses an interface is a generational index** (the slot-map pattern, ADR 0111): a number alone is
  reused once its slot is freed (ABA); a number paired with a generation that moves on each free is refused when
  stale.
- **A borrow is a lifetime.** In Rust a value made over borrowed memory carries the borrow's lifetime in its type,
  so the compiler refuses any use of it past the memory.

## Decision

### D1. Two more storages, in runs the host lends: `Lent<'a>` and `LentIndexed<'a>`

- **`Lent<'a>`** keeps the map's slots in a run of `LentSlot<K, V>` its host lends for `'a`, and no index: a lookup
  compares the key with each key the map holds, as `Linear`'s does, and hashes no key.
- **`LentIndexed<'a>`** keeps the slots in a run of `LentIndexedSlot<K, V>` and the hash index in a run of
  `LentCell`s, twice as many cells as slots, probed as `Inline`'s index is (ADR 0108 D8), so a full map leaves it
  half empty.
- **The limit is the run's length.** `BoundedMap::over(seed, slots)` (and `over(seed, slots, cells)` for
  `LentIndexed`) builds the empty map; its limit `c` is `slots.len()`, `0 <= c <= N`, read by
  `BoundedMap::limit()` (`N` in every other storage). A run of more than `N` slots is refused where the map is
  built (`LendRefused::PastBound`), and a run of cells shorter than `2c` likewise (`LendRefused::IndexShort`); a
  longer run of cells is used to `2c`. The cells are emptied where the map is built, so a run lent again holds
  nothing of the map it was lent to before. A put of an absent key into a map that holds `c` entries answers
  `Full`, which means here what ADR 0108 D4 says it means: storage is exhausted.
- **The map value holds no slot.** It holds its seed, the ends of its order, its free list, its count and the runs
  (each a start and a length), so its size does not depend on `N`, and a map lent no slot (`c = 0`) borrows no byte.
  `with_seed`, `FromSeed` and `empty_like` in a lent storage build that map, lent nothing, so `with_seed` stays a
  `const fn` for every storage.
- **What a document or a host sees is what it sees of an `Inline` map given the same operations**: the order, what
  each operation answers, the saved form and equality with a map of another storage, as the sentence that says it of
  `Linear` (ADR 0108 D4, amended 2026-11-22). Lent `c < N` slots, the map answers `Full` at `c` where an inline
  map does at `N`.
- **The host's types.** `LentSlot<K, V>`, `LentIndexedSlot<K, V>` and `LentCell` are public, each with an
  `EMPTY` constant and an `empty()` `const fn` to build a run from, and each `#[repr(transparent)]` over the slot
  or the cell the map reads, whose fields stay private. A map dropped drops the entries it left in its run
  (`LentSlot::holds_entry` is then false for every slot).
- **The storage stays sealed**; both are implemented inside the module, and the map's algorithm is still written
  once.
- **The shape of `MapStorage`.** The trait names a storage's slots by a generic associated type,
  `Slots<K, V, const N: usize>`, for keys and values of any type. A storage that carries `'a` would hold a
  `&'a mut [Slot<K, V>]`, which is a type only where `K: 'a` and `V: 'a`, and an implementation may not add to a
  generic associated type a bound its trait does not declare. Two shapes were open. The trait could take the key
  and the value (`MapStorage<K, V>`), so an implementation states `K: 'a`; every bound in the runtime, the saved
  state and each generated algorithm's signature would then spell `S0: MapStorage<K0, V0>`, for nothing the three
  storages that hold their own slots need. Or the lent storages hold each run by its start and its length, the
  lifetime in a marker: the outlives bound was proved where the map was built, by the `&'a mut [LentSlot<K, V>]` it
  took, and the map cannot outlive `'a`. This record takes the second: `MapStorage` keeps its shape and a generated
  signature takes a lent map unchanged. Its cost is one module-private type, the region, with two `unsafe` blocks
  (the shared and the exclusive slice it gives out) and the `Send` and `Sync` of the `&mut [T]` it stands for; the
  start is a raw pointer, so the region is invariant in its element type as the borrow is. The tests hold the
  invariants the blocks rest on: the run's address and length are the host's, the map takes no more slots than the
  run holds, and the host's slot and cell are laid out as the slot and the cell the map reads.

### D2. A host reads the slot an entry is held in, in the storages it lends

- **`MapSlot { index: u32, generation: u32 }`**, `#[repr(C)]`, `Copy`, the shape of ADR 0111's `SlotIndex`. The
  index is below the map's limit, so a host indexes an array of its own of `c` rows by it.
- **An entry stays in its slot from the put that inserts it to the remove that takes it out.** In the code: a put
  of an absent key takes the first free slot or the next slot never taken and links it last; a put of a present key
  writes the value where it stands, as `get_mut` and a nested write do (ADR 0108 D12 item 2); a remove unlinks its
  own slot and frees it; a rebuild of the index and a reseed rewrite cells and stamps and no slot; a `Heap` map that
  grows moves its memory but numbers its slots as before. ADR 0108 D8 says no operation moves an entry; this record
  makes that a promise a host may rely on.
- **The generation belongs to the storages that name their slots.** `MapStorage::Generation` is what each slot keeps
  of how many times it was freed: `u32` for `Lent` and `LentIndexed`, `()` for `Inline`, `Heap` and `Linear`. The
  slot handle is offered where a host lends the slots and needs it, the request that brought this record; no use of
  the three storages that hold their own slots asks for one, and such a use, when one comes, adds the generation to
  its storage then. So an `Inline`, `Heap` or `Linear` slot is as large as before this record and a position
  (`MapCursor<()>`, the default of `MapCursor<G>`) is one `u32`, which a test holds by `core::mem::size_of`, and
  nothing they do changes: a clear gives the slots back, and a clone copies the slots and the index as they stand.
- **The operations, in the lent storages.** `put` keeps its answer everywhere. Where `S::Generation` is `u32`:
  `put_slot` answers `(PutOutcome, Option<MapSlot>)`, the slot the entry is in after the put (`None` with
  `Full`); `slot_of(key)` answers the slot of a key held; `MapCursor::<u32>::slot()` the slot of a position of a
  walk; `at_slot(MapSlot)` the entry in a slot, `Option<(&K, &V)>` (`Option<&K>` for a set). Elsewhere they do not
  compile.
- **The generation moves each time a slot is freed** (a remove, a clear). `at_slot` answers `None` for a number of
  another generation, a slot that holds no entry, or a number past the slots the map took; so where a generated
  algorithm removed an entry and put another in its slot, the number the host kept for the first finds nothing. It
  counts modulo 2^32. A position of a lent map carries the generation it was read at, and `key_at` and `value_at` of
  one whose slot was freed and taken again panic, as a position read before a write of its map already does, rather
  than read the other entry.
- **A clear of a lent map frees every slot it took**, in order, so the slots of the entries it held move their
  generations and the next puts take the slots from the first, the order in which a map that gives its slots back
  takes them again.
- **What it costs.** A slot of a lent map holds four bytes more than the same slot without a generation before
  alignment (a `uint32` key and value, no index: 24 bytes where `Linear` holds 20), and a position of a lent map is
  two `u32`s. A slot never freed is of generation 0, as a link to no slot is 0.

### D3. A map's copy, and a map an algorithm builds beside one, is kept in the map's owned storage

- **`MapStorage::Owned`** names the storage a copy of a map is kept in: the storage itself for `Inline`, `Heap`
  and `Linear`; `Linear` for `Lent` and `Inline` for `LentIndexed`, which hold their slots in the value and find
  a key the same way. A run is lent to one map, so a copy has no run of its own; and the lent storages exist for
  hosts with no heap, so `Heap` is no answer.
- **`to_owned_storage()`** copies a map into its owned storage: each entry in the slot it holds, the free list, the
  order and the seed. For a storage that is its own owned storage the copy is the slots and the index copied as they
  stand, as a clone always was; for a lent storage each slot is copied without its generation, which the copy, naming
  no slot, does not keep, and `LentIndexed`'s index is built again at twice the bound from the slots' stamps, no key
  hashed again. `Clone` is that copy where it is of the same type, `S: MapStorage<Owned = S>`, so a map whose host
  lends its slots is not `Clone`, and a clone of every other map is what it was.
- **What builds a map from nothing takes storage that holds its slots.** `widened_map`, the saved state's
  `map_from_saved` and `set_from_saved`, and `SavedValue` for a map or a set require `S: MapStorage<Owned = S>`.
  A host writes a lent map's saved form with `map_to_saved`, which takes any storage, and restores one by clearing
  it and putting the entries a map read back holds.
- **The generated Rust algorithm** builds each map of its own in `<S0 as MapStorage>::Owned`, the owned storage of
  the map its seed is drawn from, where it built them in `S0`, and a walk whose body writes its map walks
  `to_owned_storage()`, where it walked `clone()`. For a caller that lends `Inline`, `Heap` or `Linear` nothing it
  does changes; a caller that lends a map whose host lends its slots gets an algorithm that compiles and keeps its
  own maps in storage of their declared bounds, as an inline caller's does. The conformance harness runs every
  algorithm fixture that takes a map on `Inline`, `Lent` and `LentIndexed` maps.

### D4. Lanes

Rust lands with this record. C11 follows (Order of work, step 3). The heap lanes (C++, Kotlin, Go, Python, Swift, the
Interpreter) grow a map's storage and have no host that lends a run; nothing changes there, and a `MapSlot` is a
host API of the two lanes that hold a map in bounded storage.

## Not in scope

- A run that grows, or a lent map lent another run: a new run is a new map.
- A document reading a slot. ADR 0108 D2 stands for documents: no handle, slot or position leaves a map into a
  document's value.
- A lent map restored from a saved form in place, beyond the clear and the puts D3 names.

## Consequences

- A session lends each table exactly the rows its configuration gives, from memory it already holds, allocates
  nothing after it opens, and a table configured empty costs the map's few words and no slot.
- A host keeps an array of its own beside each table, indexed by `MapSlot::index`, and a stale number reads nothing.
- A map in `Inline`, `Heap` or `Linear` storage is as large and does what it did (D2); a lent map's slot holds a
  generation, and its clear walks the slots it took.
- Code generic over `S: MapStorage` that clones a map now names `to_owned_storage()`, or bounds `S: MapStorage<Owned
  = S>`; `widened_map` and the saved-state readers take that bound. The generated Rust of an algorithm that holds a
  map changes as D3 says; no committed generated tree holds one.

## Alternatives rejected

1. **`MapStorage<K, V>`.** It states `K: 'a` with no `unsafe`, at the price of the keys and values in every bound of
   every crate and every generated signature, which the three storages that hold their own slots never needed (D1).
2. **`Inline` with a run-time limit.** It reserves `N` in every session whatever the limit, which is what the
   consumer cannot afford.
3. **`Heap` with a capacity.** It needs an allocator, which the consumer's ABI has none of after a session opens.
4. **A host allocator behind the map.** Rust's allocator trait is not stable, and "nothing is allocated after open"
   is a property of memory lent once, not of an allocator's discipline.
5. **The cursor's slot number alone as the handle.** It is reused once its slot is freed (ADR 0111, alternative 7).
6. **A generation in every storage.** It names the slot of a map of any storage, and costs every slot four bytes and
   every position four, eight where alignment rounds them (a `uint64` key and value with an index, 40 bytes to 48),
   on maps whose hosts never read a slot. The slot handle is an extension point added where its use is; the second
   use adds it to its storage (D2).
7. **`Clone` of a lent map into a map lent nothing, or a panic.** The first loses every entry silently; the second
   fails at run time what the type can refuse.

## Order of work

1. This record.
2. (Lands beside this record.) Rust: `Lent`, `LentIndexed`, `LentSlot`, `LentIndexedSlot`, `LentCell`,
   `LendRefused`, `BoundedMap::over`, `limit`; `MapStorage::Generation`, `MapSlot`, `put_slot`, `slot_of`,
   `at_slot`, `MapCursor::slot` and the clear that frees, in the lent storages; `MapStorage::Owned`,
   `to_owned_storage` and `Clone` through it, a clone of the other storages the copy it was; the
   saved-state readers bounded to owned storage; the generator's walk copy and working locals in owned storage; the
   conformance harness on the three storages. The map module's tests hold each lent storage to an inline map's
   answers over a long sequence of operations, the limit, a map lent nothing, the refusals, a slot kept until its
   remove, a stale number, a run lent again, the drops, the invariants of the region, and the size of a slot and a
   position in the storages that name no slot.
3. C11, in `backends/c/forge-runtime/include/sce/forge/map.h`: the same contract.
   - `sce_map_core_t` gains the runs a host lends, the slots and the index cells, and the limit; both runs are null
     for a map that holds its own arrays, which the core reaches by its layout's offsets as now, so a map copied by
     `memcpy` stays whole. The accessors (`sce_map_slot_`, `sce_map_index_`, `sce_map_cells_`) read the run where it
     is set; the cells are twice the limit; a put compares the count with the limit.
   - `SCE_MAP_DECLARE` and its kin declare `prefix##_init_over(map, slots, len, c)` and its indexed form with the
     run of cells, refusing `c` past the bound, a run shorter than `c` and cells fewer than `2c` (`false`), and the
     map type that holds only the core, which a session keeps.
   - the slot of a lent map type gains the generation, moved where a slot is freed (the slot of a map that holds its
     own arrays keeps none, as in Rust), and the header gains
     `sce_map_slot_t { uint32_t index; uint32_t generation; }` with `put_slot`, `slot_of` and `at_slot`.
   - The generator: a C11 algorithm already takes each map as `sce_map_core_t *`, the core every map type begins
     with, so its signature does not change, and a lent map is lent to it as any other. A walk's copy and a map
     local are kept in storage the caller holds, of the inline type of their bound (`sce_map_copy_for_walk`), which
     reads the lent run through the accessors; the conformance harness and the `c11-mcu` gate gain a lent map.
4. No other lane (D4).

## Open questions

- Whether C11 offers the storage with no index as well as the indexed one: the consumer's C ABI decides it in
  step 3.
