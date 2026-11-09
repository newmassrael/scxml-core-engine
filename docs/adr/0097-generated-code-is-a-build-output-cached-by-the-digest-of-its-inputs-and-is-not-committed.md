# ADR 0097 — Generated code is a build output, cached by the digest of its inputs, and is not committed

- Status: Accepted (2026-11-09, owner), with the recommended answer to each of the eight open questions; see
  "Answers (2026-11-09)". The decision reached the implementing session through the session that drafted this ADR.
  Decided under the owner's standing rule: the most long-term correct structure, textbook in its foundations, less
  network, cost ignored. Numbered 0097 because 0094 and 0096 are taken on branches.
- Date: 2026-11-09
- Amends:
  - SCE Protocol-Synthesis RFC §synth-6.2.6 (`docs/spec/synth/rfc-sce-protocol-synthesis.md:4147-4167`): its
    sentence "CI regenerates and diffs" becomes "CI generates twice and compares" (section 5); the header and the
    `manual edits to out/ are forbidden` policy are unchanged;
  - the sourcemap paragraph of the same RFC (`rfc-sce-protocol-synthesis.md:3246-3251`), which says the sourcemap
    "is checked into version control": it travels with its generated library, in a build directory or a released
    package, and is checked into version control only by a consumer that commits its own generated code;
  - `docs/SCE_INTEGRATION_FIXTURE_LAYOUT.md` "Axis 1 — Committed generated tree vs build-time generation"
    (`:1184-1209`): every backend moves to the build-time column;
  - `docs/SCE_CODEGEN_DETERMINISM.md` §8 and §9 (`:309-345`, `:411-414`): the committed Rust trees stop being the one
    committed form that is not the generator's own bytes, because there is no committed form.
- Scope: every tree a step of `scripts/regen_all_committed_trees.sh` owns (`scripts/lib/sce_generated_tree.sh:136-248`),
  the `[actions.regen-*]` of `bx.toml` (`:418-1224`), the `regen-reproduces` gate and workflow, the drift tests that
  read committed trees, the build files of each language lane, and a release pipeline that does not exist yet.
- Related:
  - ADR 0094 (Proposed, on a branch): a generated file's `source-hash` covers the closure of the documents it is
    generated from. Its per-file closure is the input set of the per-generation action of section 2 here; the two
    are complementary and neither needs the other to land first;
  - ADR 0078 (a consumer follows SCE one landing at a time): its consumer-guide gate reads the diff of generated
    code (ADR 0078 lines 73-92), which section 6 keeps readable;
  - ARCHITECTURE.md "Build-Time Plans" (`ARCHITECTURE.md:338-380`): the C++ and C11 routes already generate at build
    time with a plan, a depfile and a cached answer per input state. This ADR makes that the rule for every language.

## Context

### What was measured on 2026-11-09

- **One branch, one regeneration, 974 files.** Commit `3dbf701062` ("chore: Regenerate the trees for the door and
  the list's change") changes 974 files, +14,398 / -62,120 lines (`git show --stat 3dbf701062`): 324 under
  `backends/go`, 276 under `backends/swift`, 188 under `backends/kotlin`, 184 under `backends/rust`, 2 under
  `sce-specs/src`. Its own bullets name the authored change it carries: "The C11 slot and composed lowering, ADR 0085
  4a-4c on every lane" and "The send's run-time count check and the payload scope fix". Not one of the 974 files is
  that change; each is a consequence of it.
- **Every rebase across a generator change conflicted on generated files** and was resolved by a full regeneration of
  about one hour, since a conflict in a generated file can only be resolved by generating it again from the merged
  inputs. The regeneration script itself records "A run of this script has been measured at three hours"
  (`scripts/regen_all_committed_trees.sh:43-47`); split into bx actions it took "341 s on a build machine with
  nothing left to change, 956-1971 s with changes, against 40-60 min on this machine under load"
  (`bx.toml:430-432`).
- **Every bx tree send carried those files** to each build machine, at about 250 KB/s over the private network that joins them.

Measured from the tree at `a9253db549` for this ADR:

- 3,592 tracked files carry a generated header (`source-hash:` or a sourcemap's `"source_hash"`), 25% of the 14,084
  tracked files, and 76 MB of the 221 MB the tracked files (submodules excluded) hold on disk. By tree: 1,248 in
  `backends/rust/tests`, 1,047 in `backends/kotlin/tests`, 808 in `backends/swift/tests`, 388 in `backends/go/tests`,
  26 in `backends/rust/mesh`, 26 in `backends/kotlin/mesh`, 25 in `backends/swift/mesh`, 12 in `sce/src/mesh`, 5 in
  `backends/go/forge-runtime/round_trip`, and one or two in each of `backends/swift/mesh-wss`,
  `backends/kotlin/method-size` and the two `sce-specs` machines.
- From 2026-10-10 to 2026-11-09, `main` took 3,224 non-merge commits with 78,075 file changes and 2,181,255 added
  lines. 54,477 of those file changes (70%) and 1,092,398 of those added lines (50%) fell in the eight directories
  that hold the large generated test trees (Rust `src/generated`, `src/integration`, `tests`; Kotlin
  `com/sce/generated`, `com/sce/integration`; Go `integration`; Swift `StaticDatamodelMachines`,
  `IntegrationMachines`). Those directories
  also hold a few hand-written files, so the numbers are an upper bound; the generated files are nearly all of them.
- 96 files under `sce-build/tests`, `tools/git-hooks` and `scripts/gates` name a committed generated tree by path,
  and 61 of the 525 mutation casefiles under `sce-build/tests/mutations` mutate a committed generated file directly
  (`mutation_targets backends/go/tests/integration/static_datamodel/static_bytes/static_bytes_sm.go`, for one).

### How the trees are made and kept honest today

- **Seventeen steps.** `scripts/regen_all_committed_trees.sh --list` prints the steps `sce_regen_steps` names
  (`scripts/lib/sce_generated_tree.sh:136-155`): `rust-tests`, `kotlin-tests`, `kotlin-method-size`, `go-w3c`,
  `go-integration`, `python-w3c`, `python-integration`, `swift-tests`, `go-round-trip`, `mesh-rust`, `mesh-cpp`,
  `mesh-kotlin`, `mesh-swift`, `mesh-swift-wss`, `specs`, `forge-goldens`, `embed-manifest`. Each owns the trees
  `sce_generated_trees` lists for it (`:177-248`). `go-w3c`, `python-w3c` and `python-integration` write trees that
  are gitignored already; `forge-goldens` and `embed-manifest` own no tree.
- **Seventeen bx actions.** Each step is `[actions.regen-<step>]` in `bx.toml`, with globs for inputs and outputs
  (`bx.toml:503-550` for `rust-tests`, whose inputs are the generator's whole source, every template, the standard
  library, every fixture directory and the regeneration scripts). None is cached: "NO `cache = true` YET, on any of
  them" (`bx.toml:494-502`), and the landing queue's `derive = ["regen"]` "waits for regen to be split into actions bx
  caches; until then each candidate would pay a full regeneration" (`bx.toml:1262-1263`).
- **Per-document scripts.** `scripts/regen_static_datamodel_{rust,go,kotlin,swift}.sh` and some twenty others
  generate one fixture family each and are called by the steps (`scripts/regen_all_committed_trees.sh:77-342`).
- **The header.** Every generated file opens with `SCE-GENERATED — DO NOT EDIT` and `source-hash: <sha256 of sorted
  input SCXML + deploy.yaml>` (`sce-build/src/forge/drift.rs:4-20`, `HEADER_BANNER` at `:63`). The header deliberately
  carries no template hash, generator identity or timestamp, because each re-stamped every committed tree with no byte
  of output changed (`drift.rs:22-36`). So the header cannot see a template or generator change.
- **The content check.** `scripts/gates/regen-reproduces.sh` runs the whole procedure in a worktree of HEAD, after
  deleting a sample of generated files so the run must produce them, and fails on any difference or any committed file
  no longer produced (`regen-reproduces.sh:9-26`, `:70-113`, `:131-151`). Its workflow has no `paths:` filter, because
  its input set is "very nearly the repository" (`.github/workflows/regen-reproduces.yml:19-45`). The registry calls it
  "the one reader of the content axis" (`tools/git-hooks/gate_registry.py:1117-1144`). The per-tree tests
  `b9_drift_detection::verify_passes_on_real_committed_*` recompute each tree's `source-hash`
  (`sce-build/tests/b9_drift_detection.rs:478-752`).
- **The embed manifest.** `embed/MANIFEST.json` and its stamp `embed/MANIFEST.inputs` are the public C++ header
  surface, emitted by clang and committed (`docs/EMBED_MANIFEST.md:1-8`, `:35-45`; the step at
  `scripts/regen_all_committed_trees.sh:426-453`).

### What already generates at build time

The repository already holds every shape this ADR asks for, each in one place:

- **CMake**, for C++ and C11: every generation is a custom command with a plan, a depfile and an answer kept per input
  state (`ARCHITECTURE.md:338-380`); "build-time backends rely on CMake to regenerate on every build, so the build
  process itself is the §6.2.6 freshness invariant" (`docs/SCE_INTEGRATION_FIXTURE_LAYOUT.md:1198-1199`).
- **Cargo build scripts**: `backends/rust/feature-conformance/build.rs` (also `deploy-conformance`,
  `forge-conformance`, `world`) runs the generator in-process into `OUT_DIR`, with `sce-build` as a
  build-dependency (`backends/rust/feature-conformance/build.rs:1-24`, `Cargo.toml:35-36`).
- **Gradle**: `generateScxml` is a task with declared inputs and outputs that runs before `compileKotlin`
  (`backends/kotlin/tests/build.gradle.kts:135-173`); it writes into the committed tree today.
- **Gitignored trees regenerated by their lane**: the Go and Python W3C trees, the Python integration modules, the
  Swift W3C machines, the Swift ECMA-262 machines and the Swift feature-conformance sources (`.gitignore:189-235`).
  The Swift entry states the reason this ADR generalises: "the lane regenerates both before it runs them, so a
  template edit does not rewrite a file per document in git" (`.gitignore:203-205`).
- **A prebuilt generator**: "In CI the conformance jobs download a pre-built artifact into `target/debug` and have no
  Rust toolchain" (`build.gradle.kts:17-19`).

### Why the trees are committed: the reasons the tree states

Searched in ARCHITECTURE.md, `docs/adr/`, `docs/`, the build files and the scripts. The reasons stated:

1. **Builds without the generator.** "Generated files are kept in git so builds work without sce-codegen."
   (`backends/kotlin/tests/build.gradle.kts:51`, quoted again at `.gitignore:190-192` as the reason the Kotlin and
   Rust trees are tracked.)
2. **A C++ consumer without a Rust toolchain.** "The tree is committed rather than generated at build time so a C++
   consumer of the runtime needs no Rust toolchain" (`sce/src/mesh/generate_envelope.sh:18-20`); the same at
   `sce/CMakeLists.txt:130-132` and `sce-build/tests/b9_drift_detection.rs:637-640`.
3. **The embed package**: "No Rust toolchain, no Git, no network required to build from the embed package."
   (`scripts/package_embed.sh:7-8`); its audience is consumers with "no Rust toolchain, no git, no network at
   consumer build time — only a C++17 compiler" (`ARCHITECTURE.md:251`). The embed package does not, however, ship
   the generator, and a consumer that generates puts `sce-codegen` on `PATH` (`ARCHITECTURE.md:292`); it ships no
   committed generated tree either. Its manifest is committed "so that consumers receive the manifest with the
   vendored copy and so CI can drift-guard both against the checked-in versions" (`docs/EMBED_MANIFEST.md:35-38`).
4. **Review of generated output.** Stated as the reason the committed Rust trees are rustfmt's output: without it "a
   reviewer could not tell whitespace churn from a real emitter change"
   (`scripts/regen_all_committed_trees.sh:165-175`); and for derived schemas, "The reviewer inspects the regenerated schema and commits alongside the type change that
   triggered it" (`SCE_FORGE.md:2751`). ADR 0078's gate reads a change to a generated codec golden as a change a
   consumer must be told about (ADR 0078 lines 73-92).
5. **The Mesh libraries as packages**: the Swift Mesh package holds "the standard Mesh documents generated from
   `sce:std/mesh` (Sources/SceMesh/Generated, committed, regenerated by generate.sh and held to its sources by
   sce-build's drift test)" (`backends/swift/mesh/Package.swift:7-9`). It states the fact, not a reason.

What the tree does **not** state, though the request for this ADR named each as a possible reason:

- No document says that a downstream consumer vendors SCE's committed generated trees. The consumer of ADR 0078 "pins
  SCE at a commit and regenerates its codecs from it" (ADR 0078 line 16), and the determinism guide is written "For
  consumers who commit generated code, pin `sce-codegen` as a build dependency"
  (`docs/SCE_CODEGEN_DETERMINISM.md:3`): consumers commit their own generated code, made by the generator, not SCE's
  trees.
- No document says CI needs the committed trees because it has no generator. CI's lanes either build the generator
  (`regen-reproduces.yml:104-107`) or download it (`build.gradle.kts:17-19`).
- No document says Swift, Kotlin or Go must build from committed sources. Each of them already builds some generated
  tree that is not committed (the gitignored trees above).

So the consumers that depend on committed generated code, named by role:

- **The Mesh library consumers** (Rust `sce-rust-mesh`, `publish = false`, `backends/rust/mesh/Cargo.toml:15`; the
  Kotlin Mesh module; the Swift `SceMesh` package; the C++ `sce_mesh_common` target, `sce/CMakeLists.txt:124-160`)
  that take a library by a commit pin and compile its generated part as it stands in the repository.
- **The embed vendor consumer**, which receives `embed/MANIFEST.json` with the vendored copy; it compiles no committed
  generated tree.
- **The codec consumer of ADR 0078**, which runs the generator itself and reads SCE's goldens and migration guide.
- **SCE's own test lanes**, which compile the large test trees; they are by count nearly all of the 3,592 files.

### What is wrong

Committing the output of a deterministic function of committed inputs stores the same information twice and makes the
second copy a source of work:

- every change of the function (a template, the generator) rewrites hundreds of files that carry no authored change;
- two branches that change the function conflict in files nobody wrote, and the only resolution is to run the function
  again on the merged inputs;
- the authored change is hidden among its consequences (the 974 files of `3dbf701062` are all consequences of the
  change its bullets name, and none of them is that change);
- the copy can disagree with the function, so a gate (`regen-reproduces`) exists to regenerate everything and compare,
  at 117 s of CI per push in the registry's figure and up to an hour on a loaded machine;
- the copy is carried everywhere the repository goes: every clone, every bundle, every send to a build machine.

ADR 0094 removes one cause of the rewrite (a fixture edit re-stamping every file of a directory). It cannot remove the
others, because a template or generator change does change the bytes of every file it reaches.

## The textbook structure

Build systems that generate sources at scale treat a generator like a compiler (Bazel and the Remote Execution API,
Buck2, Pants, Gradle's build cache, CMake custom commands, Cargo build scripts):

- A generation is a **declared build action**: a command, the inputs it may read, the outputs it writes, the tool and
  its version. Its **action key** is a digest over all of them (REAPI: the digest of the `Action`, which names the
  `Command` and the Merkle root of the input tree).
- The outputs are stored in a **content-addressed store** (CAS) and the key maps to their digests in an **action
  cache**. A key seen before is never executed again; any machine that shares the cache gets the outputs by digest.
- Every consumer of the outputs, a compile step or a test, **depends on the action**, never on a checked-in copy.
  Freshness is a property of the graph, not of a gate.
- A cache is only sound if the action is **hermetic and deterministic**: it reads only declared inputs, and the same
  inputs give the same bytes. That is checked by running an action in a sandbox and, for determinism, twice.
- Review sees the **authored change**; tools render the generated change from the graph when a reader wants it.
- **Distribution** is a separate step: a release builds packages from the graph and publishes them with their
  provenance (the digests they were made from), the way a library built from interface definitions ships its
  generated code in the package it publishes.

What may still be committed is what is not a build intermediate: test oracles, and snapshots of a published contract
that exist so that a change to the contract is seen in review (an API dump, a schema).

## Decision

### 1. A generated tree is a build output and is not committed

Every tree a regeneration step owns today (`sce_generated_trees`, every step but `forge-goldens` and
`embed-manifest`; the trees of `go-w3c`, `python-w3c` and `python-integration` are gitignored already) leaves version
control. Its files, its sourcemaps, its synthesized-child stubs and the module indexes the regeneration scripts write
are outputs of build actions, written into a build directory of the language that compiles them, and gitignored.
Hand-written files beside them (the Rust harness and `src/integration/mod.rs`, the Go `*_test.go`, the Kotlin W3C base
classes, the other files of `backends/rust/tests/tests` that are not one of the 202 generated `test_*.rs`) stay
committed where they are.

### 2. One declared action per generation

- **The unit is one generator invocation**, one `(document or document set, language, flags)`: the unit
  `sce-codegen --plan` already describes and CMake already declares one command for (`ARCHITECTURE.md:340-347`). A tree
  is a named set of actions plus the build directory they are materialized into. Fine actions are what make a cache
  useful: a fixture edit reruns one document's actions, a Kotlin template edit reruns the Kotlin actions only.
- **One manifest declares them**, a repository file read by every route below (name and schema are landing 0's work;
  question 2). It replaces the step functions of `scripts/regen_all_committed_trees.sh`, the per-document regen
  scripts and the input globs of `bx.toml`'s regen actions, which are derived from it, never written beside it.
- **Inputs.** An action's inputs are:
  - the generator, identified by the digest of its source closure (`sce-build/**`, `Cargo.lock`, the toolchain
    version, the enabled features) rather than of its binary (question 1);
  - the templates its language reads, the standard library documents and schemas the generator compiles in or reads;
  - the input closure of the document: the documents it imports, starts or reads and their `deploy.yaml`, which ADR
    0094 defines and which `--reads` (named `--plan-reads` until landing 0 let a real run state it too) and the
    depfile already list (`ARCHITECTURE.md:370-376`);
  - the flags, the language, and the formatter when one runs (clang-format major 19 for C++,
    `docs/SCE_CODEGEN_DETERMINISM.md:361-372`).
  The input set of a document is computed by the generator's plan, not written as globs: a glob list "enumerates the
  trees that exist today and by construction cannot name the one whose arrival it exists to catch"
  (`regen-reproduces.yml:26-28`). A computed input set is sound only if it is complete, so the action runs sandboxed
  and a read outside its declared set fails the action (bx's traced run, RFC 0003 section 9 as `bx.toml:494-496`
  describes it, is that check).
- **Outputs** are the files the plan names. An action writes nothing else; a file the generator no longer writes is
  simply absent from the next materialization, so the stash-and-settle machinery that removes stale committed files
  (`scripts/lib/sce_generated_tree.sh:19-35`, `:258-338`) has nothing left to do for these trees.
- **Rustfmt is no longer part of the output.** The committed Rust trees are formatted after generation so that
  `fmt-check` passes over the workspace (`scripts/regen_all_committed_trees.sh:165-182`). A file in a build directory is
  not under `fmt-check`, so the Rust output becomes the generator's own bytes, as every other language's already is
  (`docs/SCE_CODEGEN_DETERMINISM.md:411-414`).

### 3. A content-addressed action cache, local and fleet

- **Two tables**: the action cache, from action key to the list of `(path, digest)` it produced; and the CAS, from
  digest to bytes. The layout is REAPI's, so any compliant server can hold the fleet copy (question 3).
- **Local**: under the user's cache directory, shared by every checkout and worktree of the machine, so a worktree
  made for a branch gets every output its base already produced without generating.
- **Fleet**: on the build machines, written only by CI and by the build machines' own runs, read by everyone. A
  developer machine never writes to it, so a local defect cannot poison another machine.
- **A miss** runs the action (in-process for Rust, through `sce-codegen` otherwise), checks it, stores it locally,
  and uploads it when the caller may write.
- **A hit** needs no generator: the key is computed from input digests, and the generator is identified by its
  source digest. A Kotlin-only or Swift-only contributor with a warm fleet cache builds without ever building the
  generator.

### 4. Every language build obtains its generated sources from the action

One entry point materializes a tree into a directory: `sce_build::action::materialize(tree, into)` for a build script,
and `sce-codegen materialize <tree> --into <dir>` for everything else. No build file holds generation logic of its
own; each calls this and declares the result as its sources.

| Language | Hook | Build directory | Precedent in the tree |
|---|---|---|---|
| Rust | `build.rs`, `sce-build` as a build-dependency | `OUT_DIR`, each file wrapped in a `pub mod` and `include!`d | `backends/rust/feature-conformance/build.rs:22-24` |
| Kotlin | a Gradle task with declared inputs and outputs, `@CacheableTask`, its output added as a `srcDir` | `build/generated/sce/<tree>` | `backends/kotlin/tests/build.gradle.kts:135-173` |
| Go | `go generate` before `go test`, the directive calling `sce-codegen materialize` | a gitignored directory inside the module, since Go imports only from the module | `.gitignore:189-195` (the Go W3C tree) |
| Swift | a SwiftPM build tool plugin running a binary target of `sce-codegen`; until that binary is published, the lane's pre-step | the plugin work directory | `backends/swift/tests/Package.swift:8-12` (the W3C machines) |
| C++, C11 | `sce_codegen_generation`, a custom command with a depfile | the CMake binary directory | `ARCHITECTURE.md:338-380` |
| Python | a build backend hook for a package that ships generated modules; the lane's pre-step for tests | the wheel's build directory; the gitignored test tree | `.gitignore:218-235` |

Two shapes change with the move and are named here so they are not discovered late:

- **Rust test targets.** The 202 generated `backends/rust/tests/tests/test_*.rs` are each a Cargo test target, and
  Cargo discovers a target only as a file in `tests/`. They become modules of one target that includes them from
  `OUT_DIR`, so `cargo test -p sce-rust-tests --test test_144` becomes a filter on that target. Every gate, workflow
  and casefile that names a per-test target is re-aimed in the same landing.
- **Mutation casefiles.** The 61 casefiles that mutate a committed generated file name it by its repository path. The
  mutation runner materializes the tree first and resolves a target relative to the tree, so a casefile names
  `<tree>:<path>` (question 7).

### 5. The drift check becomes a determinism check

With nothing committed there is nothing to drift: a build compiles exactly what the action produces from the inputs
of its commit. What can still go wrong is an action that is not a function of its declared inputs, and the cache
would spread that. So:

- the gate `generated-deterministic` replaces `regen-reproduces`. For every action whose key the change moves (and a
  rotating sample of the others), it runs the action twice, in two clean sandboxes with different paths, and on two
  machines when the fleet is reachable, and fails if the outputs differ in any byte; and it fails if a fleet entry for
  the same key holds different bytes;
- the `b9_drift_detection::verify_passes_on_real_committed_*` tests read the materialized trees: the `source-hash` each
  file carries still matches its inputs, which is the §synth-6.2.6 contract a consumer relies on;
- `--assert-unchanged` and `verify` stay as they are, for consumers that commit their own generated code
  (`docs/SCE_CODEGEN_DETERMINISM.md:309-345`).

### 6. Review sees the generated diff when it wants it

- A CI job `generated-diff` materializes every tree at the change's merge base and at its head (cache hits, except
  where the change moved a key), and publishes the difference as a build artifact (a patch, per tree) and a short
  comment on the change: per tree, the files changed and the lines added and removed, then the first hunks of each
  tree.
- The landing queue renders the same diff for each item, beside its verdict.
- `sce-codegen materialize --diff <base> <head> [tree...]` renders it locally.
- ADR 0078's consumer-guide gate reads the rendered diff of the Mesh library trees where it read `git diff`; for the
  goldens it reads `git diff`, since the goldens stay committed (section 8).

### 7. Consumers get generated code from released packages

- A **release pipeline** builds, from one commit of `main`, the packages a consumer takes, with their generated code
  materialized inside them: the Rust Mesh crate in a Cargo registry, the Kotlin Mesh module as a Maven publication
  (the shape `backends/kotlin/rhino` and `spring-boot-starter` already publish), the Swift Mesh package, the C++ Mesh
  envelope in a source package of the embed kind, and the embed package itself.
- Each package carries **`generated-provenance.json`**: for each tree, the action keys, the digests of their inputs
  (the generator's source closure, the templates, the documents' closures) and the digest of the output tree. Anyone
  with the commit can rebuild the tree and compare digests.
- For Go and SwiftPM, which resolve source by a VCS tag, the pipeline writes a **release commit**: a child of the
  `main` commit that adds the materialized trees, tagged, on a release ref, never merged back (question 4).
- A consumer that **pins a commit** keeps working: the Rust crate builds its generated part in its build script with
  `sce-build` as a build-dependency, and the other languages call the generator, which the consumer then needs. The
  consumer that cannot take a toolchain takes a release. This removes reason 2 of "Context" for every consumer that
  takes a release, and keeps it as a cost for one that takes a commit (question 6).

### 8. What stays committed, and the line

A file is committed when a person or a test reads it as **the statement** of what must be true; it is a build output
when a compiler or interpreter reads it as **source** of a target. Applied:

| Stays committed | Why |
|---|---|
| `tests/forge/expected/*` (1,019 files, 8.3 MB) | Test oracles: `forge_conformance` compares fresh generation against them, and ADR 0078's gate reads a changed public item in them as a consumer-visible change. Changing one is a deliberate act (`UPDATE_GOLDEN=1`), reviewed as such. |
| `embed/MANIFEST.json`, `embed/MANIFEST.inputs` | A snapshot of a published contract (the public C++ header surface), committed so that a change to the surface shows in review and reaches the vendoring consumer (`docs/EMBED_MANIFEST.md:1-8`, `:35-38`); the textbook API dump. Its stamp records what it was emitted from and is judged with it. |
| `schemas/`, `apis/`, `tests/forge/conformance/fixtures.schema.json` and every wire-contract file of `SCE_WIRE_CONTRACTS.md` | Contracts a reader outside the code reads by path, versioned with their status. |
| Hand-written oracles and scenarios (`tests/forge/conformance/numerical_reference.json`, the replay scenarios, the fixtures) | Inputs, not outputs. |
| Hand-written files beside a generated tree | Not generated. |

| Leaves | Why |
|---|---|
| Every tree of the steps `rust-tests`, `kotlin-tests`, `kotlin-method-size`, `go-integration`, `swift-tests`, `go-round-trip`, `mesh-rust`, `mesh-cpp`, `mesh-kotlin`, `mesh-swift`, `mesh-swift-wss`, `specs` (and `go-w3c`, `python-w3c`, `python-integration`, gitignored already) | Read by a compiler as source. |
| The sourcemaps, payload schema instances, transition tables and stubs inside those trees | Outputs of the same actions. Their schemas stay committed (row above). |

A file that is both, an oracle and compiled, is committed as the oracle; the compiled copy comes from the action. A
golden that no test reads is neither and is deleted, as the sixteen `inline_*` goldens were
(`scripts/regen_all_committed_trees.sh:416-420`).

## Migration in landings

One family at a time, Rust first as the reference. Each landing keeps every lane green and names its rollback. A
landing is judged by the lanes it touches running the same tests with the same counts before and after.

0. **The action layer; nothing moves.** The manifest of section 2, `materialize` and the local cache, the
   `generated-deterministic` gate run beside `regen-reproduces`, and the `generated-diff` job. One bridge test: for
   every tree, the materialized output equals the committed tree byte for byte, which is `regen-reproduces` expressed
   through the actions and proves the manifest declares what the scripts do. The fleet cache is set up and CI writes
   to it. *Rollback*: revert; nothing reads the layer yet.
1. **The Rust test trees** (`rust-tests`: `src/generated`, `src/integration`, `tests/test_*.rs`, 1,248 files). The
   crate gets a `build.rs`; the W3C tests become one target; the trees are deleted and gitignored; the step and its bx
   action are removed; the gates, workflows, drift tests and casefiles that name the Rust trees are re-aimed. *Green*:
   the Rust lane's test count is unchanged. *Rollback*: revert the landing. Landing 0's bridge test proved the action
   produces the deleted tree, so the revert restores current bytes, or bytes one `materialize --into` away when a
   generator change landed after it.
2. **The Kotlin test trees** (`kotlin-tests`, `kotlin-method-size`). `generateScxml` writes into `build/generated` and
   becomes cacheable. *Rollback*: revert.
3. **The Go trees** (`go-integration`, `go-round-trip`), as the Go W3C tree already is. *Rollback*: revert.
4. **The Swift test trees** (`swift-tests`), by the lane's pre-step first, as the W3C machines are, then by the build
   tool plugin once a `sce-codegen` binary target exists. *Rollback*: revert.
5. **The specs machines** (`specs`), by the Rust pattern of landing 1. *Rollback*: revert.
6. **The release pipeline** of section 7, with provenance, before any consumer-facing tree moves. A section of
   `docs/SCE_CODEC_CONSUMER_MIGRATION.md` tells the Mesh consumers how to move from a commit pin to a release or to a
   commit pin with the generator (ADR 0078 section 1). *Rollback*: the pipeline publishes nothing a consumer already
   takes.
7. **The Mesh libraries** (`mesh-rust`, `mesh-kotlin`, `mesh-swift`, `mesh-swift-wss`, `mesh-cpp`). The Rust crate by
   `build.rs`, Kotlin by its Gradle task, Swift by its plugin, the C++ envelope by `sce_codegen_generation` in
   `sce/CMakeLists.txt`. The consumer-guide gate reads the rendered diff. *Rollback*: revert; consumers on a release
   are not affected, consumers on a commit pin are told in the migration guide.
8. **Retirement.** `regen-reproduces` and its workflow, the `[actions.regen-*]` whose steps are gone, the stash and
   settle of `sce_generated_tree.sh` for those trees, `[land] derive = ["regen"]`. What remains of
   `scripts/regen_all_committed_trees.sh` regenerates the goldens and the embed manifest only, and is renamed for it.

## Consequences

Better:

- A change's diff is its authored change. Measured, that removes most of `main`'s file changes of the last month (70%
  of them fell in the generated test trees).
- A rebase across a generator change no longer conflicts on generated files and needs no regeneration: the build
  regenerates what the merged inputs require, from cache where it can.
- A bx send and every clone carry 3,592 fewer files and 76 MB less in the tree; the history stops growing by the
  generated churn.
- The landing queue judges a candidate without a full regeneration (`bx.toml:1262-1263`).
- `regen-reproduces` (117 s per push, `gate_registry.py:1135`, up to an hour under load) gives way to a determinism
  check whose work is proportional to the keys a change moves.
- A stale generated file cannot exist, so the class of defect `sce_generated_tree.sh:9-18` records (files the
  generator no longer writes, left committed for days) is gone rather than gated.
- The Rust output is the generator's own bytes, as every other language's is.

Worse, and what pays for each:

- **The first build needs the generator, or a cache hit.** A cold Rust build compiles `sce-build` as a
  build-dependency, minutes on a laptop. A Kotlin, Swift or Go build needs `sce-codegen` on the machine; with a warm
  fleet cache it needs none, since the key is computed from digests (section 3).
- **Toolchain bootstrap.** A contributor to one language lane can no longer build that lane from a checkout with
  only that language's toolchain and a cold cache. The release pipeline publishes a prebuilt `sce-codegen` per
  platform, which the locators already resolve (`gradle/sce-codegen.gradle.kts`, `cmake/SCEFindCodegen.cmake`,
  `scripts/lib/sce_codegen.sh`).
- **Offline builds.** A machine that has built once builds offline from its local cache. A fresh clone offline,
  with no generator and no cache, cannot build the generated lanes; before this decision it could build the
  committed ones.
- **IDE indexing.** Until the first build, an IDE sees unresolved references to generated types. rust-analyzer runs
  build scripts, IntelliJ indexes a Gradle `srcDir` after its task runs, SourceKit-LSP sees plugin outputs after a
  build, and a Go IDE sees the gitignored directory after `go generate`. Each needs one build first.
- **Reading generated code.** It is no longer on the forge's web view or under `git grep`. It is in the
  `generated-diff` artifact, in a build directory, or one `materialize --into` away. History before the move keeps it.
- **A cache is a trust surface.** A wrong entry would spread to every machine. The fleet cache is written only by CI
  and the build machines, the determinism gate compares entries with fresh runs, and `generated-provenance.json`
  lets a consumer check a package against a rebuild.
- **A commit-pinned consumer needs the generator** for the languages other than Rust, where it did not before
  (section 7, question 6).
- **The migration touches 96 tests, gates and casefiles that name the trees by path**, and the names of the Rust W3C
  test targets.

## Alternatives rejected

1. **Keep committing, but compress.** Git already stores blobs compressed and deltified in packs; the cost is not
   bytes at rest. A compressed tree still changes on every generator change, still conflicts on every rebase across
   one (now in an opaque blob that cannot be merged at all), still needs the full regeneration to resolve, still hides
   the authored change, and review loses the text it had. It treats the size and none of the causes.
2. **Commit only a manifest of output hashes.** One file per tree instead of hundreds, so a rebase conflicts in one
   file instead of 974. But the manifest still moves on every generator change, its conflict is still resolved only
   by regenerating, the author still regenerates before every commit to update it, a build still needs the generator
   to produce the bytes the manifest describes, and a consumer gets nothing it can compile. It is the action cache of
   section 3 committed into the repository, keyed by commit instead of by input digest, without the cache.
3. **Git LFS.** It moves the bytes to a second store and keeps them committed: every regeneration still rewrites the
   pointers, rebases still conflict on them and still need a regeneration, review sees pointer diffs instead of code,
   every clone and every build machine needs LFS and its server, the forge's LFS bandwidth is metered, and a consumer
   pinning a commit needs LFS too. It is designed for large binary assets that are inputs, not for outputs of a
   function of the repository.
4. **A separate repository of generated code.** The two repositories must move in lockstep, so every change becomes
   two commits and a pin, and the pin conflicts on every rebase across a generator change. Atomicity of a change is
   lost, every regeneration still happens on the author's side, and the generated code is still committed, only
   elsewhere. The release commits of section 7 differ: they are written only by the release pipeline, from an output
   it has verified, and nothing is ever developed on them.
5. **Keep committing, and let the landing queue regenerate.** `[land] derive = ["regen"]` would make the integrator
   regenerate each candidate on `main` after the rebase. It removes the author's rebase regeneration, but branches
   still conflict with each other before landing, still carry generated diffs for review and tests, every send and
   clone still carries the trees, and `main` gains a regeneration commit written by a tool for every landing that
   touches the generator. It moves the work and keeps all of it.

## Open questions for the owner

Each has a recommendation; the sections are written to it. The owner accepted each recommendation; see "Answers
(2026-11-09)".

1. **How the generator is identified in an action key.** Recommended: by the digest of its source closure
   (`sce-build/**`, `Cargo.lock`, the rustc version, the features), so that a cache hit needs no generator built and
   a key does not depend on rustc producing identical binaries on every host. The alternative, the binary's digest,
   is exact but makes every host's build its own key unless the generator's build is itself reproducible.
2. **Where the action manifest lives.** Recommended: a repository file the generator reads (`sce-build` owns its
   schema), from which `bx.toml`'s actions and every build hook are derived, so that a laptop builds without bx and the
   fleet runs the same actions. The alternative keeps `bx.toml` as the source and makes bx a dependency of every build.
3. **What serves the fleet cache.** Recommended: an REAPI-compatible cache server on the build machines, with the
   layout of section 3, used by both `materialize` and bx, so that a standard server can replace either side. The
   alternative is bx's own action cache alone, which ties every language build to bx's protocol.
4. **How Go and SwiftPM consumers get a release.** Recommended: release commits on a release ref, tagged, written only
   by the pipeline (section 7), since both resolve source by tag; a Swift package registry later if one is adopted.
   The alternative is a Go module proxy and a Swift package registry from the start.
5. **The forge goldens.** Recommended: keep the 1,019 committed, as oracles (section 8). Revisit only if their
   regeneration becomes the next measured cost, by narrowing them to the public items ADR 0078's gate reads.
6. **A commit-pinned Mesh consumer.** Recommended: support both routes, a release for a consumer without a toolchain
   and a commit pin that runs the generator (in-process for Rust), and say so in the migration guide before landing 7.
   The alternative, releases only, would drop the commit pin ADR 0078's consumer uses.
7. **Mutation casefiles that mutate generated output.** Recommended: keep them, naming the target as `<tree>:<path>`,
   which the runner resolves after materializing; mutating the generated output is what proves a test reads it. The
   alternative re-aims each to the template line that writes it, which tests the template and loses the direct check.
8. **The order.** Recommended: Rust test trees first (largest, no consumer), the Mesh libraries last, after the
   release pipeline. The alternative, the Mesh libraries first, would meet the consumer cost before the reference
   pattern is proven.

## Answers (2026-11-09)

The owner accepted the recommended answer to every question. Restated as decisions:

1. **The generator is identified in an action key by the digest of its source closure**: the files
   `sce-build/src/generator_witness.rs` already names (`WITNESS_FILES` and `WITNESS_TREES`: `Cargo.lock`,
   `sce-build/Cargo.toml`, `sce-build/build.rs`, `sce-build/src`, `stdlib`), with the rustc version and the enabled
   features beside them. Not by its binary.
2. **The action manifest is a repository file the generator reads**, whose schema `sce-build` owns. `bx.toml`'s
   regeneration actions and every language build hook are derived from it, never written beside it.
3. **The fleet cache is REAPI-compatible, on the build machines, and used by both `materialize` and bx.** bx RFC 0049
   (proposed 2026-11-09: a content leaves the workstation once and machines take it from their site) makes each
   site's REAPI CAS, the one bx already runs for RFC 0001 and RFC 0044, the one store every send goes through. The
   action cache of section 3 is the REAPI Action Cache beside that CAS, so the fleet holds one cache, not two: an
   action key is an REAPI `Action` digest, an entry is an REAPI `ActionResult`, and bx and `materialize` read and
   write the same entries. Settled with bx on 2026-11-09 and written as RFC 0049 section 4.11: the entries live under
   the project's own instance, named in the site registry (`$(bx-paths sites)`), so sources and generated outputs
   share one content store; only the site's build machines and the landing integrator write, through the worker port
   (a machine certificate where the site runs a CA), and a workstation only reads; a workstation outside the site
   reads through the ssh forward `bx-fleet site-tunnel <site>` opens, never through bx's upload relay. The client is a
   plain REAPI v2 client with SHA-256 digests and no dependency on bx.
4. **Go and SwiftPM releases come from tagged release refs written only by the release pipeline** (section 7).
5. **The 1,019 forge goldens stay committed as oracles** (section 8).
6. **A commit-pinned Mesh consumer stays supported**, by running the generator (in-process for Rust); the migration
   guide says so before landing 7.
7. **Mutation casefiles that mutate generated output stay**, naming their target as `<tree>:<path>`, which the runner
   resolves after materializing the tree.
8. **The order is that of "Migration in landings"**: the Rust test trees first, the Mesh libraries last, after the
   release pipeline.

## References

- `scripts/regen_all_committed_trees.sh`, `scripts/lib/sce_generated_tree.sh`, `bx.toml` `[actions.regen-*]` and
  `[land]`.
- `scripts/gates/regen-reproduces.sh`, `.github/workflows/regen-reproduces.yml`, `tools/git-hooks/gate_registry.py`
  (`regen-reproduces`), `sce-build/tests/b9_drift_detection.rs`.
- `sce-build/src/forge/drift.rs`, `docs/SCE_CODEGEN_DETERMINISM.md`, SCE Protocol-Synthesis RFC §synth-6.2.6.
- `docs/EMBED_MANIFEST.md`, `ARCHITECTURE.md` "Distribution Models" and "Build-Time Plans",
  `docs/SCE_INTEGRATION_FIXTURE_LAYOUT.md` "Two architectural axes".
- ADR 0078, ADR 0094 (Proposed).
- Bazel Remote Execution API (the `Action`, `ActionResult`, Action Cache and CAS); Gradle build cache; Cargo build
  scripts; SwiftPM build tool plugins; `go generate`.
