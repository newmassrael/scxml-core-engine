# ADR 0155 — A cold codec is a table of its fields, read and written by one walker, and a cold chain's rule is data

- Status: Proposed (2026-11-28), under the owner's standing rule: the most long-term correct, textbook structure, cost
  ignored, decided by measurement. The figures in "Context" are the consumer's reading of its size ledger of
  2026-11-28; the effect is measured by the consumer at each landing (section "Measured").
- Amends: ADR 0137 (section 1, a body is read by one function placed by its sites): in a set whose build names its hot
  roots, a body outside the hot closure is read by the runtime's one table walker over the codec's table, and the
  function its sites call is one call of the walker. ADR 0136 (a chain's rule is a type): a chain of a cold codec whose
  kinds no gate reads is read by one routine over a table of its rule; a hot codec's chain keeps its typed rule. ADR
  0140 is unchanged: the hot closure and its forms held in place are what they are, and no hot body calls the walker.
- Related: ADR 0135 (a document of the static class is run by one walker that reads its tables), ADR 0138 (one
  function out of line where a generic copy bought nothing), ADR 0141 (one push for every entry type), ADR 0058 (a
  value is written through its source), ADR 0077 section 7 (storage profiles), ADR 0128 (every read total, one
  verdict).
- Requested by: a downstream bare-metal consumer, 2026-11-28.

## Context

The consumer's like-for-like library holds every message kind of its protocol, as the C implementation it is
compared with does. Its codec area is 77,953 bytes against that implementation's 36,707 (2.12). Read body by body
from the line tables of the same build, 24,323 of its bytes are lines of this runtime inlined into generated and
consumer bodies: the frame reader's `take_array::<1>` in 25 bodies, `read_vle_u64` called from 30, the extension
walk's `next` in 13, `SliceSink::put::<1>` in 10, `Bounded::next` in 11, `take_slice` in 15, `settle` in 12, the
storage's `from_fitting::<16>` and `from_slice::<16>` in 8 each. They are the reads and writes of one field each,
printed again at every field of every codec the build links. The C implementation reads a ZInt by one function and
an extension chain by one iterator with a callback, for every message.

ADRs 0136 to 0141 took the loops out of the fields: one chain routine per rule, one VLE encoder, one ZInt reader,
one fixed-list push. What is left is each field's own straight-line code. It is the right shape for a codec read per
message, which ADRs 0139 and 0140 let a build name and hold in place, and the wrong one for a codec read once per
entity, session or link event: a declaration, an interest, a handshake, a join, a close, a scout. Those are most of
the kinds of a full library, and each costs its own printed body.

Two more shapes follow from the same printing:

- **Documents of one layout compile apart.** In the consumer's set three declaration documents differ in their names
  only, and four undeclaration documents likewise; each is printed as its own body.
- **Each chain is a type.** Every chain of every codec instantiates `read_chain_into` at its rule, and the entry read
  is held in each instance; the consumer's extension area is 5,742 bytes against the C implementation's 1,459.

## Decision

### 1. The cold set is the complement of the hot closure

A build that names its hot roots (`generate --with-imports --inline-decode-for`, `InlineDecode::Codecs`) says
which codecs its per-message path reads (ADR 0140 section 1). Every codec of the set outside that closure is cold:
read and written once per event its consumer handles, where a few instructions a field are nothing and a body per
codec is all of its cost. A cold codec whose fields are all of kinds a table states (section 2) is generated as a
table; any other cold codec, and every hot codec, is printed as today.

With no hot root named (`InlineDecode::None`) or every read asked inlined (`InlineDecode::Every`) the build has not
said which reads are cold, and nothing is generated as a table: every golden of every backend is unchanged. The
other backends are unchanged in every build; this ADR is the Rust backend's.

### 2. A table is a row per field, and the value is reached by its rows' indexes

A codec's table is a `static` `FieldTable` of rows in wire order. A row says:

- **its read**: an unsigned integer of 1 to 8 bytes in a byte order (a flags carrier, a fixed field); a ZInt held to
  16, 32 or 64 bits; a byte string or a text whose count is an earlier row's value or a ZInt prefix, with the
  length's arithmetic and the field's bound; the bytes left in the frame; an embedded body, a variant arm, a repeat
  or a chain, which the value reads itself (below), handed the inputs a flag-bind gives it as bits of earlier rows;
- **its gate**: always, or a bit range of an earlier row set or clear (`sce:present-if` on a flag of a carrier);
- **its place in the value**: the row's index, which the value's store answers.

The walker keeps the number each row read in a frame of its own (`[u64; TABLE_ROWS]`), so a gate, a length, a count,
a tag and an input are bit ranges of rows already read. A field whose kind, gate or length no row states (a predicate
over a member of a body, a chain's kind, a length past an embedded body, DMA alignment) keeps the codec printed; the
generator answers which codecs are tables and why the others are not, as the printers of ADR 0081 answer the forms
they lower, and a generator test lists them for the conformance set.

The value is reached through `TableOut`, a trait the generated code implements once for each form of the codec
(`{Codec}In<S>` at its profile and `{Codec}View<'a>`): one method per kind of store (a number, bytes, a text, a
body, a list, an absent field), each a `match` on the row index whose arms are the stores of the fields. The walker
calls it through `&mut dyn TableOut<'f>`:

    pub fn read_table<'f>(reader: &mut SceReader<'f>, table: &FieldTable, inputs: &[u64],
                          out: &mut dyn TableOut<'f>);

An offset into a `repr(C)` value, the form the C table-driven parsers use, is not open to this backend: the owned
value is generic over its storage profile (ADR 0077 section 7), whose containers decide the layout of each field and
hold invariants (a list's length, a text's validity) that a write at an offset would bypass. The store by index is
the same split in safe Rust: the reads, the gates, the bounds, the order and the refusals are the walker's, once for
every table; the stores are the value's, a jump table of moves. No public type changes, so the public face of a set
is the same text with and without tables (ADR 0140, first amendment).

`read_into` of a cold codec is one call of `read_table` over its table and its value. The rules of ADR 0128 hold:
every read is total, the first refusal recorded in the reader is the frame's verdict, and no read returns early. The
function is placed by its sites as ADR 0137 places every body read: with no attribute where the set reads it from one
site, out of line where it reads it from two or more. The call is short, but it is still the table's address, the
inputs and the value's stores handed to the walker, which two sites would each hold a copy of; one rule for every
entry point keeps a codec's place in the set the same whether it is a table or printed.

### 3. One writer for every table

The runtime gains the writer of a table, generic over the sink's error type alone, of which a program has at most two
(`CodecError` and `Infallible`):

    pub fn write_table<E>(sink: &mut dyn SceSink<Error = E>, table: &FieldTable,
                          src: &dyn TableSource<E>) -> Result<(), E>;

`TableSource` is the read side of `TableOut`: the value's numbers, bytes and texts by row, whether a gated field is
present, and the write of each body, arm, repeat and chain the value holds. A row the encode derives (ADR 0058: a
length, a count, a flag a gated field is announced by, the arm flag of an embedded variant, a flag-bind input) is a
row of the table whose value the writer computes from the rows it names, as the encode plan says it
(`encode_plan::Derive`). A cold codec whose plan derives a value no row states keeps its write printed.
`{Codec}Wire::write_source` of a cold codec is one call of `write_table` over an adapter of the source, so every
form of the value writes through it as before.

### 4. Identical layouts are one table

Two codecs of a set whose tables are the same rows (the same reads, gates, bounds and inputs, row for row) name one
`static`, written in the module of the first of them in the set's order; the others name it by its path in the set
(`super::{module}::{TABLE}`). Their types stay theirs and each keeps its own stores. With section 2 the cost of a kind
of declaration is its stores and, where its layout is another's, no table at all.

### 5. A cold chain's rule is data

A chain of a cold codec whose kinds no gate reads is a `ChainTable`: how it ends (with the frame, or at an entry
whose flag is clear), its depth, the bit range of the entry's identifier carrier the flag is, and the bit ranges the
chain carries from one entry to the next (`<sce:entry-flag-bind>`). It is read by one routine for every such chain:

    pub fn read_chain_table<'f>(reader: &mut SceReader<'f>, chain: &ChainTable, args: &mut [u8],
                                entries: &mut dyn ChainEntries<'f>) -> Result<(), CodecError>;

`ChainEntries` reads one entry from the reader with the inputs the chain hands it, holds it as `push_keeping` holds
an entry no gate asks a kind of, and answers the entry's identifier carrier, the number the table reads. The runtime
implements it once for every list and entry read (`HeldEntries`), so the one entry read of a chain's entry codec is
its entry point, out of line, whatever chain reads it. A chain whose kinds a gate reads keeps its typed rule (ADR
0136): the keep rule past a full list tests each held entry's kinds by name, which a table does not state.

A refusal of an extension the consumer does not know and its protocol marks mandatory is not added here: no document
states it, and a view's chain hook says it (ADR 0130, `ChainEntryRefused`).

### 6. A hot read never calls the walker

The forms held in place and the entry points of a hot codec are printed as ADR 0140 prints them. A hot body that
reads a cold codec (an arm the hot path does not take) calls its entry point, which is the walker's call; no hot body
holds the walker or a table read. A generator test of a set with hot roots named asserts it.

## Consequences

- No golden moves: no committed set names hot roots. A set that names them generates its cold codecs as tables and
  keeps its public types, its encodes' bytes and its decodes' verdicts; its codec vectors and round trips hold.
- The runtime gains the `table` module: the table types, `read_table`, `write_table`, `read_chain_table` and
  `HeldEntries`, each one function for every table, out of line, with unit tests over tables written by hand.
- A cold read costs the walker's dispatch, a few instructions a field, once per event; a hot read costs what it costs.
- The stores of a cold codec are still printed once per form and profile, as its types are.
- The table and the stores are printed by the codec template from a structured context (rows, gates, derives, store
  arms), as every template-printed form is. The adapter the writer reads a source through (`{Codec}Rows`) is printed by
  the source interface's printer (`rust_source`, docs/adr/0081), which writes Rust text by `format!` as the rest of
  that printer does. That is a debt: it migrates to a template with the rest of the printer, not in this change.

## Alternatives considered

- **Offsets into a `repr(C)` value.** The table-driven parsers of the protobuf family and of the embedded C
  libraries do it. Refused here: the owned value's layout is its storage profile's, and a write at an offset into a
  container bypasses its invariants. The store by index is the same split without `unsafe`.
- **A table walker generic over the value.** Each table would instantiate the walker, the copy per codec this ADR
  removes. Refused: the walker is generic over nothing, the stores are dynamic calls.
- **Tables for every codec, hot ones too.** ADRs 0138 and 0140 measured that a call per field on the per-message path
  shows in its instructions. Refused: a hot body stays printed.
- **A cold chain's rule as a type, as now.** Each chain instantiates the routine and holds its entry read. Refused for
  the chains whose rule is data; kept where a gate reads kinds.
- **Merge documents of one layout into one document.** That is the consumer's decision about its documents (one
  document of a declared entity whose kind is the header's identifier), and it may make it; the generator's part is
  that one layout is one table whatever the documents.

## Measured

To be measured by the consumer at each landing, on the same images as the "Context" figures, with its hot roots
named. Expected direction: the codec area of the like-for-like library down by most of the cold kinds' bodies and
their share of the inlined runtime, the receive window of a data message unchanged (its codecs are hot), and a
declaration's operation in instructions up by the walker's dispatch.

## Order of the changes

1. This ADR.
2. The runtime: the `table` module's read side (`FieldTable`, `TableOut`, `read_table`), the chain's data rule
   (`ChainTable`, `ChainEntries`, `HeldEntries`, `read_chain_table`), and their tests.
3. The runtime: the write side (`TableSource`, `write_table`) and its tests.
4. `sce-build`: the table of a cold codec, its stores and its `read_into` as the walker's call; the list of codecs
   that are tables and why the others are not; the test that no hot body calls the walker.
5. `sce-build`: the write of a cold codec as the walker's call.
6. `sce-build`: identical tables shared in a set.
7. The consumer generates its set with its hot roots named and measures its images and its windows.

## Acceptance

Accepted on all of these: the Rust goldens are unchanged; a set with hot roots named compiles with and without views;
its codec vectors and its consumer's round trips hold; its public declarations are the same text as with no tables;
no hot body calls `read_table`, `write_table` or `read_chain_table`; the consumer measures its images and windows.
