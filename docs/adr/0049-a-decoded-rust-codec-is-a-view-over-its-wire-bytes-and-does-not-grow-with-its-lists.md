# ADR 0049 — A decoded Rust codec is a view over its wire bytes, and does not grow with its lists

- Status: Proposed (2026-10-21). A draft for the owner: the decision it asks for is in "Decisions to take". Step 0 (a
  hand prototype) is done and its result is below; nothing is built in the generator. (Numbered 0047 when first
  written; renumbered 0049 because another ADR took 0047 first.)
- Date: 2026-10-21
- Scope: `tools/codegen/templates/forge/rust/codec.rs.jinja2` and the Rust half of `sce-build/src/forge/generator.rs`
  (the field, list, embed and variant statements); `backends/rust/forge-runtime/src/codec.rs` (a list view and its
  iterator); SCE_FORGE.md §4.6 (the paragraph on filling a value in place); the golden trees under
  `tests/forge/expected/` and `backends/rust/mesh/src/generated/`. Rust only: the other five backends keep their own
  list forms (growable lists, and a fixed array in C11) and are not touched.
- Related: the commits that added `decode_into` and `try_as_borrowed_into` and the owned `encode`; ADR 0010,
  decision 2 (the wire rules are written once, and every backend writes the same bytes).

## Context

A Rust codec decodes into a struct that borrows the input and holds its lists inline. A `<sce:repeat>` or
`<sce:tlv-chain>` is a `heapless::Vec<Entry<'a>, N>`, where `N` is the declared `sce:max-count`, and a body that
embeds such a list holds it inside itself. Three facts follow from that, and the first two were measured.

**A move copies the declared capacity.** A `heapless::Vec` is a length and an uninitialised array of `N` entries, and
moving it copies the array, not the `len` entries it holds. On a Zenoh request codec built for
`thumbv7em-none-eabihf` at `opt-level = "s"`, the decoded record is about 1.8 KB (a request) and the codec it
dispatches to about 1.4 KB, with two extension lists of up to eight entries each. A `decode` that returned its value
made 15 copy calls of up to 1439 bytes, about 7.2 KB in all, to return one request; the owned-to-borrowed projection made
about 3.9 KB more for one response. The hops were the struct literal, `Ok(..)`, the variant wrapper and the caller's
own binding.

**Filling in place removed the moves.** `decode_into` and `try_as_borrowed_into` build each list, embedded body and
variant arm in the destination, so the same request decodes with 2 copy calls of 22 bytes, and the projection with none
beyond one entry at a time. A downstream MCU consumer adopted both and counted, per query, 48 copy calls and
11.76 KB before, 29 calls and 2.5 KB after. Its application window fell from 78.3 to 73.5 µs and its wire
turnaround, 12 boots each, from 0.244 to 0.242 ms, which does not resolve. That is about 1.7 µs per KB removed. The gap to the C reference implementation on the same board
is tens of microseconds (it differs between instrumented and plain images, and the consumer's own figures should be
read for it), so what remains of the copies, about 2.5 KB or 4 µs at that rate, is not where most of the gap is.

**The record is still as large as its lists can be.** `decode_into` does not shrink what it fills. The destination is
1.8 KB of stack or static storage whatever the wire holds, and it grows with every `sce:max-count`: a list declared at
sixteen entries instead of eight doubles that list's share of it, and the cost of a move with it. On a target with kilobytes of RAM
that is a footprint, not a speed, and the byte-exact `.text` gate a consumer may hold on its image sees generated code
size too.

What exists now is a mitigation at the API: two entry points per operation (`decode` and `decode_into`,
`try_as_borrowed` and `try_as_borrowed_into`), the value-returning one kept as a wrapper that still pays one move of
the whole record, and a caller who has to know to call the other. It removed the copies and left their cause. The send
side has since gone further: every owned mirror encodes straight from its own fields (see "Considered alternatives").

## Decisions to take

The draft proposes the following; each is the owner's to accept, change or refuse.

### 1. A decoded value is a view, and a view is small

Each borrowed codec `X` gains a generated `XView<'a>`, additive: `X`, `decode`, `decode_into` and the owned mirror
stay as they are. A view holds what a decode learned, not a copy of what the wire repeated:

- a scalar, a string or a byte field: as today (decoded eagerly; a string is validated as UTF-8 at decode);
- an embedded body: its own `YView<'a>`;
- a variant: an enum whose arms are views, so its size is the largest view and not the largest record;
- a `<sce:repeat>` or `<sce:tlv-chain>`: a `ListView<'a, EView<'a>>` of the byte span the entries occupy, their count,
  and the few values a re-walk needs (what an `entry-flag-bind` carries from one entry to the next).

A view is `Copy`, and its size depends on the number of fields and not on any `sce:max-count`.

### 2. Decode refuses exactly what it refused before

A view is built by one pass over the wire that checks everything the eager decode checked: the bound `N` on a list
(`TooManyElements`), the termination of a chain (`TlvChainOverflow`, `NeedMoreBytes`), every enum carrier, every
string. After it the list can be walked without failure. The set of frames that decode, and the error each other
frame gets, do not change, and the conformance vectors that hold that for the eager decode hold it for the view
(see "Order of work"). Chain-membership predicates (`<chain>.has(..)`) are evaluated during that pass, as now.

### 3. A list is read by walking it

`ListView` offers `len()`, `iter()` and `get(i)`. `iter()` decodes each entry as it is reached, which is the work
`decode` did once, done again per walk; `get(i)` walks from the start. A consumer that reads a list twice pays
twice, and one that wants random access collects the entries first. This is the price of a record that does not hold
them, and it is stated in the API, not hidden behind an indexing operator.

### 4. Writing stays on the builder form

`X<'a>` remains the type a consumer fills (`X { .., ..X::new() }`) and encodes, and `encode` lives only there. A view
converts into it in place, `XView::into_builder(&self, &mut X<'a>)`, and `XView::encode` writes the canonical wire from
the decoded fields by walking them; it does not copy the input span, so a frame the decoder accepted in a
non-canonical form is not passed on as it came. The owned mirror projects from a view
(`try_into_owned_in`) as it does from `X`, and `try_as_borrowed_into` is unchanged.

### 5. A view is compared by what it says

`Debug` and `PartialEq` on a view go through the fields and the walked entries, not through the span, so two frames
that decode to the same value are equal. `Clone` is `Copy`.

## Order of work

Each step is gated on a number, taken the way the existing ones were: the generated crate built for
`thumbv7em-none-eabihf` at `opt-level = "s"`, `size_of` of each type, and copy calls counted per function.

0. **A prototype by hand**, on the request and query codecs, outside the generator. Gate: `size_of::<RequestView>()`
   is a small multiple of the field count and does not move when a `max-count` is doubled; one decode makes no copy of
   a list; the cost of walking a list twice is measured and written down. If the gate fails, the draft stops here.
1. **The generator**, Rust only, one commit for the templates and one for the goldens. Gate: for every codec
   fixture, the entries a view yields equal the entries `decode` returned, and a frame `decode` refuses a view refuses
   with the same error.
2. **An in-tree consumer**: the Rust mesh host reads its envelopes through views. Gate: its tests unchanged, and
   no list copied on its decode path.
3. **The downstream consumer** takes the view and measures. Gate: its footprint check passes without a rebase of its
   baseline, or the rebase is named and accepted.

## Step 0 result (2026-10-21)

A prototype was written by hand, outside the generator, over the generated Zenoh request, query, put and delete
codecs: a `ListView` for the two kinds of list (a chain that ends on a flag, with the per-entry state one of them
carries, and a list the wire counts), and views for the four codecs, including the field that is gated on the chain
holding an entry of one id (answered during the scan, as decision 2 says). Built for `thumbv7em-none-eabihf` at
`opt-level = "s"`; times are on the development host (x86-64), for comparison between rows and not for the target.

| Gate | Result |
|---|---|
| A view is small | `Request` 1792 bytes, `RequestView` 184; `Query` 1456, `QueryView` 40 |
| It does not move with a bound | `max-depth` 8 to 16 in the codec documents: `Request` 3456 and `Query` 2864 bytes, the views 184 and 40, unchanged |
| A decode copies no list | the view's decode makes 9 copy calls of at most 71 bytes (the arm views moved into the enum); the eager `decode_into` makes 2 of 22 into a 1792-byte destination |
| It decides as the eager decode does | a valid frame at six list lengths reads equal; every prefix of two frames is refused with the same error; 300,000 randomly damaged frames split 190,585 decoded alike (entries compared one by one) and 109,415 refused with the same error |
| What a walk costs | request with 3 and 2 entries: eager `decode` 138 ns, eager `decode_into` into a reused destination 96, view built 75, view built and every list walked once 165, walked twice 235 |

The first four gates pass. The last one is the finding: **a view is cheaper to build and dearer to read.** It is
faster than `decode_into` only while its lists are not read (75 against 96 ns), and a consumer that reads every entry
once is slower with it (165 against 96 ns), because the walk decodes the entries the eager decode had decoded already.
So the case for the view is footprint and independence from `sce:max-count`, not time, and decision 1 of "Decisions to
take" is the whole question: if a consumer reads most of what it decodes and its RAM is not the limit, the built
mitigation is the better design. The prototype used small entries (the default arms); entries with a heavier body
would move the walk rows, and the generator work should not start on these numbers alone.

## What this does not do

- **It does not touch the send path.** An owned value no longer needs the borrowed record to be sent: every owned
  mirror encodes straight from its own fields, and a view would not change that. A view helps a receive and a relay
  (decode, then encode what was read).
- **It is not a latency fix.** What is left to remove is about 4 µs on the measured board, against a gap of tens of
  microseconds. If time on the wire is the only aim, this is not worth its size; the levers are elsewhere in the
  consumer's application window.
- **It does not make the lists free.** A walk costs a decode of each entry; a payload that is read several times costs
  several.

## Costs and risks

- **Generated code grows.** Two types per codec, a list view and an iterator per list. On a target with a byte-exact
  `.text` gate this moves the baseline, and the amount is not known: step 0 measures it before the generator work.
- **The other five backends keep their own list forms.** Byte parity is untouched, because the wire rules do not move; the
  Rust API grows a form the others do not have, which is a difference between backends that SCE_FORGE.md has to say.
- **Two decode forms to maintain** until the old one is retired, which this draft does not propose.

## Considered alternatives

- *Keep what is built* (`decode_into`, `try_as_borrowed_into`, the owned `encode`). The cheapest, and it has already
  removed the moves. It leaves the footprint and the cost growing with `sce:max-count`. If footprint is not a goal,
  this is the answer.
- *Lower `sce:max-count` in the codec documents.* The cost is proportional to it, so this is a direct fix, and a
  consumer can make it today. It is a protocol decision (how many extensions a frame may carry), not a generator one.
- *A caller-supplied entry store*: `decode_into(cursor, &mut out, &mut [Entry; N])`, with the record holding slices.
  It keeps eager decode and random access and shrinks the record to pointers, but the same bytes of store exist, now in
  a second place the caller declares; the footprint is the same.
- *An owned `encode`.* First rejected on the expectation that it would double roughly fifteen encode emit sites, and
  built later at a consumer's request in a form that does not: the encode body is one skeleton included by the borrowed
  type and by its owned mirror, and only the few reads that go through a storage container differ. It writes the bytes
  the projection followed by `encode` writes, with no record built; the receive path is not touched by it.
- *Boxing or an arena for the lists.* The heap-free tier forbids the first, and the second is the caller-supplied store
  above under another name.

## Decisions to take (summary)

1. Is footprint (RAM and generated size) a goal of the Rust codecs, or only time? If only time, stop at the built
   mitigation.
2. If footprint is: accept step 0 as the gate, and the names `XView` and `into_builder`.
3. Is a Rust-only API form acceptable beside five backends that keep their own list forms?
4. Is the value-returning `decode` ever retired, or kept for good beside the view?
