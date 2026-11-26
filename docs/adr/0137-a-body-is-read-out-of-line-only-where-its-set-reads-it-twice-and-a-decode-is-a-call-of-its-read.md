# ADR 0137 — A body is read out of line only where its set reads it twice, and a decode is a call of its read

- Status: Accepted (2026-11-26), under the owner's standing rule: the most long-term correct, textbook structure, cost
  ignored. Its figures are the consumer's measurement of the landing of ADRs 0127 to 0136 (section "Context").
- Amends: ADR 0136 (a body by the one function of its codec): section 3's default, `#[inline(never)]` on every body
  read, becomes an attribute decided by the sites of the generated set that read the body, and the alternative it
  refused, "mark only the bodies a parent dispatches to or embeds", is taken, the set counted where it is generated.
  ADR 0128 (a frame's reads are total and its verdict is read once): `SceCursor::read_frame` answers the value the read
  returns, and its verdict is one function of the runtime; a refusal is recorded by a call. ADR 0130 (a ZInt is read
  by one body): the longer read of a ZInt is one function of the reader, and `codec::zint_tail` one loop.
- Amended by ADR 0138 (2026-11-26): the one-byte read of a ZInt at each field is withdrawn, the reader's longer read
  becomes its one read of every ZInt (section 3), and the reader's start is one out-of-line function of the runtime,
  `SceCursor::begin_frame`, as the verdict is (section 4).
- Amended by ADR 0139 (2026-11-26): a body read of a codec the build names as on its hot path
  (`--inline-decode-for`, `InlineDecode::Codecs`) is `#[inline(always)]` whatever its sites; every other read keeps
  the rule of section 1, and a name that is no codec of the set is refused.
- Related: ADR 0049 (a decoded Rust codec is a view over its wire bytes), ADR 0061 (`generate --inline-encode`),
  ADR 0077 §7 (a value past a capacity).
- Requested by: a downstream bare-metal consumer, 2026-11-26.

## Context

The consumer moved its pin from `b616325c651` to `108be402cbf` (ADRs 0127 to 0136) and measured its release no_std
image, fat LTO, by symbol size. Its transport area shrank by 1,162 bytes; its codec area grew by 5,942. The symbols
that moved, old size to new, in the consumer's names:

| Symbol | Before | After |
|---|---:|---:|
| `DeclareIn::read_into` | 0 (inlined) | 1,267 |
| the consumer's `read_declare` | 1,393 | 319 |
| `InitBodyIn` decode | 509 (`decode_into`) | 849 (`read_into`) |
| `OpenBodyIn` decode | 236 | 542 |
| `OamView` decode | 444 (`decode_into`) | 565 (`read_with`) + 344 (`decode`) |
| `TransportInitView::decode` | 177 | 386 |
| `TransportOpenView::decode` | 137 | 320 |
| `ExtEntryView::decode` | 89 | 267 |
| `EncodingIn`, `InterestIn`, `TimestampIn`, `ZbufSliceIn` `read_into` | inlined or smaller | 503, 503, 452, 442 |
| `DeclTokenIn`, `DeclKexprIn` | 462, 447 | 407, 401 |
| `codec::zint_tail` (was `SceCursor::read_vle_inner`) | 423 | 371 |
| `read_chain_into::<OamExtensionsChain, ..>`, `walk_chain`, `close_chain` | none | 353, 130, 73 |

Comparing the goldens of the two pins (`codec_zenoh_init_body`, `codec_zenoh_open_body`, `codec_zenoh_oam` and its
view, `codec_zenoh_ext_entry` and its view, and the runtime they call) finds six shapes of the generated code and the
runtime behind the growth. The last row is ADR 0136's: one routine for each chain's rule, which the consumer's image
holds once for each chain, as decided.

**S1: a body read from one site is kept out of line.** ADR 0136 marked every body read `#[inline(never)]`, so a body
a frame reads from one site, an arm of one envelope or the body of the one message that embeds it, is a function of
its own with one caller: the call frame, the saving and restoring of registers and the arguments are paid for, and
the caller can no longer fold the body into its own code. `Declare` is read by one arm of the network envelope; its
read stood out of line at 1,267 bytes while the consumer's caller shrank by 1,074, 193 bytes more than the two had
been. The four bodies of the table's middle row stood out of line beside callers that had held them. The copy
`#[inline(never)]` spares does not exist for a body read from one site.

**S2: every read wrote its own refusal.** `SceReader::fail`, which records a refusal, was `#[cold]` and small, so
the compiler inlined it into each read that can refuse: each `take_array`, `take_slice`, `peek_u8`, each `settle`,
and both arms of each `hold`, wrote the test of the recorded refusal, the store of the code and the end of the input
in place. A frame of ten reads held ten copies, which the old decode, whose refusal was one return of the frame's,
did not.

**S3: every ZInt field held the longer read's answer.** `SceReader::read_vle_inner` was `#[inline(always)]` and
called the shared `codec::zint_tail` for a ZInt of more than a byte, then matched the `(Result<u64, CodecError>,
usize)` it answered through memory and recorded its refusal, at each field: `OpenBody`'s three ZInt fields held the
match three times.

**S4: a decode instantiated the whole verdict.** `SceCursor::read_frame` is generic over the read it is handed, a
closure, so each codec's `decode_into` held its own copy of the reader's start, the verdict's match and the move of
the cursor, none of which depends on the codec.

**S5: a view's decode built the view three times.** A view's `decode` made a default view, called `decode_into`,
whose closure read the view into a temporary and copied it over the default, then moved the result into `Ok`: three
writes of the whole view and its default's construction, where the view is the value its read returns
(`ExtEntryView::decode` 89 to 267, `TransportInitView::decode` 177 to 386, `OamView::decode` 344). An eager codec
built as a value decoded the same way.

**S6: `zint_tail` read the chain by two loops.** It took nine bytes as one array where nine remained and read the
chain from it with no test of the end, and read a shorter tail byte by byte; both were the one `decode_zint`,
inlined twice, the first with its length known, so the loop is in the function twice.

Two shapes the brief named were examined and are not causes: the checked arithmetic of a length (`(_len as
usize).checked_add(1)` of a length of four bits) has a range the compiler knows and folds; and the per-field `settle`
and `Option` wrappers inline to the value they hold, their cost being the refusal each wrote (S2).

## Decision

### 1. A body read is out of line where its set reads it from two sites or more

The attribute of the function a parent reads a codec's body through (the eager `read_into` of a codec that reads into
its destination, `read` of one built as a value, and the view's `read_with`) is decided by the sites of the
generated set that call it:

- **none**, where the set reads the body from one site or none: the compiler inlines it into its one caller or keeps
  it apart, as it judges;
- `#[inline(never)]`, where the set reads it from two sites or more, an arm two dispatching codecs share, a body
  several codecs embed, the entry of a list a view both scans and walks: it is one function every site calls, and no
  site holds a copy;
- `#[inline(always)]` under `--inline-decode` (`ForgeCompileOptions::inline_decode`), whatever the sites, as
  ADR 0136 section 4 says.

A site is a call in the code the set generates. The decode a consumer calls is the codec's own wrapper, which the
consumer's build inlines or not as it calls it, and is not counted.

The sites are counted once, from the set's import graph, by one function, `forge::body_reads::BodyReadSites::count`:
each arm of a variant and its default arm is a site of the arm's codec, and so is each embedded body; a list's entry
(`<sce:repeat>`, `<sce:tlv-chain>`) is read at one site of the eager decode (its fill, or the rule of its chain,
ADR 0136 section 1) and at two of a view, the scan that counts the entries and the walk that yields them. The eager
read and the view's are two functions, each with its own count (`BodyReads { eager, view }`); a codec that is its own
view (`{Codec}View` an alias of `{Codec}`) is read by the views of the set through its eager read, so a parent's
view adds to its eager count.

What a set reads is a fact of the set: a document does not know which documents import it. The count reaches each
generation as an option, `ForgeCompileOptions::body_reads`, the way `views` and `codec_capacities` reach it:

- `generate --with-imports` counts the set it walks (`BodyReadSites::of_documents`) and hands the count to the
  generation of each document;
- a build that generates a set one document at a time counts it once with the same function and hands it to each
  call of `compile_forge_with_imports`;
- the Rust goldens are one crate each (`tests/forge/expected/` and `expected/views/`), so each family is counted as
  one set;
- a document generated alone is a set of one, which reads no body of its own: no body read of it carries an
  attribute but the one `--inline-decode` asks for.

### 2. A refusal is recorded by one function of the reader

`SceReader::fail` is `#[cold]` and `#[inline(never)]`: a read that can refuse calls it where the frame cannot give
what it asks, so each read is its value path and a call. `SceReader::hold` records what it does not hold through one
such function too, `refuse_held`, which notes a capacity or records the refusal. The reads stay total and the first
refusal sticks (ADR 0128 section 1).

### 3. The longer read of a ZInt is one function of the reader, and `zint_tail` one loop

A ZInt of one byte is still read where the field is, a test and a branch. A longer one is a call of
`SceReader::read_vle_long`, `#[cold]` and `#[inline(never)]`, which runs the cursor's `codec::zint_tail`, consumes
the ZInt and records its refusal: the cursor and the reader still run one decode (ADR 0130). `zint_tail` reads the
tail by one loop, `decode_zint`, whatever remains of it; the test of the end it spared a tail of nine bytes cost a
second copy of the loop.

### 4. A decode is a call of its read, and the verdict one function of the runtime

`SceCursor::read_frame` answers what the read returns, under the reader's verdict:

    pub fn read_frame<T>(&mut self, read: impl FnOnce(&mut SceReader<'a>) -> T) -> Result<T, CodecError>;

It is generic over the read, so it is inlined at each decode, and holds only the reader's start and the call; the
verdict and the move of the cursor are `SceCursor::end_frame`, `#[inline(never)]` and generic over nothing a decode
reads, one function for every codec of a program. A read into a destination answers `()`, as before.

A view's `decode` is the read of its frame:

    pub fn decode(cursor: &mut SceCursor<'a>) -> Result<Self, CodecError> {
        cursor.read_frame(|cursor| Self::read(cursor))
    }

The view is the value its read builds (ADR 0128 section 4), answered as it is: no default of it is made first and
nothing is copied over one. An eager codec built as a value (`read` its body read) decodes the same way. An eager
codec that reads into its destination keeps its `decode` into a default of itself, which its lists are built in
(ADR 0128 section 5), and every `decode_into` stays `cursor.read_frame(|cursor| ..)`, one statement and one verdict.

## Consequences

- Every Rust codec golden under `tests/forge/expected/` and `tests/forge/expected/views/` changes: a body read of one
  site loses its attribute, a view's and a value's `decode` is one `read_frame`; each is generated again, and so is
  every committed tree of generated Rust codecs.
- No frame decodes differently: the reads, their refusals and their verdicts are those of ADR 0128, and the
  conformance lanes hold every codec's bytes and verdicts as before.
- ADR 0128's invariants hold: every read is total, a frame has one verdict, a read body has no `?` and no
  `return Err`, and no panic path is added. ADR 0136's chains keep their static dispatch.
- `SceCursor::read_frame` returns `Result<T, CodecError>`; a hand-written call whose closure returns `()` compiles
  as before (`docs/SCE_CODEC_CONSUMER_MIGRATION.md`).
- A consumer that generates its set one document at a time, or with `generate` per document, gets no shared body
  out of line until it counts the set (section 1); `generate --with-imports` counts it.

## Alternatives considered

- **Keep `#[inline(never)]` on every body read (ADR 0136).** Measured: a body of one caller stands apart from it, and
  `Declare`'s read and its caller grew by 193 bytes. Refused.
- **No attribute on any body read.** The compiler then copies a body several codecs read into each, the G2 shape
  ADR 0136 measured, nine bodies in one dispatch. Refused for a body two sites read.
- **Count the sites in each codec's own generation.** A codec does not know which documents import it; the count
  would be of the codecs it imports, not of the readers of its body. Refused: the set is counted where it is
  generated.
- **Count a site per call a consumer writes.** The generator does not see the consumer's code, and the consumer's
  wrapper call is the compiler's to place. Refused.
- **Read each view into a destination the caller holds** (`read_with(cursor, hook, &mut out)`). A view's `decode`
  would still build a default to read into, and ADR 0128 section 4 builds a view once as a value. Refused; the
  value is answered as it is.
- **Make `read_frame` take the read as a function pointer**, so one copy serves every codec. An indirect call per
  decode, which a stack bound cannot follow (ADR 0136, amendment). Refused; the generic part is the reader's start
  and the call, and the rest is one function.

## Measured

To be measured by the consumer at the landing, on the same image. Expected direction of each change:

- S1: `DeclareIn::read_into` folds back into its one caller (about -193 bytes against the pin), and each body of one
  site (`InitBody`, `OpenBody`, `Interest` where one envelope reads it) with it; a body two sites read
  (`Timestamp`, `Encoding`, the extension bodies) stays one function.
- S2: every read body smaller, by the record of a refusal at each read that can refuse, the largest saving in the
  bodies of many fields (`InitBody`, `OpenBody`, the declarations); one `fail` and one `refuse_held` added.
- S3: each ZInt field smaller by the longer read's answer and record; one `read_vle_long` added.
- S4: every `decode_into` and `decode` wrapper smaller by the verdict; one `end_frame` added.
- S5: each view's `decode` back to about its size before the pin or below (`ExtEntryView`, `TransportInitView`,
  `TransportOpenView`, `OamView`), with no default of the view built.
- S6: `zint_tail` smaller, by one of its two loops.

## Order of the changes

1. This ADR, with the amendments of ADRs 0128 and 0136.
2. The runtime: `fail`, `refuse_held`, `read_vle_long`, `zint_tail`, `read_frame` and `end_frame`.
3. The count of a set's body reads, its option, `generate --with-imports`'s count and the goldens' harness; the
   printer's attribute and its `decode`.
4. The main session regenerates the Rust goldens and the committed Rust trees and runs the Rust golden tests, the
   Rust conformance lane, the runtime's tests and the codec clippy check.
5. The consumer regenerates its codecs as one set and measures its image.

## Acceptance

Accepted on all of these: the Rust goldens and conformance lane pass with every frame's bytes and verdict as before;
the runtime's unit tests pass; `scripts/check_clippy_codec.sh` passes over both golden families; the nostd-mcu gate's
worst-case stack finds no indirect call; the consumer measures its codec area at the landing.
