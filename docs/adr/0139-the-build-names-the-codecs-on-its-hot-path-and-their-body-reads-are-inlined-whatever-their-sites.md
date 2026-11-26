# ADR 0139 — The build names the codecs on its hot path, and their body reads are inlined whatever their sites

- Status: Accepted (2026-11-26), under the owner's standing rule: the most long-term correct, textbook structure, cost
  ignored, decided by measurement. Its figures are the consumer's measurement at `cc6de04cb24` (section "Context").
- Amends: ADR 0136 (section 4, `--inline-decode`): the build's choice is no longer every body read or none; it may
  name the codecs whose reads it wants inlined, and `ForgeCompileOptions::inline_decode` is a choice of three, not a
  flag. ADR 0137 (section 1, a body read is out of line where its set reads it from two sites or more): a body read
  of a codec the build names is `#[inline(always)]` whatever its sites; every other read keeps the rule.
- Amended by ADR 0140 (2026-11-26): hotness is a property of a read's site, not of a codec. The body of a codec named
  is hot and so is every read a hot body makes, transitively; a hot body is printed once in forms held in place that
  the hot sites call, its entry points calls of them that keep ADR 0137's rule over the cold sites; a hot body reads
  its ZInts held in place (section 4); a codec is named by its document's stem or by an import alias, not the `name`
  of its `<scxml>` (section 1), and a name of two codecs is refused.
- Related: ADR 0061 (`generate --inline-encode`), ADR 0138 (a ZInt read by one function out of line), ADR 0077 §7
  (what a deployment's storing forms hold, the build's and not the document's).
- Requested by: a downstream bare-metal consumer, 2026-11-26.

## Context

The consumer measured its receive path, an 8-byte put, in instructions per message, by callgrind over the same
window, at SCE `cc6de04cb24`: 4,363.2 against its baseline's 4,332.3 (+31). The size work of ADRs 0136 to 0138
put the body reads out of line by ADR 0137's rule, one site no attribute, two sites or more `#[inline(never)]`, and
on the receive path the generated reads of the push message and its body, the frame header, and the batch and
extension reads are reads the set shares, so each now pays a call. In the consumer's names: `walk_records` +12,
`lend_body::<PushBody>` +12, the frame's step about +14.

The consumer's size gains come mostly from codecs its receive path does not run: the declarations, the handshake,
the control messages. The two choices the build had serve neither half alone:

- ADR 0137's rule (the default) keeps every shared read out of line, the hot ones with the cold, and the receive
  path pays the calls;
- `--inline-decode` (ADR 0136 section 4) inlines every read, the cold ones with the hot, and gives back the size.

Which codecs are hot is a fact of the consumer's workload. The document is the protocol, shared by every build that
speaks it, and says nothing of what a build receives most; the generator sees the document and the set, and the
count of a set's sites says how often a body is written into the program, not how often it runs. The one that knows
is the build.

## Decision

### 1. The build names the codecs whose reads are on its hot path

Amended by ADR 0140: the hot path is made of sites, not codecs. The codecs named are the roots of the hot closure,
whose bodies are printed once in forms held in place that every hot site calls, while the cold sites call the entry
points; a codec is named by its document's stem, the name the count keeps it by, or by an alias an import gives it.

`ForgeCompileOptions::inline_decode` is `forge::body_reads::InlineDecode`, a choice of three:

- `None` (the default): no read is asked inlined, and each body read is placed by ADR 0137's rule;
- `Every`: every body read is `#[inline(always)]` (`generate --inline-decode`, ADR 0136 section 4);
- `Codecs(HotCodecs)`: the body reads of the codecs named, the eager read and the view's alike, are
  `#[inline(always)]` whatever the sites of the set that read them; every other body read keeps ADR 0137's rule.

Hotness is the build's declaration, the way `--inline-encode` (ADR 0061) and the capacities of a deployment (ADR
0077 §7) are. Nothing in a document says it, and the generator infers nothing.

A codec is named by its name, the `name` of its document's `<scxml>`: the name the set's count is kept by
(`BodyReadSites::of`) and the one the generation reads the count under, so the name a build gives is the one the
generator decides by. A path would be spelled several ways for one document (relative, absolute, `sce:std/`).

### 2. The names are checked against the set they name codecs of

A name means a codec of a set, so it is checked against the set: `BodyReadSites` keeps the name of every codec of
the set it counted, read by a site of it or not, and `BodyReadSites::hot_codecs(names)` is the one way a
`HotCodecs` is made. It refuses a name that is no codec of the set, the first such one as the names are given, with
the generation stage's `codegen/inline-decode-codec-not-in-set`: a name misspelt would otherwise leave the hot read
it meant out of line, and the build would measure the slower image with no word said of why. A generation never sees
a name that was not checked.

The record carries no location, no `actual` and no `fix`: the name is a build's argument and in no document, so
it is no token a consumer can find in one, and the repair is a change of that argument, which no edit of a document
spells (SCE_ERROR_CONTRACT §3.1.1). Its message names the codecs the set holds, which are the names a build can
give. It is keyed by the name alone, so a set that gains a codec keeps the record's id.

### 3. Every API that generates a set carries the names

- `generate --with-imports --inline-decode-for <codec>[,<codec>...]`, which may be given again: the run counts the
  set it walks (`BodyReadSites::of_documents`), checks the names against it before any document of it is
  generated, and hands the count and the checked names to the generation of each document. The flag needs
  `--with-imports`: a run of one document is a set of one that is not counted (ADR 0137 section 1), so it has no
  set to check a name of another document against, and could not tell such a name from a misspelt one. It conflicts
  with `--inline-decode`, which names every codec already.
- A build that generates its set through the library counts it once with `BodyReadSites::of_documents`, asks the
  count for its hot codecs (`hot_codecs`), and sets `ForgeCompileOptions { inline_decode:
  InlineDecode::Codecs(hot), body_reads: sites, .. }` on each generation.

### 4. What a named read is, and what stays out of line

Amended by ADR 0140: a hot body reads its ZInts by the frame reader's reads held in place; every other body calls the
reader's read out of line. The runtime's routines of a chain stay out of line; the rule a hot body's chain is read by
calls the entry's form held in place.

The reads named are those ADR 0137 places by their sites: the eager `read_into` of a codec that reads into its
destination, `read` of one built as a value, and the view's `read_with`. The wrappers around them keep no
attribute, as before. The runtime's routines of a chain (ADR 0136 sections 1 and 2) and its reads of a ZInt (ADR
0138) stay out of line: they are no codec's body, and a build that names a codec still calls them.

## Consequences

- No golden moves: the default is `InlineDecode::None`, which prints what ADR 0137 prints, and `Every` prints what
  `--inline-decode` printed. No committed tree of generated code changes.
- `ForgeCompileOptions::inline_decode` is no longer a `bool`: a library build that set it to `true` sets
  `InlineDecode::Every` (`docs/SCE_CODEC_CONSUMER_MIGRATION.md`). `BodyReadSites` holds the names of the set's
  codecs beside the counts, so two counts of sets that read alike and hold different codecs are no longer equal.
- One diagnostic code is added, `codegen/inline-decode-codec-not-in-set` (stage `generate`, no fix), in the
  error contract's catalog, the wire schema and the acceptance appendix.
- No frame decodes differently: an attribute decides where a read's instructions are placed, not what they read.
  ADR 0128's invariants and ADR 0136's static dispatch hold.

## Alternatives considered

- **Keep the two choices, every read or the rule of the sites.** Measured: the rule costs the receive path +31
  instructions a message, and inlining every read gives back the size the consumer's cold codecs saved. Refused.
- **Say in the document which codecs are hot** (an attribute on the codec's `<scxml>`). The document is the
  protocol, shared by builds whose workloads differ: a gateway's hot path is not a sensor's. A hotness written there
  would be one build's measurement imposed on every other. Refused: the build declares it.
- **Have the generator decide** (inline a small body, or the first a frame reads). The generator sees the set, not
  the workload; how often a body runs is not in anything it reads, and a guess would move as the documents change
  while the workload did not. Refused.
- **A `#[inline]` hint on the named reads instead of `#[inline(always)]`.** ADR 0137 left a read of one site with
  no attribute for the compiler to place, and the reads the consumer measured are reads of two sites or more, which
  the compiler weighs by size, not by the receive path it does not see. The build that names a read has measured it,
  and the attribute that does what it asks is `#[inline(always)]`, as `--inline-decode` already marks. Refused.
- **Ignore a name that is no codec of the set, or warn and go on.** A misspelt name then leaves the hot read out of
  line and the run succeeds; a warning on a build's output is read by nobody once the build is green. Refused: the
  run is refused.
- **Accept the names on `generate` of one document.** The run's set is the one document, so the name of a codec of
  another document of the build's set could not be told from a misspelt one: either every such name is refused, which
  makes the flag useless one document at a time, or none is checked. Refused: a set's names are given where a set is
  counted.
- **Name a codec by its document's path.** Section 1. Refused.

## Measured

To be measured by the consumer at the landing, on the same image and the same window, with its hot codecs named
(the frame header, the push message and its body, the batch and the extension reads). Expected direction:

- the receive path back toward its baseline's 4,332.3 instructions a message, the calls of `walk_records`,
  `lend_body::<PushBody>` and the frame's step folded into their callers;
- the codec area larger than at `cc6de04cb24` by the copies of the named bodies only, the cold codecs keeping the
  sizes ADRs 0137 and 0138 measured.

## Order of the changes

1. This ADR, with the amendment lines of ADRs 0136 and 0137.
2. `sce-build`: `InlineDecode` and `HotCodecs`, the set's codecs in `BodyReadSites` and `hot_codecs`, the new
   diagnostic code, `generate --inline-decode-for`, and their tests.
3. The main session runs the `sce-build` tests of the body reads, of `generate --with-imports`, of the CLI flags and
   of the diagnostic contract; no golden is regenerated.
4. The consumer generates its set with its hot codecs named and measures its receive path and its image.

## Acceptance

Accepted on all of these: the Rust goldens are unchanged; a named codec's eager and view body reads are
`#[inline(always)]` whatever their sites, and an unnamed codec's keep ADR 0137's rule; `--inline-decode` still marks
every body read; a name that is no codec of the set is refused with `codegen/inline-decode-codec-not-in-set` before
anything is written; the consumer measures its receive path at the landing.
