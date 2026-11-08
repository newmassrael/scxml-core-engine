# ADR 0094 — A generated file's source hash covers the documents it is generated from and what they read, and no other document

- Status: Proposed (2026-11-08). Written at the owner's order of that day, after the measurement in "Context". Nothing
  of it has landed; "Landing order" lists the steps, each with the tests that hold it. "Open questions for the owner"
  lists what must be answered before acceptance, each with a recommendation the sections are written to.
- Date: 2026-11-08
- Amends, once accepted:
  - SCE Protocol-Synthesis RFC §synth-6.2.6 (`docs/spec/synth/rfc-sce-protocol-synthesis.md`): what the
    `source-hash` covers, and the header's form (section 10 below gives the text);
  - the same RFC's sourcemap section (§synth-5-O): the sentence "Its `source_hash` MUST match the corresponding
    generated file's §6.2.6 header hash" becomes the rule the code already follows for a directory of several
    machines (section 4);
  - `SCE_WIRE_CONTRACTS.md`, the prose of three rows whose value is this hash (sourcemap sidecar, transition table,
    payload schema): shape and status unchanged (section 9);
  - the rows of `forge/source-hash-mismatch`, `forge/source-hash-input-uncovered` and
    `forge/source-hash-walk-unbounded` in `docs/SCE_ACCEPTED_SUBSET.md` and `SCE_ERROR_CONTRACT.md` (section 8).
- Scope:
  - `sce-build/src/forge/drift.rs` (`SourceSet`, the header, its parser), `sce-build/src/import_closure.rs` (each
    unit's reads), `sce-build/src/parser.rs` (one read it does not record), `sce-build/src/lib.rs`
    (`bind_run_digest`, `RunDigest`, `apply_drift_headers_to_output`), `sce-build/src/forge/sourcemap.rs`, and in
    `sce-build/src/bin/sce_codegen.rs` `DriftContext`, every route that writes a drift-headed file, the depfile and
    `cmd_verify`;
  - the tests that hold the old rule, chiefly in `sce-build/tests/b9_drift_detection.rs`, re-aimed (section 12);
  - every committed generated tree, re-stamped once (section 11).
- Related:
  - the module documentation of `sce-build/src/forge/drift.rs`, which retired the template hash, the timestamp and
    the generator identity from the header (2026-09) because each re-stamped every committed tree with no byte of
    output changed; this ADR applies the same principle to the input set;
  - the fix of 2026-09-28 in the same file, which stopped folding the whole standard library into every tree that
    named it and folds only the standard documents a tree's documents reach: the closure rule of this ADR, already
    in force for the standard library;
  - ADR 0057 section 7 (a variant's transition table carries the hash every build of its document carries; kept);
  - the acceptance record (`sce-build/src/acceptance_record.rs`), which pins every file a design's parse read
    (section 7).

## Context

### What was measured

On 2026-11-08, at `81a907ec32`, 753 committed generated files carry one and the same header line,
`source-hash: a0d514a5bb9842543c3a83f1c232fcee900a165f2a3ed19fb29e42d249c8aa22`: 171 in the Go tree
(`backends/go/tests/integration`), 171 in the Kotlin tree, 169 in the Rust tree
(`backends/rust/tests/src/integration/static_datamodel`) and 242 in the Swift tree. It is the digest of the directory
`sce-build/tests/fixtures/static_datamodel`, which holds 231 documents (`**/*.scxml`). 109 of those files carry the
value again in their body: 105 hold it as the `x-sce-source-hash` of a payload schema (ADR 0087 section 1), and a
declared transition table or transition function holds it as its `source_hash` (SCE Accepted Subset §2.20, §2.21).

So:

- editing or adding any one fixture re-stamps those 753 files, though the content of nearly all of them, past the
  hash, does not change;
- two branches that each touch a fixture conflict in hundreds of generated headers, whatever the fixtures are;
- the landing queue returned another session's item on exactly such a conflict on 2026-11-08 (item 30, after a
  union fixture rename landed);
- lanes of work cannot proceed in parallel, and every landing that touches a fixture carries a regeneration diff of
  about 750 files that hides its real change.

`scripts/regen_all_committed_trees.sh` already says in its header that "the §synth-6.2.6 header carries only facts
about each file's inputs, so nothing is re-stamped that did not change". For the documents it is not true.

### The rule, read from the tree

`SourceSet::collect` (`sce-build/src/forge/drift.rs`) walks `**/*.scxml` under an input root, adds `deploy.yaml`
when the run is given one, folds in the standard documents the walked documents import or start
(`forge::stdlib::closure`), and the limits the build runs under when any is moved (`fold_limits`). The fold is
`generator_witness::hash_btreemap`, a length-prefixed fold of `(root-relative path, sha256 of raw bytes)` in key
order. `generate` infers the root as the directory of the document (`containing_dir(scxml_path)` in
`sce-build/src/bin/sce_codegen.rs`, where `DriftContext::compute` is called), and `--input-root` overrides it. The
regeneration scripts of the four `static_datamodel` trees call `generate` once per document with no
`--input-root`, so every document's files carry the digest of the whole directory. `generate-w3c` hashes the whole
W3C resources tree once for all its tests (`generate_w3c_unified`), `generate-conformance` the resource directory of
its harness, `orchestrate` the directory of the first document of its set, and
`backends/go/forge-runtime/round_trip/generate.sh` generates five codecs of `tests/forge/resources` whose headers
cover that whole directory.

The same digest is bound into content: `bind_run_digest` (`sce-build/src/lib.rs`) gives it to a declared transition
table and function, `payload_schema` writes it as `x-sce-source-hash`, `transition-table --header` and
`payload-schema` recompute it from the same root, and the sourcemap sidecar takes it (or, for a directory of
machines of several source sets, `sourcemap::source_hash_of_machines` over their hashes).

### The rule is wrong in both directions

The template hash was retired because it moved when output did not and did not move when output did. The directory
rule has both defects too:

- **Not minimal.** A document's files move when any sibling moves (the measurement above).
- **Not sound.** A document's output can change while its header does not. Two cases are in the tree today:
  `backends/rust/forge-conformance/fixtures/codec_feature_gated_arm.scxml` imports
  `../../../../tests/forge/resources/codec_default_marker_arm_a.scxml`, a document outside its root, which the walk
  never reads; and `tests/w3c_template_parity/fixtures/coord_xinclude_fragment/main.scxml` includes `frag.xml`, a
  fragment the walk's `.scxml` filter drops. Editing either changes what is generated and leaves the header as it
  was.

### What already computes a closure

Four pieces of the tree already answer "what does this document read", each for its own consumer. The
implementation reuses them and adds no fifth:

- `import_closure::walk` (`sce-build/src/import_closure.rs`, for `generate --with-imports`) follows every
  `<sce:import>` (local and standard), every `<invoke src="sce:std/...">`, every `sce:candidates` entry and every
  `<invoke type="scxml" src>` child, transitively, and loads each document through `load_forge_source`, which
  expands `<xi:include>` and `<sce:use>` and returns the files it read (`LoadedForgeSource::deps`). It does not keep
  those reads on the `Unit` it returns.
- `forge::stdlib::closure`, which `SourceSet` already uses for the standard library: the closure rule of this ADR,
  restricted to standard documents.
- The recorded reads of a generation: `SCXMLParser::preprocessor_deps`, `LoadedForgeSource::deps` and
  `GeneratedOutput::deps`, which the depfile declares (`depfile_entry` in `sce_codegen.rs`), and which the acceptance
  record pins. They are not complete: the parser reads a static `src` child to learn its interface
  (`parse_child_metadata`, in the `Invoke::Scxml` branch of `parser.rs`) and does not record the read, while it does
  record a `sce:candidates` document read the same way. The depfile's directory source set hid the gap.
- `SourceSet`'s machinery: the coherent read (each file witnessed before its read and re-checked after the walk),
  `covers`, `contributing_paths`, `fold_limits`, and the fold `generator_witness::hash_btreemap`.

`sourcemap::source_hash_of_machines` already combines the hashes of several machines into a sidecar's one value.

## The textbook structure

**A derived artifact is identified by its inputs.** Make relates a target to its prerequisites, not to the
directory it sits in (Feldman 1979). In "Build Systems à la Carte" a task is a function of the keys it depends on,
and a verifying trace records, for each key it built, the hashes of the dependencies the task actually used, so a
change to any other input leaves the key alone (Mokhov, Mitchell and Peyton Jones 2018, 2020). Nix names a derived
path by the hash of its derivation's input closure, transitively (Dolstra 2006); Bazel keys an action by the digest
of its command and its declared inputs.

**Soundness and minimality.** An incremental build is correct when every output equals what its task computes from
the current inputs, and minimal when a task runs only if an input it transitively depends on changed (Mokhov et al.;
Erdweg, Lichter and Weiel 2015 call the two soundness and optimality). An undeclared input yields an output that
passes for current when it is stale; a declared input that is not one yields spurious work, and between branches,
spurious conflicts.

**A digest of a set is a fold of its members' digests in a canonical order, under an injective encoding** (Merkle
1987). `hash_btreemap` is one.

Measured against these, the header is meant to be a verifying trace of one file, and the directory rule makes it a
trace of a directory: neither sound nor minimal. The closure of the documents a file is generated from is the set
the trace must name.

## Decision

**A generated file's `source-hash` is the digest of the input closure of the documents it is generated from: those
documents and every file their generation reads, transitively, with `deploy.yaml` when the run is given one and the
limits when any is moved, each keyed by its spelling relative to the files that reach it and hashed by its raw
bytes, folded in key order. The header names those documents on `source:` lines, so `verify` recomputes the closure
from the file alone, without generating. No file's header covers a document outside its closure. A file generated
from every document of a directory names the directory, and its closure is the union of theirs. `--assert-unchanged`
stays the judge of content.**

### 1. The input closure

The closure of a set of seed documents is the smallest set of files that holds the seeds and, for each member, every
file the generation of that member reads:

1. the targets of its `<xi:include>`s, whatever their extension;
2. the fragments of its `<sce:use>`s;
3. the targets of its `<sce:import>`s, local or standard;
4. the documents its `<invoke>`s start by `src`, and those its hybrid `<invoke>`s name in `sce:candidates`: the
   parent's generation reads each to learn its interface (which names a `<param>` or a `namelist` item may add to
   the child depends on the child);
5. the standard documents its `<invoke src="sce:std/...">`s start, which the parent's run generates in-line;
6. what an in-line child (`<invoke>` with `<content>`) reads; the child's text is already part of the member's
   bytes.

The rule is transitive over every kind of reference: the closure of a child a statechart starts holds the child's
own imports. That over-approximates where the parent reads only the child's interface, and it is the direction to
be wrong in: a header that moves without need costs one re-stamped file, a header that stays while content moves is
the state the header exists to rule out. Precision is narrowed only where a test proves the narrowing sound.

Two entries that are not documents are added, as today: `deploy.yaml` when the run is given one (it is read into
the options every document of the run is generated under: queue capacities, codec capacities, the Mesh topology),
and the limits when the build moves any (`crate::limits`).

What the closure does not cover, as today: the generator, its templates, and the command-line options other than
the limits (the language, the output directory, the Go module and Kotlin package prefixes, `--parent-stem`). Those
change content without being inputs a document names, and the content judge is what answers for them (section 6).

### 2. The canonical form

Each member is an entry `(key, sha256 of its raw bytes)`, pre-expansion, as today. The keys:

- The **anchor** is the deepest directory that holds every seed; for one seed, its own directory.
- A **seed** is keyed by its path relative to the anchor: for one seed, its file name.
- A **member** reached from member `m` by a reference written `r` is keyed by `r` resolved lexically against the
  directory of `m`'s key (`.` and `..` folded, a leading `..` kept): the spelling the documents themselves give it,
  not the place a search path found it.
- A **standard document** is keyed by its library name (`sce:std/...`), as today.
- `deploy.yaml` and `limits` keep their canonical keys.

The entries are folded by `hash_btreemap`, unchanged. A closure's digest is therefore a function of the documents'
texts and bytes, the seeds' file names and their layout relative to each other, and nothing else: not where the
checkout sits, not the working directory, not where a build staged a copy, not `--input-root`.

A **staged copy** hashes as the document it was copied from. The integration regeneration scripts stage a fixture
into a temporary directory and pass `--input-root` so that the header matches the tracked location; under this rule
the copy's key is its file name, which is the original's, and a candidate found under the input root is keyed by the
spelling that names it, which is the same from either place. `--input-root` keeps its other role, where candidates
are looked for, and stops bearing on the hash.

The coherent read of today's walk carries over: every member is witnessed before its bytes are taken and re-checked
after the closure is complete, and a closure whose members moved while it was read is read again, then refused
(`RootNotQuiescent`).

### 3. The header

```text
// SCE-GENERATED — DO NOT EDIT
// source-hash: <sha256 of the file's input closure>
// source: <seed key>
```

One `source:` line per seed, in key order, after the `source-hash` line; `#` replaces `//` for Python, as today. A
file generated from every document of a directory has one line, `source: ./` (section 4). The line states which
documents the hash is a closure of; without it the hash is a fact nobody can check without knowing what it is over,
and `verify` could only test membership in a set (rejected under "Alternatives rejected"). It changes only when the
file is generated from another document, which changes the file's content anyway.

`parse_embedded_header` reads every key line under the banner rather than a window of twelve lines, since an
`orchestrate` file has one `source:` line per document of its set. `strip_header` already removes every key line it
recognises, so `source` joins the keys it recognises; `RETIRED_HEADER_KEYS` is unchanged.

### 4. What each kind of file covers

- **A forge unit** (an algorithm, a schema, an enum, a union, a codec, a template, a test table): its document.
- **A statechart's machine**: its document.
- **An in-line child's machine** (`<machine>__sce_synth_invoke__<id>_sm`): the document that holds its text, the
  parent's; so its header equals its parent's, hash and `source:`.
- **A standard statechart a parent starts in-line**: the parent's document, whose closure holds it by rule 5.
- **A child generated from its own file** (`generate --as-child`, a `src` child or a candidate, a child of
  `--with-imports`): the child's document. A child document a run writes itself (a Mesh §9.6.6 remote child's
  `.scxml`, a hybrid invoke's stub) is generated from the document it was written from; open question 2.
- **`generate --with-imports`**: each unit its own document.
- **The Rust per-test `mod.rs`** of `generate-w3c`, which names the test's machine and its children: the test's
  document.
- **`orchestrate`**: every document of the named set, for every file it writes. Its compile is joint
  (`compile_scxml_with_imports` resolves cross-document `<sce:on-sample link>` and `<sce:outbox ref>` references
  across the set), and until the joint compile records which documents each output read, the set is the honest
  answer (open question 5).
- **Files generated from every document of a directory**, the set form, `source: ./`: the aggregator `mod.rs` of
  the Rust W3C tree (it lists the tests that generated, and whether a test generates depends on its document's
  bytes) and the harness of `generate-conformance` (it is rendered from every fixture of its resource directory).
  Their closure is the union of the closures of every document under that directory, keyed relative to it. This is
  today's source set, made sound, kept for the files whose content really depends on the whole set.
- **A payload schema's `x-sce-source-hash`**: the `source-hash` of the unit it is written into, its document's
  closure. `payload-schema` recomputes it from the document alone.
- **A declared transition table's and function's `source_hash`**: the closure digest of the document that declares
  them. `bind_run_digest` and `RunDigest` are renamed for what they then carry (a document's digest, not a run's).
- **`transition-table --header`**: the closure digest of the document, so an export kept apart from a build is still
  checkable against the machine compiled from the same document.
- **A sourcemap sidecar's `source_hash`**: `source_hash_of_machines` over the closure digests of the machines it
  describes, which is their one digest when they share it (a test's parent and its in-line children) and the sha256
  of the distinct digests otherwise. The function exists; only its inputs change.
- **Files that carry no drift header** keep none: the module index each Rust regeneration script writes
  (`// GENERATED -- DO NOT EDIT`) and `embed/MANIFEST.json`. A run-wide index the generator writes with a header in
  future takes the set form.

### 5. `verify`

`sce-codegen verify <out-dir> --input-root <root>...` (the flag becomes repeatable) recomputes each file's closure
from its header and never generates:

1. For each drift-headed file under the output directory, read its `source:` lines.
2. Locate the anchor: the one directory at or below a given root under which every seed key names a document; for
   the set form, the root itself. A seed found under no root is refused as `forge/source-hash-input-uncovered`; one
   found under two places is refused the same way, naming both.
3. Compute the closure of the seeds at that anchor (with `--deploy`, and the limits when the generation moved any,
   given as the generation was given them), by the function generation uses, memoised per seed set.
4. Compare with the file's `source-hash`; a difference is `forge/source-hash-mismatch`, whose `expected` is the
   recomputed digest of that file's own closure.

A header without a `source:` line (the shape before this ADR) is refused as a mismatch that names its shape, so a
tree that missed the migration cannot pass. A headerless file is reported as today. The walk ceiling
(`MAX_DIRECTORY_DESCENTS`) bounds the anchor search and the set form, so `forge/source-hash-walk-unbounded` stays.

`verify` keeps the question it answers: were these files generated from the current inputs? It now answers it per
file. A stale file (one whose closure moved) is named, a file of an untouched document is not, and the refusal says
what to regenerate.

### 6. `--assert-unchanged` stays the content judge

The header is a necessary condition of currency, not a sufficient one: it does not move for a template edit, a
generator edit or a command-line option. `--assert-unchanged` generates in memory and names every file on disk that
is not as the run would leave it, comparing full bytes, header included; nothing in this ADR narrows it. In
particular no route may skip a file because its header is unchanged; a test holds that (section 12, test 10). The
`regen-reproduces` gate, which regenerates every committed tree and diffs, is unchanged.

### 7. The depfile and the acceptance record

The depfile's `source_set` (`DepfileInputs::source_set`) becomes the closure: a generation declares what its header
covers, as today, and that set is now what it reads. A sibling's edit no longer re-runs a build step, and the long
comment in `depfile_entry` on why a never-parsed sibling had to be declared is retired with the reason.

The acceptance record keeps its inputs as they are, the parser's recorded reads: changing them would lapse every
record already taken ("The parse now reads a file the acceptance never saw"). A test holds the record's inputs to be
a subset of the closure (section 12, test 3); whether the record should pin the closure is open question 4.

### 8. Diagnostics

No code is added or retired.

- `forge/source-hash-mismatch` keeps its fields; `expected` is the recomputed digest of the file's own closure.
- `forge/source-hash-input-uncovered` is no longer reachable by `generate` for one seed (a seed is in its own closure
  by construction, so the closure is never empty); it is reached when a set-form directory holds no document, and by
  `verify` when a seed is found under no root, or under more than one.
- `forge/source-hash-walk-unbounded` is reached by the set form and by `verify`'s anchor search.

The appendix row of `forge/source-hash-mismatch` in `docs/SCE_ACCEPTED_SUBSET.md` still speaks of "current source +
template state", stale since 2026-09; the landing rewrites the three rows.

### 9. Wire surfaces

The drift header itself is not a row of `SCE_WIRE_CONTRACTS.md`: it has no schema file, so
`wire_surface_stability.rs` cannot key it, and the registry's rule puts a claim with no artifact on the wire in the
document that owns the behaviour, here §synth-6.2.6 and the tests of `drift.rs`. Its form gains the `source:` key;
a consumer that recognises a generated file by the banner, as the registry and `sce_generated_tree.sh` do, is
unaffected.

Three registered surfaces carry the hash as a value: the sourcemap sidecar's `source_hash`, the transition table
header's `source_hash` and the payload schema's `x-sce-source-hash`. All three are `pre-release`. Their shape does
not change; the meaning of the value narrows from "the source set of the run" to "the input closure of the
document(s)". Policy item 1 allows that in place; no status flips, no schema file changes, and each row's prose is
rewritten in the landing that switches the value. The reasons the registry gives for exempting the three from the
`generator` stamp (policy item 2) stay true and become stronger: a value that moved with every sibling's edit was
already a weak name for a file's inputs.

### 10. The amendment of §synth-6.2.6

The section's first two paragraphs become:

> Every emitted file carries a header of the form:
>
> ```
> // SCE-GENERATED — DO NOT EDIT
> // source-hash: <sha256 of the file's input closure>
> // source: <document the file is generated from>
> ```
>
> The input closure of a set of documents is the smallest set of files that holds them and every file the
> generation of any member reads: its `<xi:include>` targets, its `<sce:use>` fragments, its `<sce:import>` targets,
> the documents its `<invoke>`s start by `src` or name in `sce:candidates`, and the standard documents it imports or
> starts; with `deploy.yaml` when the run is given one, and the limits the build runs under when any is moved. Each
> member is keyed by its spelling relative to the files that reach it and hashed by its raw bytes, and the entries
> are folded in key order. A file names each document it is generated from on a `source:` line; a file generated
> from every document of a directory names it as `./`, and its closure is the union of theirs.
>
> The header states facts about the file's own inputs and none that can change while the file does not: no other
> document, no timestamp, no template or generator hash (both retired 2026-09). A run names its `generator` on its
> stdout manifest. Drift is judged on content: `--assert-unchanged` writes nothing and names each file on disk the
> generation would change; `verify` recomputes each file's closure from its `source:` lines, without generating;
> CI regenerates and diffs.

and in §synth-5-O, "Its `source_hash` MUST match the corresponding generated file's §6.2.6 header hash; mismatch is
a build error" becomes "Its `source_hash` is the §6.2.6 `source-hash` of the machines it maps when they share one,
and otherwise the sha256 of their distinct hashes, sorted, one per line; a mismatch is a build error." The synth
section mirror (`docs/spec/synth/.atomic/`) is regenerated from the RFC by `synth_rfc_to_manifest.py` in the same
landing.

### 11. The migration

One regeneration re-stamps every committed tree once: `scripts/regen_all_committed_trees.sh`, the
`static_datamodel` scripts of every language and the trees that script does not reach, in the landing that switches
the routes. Every route switches in that one landing, so no tree is re-stamped twice. The diff of that landing is
mechanical and is reviewed as such: every changed line is a header line (the `source-hash` value and the new
`source:` lines) or a body line holding the old digest (the 105 payload schemas and the declared tables and
functions), and a filter over the diff that drops those lines leaves nothing. After it, an edit to a document
re-stamps the files whose closure holds that document, and only those.

### 12. The tests

Each test is written so that the structure it replaces turns it red; the mutant that does is named with it, and each
new guard gets a casefile under `sce-build/tests/mutations`.

1. **`editing_one_document_moves_no_header_whose_closure_excludes_it`** (`sce-build/tests/`, the CLI in a temporary
   directory): an algorithm `a.scxml`, a statechart `b.scxml` that does not import it, a statechart `c.scxml` that
   does; generate `b` and `c` for one language, edit `a`'s body, generate again. Every file of `b` is byte-identical;
   every header of `c` moved. Red under: `DriftContext` computing `SourceSet::collect(containing_dir(seed))`, the
   rule of today.
2. **`a_transitive_import_moves_the_importers_header`**: `c` imports `a`, which imports `a2`; editing `a2` moves
   `c`'s header. Red under: the closure walk that does not follow a reached member's own references.
3. **`every_file_a_generation_reads_is_in_its_closure`**, the soundness sweep: for every document of
   `sce-build/tests/fixtures/static_datamodel`, `test_profile` and `union`, generated in memory with its reads
   recorded, the recorded reads (`GeneratedOutput::deps`, the parser's, and the static `src` child read landing step
   1 records) and the acceptance record's inputs are a subset of the closure's members. The sweep asserts that it met
   at least one `<invoke src>`, one `sce:candidates`, one `<sce:import>` and one `<xi:include>`, so it cannot pass
   empty. Red under: the closure walk without `<invoke src>` children (`By::FileChild` dropped).
4. **`an_outside_import_and_a_fragment_that_is_not_scxml_move_the_header`**: a document that imports
   `../other/x.scxml` and includes `frag.xml`; editing either moves its header. Red under today's rule (the walk
   reads neither) and under a closure filtered to `.scxml` members.
5. **`a_staged_copy_hashes_as_the_document_it_was_copied_from`**: a copy in a temporary directory, generated with
   `--input-root` naming the original's directory where a candidate lives, carries the original's header. Red
   under: a seed keyed by its path relative to `--input-root` (the copy then sits under `..`).
6. **`the_closure_does_not_depend_on_where_the_tree_sits`**: a directory of documents copied elsewhere whole gives
   the same headers. Red under: a member keyed by its canonical path.
7. **`an_inline_child_carries_its_parents_closure`**: the files of `<machine>__sce_synth_invoke__<id>_sm` carry
   their parent's `source-hash` and `source:`. Red under: an in-line child seeded by its synthesised name.
8. **`verify_refuses_exactly_the_files_whose_closure_moved`**: generate `b` and `c` as in test 1, edit `a`; `verify`
   refuses `c`'s file with `expected` equal to `c`'s recomputed closure digest and passes `b`'s; after `c` alone is
   generated again, `verify` passes. Then delete `c.scxml`: `verify` refuses `c`'s files with
   `forge/source-hash-input-uncovered`. Red under: `verify` comparing every file with the set-form digest of the root
   (`b` refused), and under `verify` passing a file whose seed it cannot find.
9. **`verify_generates_nothing`**: `verify` passes on a generated tree when no template tree is reachable from the
   workspace it is run in. Red under: a `verify` that regenerates in memory.
10. **`assert_unchanged_names_a_change_no_header_covers`**: generate a Go unit with `--go-module-prefix p1`, then run
    `--assert-unchanged` with `p2`: the run is refused with `forge/generated-output-changed` though every header is
    unchanged. Red under: a route that skips a file whose header did not move.
11. **The payload schema and the transition table name their unit's hash**: the existing payload-schema tests and
    `transition_table_closure.rs::the_header_and_the_transitions_agree_with_what_generate_wrote` gain a sibling
    document in their directory, and assert `x-sce-source-hash` and the export's `source_hash` equal the header of
    the generated unit, with `payload-schema` and `transition-table --header` given no `--input-root`. Red under:
    either command keeping the directory set.
12. **`deploy_yaml_is_in_every_closure_of_a_run_given_one`** (re-aiming `source_hash_includes_deploy_yaml_when_given`
    in `drift.rs`). Red under: the deploy entry dropped from the closure.
13. **`the_depfile_names_the_closure_and_no_sibling`**: `generate --write-deps` for `b` beside `a` declares no `a`.
    Red under: the depfile still extended with the directory set.
14. **`the_set_form_covers_every_document_of_its_directory`**: the Rust W3C aggregator `mod.rs` moves when any test's
    document is edited; the files of another test do not. Red under: the aggregator seeded by its first test only.
15. **`the_header_round_trips_its_seeds`** (`drift.rs` unit): render and parse a header of twenty `source:` lines;
    replace a header of each older shape. Red under: the twelve-line window of `parse_embedded_header`.
16. **`committed_trees_state_their_own_closures`**: for every drift-headed file of the committed trees (the four
    `static_datamodel` trees, the W3C, integration, Mesh and round-trip trees), the closure recomputed from its
    `source:` lines equals its `source-hash`. The `verify_passes_on_real_committed_*` tests of
    `b9_drift_detection.rs` become cases of it. Red under: any route left on the directory set, and under a tree not
    regenerated.
17. **A sidecar names the closures of its machines**: a directory holding one test's parent and its in-line children
    has a sidecar whose `source_hash` equals their header. Red under: a sidecar hashed from the directory.

Re-aimed or retired with the rule they held: in `b9_drift_detection.rs`,
`verify_passes_when_input_set_is_empty_scxml_directory`, `input_root_override_pins_hash_to_canonical_location`
(becomes test 5), `generate_refuses_when_the_source_set_is_empty`,
`generate_allows_a_staged_derivative_under_a_declared_root`,
`orchestrate_requires_a_root_covering_the_entire_document_set` and
`orchestrate_refuses_an_outside_input_that_duplicates_a_covered_file` (the set of an `orchestrate` run is its named
documents, so a root covers nothing), and `committed_generated_files_carry_the_current_header_shape` (gains the
`source:` key); in `drift.rs`, the walk tests stay for the set form.

## Landing order

1. **The closure, unused.** In `forge::drift`, a closure of a seed set, built on `import_closure::walk` with each
   `Unit` carrying the reads its load recorded, keyed by section 2, read coherently; `SourceSet::collect` becomes
   the set form over it. The parser records the static `src` child it reads. No generated byte changes; depfiles gain
   the child as a prerequisite. Tests 2, 4, 5, 6, 12, 15 at the library level, and test 3, which on this step
   proves the recorded reads inside the closure before anything depends on it.
2. **The switch, in one landing with the regeneration.** Every route that writes a drift-headed file (`generate`,
   `--with-imports`, `--as-child`, `orchestrate`, `generate-w3c`, `generate-integration`, `generate-conformance`),
   `payload-schema`, `transition-table --header`, the bound digest, the sidecar, the depfile and `verify`; the
   three-line header; §synth-6.2.6 and §synth-5-O amended and the synth mirror regenerated; the module
   documentation of `drift.rs` and the comment of `regen_all_committed_trees.sh`; the three rows of
   `SCE_WIRE_CONTRACTS.md`; the three diagnostic rows. Every committed tree regenerated once (section 11). Tests 1,
   7, 8, 9, 10, 11, 13, 14, 16, 17, and the re-aimed tests.
3. **The measurement, recorded here.** On a branch, one `static_datamodel` fixture edited and every tree
   regenerated; the number of files that move is written into this ADR beside today's 753, with the files named.
   Expected: the fixture's own units and in-line children in each tree, and the units of the documents that import
   or start it.

## Consequences

- An edit to a document re-stamps the files whose closure holds it, in every tree, and no other file. A landing's
  regeneration diff is its real change.
- **Parallel lanes.** Two branches that touch different documents conflict in no generated file unless one document
  is in the closure of the other's files, or both sit under the directory of a set-form file (the W3C aggregator, a
  conformance harness: one header line each, resolved by regenerating). Such a conflict is a real overlap of inputs,
  and the regeneration that resolves it is the one either branch would need anyway.
- **The landing queue** returns an item on a generated file only when the item and the landing before it changed an
  input of that file. The return of 2026-11-08 is of the kind that disappears, unless the renamed union fixture is in
  the closure of a file the returned item regenerated.
- The headers become sound: an import outside the old root, and a fragment that is not `.scxml`, move them.
- Depfiles shrink to what a generation reads; a sibling's edit rebuilds nothing.
- `--input-root` stops bearing on the hash; staging scripts keep passing it for candidates, and the hash of a staged
  copy needs no override.
- The header gains a key, and a consumer that read the equality of headers across a tree as "one run" loses a fact
  the header never stated.
- `verify` takes several roots and walks one closure per seed set; `--deploy`, and the limits when the generation
  moved any, must be given to it as to the generation.
- Every committed tree is re-stamped once.

## Alternatives rejected

- **Drop the hash.** The content judge exists, but the header is what answers without generating whether a file is
  current, what ties a transition-table export, a payload schema or a sidecar kept apart from a build to the machine
  compiled from the same document, and what §synth-6.2.6 promises every emitted file carries. The defect is the input
  set, not the digest.
- **Hash per directory.** It is today's rule. `static_datamodel` is one directory of 231 documents, so every edit
  still re-stamps every file generated from it; moving a document between directories re-stamps files whose content
  did not change; and an import or fragment outside the directory is still missed. A directory is a fact of layout,
  not of input.
- **Keep the run hash and add a per-file hash.** The run line still moves on every sibling's edit, so the churn and
  the conflicts stay, and two digests in one header leave the reader to guess which one `verify` judges. A fact about
  a run belongs on the run's stdout manifest, where `generator` went, not in a committed file.
- **`verify` by membership, without `source:` lines.** `verify` would compute the closure of every document under
  its roots and pass a file whose hash is any of them. It needs no header change and catches a stale file, but it
  judges a file against documents it was not generated from, cannot give a mismatch an `expected` value, cannot
  verify an `orchestrate` file (no root enumerates the subsets a set can be) and cannot say which document to
  regenerate. Open question 1 keeps it on the table.
- **The recorded reads as the closure.** They are known only after generating, so `verify` could not recompute them
  without generating, and they are incomplete today (the static `src` child). They are held to the closure instead
  (test 3).
- **Members keyed by content alone.** A rename of an imported document that changes the name its importer's code
  uses would not move the importer's header.
- **Members keyed by where they were found.** It makes the digest depend on the checkout's location or on a staging
  directory, which is what `--input-root` was added to patch.
- **A closure over the expanded text or the parsed model.** Raw bytes are what an author edits and what every other
  pin here (the acceptance record, the old source set) takes; a pin over a model would rest on the model's
  serialisation being total over behaviour, which nothing checks.

## Open questions for the owner

Each has a recommendation; the sections are written to it.

1. **The `source:` line.** Recommended: add it (section 3), so `verify` recomputes each file's own closure and says
   what to regenerate. The alternative keeps the two-line header and verifies by membership (see "Alternatives
   rejected"), at the cost of `orchestrate` files and of an `expected` value.
2. **A child document a run writes itself** (a Mesh §9.6.6 remote child's `.scxml`, which the integration scripts
   then generate `--as-child` from the staged copy). Recommended: such a file is seeded by the authored document it
   was written from, which requires the written child to record that document if it does not today (its in-line
   provenance, `document#invoke-path`, is the spelling the sourcemap already uses). The alternative seeds it by its
   own bytes, which is sound, since they are a function of the parent's, but leaves `verify` no document under any
   root to find.
3. **`deploy.yaml`.** Recommended: in the closure of every file of a run given one (section 1). The alternative
   puts it only in the closures of the files whose generation reads one of its keys, which needs a record of the
   keys a generation reads that does not exist.
4. **The acceptance record.** Recommended: unchanged in this ADR, held to the closure by test 3. Pinning the closure
   would lapse every record already taken, and a design's acceptance is a person's act, not a build's.
5. **`orchestrate`.** Recommended: every file of a run covers its whole named set (section 4) until the joint compile
   records which documents each output read; then narrowed, with a test that the narrowed closure holds every
   recorded read.
6. **Registries.** The W3C aggregator and the conformance harness are also generated from `fixtures.json`, which no
   closure holds today or under this ADR, so an edit to a registry changes their content and not their header.
   Recommended: leave it to the content judge, as for a template; folding a registry in means `verify` must be told
   where it is.

## Owner's answers

## References

- Dolstra, E. *The Purely Functional Software Deployment Model.* PhD thesis, Utrecht University, 2006.
- Erdweg, S., Lichter, M., Weiel, M. "A Sound and Optimal Incremental Build System with Dynamic Dependencies."
  OOPSLA 2015, pp. 89-106.
- Feldman, S. I. "Make — A Program for Maintaining Computer Programs." Software: Practice and Experience 9(4), 1979,
  pp. 255-265.
- Merkle, R. C. "A Digital Signature Based on a Conventional Encryption Function." CRYPTO '87, LNCS 293, 1988,
  pp. 369-378.
- Mokhov, A., Mitchell, N., Peyton Jones, S. "Build Systems à la Carte." Proceedings of the ACM on Programming
  Languages 2 (ICFP), 2018, article 79; extended as "Build Systems à la Carte: Theory and Practice," Journal of
  Functional Programming 30, 2020, e11.
