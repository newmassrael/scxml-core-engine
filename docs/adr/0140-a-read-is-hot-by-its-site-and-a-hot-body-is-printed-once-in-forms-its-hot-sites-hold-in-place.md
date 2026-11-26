# ADR 0140 — A read is hot by its site, and a hot body is printed once in forms its hot sites hold in place

- Status: Accepted (2026-11-26), under the owner's standing rule: the most long-term correct, textbook structure, cost
  ignored, decided by measurement. Its figures are the consumer's measurement of ADR 0139 at its landing (section
  "Context").
- Amends: ADR 0139 (section 1, the build names the codecs on its hot path and their body reads are inlined whatever
  their sites; section 4, the runtime's ZInt reads stay out of line): hotness is a property of a read's site, not of
  a codec; the body of a codec named is hot, and so is every read a hot body makes, transitively; a hot body is
  printed once, in forms held in place that every hot site calls, and its entry points are calls of them, which keep
  the rule of ADR 0137 over the cold sites alone; a hot body reads its ZInts held in place. A codec is named by its
  document's stem or by an alias an import of the set gives it. ADR 0138 (section 2, a ZInt is read by one call of
  its reader's one read): a hot body reads a ZInt by the frame reader's read held in place, the same decode; every
  other body keeps the call. ADR 0137 (section 1): the sites that place an entry point are the cold ones.
- Related: ADR 0128 (a frame's reads are total and its verdict is read once), ADR 0136 (a chain is read by one routine,
  dispatched statically), ADR 0061 (`generate --inline-encode`).
- Requested by: a downstream bare-metal consumer, 2026-11-26.

## Context

The consumer measured ADR 0139 with its hot set named (the transport frame header, the frame, the push, the put, the
extension entry and its unit, ZInt and ZBuf bodies, the timestamp, the encoding), by callgrind over the same window
and on the same image as before:

- the receive path, an 8-byte put, is +22 instructions a message over its baseline, from +31; the push records are
  back at their baseline;
- the codec area grew +1,434 B over its baseline, from +798.

The size is ADR 0139's own doing. A codec it names is `#[inline(always)]` whatever its sites, and hot codecs such as
the extension entry, the ZInt extension and the encoding are read by cold codecs too: the declarations, the
handshake's init body, the delete, the final declaration's chain. Inlined per codec, their bodies were copied into
those cold sites: `read_declare` 1,500 to 2,469 B, `InitBodyIn::read_into` +868, `MsgDelView::read_with` +450,
`read_chain_into::<DeclFinalExtensionsChain>` +435. The workload made none of those reads hot; the attribute did,
because it was put on the codec.

The remaining +22 is in the frame's step: `SceReader::read_vle_inner` (454 B, out of line, ADR 0138) is called for
each message by the frame's read although the frame and its header are named hot, and the frame's sequence number
costs 62 instructions against the baseline's 57. ADR 0139 section 4 kept the runtime's ZInt reads out of line for
every body, hot or not.

The names were accepted only as the stems of the documents, the name the set's count keeps a codec by, while the
consumer's documents import one another under aliases of their own; ADR 0139 section 1 said the `name` of the
document's `<scxml>`, which is not what the count keeps. The refusal listing the set's codecs made the slip plain,
which is what it was for.

## Decision

### 1. Hotness is a property of a site

A site is the read of one codec's body inside another's: an arm of a variant, an embedded body, the entry of a list
or of a chain (ADR 0137 section 1 counts them). Whether a read is on the hot path is a fact of the path the read is
made on, not of the codec read: the extension entry is read hot inside the put and cold inside a declaration, in the
same image.

The build names the hot roots (`generate --with-imports --inline-decode-for`, `InlineDecode::Codecs`). The body of a
codec named is hot, and the site of each read a hot body makes is hot: the closure of the names over the reads of the
set (`BodyReadSites::hot_codecs`, which walks the edges the count keeps, each holder's sites of each codec it holds).
A codec read from a hot site is hot in turn, whatever else reads it. No codec is made hot by a codec it reads: the
closure goes down the reads, never up.

### 2. A hot body is printed once, in forms held in place; its entry points are calls of them

A codec whose body is hot is printed in two forms from its one body:

- the forms held in place, `#[inline(always)]`: `read_into_inline` of a codec that reads into its destination,
  `read_inline` of one built as a value, `read_with_inline` of a view of its own, and the wrappers of each
  (`read_inline` around `read_into_inline`, `read_into_inline` around `read_inline`, a view alias's
  `read_with_inline`). The body is printed in exactly one of them;
- the entry points `read_into`, `read` and `read_with`, as before, each now one call of its form held in place.

Every hot site calls the form held in place; every cold site calls the entry point. A codec read only from hot sites
still has its entry points, the functions its own `decode` and an external caller reach. A codec no hot body reads
prints what it printed, byte for byte.

The entry point keeps ADR 0137's rule over the sites that call it, the cold ones: none where one site or none calls
it, `#[inline(never)]` where two or more do. The hot sites are not counted: they call the form held in place. So a
body read from hot and from cold sites has its code once in each hot site and once in its entry point, and a cold
body receives only a call. `BodyReadForms` is what the generation of one codec is told: whether its body is hot, and
the sites that call its entry points.

What a hot body reads it reads by the forms held in place, at every kind of site: the arms of its variant, its
embedded bodies, the entries of its lists, its records (ADR 0130), the scan of a view's lists, and the walk of each
list, which is `{Codec}InlineWalk` where the list shares its entry's walk. The rule a hot body's chain is read by
(ADR 0136) calls the entry's form held in place; the runtime's routine of the chain stays one out-of-line function per
rule, as ADR 0136 dispatches it statically.

### 3. A hot body reads its ZInts held in place

The frame reader gains `read_vle_u16_inline`, `read_vle_u32_inline` and `read_vle_u64_inline`, `#[inline(always)]`,
which hold the reader's one decode of a ZInt in place (`read_vle_held`). `read_vle_inner`, the out-of-line read every
other body calls, is one call of the same held decode, and `codec::read_zint` stays the one body of every ZInt of
every reader: one decode in the source, two placements of it. A hot body prints the forms held in place, every other
body the calls.

The generated reads are made through the frame reader, which a decode names `cursor`; no generated body reads a ZInt
through `SceCursor`, so the cursor's reads, which only a consumer's own code calls, keep their one form.

### 4. A codec is named by its document's stem or by an alias an import of the set gives it

The count keeps each codec by its document's stem and keeps each import alias of the set with the codecs it names.
A name resolves to the codecs it is the stem of or an alias of: one is the codec named, whichever way it was named;
none is refused with `codegen/inline-decode-codec-not-in-set`, as before; two, the stem of one codec and an alias
another's import gives another, or one alias two imports give two codecs, is refused with the new
`codegen/inline-decode-codec-ambiguous`, which names the codecs the name names. Resolving it to either would inline a
codec the build did not measure with no word said, the defect section 2 of ADR 0139 refuses a misspelt name for. The
record carries no location, no `actual` and no `fix`, for the reasons ADR 0139 gives, and is keyed by the name.

### 5. `InlineDecode::Every` is unchanged

`--inline-decode` marks the one function that reads each body `#[inline(always)]`, as ADR 0136 section 4 says, and
prints no form held in place. It is a property of one document, generated with no knowledge of the forms its imports
print, so it cannot call a form an import may not have. `InlineDecode::Codecs` is a property of a set: every document
of the set is generated with the same names and the same count, so a hot body's every child is hot and has its forms.

### 6. The invariants of ADRs 0128 and 0136 hold

Every read stays total, the frame has one verdict, read once, and no generated read returns early or uses `?`. A
form held in place is the same text as the read it replaced, in another place; the chain is dispatched statically by
its rule, whichever form the rule's read calls.

## Consequences

- No golden moves: with no codec named a body is printed in its entry points alone, as ADR 0137 prints it, and the
  ZInt reads are the calls. `--inline-decode` prints what it printed. No committed tree of generated code changes.
- `HotCodecs` answers `named`, `is_hot` and `sites_of`; its `contains` of ADR 0139 is `named`.
  `InlineDecode::inlines` gives way to `InlineDecode::forms`, which answers `BodyReadForms`. `BodyReadSites` holds the
  sites of each holder and the aliases of the set beside the counts.
- The frame reader has three public reads more; their answers are the out-of-line reads' at every length, cut and width.
- One diagnostic code is added, `codegen/inline-decode-codec-ambiguous` (stage `generate`, no fix), in the error
  contract's catalog, the wire schema and the acceptance appendix.
- A build that names hot codecs gets, for each, its body once per hot site and once in its entry point, where ADR 0139
  gave it once per site of every kind; a codec only cold sites read gets nothing more.

## Alternatives considered

- **Keep hotness on the codec (ADR 0139).** Measured: the codec area +1,434 B, the cold readers of hot codecs carrying
  their bodies. Refused.
- **Name hot sites one by one** (a pair of holder and held). The build would have to spell every edge under a root it
  measured, and the edges below a hot body are hot by construction: a hot body that calls out of line pays the call the
  build named it to remove. Refused: the build names roots, the closure follows the reads.
- **Print a copy of the body for each hot parent** (a hot read written into the parent's module). The text would be
  duplicated in the generated tree, once per parent, and a change of the child would move every parent's file. Refused:
  one source body, a form held in place that the compiler copies.
- **Make the entry point the form held in place and add an out-of-line form for cold sites.** Every cold site and
  every consumer that calls the entry point would change name, and the cold path, the larger, would move. Refused: the
  entry points keep their names and ADR 0137's placement; the hot path names its forms.
- **Hold the whole ZInt decode in place at every body.** That is the size ADRs 0137 and 0138 removed. Refused: held in
  place in hot bodies, a call elsewhere.
- **Give `SceCursor` forms held in place too.** No generated body reads through it. Refused until one does.
- **Resolve a name of two codecs to its stem's codec.** A build that meant the alias would measure the stem's codec
  inlined and its own not, with no word said. Refused: the name is refused, and the stem names the codec.

## Measured

To be measured by the consumer at the landing, on the same image and window, with the same hot set named. Expected
direction: the receive path toward its baseline, the frame's step making no call of `read_vle_inner`; the codec area
back toward +798 B over the baseline, `read_declare`, `InitBodyIn::read_into`, `MsgDelView::read_with` and
`read_chain_into::<DeclFinalExtensionsChain>` at their sizes before ADR 0139, each calling the entry point of the hot
codec it reads.

## Order of the changes

1. The runtime: the frame reader's ZInt reads held in place, `read_vle_inner` a call of the same held decode.
2. This ADR, with the amendment lines of ADRs 0137, 0138 and 0139.
3. `sce-build`: the closure and the aliases in `BodyReadSites`, `BodyReadForms`, the forms held in place in the
   template and in every site a hot body reads through, the new diagnostic code, and their tests.
4. The main session runs the runtime's tests, the `sce-build` tests of the body reads, of `generate --with-imports`,
   of the CLI flags, of the diagnostic contract and of the conformance set; no golden is regenerated.
5. The consumer generates its set with its hot codecs named and measures its receive path and its image.

## Acceptance

Accepted on all of these: the Rust goldens are unchanged; a codec read from a hot and a cold body prints both forms
and each parent calls its own; the hot closure is transitive; a hot body reads its ZInts by the reads held in place; a
codec is named by its stem or an import alias, and a name of two codecs is refused; a set with hot codecs compiles
with and without views; the consumer measures its receive path and its image at the landing.
