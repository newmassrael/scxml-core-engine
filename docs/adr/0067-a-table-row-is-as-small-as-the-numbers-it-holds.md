# ADR 0067 — A table row is as small as the numbers it holds

- Status: Accepted (2026-10-30), on the owner's instruction to optimise the footprint structurally
  until no optimisation point is left.
- Amends: ADR 0064 (decision 3, the compact tables).
- Evidence: the watching-zenoh documents measured after ADR 0066 (below), and
  `backends/rust/probes/nostd-footprint`.

## Context

ADR 0064 moved what a machine brings from code to `.rodata`: its document's tables. After ADR
0066, the consumer's two documents are measured against 49e40c9a72 (`thumbv7em-none-eabihf`,
`opt-level = "s"`, the same generator flags as the consumer):

| | `.text` | `.rodata` |
|---|---|---|
| 49e40c9a72 | 185,040 | 7,140 |
| after ADR 0066 | 185,272 | 7,997 |

The code is within 232 bytes. The `.rodata` grew by 857 bytes, and that is the tables:

- 16 + 10 states at 8 bytes;
- 46 + 19 transitions at 8 bytes;
- a pool holding every target of every transition, at 2 bytes each.

The rows were sized by the types that hold their fields, not by the numbers they hold:

- **State row:** three `u16` numbers and a one-byte kind take 7 bytes; alignment pads them to 8.
- **Transition row:** a `u16` source, a four-byte `Span` into the pool and two `bool`s.
- **Target lists:** almost every transition has exactly one target, so a `Span` and a pool slot
  spend 6 bytes to say what one `u16` says.

## Decision

### 1. A document names at most 16,383 states, histories and transitions

Each id is 14 bits, and the largest 14-bit number (0x3FFF) stands for "absent". That frees the
top two bits of every id field for what the row would otherwise spend a byte on. The limit was
32,767. A document that reaches 16,384 of any of them is refused by the generator with the
limit named, as before.

### 2. The rows hold their numbers and nothing else

- **`StateRow`, 6 bytes:**
  - the parent;
  - the last descendant;
  - the initial transition, with the kind in its top two bits. The parent is read on every
    walk up the tree, and the initial transition only when a compound state is entered by
    default, so the field read most needs no mask.
- **`TransitionRow`, 4 bytes:**
  - whether it is internal and whether it has content in the top two bits of its source;
  - one `Target`, which is either the transition's only target, the marker of a targetless
    transition, or the position of a list in the pool.
- **`HistoryRow`, 4 bytes:**
  - whether it is deep in the top bit of its parent;
  - its default transition.

### 3. The pool holds only lists of more than one target

A list in the pool is its length followed by its targets. `DocumentTables::targets` answers with
a slice in every case:
- of the row's own `Target` for a single target;
- empty for none;
- of the pool after the length for a list.

So the procedures read targets as they did, with no copy.

### 4. The generator's model stays logical; the Rust emission packs it

`sce_build::document_tables` keeps one target list per transition: it says what the document
has, for any backend. The Rust emission:
- packs it into the rows above;
- writes the pool of lists;
- holds the document to the limit of decision 1.

The runtime's `check` refuses:
- an id at or past the limit;
- a list that runs past the pool.

### 5. Decoding is written once

`DocumentTables::targets` is one out-of-line function. The procedures ask it in a handful of
places, and they are compiled once for every program. Decoding inlined at each place made the
probe's core 310 bytes larger and ate what the rows saved. One copy and a call cost 104 bytes.

## Measured (2026-10-30)

The consumer's two documents, as in the Context:

| | `.text` | `.rodata` | against 49e40c9a72 |
|---|---|---|---|
| after ADR 0066 | 185,272 | 7,997 | +1,089 |
| after this ADR | 185,218 | 7,715 | +753 |

The probe (`footprint.budget`):
- two machines without either feature: 192,184 → 192,040;
- their core: 4,528 → 4,474;
- their worst-case stack: 1,152 → 1,128.

## Consequences

- The consumer's two documents are 336 bytes smaller, and still 753 bytes above 49e40c9a72. The
  rest is each machine's own engine, which is the next decision.
- A document of 16,384 or more states, histories or transitions is refused. That is half the
  previous limit. The largest committed Rust machine (2026-10-30) has 29 states and 36
  transitions.
- Hand-written tables in the tests name a single target as the `Target` itself, and a list as
  `Target::list` into a pool whose list starts with `Target::count`.
