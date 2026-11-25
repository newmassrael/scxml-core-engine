# ADR 0136 — A chain is read by one routine for every chain of its entry, and a body by the one function of its codec

- Status: Accepted (2026-11-26), under the owner's standing rule: the most long-term correct, textbook structure, cost
  ignored. Its figures are the consumer's measurement at the landing (section "Measured").
- Amends: ADR 0128 (a frame's reads are total and its verdict is read once): a chain's loop is the runtime's, not each
  codec's. ADR 0130 (a chain's entries are shown as they are read): a view's scan of a chain is the runtime's, and a
  walk reads an entry with a hook. ADR 0049 (a decoded Rust codec is a view over its wire bytes): `ListWalk` reads an
  entry through `read_with`.
- Related: ADR 0061 (`generate --inline-encode`), ADR 0126 (the head of a variant is written once, out of line, and
  inlined under `--inline-encode`), ADR 0077 §7 (a value past a capacity), ADR 0065 §6 R6, R7 (a chain's kinds and an
  entry-flag chain).
- Requested by: a downstream bare-metal consumer, 2026-11-26.

## Context

The consumer builds its no_std image with fat LTO and measured its codec area past the size it has to beat. Two
shapes of the generated Rust decode account for the excess.

**G1: a chain's loop is printed into every codec that holds the chain.** A `<sce:tlv-chain>` is read by a loop: read
an entry, read off it the flag that ends the chain and the identifier the next entry's input waits for, note the kinds
of entry the gates ask for, hold the entry where the list has room, then judge how the chain ended. The printer wrote
that loop into the decode of each codec that holds a chain. A protocol whose every message carries an extension chain
of one entry type therefore carries one copy of the loop for each message, each with the entry's read
(`ExtEntryOwned::read`) and the list's `push_keeping` inlined into it. A view's scan of the chain (ADR 0130) is the
same loop with a count and a hook, and is copied the same way.

**G2: a dispatching variant inlines every arm's body read.** A codec that dispatches on a tag reads the arm it finds
through the arm codec's `read_into`. Each arm codec of a message envelope has that one caller, so the compiler inlines
every arm's body into the dispatch: the consumer's `Declare` decode holds nine bodies in one function. A body embedded
by several codecs (a key expression held by every declaration that names one) is inlined into each of them, and so is
read by as many copies.

Both are copies of one piece of code made because of where it is written, not because the reads differ: ADR 0126 made
the same observation of the write side and wrote the head once, out of line.

## Decision

### 1. An eager decode reads a chain by one routine of the runtime

The forge runtime gains `backends/rust/forge-runtime/src/chain.rs`, re-exported from `codec`:

    #[inline(never)]
    pub fn read_chain_into<'f, E, L, const K: usize>(
        reader: &mut SceReader<'f>,
        list: &mut L,
        chain: ChainRead<'_, 'f, E, K>,
        found: &mut [bool],
    ) -> Result<(), CodecError>
    where
        L: HeldList<Entry = E>;

It is generic over the entry type `E`, the list `L` and the count `K` of inputs an entry is read with, and over
nothing else, so every chain of one entry type held in one kind of list is read by one copy of it, out of line. What
differs from one chain to another is data the call passes in `ChainRead`:

- `read`, the entry's read handed its inputs (`fn(&mut SceReader<'f>, &[u8; K]) -> E`), and `args`, the inputs the
  first entry is read with: what the parent hands every entry, and 0 for each value the chain carries;
- `depth`, the chain's `max-depth` (`None` where the protocol states none);
- `end`, where the chain ends: `ChainEnd::Frame`, with its frame or at its depth, or `ChainEnd::Flag(flag)`, after the
  first entry whose flag is clear;
- `carry`, which rewrites what the chain carries from the entry just read (`<sce:entry-flag-bind>`);
- `kinds`, the kinds of entry the gates on the chain ask for (ADR 0065 §6 R6). `found[i]` is set where an entry read,
  held or not, is of `kinds[i]`, which is how a gate after the chain reads the kinds the frame held.

The routine is the loop the printer wrote, statement for statement: the loop is left where the frame has failed, an
entry is read through the frame's reader (ADR 0128), held through `push_keeping` (ADR 0077 §7), and the guards after
the loop are `tlv_chain_guard`'s: an entry past the depth of a chain that owns the rest of its frame, and a chain
closed by an entry flag cut short or run past its depth. It records every refusal in the reader and returns only the
list's capacity (`CapacityExceeded`), which the list's `refill` and the reader's `hold_with` take as they took the
printed fill's: a read body still has no `?` and no `return Err`. The printer writes the call inside the list's
`refill`, as the fill:

    cursor.hold_with(|cursor| _out.extensions.refill(|_vec| {
        ::sce_forge_runtime::codec::read_chain_into::<ExtEntryOwned<S>, _, 0>(cursor, _vec, ChainRead { .. }, &mut [])
    }));

The closures in `ChainRead` capture nothing, so each is a function pointer; a parent's input to every entry is an
element of `args`, evaluated at the call. A group whose memberships are its chain's kinds (ADR 0065 §6 R6a) reads the
chain through the same fill, and keeps the kinds the fill noted in the one array `found` it declares.

### 2. A view scans a chain by one routine of the runtime

    #[inline(never)]
    pub fn walk_chain<'f, W, H, const K: usize>(
        reader: &mut SceReader<'f>,
        hook: &mut H,
        scan: ChainScan<W::Entry<'f>, K>,
    ) -> u32
    where
        W: ListWalk<K>,
        H: ChainHook;

`W` is the list's walk (ADR 0049): how an entry is read and what it carries from one entry to the next. A chain that
carries nothing reads by the walk of its entry's module, so every such chain of one entry type is scanned by one copy
for each hook. `ChainScan` holds the site the hook is told (ADR 0130), the inputs the first entry is read with, the
depth and the end, as `ChainRead` does. The scan answers the count of entries the walk of the list will yield.

The rules of the printed scan hold in the routine: each entry is shown to the hook right after it is read and before
the next is, the entry's own body is read with the same hook, what the hook refuses is the frame's refusal, and an
entry is counted only where its read was whole (`count += u32::from(!reader.failed())`), since the walk stops at an
entry whose read was refused. The guards after the loop are the eager routine's, one function for both.

To read an entry with a hook, `ListWalk` takes the hook: its required method is `read_with<'b, H: ChainHook>(reader,
args, hook)`, and `read`, which a walk of a list (`ListIter`) and `NonEmptyListView::from_scan` read with, is
`read_with` with `()`. A walk of a list still shows no entry to a consumer's hook: the scan did.

### 3. A codec's body is read by its own function, out of line

The function that reads a binary codec's body, the one every parent reads it through, is `#[inline(never)]`: the
eager `read_into` of a codec that reads into its destination (`read` of one built as a value), and the view's
`read_with`. A dispatching variant then calls one function per arm and holds no arm's body; a body embedded by
several codecs, or held by several chains, is the one function of its codec, called from each, not a copy at each
site. The wrapper around it (`read` of a codec that reads into place, `read_into` of one built as a value, a view's
`read` and `read_into`) keeps no attribute: it is a call and a move, and inlined where it is called.

### 4. `--inline-decode` asks for every body read inlined

`ForgeCompileOptions::inline_decode` (`generate --inline-decode`) is the decode's `inline_encode`: it marks each body
read of section 3 `#[inline(always)]` in place of `#[inline(never)]`, so a frame's decode becomes one function, at the
price of a copy of each body at each site, which a build measures before it keeps. It is a property of one document, as
`--inline-encode` is: an attribute needs nothing of the functions it calls. The routines of sections 1 and 2 stay out
of line under it; they are the loop of a list, not a body, and a build that inlines bodies still shares them.

The option and the default reach the binary codec, whose body is the arm or the embedded body of another codec. A CBOR
map and a content-line component are read by their own runtime readers and are no codec's body; the option leaves
them as they are.

## Consequences

- Every Rust codec golden under `tests/forge/expected/` and `tests/forge/expected/views/` whose codec holds a chain
  prints a call in place of its loop, and every one marks its body read `#[inline(never)]`; each is generated again.
- `ListWalk` is implemented by `read_with`. A generated walk is generated again; a walk written by hand outside the
  tree implements `read_with` and drops `read` (`docs/SCE_CODEC_CONSUMER_MIGRATION.md`).
- No frame is decoded differently: the routines are the printed loops, and the conformance lanes hold every codec's
  bytes and verdicts as before.

## Alternatives considered

- **A trait the entry codec implements to be read as a chain entry** (`impl ChainEntry<'f, K> for ExtEntryOwned<S>`).
  Static dispatch for the entry's read, at the price of an impl in every codec's module for an arity and a tag the
  entry does not know it is read with. The entry's read is out of line by section 3 whatever calls it, so reading it
  through a function pointer costs an indirect call per entry and a thunk of one call at each chain, and no copy of the
  read. Refused for the eager read; the view keeps its walk, a type it has already.
- **Mark the printed loop's closure `#[inline(never)]`.** Each closure is a type of its own, so each codec keeps its
  copy, out of line. Refused.
- **One wrapper per arm in the dispatching codec, `#[inline(never)]`.** The wrapper has one caller and inlines the body
  it calls: a copy per site, the problem G2 names for embedded bodies. Refused.
- **Mark only the bodies a parent dispatches to or embeds.** A codec does not know who reads it, and a document set
  does not reach the generation of one codec. The rule is one for every body read, and `--inline-decode` the one choice
  against it.

## Measured

To be measured at the landing, by the consumer, on its release no_std image with fat LTO: the codec area before and
after, the decode of a message with an extension chain and of `Declare`. The expectation is one copy of the chain
loop for each entry type and list, where there was one per message, and a `Declare` decode of a dispatch and nine calls.

## Order of the changes

1. This ADR.
2. The runtime routines, `ListWalk::read_with`, and the printer of the eager and the view decodes of a chain.
3. The body read's attribute and `--inline-decode`.
4. The main session regenerates the Rust goldens and the committed Rust trees, and runs the Rust golden tests, the
   Rust conformance lane (`forge_source_round_trip`, `forge_view_round_trip`), the runtime's tests and the codec clippy
   check.
5. The consumer regenerates its codecs and measures its image.

## Acceptance

Accepted on all of these: the Rust goldens and conformance lane pass with every frame's bytes and verdict as before;
the runtime's unit tests of the routines pass; `scripts/check_clippy_codec.sh` passes over both golden families; the
consumer measures its codec area at the landing.
