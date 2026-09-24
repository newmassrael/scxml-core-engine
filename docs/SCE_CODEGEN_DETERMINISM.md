# Reproducible `sce-codegen` output

For consumers who commit generated code, pin `sce-codegen` as a build
dependency, or gate CI on "regenerate and expect no diff".

The guarantee this document describes is: **a given `sce-codegen` build,
given the same inputs and the flags below, writes the same bytes** — on
another machine, from another directory, on another day.

Everything here is enforced by tests, named per section. If a claim below
is wrong, one of those tests is failing.

---

## 1. The short version

```bash
sce-codegen generate path/to/machine.scxml \
    --source-root "$(git rev-parse --show-toplevel)" \
    -o generated/ -l cpp
```

- `--source-root` pins the `// From:` provenance path.

Without it the output is still *correct*, and still byte-identical from
one run to the next, but the provenance path is spelled exactly as you
named the input — an absolute path stays absolute, and resolves only on the
machine that produced it.

---

## 2. What each generated file carries

Every emitted source file starts with the §synth-6.2.6 drift header:

```
// SCE-GENERATED — DO NOT EDIT
// source-hash: <sha256 over the input source set>
```

followed by a license block carrying `// From: <input path>`.

The `source-hash` is a function of the inputs (§5). The provenance path is
a function of *how you invoked the generator*, and is what `--source-root`
controls (§4). Nothing in the header depends on when the generator ran, or
on which build of it ran.

---

## 3. What the header does not carry

- **No timestamp.** Two runs over the same inputs write the same bytes with
  nothing pinned — no `SOURCE_DATE_EPOCH`, no clock. The header carried a
  `generated-at` stamp until 2026-09; it read the wall clock unless the
  variable pinned it, and a regeneration that forgot the pin rewrote every
  file it touched.
- **No template or generator hash.** Until 2026-09 the header also carried
  a `template-hash` over the template tree and `Cargo.lock`. It was wrong
  in both directions: an edit to the lock or to any template re-stamped
  every committed file with no byte of output changed, while an edit to
  the generator's own code changed output without moving it. Whether a file
  holds what the current templates and generator produce is judged on its
  content instead — `--assert-unchanged` (§8) for one invocation, and
  regenerating and diffing for a whole pipeline:

  ```bash
  sce-codegen generate src/machine.scxml -o generated/ -l cpp
  git diff --exit-code generated/     # fails iff generation actually moved
  ```

- **No generator identity.** That is a fact about a run, not a file, and
  the run's stdout manifest carries it (§7). Stamped into a committed file
  it would change with every generator commit, whether the output did or
  not.

A file generated before the change still carries the two retired lines;
regenerating it writes the current header in place of the old one.

*Tests:* `regeneration_is_byte_stable_with_nothing_pinned`
(`sce-build/tests/codegen_invocation_determinism.rs`);
`committed_generated_files_carry_the_current_header_shape`,
`verify_reads_a_header_of_the_older_shape`
(`sce-build/tests/b9_drift_detection.rs`);
`prepend_header_replaces_a_header_of_the_older_shape`
(`sce-build/src/forge/drift.rs`).

---

## 4. `// From:` — pin it with `--source-root`

The provenance path is **never** relative to the process working
directory. Two shapes are available:

| invocation | emitted `// From:` |
|---|---|
| `sce-codegen generate src/m.scxml …` | `src/m.scxml` |
| `sce-codegen generate /abs/repo/src/m.scxml …` | `/abs/repo/src/m.scxml` |
| `… /abs/repo/src/m.scxml --source-root /abs/repo` | `src/m.scxml` |

- **Without `--source-root`** the path is emitted exactly as named on the
  command line. Deterministic, but only as portable as the string you
  passed — an absolute path stays absolute.
- **With `--source-root`** an input under that root is re-expressed
  relative to it. This is what you want for committed output: a path that
  still means something on a machine that never ran the generator.
- An input *outside* the named root falls back to the path as given,
  rather than emitting a `../../..` chain that only resolves locally.

*Tests:* `from_line_does_not_vary_with_working_directory`,
`from_line_is_the_path_as_given`,
`source_root_makes_the_from_line_relative_to_it`,
`source_root_falls_back_when_input_is_outside_it`
(`sce-build/tests/codegen_invocation_determinism.rs`).

---

## 5. `source-hash` — what is actually hashed

The source set is **every `*.scxml` under an input root**, recursively,
plus `deploy.yaml` when one is passed. Not just the document you named.

The root defaults to the parent directory of the input document. Override
it with `--input-root`:

```bash
sce-codegen generate stage/machine.scxml --input-root src/scxml -o out/ -l cpp
```

Two consequences worth planning around:

- **The hash is sensitive to neighbours.** Adding an unrelated `.scxml`
  beside your input changes the embedded `source-hash`. If your build
  generates from a sandbox holding one declared input while a developer
  generates from a source tree holding five, the two produce different
  hashes for the same document. Pass `--input-root` explicitly on both
  paths so they agree.
- **Symlinked inputs are followed.** Build sandboxes (Bazel execroot, Nix,
  staged CMake inputs) materialise declared inputs as links into the real
  tree; those are resolved and hashed as the files they point at.
  Symlinked directories are followed too.
- **A directory reachable under several names contributes under each of
  them.** The set is keyed by root-relative path, so two links naming one
  directory are two sets of paths, not a duplicate to collapse. Removing
  one of them therefore moves the hash — which is the point: it changed the
  input set. This repository's own tree relies on it (`resources/403a`,
  `403b` and `403c` all name `resources/403`).
- **A link onto a directory already being descended contributes nothing.**
  Every file it reaches is one the walk is already collecting, reachable
  under unboundedly many spellings; cutting there is what terminates a
  cycle. That is the only case in which a resolved directory is skipped,
  and it is decided by the link's target, never by the order the
  filesystem lists entries in.

### The empty-set refusal

The fold is total over whatever the walk collected, so a walk that
collected *nothing* still produces a well-formed 64-hex digest — sha256 of
the empty input, `e3b0c442…`. On the wire that is indistinguishable from a
successful hash, which makes the header unauditable rather than merely
wrong.

`sce-codegen` therefore refuses to emit rather than embed it:

```
forge/source-hash-input-uncovered
src/scxml/m.scxml: §6.2.6 source-hash would not describe it —
0 file(s) collected from src/scxml; pass --input-root <DIR> containing the input
```

Exit code 20; nothing is written. Two ways to trigger it:

- the collected set is empty — always refused;
- the root was *inferred* from the input's own location, yet the input is
  absent from the set. A root you named with `--input-root` is treated as
  an assertion about where the sources live, not an inference to
  second-guess, so generating from a staged derivative of a tracked source
  is allowed (this repository's fixture regen scripts rely on it). The
  empty-set floor still applies.

### The enumeration ceiling

Because a directory link naming a sibling contributes under every name that
reaches it, nested levels of such links name a number of root-relative
paths exponential in the depth — n levels of k links each name k^n paths.
All of them are inputs under the rule above, so there is no subset the
walk could legitimately settle for.

The walk therefore carries a ceiling on directories descended and refuses
when it is reached:

```
forge/source-hash-walk-unbounded
src/scxml: §6.2.6 source set exceeds 1000000 directories — a directory
symlink reaching a sibling multiplies the paths under it; re-point
--input-root at a tree without the aliasing, or remove it
```

Exit code 20; nothing is written. This is a liveness bound, not a size
policy: the ceiling only has to sit above every real source tree and stay
finite, and it is reachable by link multiplication and by nothing else —
the widest input root in this repository holds 201 directories. The message
names the ceiling rather than how far the traversal got, so one tree
produces one record on every machine.

A link resolving to a directory already on the current descent path is a
cycle, not aliasing: it is skipped without being descended, so it costs
nothing against the ceiling and a cyclic layout never approaches the bound.

*Tests:* `generate_refuses_when_the_source_set_is_empty`,
`generate_allows_a_staged_derivative_under_a_declared_root`,
`generate_hashes_a_symlinked_input_rather_than_the_empty_digest`
(`sce-build/tests/b9_drift_detection.rs`);
`source_hash_follows_symlinked_scxml_file`,
`source_hash_follows_symlinked_directory`,
`source_hash_counts_every_alias_of_one_directory`,
`source_hash_of_aliased_directories_ignores_creation_order`,
`source_hash_terminates_on_symlink_cycle`,
`source_hash_terminates_on_a_link_back_to_the_root`,
`source_hash_refuses_a_tree_that_exceeds_the_descent_ceiling`,
`the_descent_ceiling_does_not_fire_on_an_ordinary_tree`
(`sce-build/src/forge/drift.rs`).

---

## 6. `--workspace-root` — which checkout judges the binary

`sce-codegen verify-generator` answers whether this binary was built from
the sources of a given checkout, and with no `--root` it resolves that
checkout in this order: `--workspace-root` → `$SCE_WORKSPACE_ROOT` →
`CARGO_MANIFEST_DIR/..` → walk up from the working directory. A vendored
or relocated build should pass `--workspace-root` rather than rely on the
walk.

The flag no longer affects generated output: it located the tree the
header's `template-hash` was computed over, and that line is retired (§3).

*Tests:* `workspace_root_explicit_flag_resolves_the_workspace`,
`workspace_root_env_var_resolves_the_workspace`,
`workspace_root_invalid_explicit_flag_warns_and_falls_through`,
`workspace_root_compile_time_fallback_resolves_for_vendored_layout`
(`sce-build/tests/cli_meta_and_workspace_root.rs`).

---

## 7. Which generator produced this artifact

The crate version is frozen at `0.1.0` pre-1.0 and identifies nothing.
The commit is the field to record:

```console
$ sce-codegen --version
sce-codegen 0.1.0 (619ff5be8834)
```

It is also in the stdout manifest of every `generate` run, so a build
system capturing that JSON attributes its output with no extra
invocation:

```json
{"v":1,"kind":"generate","generator":"619ff5be8834","artifacts":[…]}
```

If you keep a version sidecar next to committed output, derive it from
one of these rather than maintaining it by hand — a hand-maintained
record drifts silently, and a tree whose recorded generator no longer
reproduces it is a tree nobody can audit.

`"unknown"` appears when the generator was built without a git checkout
to read (vendored crate, release tarball). The value names the *committed*
state the generator was built from; uncommitted edits to the generator
itself are not reflected, which is why whether a tree is current is judged
on its bytes (§8) rather than trusted from the binary's identity.

*Tests:* `version_reports_the_generator_commit`,
`manifest_generator_matches_version_output`
(`sce-build/tests/error_format_json.rs`).

---

For `orchestrate`, the inferred root is the first input's parent directory.
Every `--scxml` and `--forge` input must be covered by that root; otherwise
code generation refuses with `forge/source-hash-input-uncovered`. For a
batch spanning directories, pass `--input-root` naming their shared source
root, and use the same root for `verify`. An explicit root also supports
staged derivatives, with the same nonempty-source requirement as `generate`.

## 8. Verifying after the fact

```bash
sce-codegen generate path/to/machine.scxml \
    --source-root "$(git rev-parse --show-toplevel)" \
    -o generated/ -l cpp --assert-unchanged
```

`--assert-unchanged` runs the same generation, writes nothing, and fails
if the tree on disk is not the one that generation would leave: a file it
would write that differs or is absent, or a file it would remove that is
still there — `forge/generated-output-changed`, exit 20, naming every such
file. Pass exactly the arguments and environment that produced the tree:
the check is whether *this* invocation would change anything. It is the
gate to use when you commit generated output. Because it compares content,
it catches a hand edit to a generated file, a changed template and a changed
generator, as well as changed inputs. `generate-integration`, whose stems
are shell pipelines outside the process, refuses the flag, and so do
`generate-w3c --clean` and `--list`, which generate nothing.

The run is the whole generation, only held in memory: a file it writes and
then reads back — the Rust suite extends each test's `mod.rs` that way —
is read from what the run produced, and every file is judged once, on what
it would finally hold.

It compares what the generator writes, so a tree you post-process after
generating cannot pass it: the post-processed bytes are not the generator's.
This repository's committed Rust trees are one — they are generator output
*as rustfmt leaves it* (`scripts/regen_all_committed_trees.sh` runs
`cargo fmt -p sce-rust-tests` after generating) — and so is anything a
formatter rewrites in place. Such trees are checked by regenerating the
whole pipeline and diffing, which is what `scripts/gate regen-reproduces`
does.

```bash
sce-codegen verify generated/ --input-root src/scxml
```

Recomputes the `source-hash` from the current source set and compares it
against the value embedded in each file: were these files generated from
these inputs? Mismatch is `forge/source-hash-mismatch`, exit 20. It needs no
generation, and it reads headers, not content, so a hand edit, a changed
template or a changed generator passes it — use `--assert-unchanged` above
where that matters.

*Tests:* `generation_can_assert_its_output_unchanged`
(`sce-build/tests/generation_can_assert_its_output_unchanged.rs`);
`verify_passes_on_clean_round_trip`, `verify_fails_when_source_drifts`
(`sce-build/tests/b9_drift_detection.rs`).

---

## 9. Formatters — a declared input, or none

A formatter is part of the function from document to bytes, so its version
is an input like the generator's own. The rule is the one §3 applies to the
header: the bytes may depend only on inputs a reader can name.

- **C++ is formatted by default, by clang-format pinned to major 19**
  (`formatter::CLANG_FORMAT_MAJOR`). Measured 2026-09-24 on the 288 raw
  C++ forge goldens with the bundled style: clang-format 18.1.3 and 19.1.7
  disagreed on 17 files, while 19.1.1 and 19.1.7 agreed on every one. So
  the major is pinned and the patch level is not. 19 is also the major the
  repository's own C++ is held to (`clang-format-check.yml`).
- **Resolution** (`formatter::resolve_clang_format`). `SCE_TOOL_CLANG_FORMAT`,
  when set, is the one answer: it is refused if it is not major 19, never
  fallen through. Otherwise every discovered `clang-format`, `clang-format-<N>`
  on `PATH` and `llvm-<N>/bin` install is asked its version, best-ranked
  first, and the first of major 19 wins — so a host whose unsuffixed
  `clang-format` is 18, with `clang-format-19` beside it, formats with 19.
- **No quiet fallback.** A run that would format C++ and finds no clang-format
  19 stops with `cli/formatter-unavailable`, naming what it found and the three
  ways out: install `clang-format-19`, point `SCE_TOOL_CLANG_FORMAT` at one,
  or pass `--no-format`. A file clang-format refuses stops the run with
  `cli/format-failed`. Until 2026-09-24 both cases wrote unformatted C++
  behind a note on stderr, so one document produced different bytes on a host
  that happened to lack the tool.
- **Only when there is C++ to shape.** The formatter is resolved at the first
  C++ artefact a run writes. A document that fails validation reports its own
  diagnostic, and `--list`, `--clean` and a rejected document's stubs never
  need clang-format.
- **`--no-format`** emits the templates' own bytes. It is the opt-out on every
  subcommand that formats — `generate`, `generate-w3c`, `orchestrate` — and it
  is what a test asserting on generated text passes, since the text it anchors
  on should be the generator's.
- **The manifest names it.** A run that formatted records
  `"formatter": {"tool": "clang-format", "version": "19.1.1"}` beside
  `generator` (`SCE_ERROR_CONTRACT.md` §10.1).
- **One formatter, in one place.** `orchestrate` formats exactly as `generate`
  does, so a document set is not shaped differently from the same documents
  generated one at a time. The library entry points (`sce_build::generate_*`)
  never format: their output is the raw template bytes, which is what the
  forge goldens hold.
- **CMake passes the choice through.** `SCEClangFormat.cmake` no longer runs a
  formatter of its own after generation — that was a second, unversioned
  clang-format over the first one's output, silently skipped where none was
  found. It hands the build's choice to `sce-codegen` instead:
  `SCE_FORMAT_GENERATED=OFF` becomes `--no-format`, and `SCE_CLANG_FORMAT_STYLE`
  becomes `--format-style` (`sce_codegen_format_args`).
- **Where it must be installed.** Every CI lane that generates C++ installs
  `clang-format-19`, and the build machines declare it in
  `.claude/remote-build.toml` `needs`.
- **Other backends.** Go, Kotlin, Python and C11 are emitted as the templates
  write them, with no formatter. The committed Rust trees are rustfmt's output
  applied after generation (§8), the one committed form that is not yet the
  generator's own.

*Tests:* `formatter::tests` in `sce-build/src/formatter.rs` (resolution,
version parsing, the refusals that replaced the fallback);
`every_apt_sourced_tool_is_installed_by_the_requiring_lane`
(`sce-build/tests/hook_ci_parity.rs` — the lane that requires its tools
installs `clang-format-19`).

---

## 10. Related contracts

- `SCE_ERROR_CONTRACT.md` §10 — the stdout manifest, including
  `generator`.
- `SCE_WIRE_CONTRACTS.md` — stability status of every wire
  surface.
- `docs/SCE_ACCEPTED_SUBSET.md` — the `DiagnosticCode` index, including
  `forge/source-hash-input-uncovered`.
