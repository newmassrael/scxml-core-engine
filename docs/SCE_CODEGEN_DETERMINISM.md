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
SOURCE_DATE_EPOCH=0 sce-codegen generate path/to/machine.scxml \
    --source-root "$(git rev-parse --show-toplevel)" \
    -o generated/ -l cpp
```

- `SOURCE_DATE_EPOCH` pins the `generated-at` header stamp.
- `--source-root` pins the `// From:` provenance path.

Without them the output is still *correct*, but two runs will not be
byte-identical and the artifact will carry a path that only resolves on
the machine that produced it.

---

## 2. What each generated file carries

Every emitted source file starts with the §synth-6.2.6 drift header:

```
// SCE-GENERATED — DO NOT EDIT
// source-hash: <sha256 over the input source set>
// template-hash: <sha256 over the codegen template tree + Cargo.lock>
// generated-at: <unix seconds>
```

followed by a license block carrying `// From: <input path>`.

Three of those five values are functions of the inputs. Two — the stamp
and the provenance path — are functions of *how you invoked the
generator*, and are what the flags below control.

---

## 3. `generated-at` — pin it with `SOURCE_DATE_EPOCH`

The stamp defaults to wall-clock seconds, so no two runs agree. It feeds
neither hash, so pinning it costs nothing:

```bash
SOURCE_DATE_EPOCH=0 sce-codegen generate ...
```

`sce-codegen` honours the [reproducible-builds][rb] convention: any
integer value is used verbatim as the stamp. Pinning it is what makes a
regeneration gate expressible:

```bash
SOURCE_DATE_EPOCH=0 sce-codegen generate src/machine.scxml -o generated/ -l cpp
git diff --exit-code generated/     # fails iff generation actually moved
```

This repository does exactly that for its own committed trees —
`scripts/regen_all_committed_trees.sh` exports the variable, and
`committed_trees_carry_a_pinned_generated_at` fails if a regeneration ever
lands without it.

*Tests:* `source_date_epoch_pins_generated_at_for_byte_stable_regen`,
`generated_at_tracks_the_clock_without_the_pin`
(`sce-build/tests/codegen_invocation_determinism.rs`).

[rb]: https://reproducible-builds.org/docs/source-date-epoch/

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

## 6. `template-hash` — pin it with `--workspace-root`

Covers `tools/codegen/templates/**` plus `Cargo.lock`. Resolution order:
`--workspace-root` → `$SCE_WORKSPACE_ROOT` → `CARGO_MANIFEST_DIR/..` →
walk up from the working directory.

If every layer fails, the axis degrades to a zero hash and says so on
stderr. Unlike the source axis this is a signposted fallback, not a
silent one — but a vendored or relocated build should pass
`--workspace-root` rather than rely on the walk.

---

## 7. Which generator produced this artifact

The crate version is frozen at `0.1.0` pre-1.0 and identifies nothing.
The commit is the field to record:

```console
$ sce-codegen --version
sce-codegen 0.1.0 (b497eacf7d94)
```

It is also in the stdout manifest of every `generate` run, so a build
system capturing that JSON attributes its output with no extra
invocation:

```json
{"v":1,"kind":"generate","generator":"b497eacf7d94","artifacts":[…]}
```

If you keep a version sidecar next to committed output, derive it from
one of these rather than maintaining it by hand — a hand-maintained
record drifts silently, and a tree whose recorded generator no longer
reproduces it is a tree nobody can audit.

`"unknown"` appears when the generator was built without a git checkout
to read (vendored crate, release tarball). The value names the *committed*
state the generator was built from; uncommitted edits to the generator
itself are not reflected, which is why the input hashes are computed per
run from actual bytes rather than trusted from the binary's identity.

*Tests:* `version_reports_the_generator_commit`,
`manifest_generator_matches_version_output`
(`sce-build/tests/error_format_json.rs`).

---

## 8. Verifying after the fact

```bash
sce-codegen verify generated/ --input-root src/scxml
```

Recomputes both hashes from the current source and template state and
compares against the values embedded in each file. Mismatch is
`forge/source-hash-mismatch`, exit 20. Use it as a CI gate when you commit
generated output and want drift caught at review time rather than at the
next regeneration.

---

## 9. Related contracts

- `SCE_ERROR_CONTRACT.md` §10 — the stdout manifest, including
  `generator`.
- `SCE_WIRE_CONTRACTS.md` — stability status of every wire
  surface.
- `docs/SCE_ACCEPTED_SUBSET.md` — the `DiagnosticCode` index, including
  `forge/source-hash-input-uncovered`.
