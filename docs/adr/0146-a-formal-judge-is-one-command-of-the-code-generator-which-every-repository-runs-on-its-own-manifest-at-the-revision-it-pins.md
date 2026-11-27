# ADR 0146 — A formal judge is one command of the code generator, which every repository runs on its own manifest at the revision it pins, this one included

- Status: Accepted (2026-11-26, by the owner). The owner asked for the plan to be reviewed against the most long-term
  correct structure (textbook, general, cost ignored), improved and adopted. The plan first put to a downstream
  consumer the same day ran the Python judges by their path in a vendored tree. The review found that this would
  leave two TLC runners, the second in the generator that levels 4 and 5 need. It also left a path a consumer would
  depend on that is no contract, and the tools pinned inside manifests that a consumer would copy. This record is
  the improved plan.
- Amended 2026-11-27 by the owner: Apalache joins the toolchain of D3, approved that day as the tool of ADR 0115 D3.
  `formal models` judges with it the inductive invariants a manifest states (D1), by one runner (D4), and TLC is given
  its standard module `Apalache` for a model that extends it.
- Amended 2026-11-27: the Java runtime TLC and Apalache run on joins the toolchain of D3. Apalache 0.63.0 is built
  for Java 21, and a build machine whose `java` was 17 answered every inductive run with
  `UnsupportedClassVersionError`; the CI host image carries Java 17 for the Kotlin lanes alone.
- Date: 2026-11-26
- Also amends `SCE_ERROR_CONTRACT.md` (the stage `formal`, its three codes, and the exit status 15) and
  `docs/SCE_ACCEPTED_SUBSET.md` (its appendix of codes).
- Amends ADR 0115 (each gains the line "Amended by: ADR 0146" in the landing that moves it): D5 (the gate reads a
  manifest it is given, not one at a fixed path), D13's "How a proof stands beside the code" (Verus is pinned by the
  judge, not by `proofs.json`), D11 (its refusals move into the judge, order step 5), and its "Order of work" (D7's
  generated module and D4 and D8's traces are judged by the runner this record builds). Also amends
  `docs/formal/README.md`, `SCE_WIRE_CONTRACTS.md` (two surfaces), `bx.toml` and the two formal workflows (where
  they read a pin).
- Scope: `sce-build` (the module `formal` and the subcommand `sce-codegen formal`), `scripts/formal` (removed),
  `scripts/gates/formal-*.sh`, `tools/git-hooks` (D11's judge), `docs/formal`. A generated machine's output does not
  change.
- Related: ADR 0115 (D4, D5, D7, D8, D11, D13), ADR 0119 (D4: TLC's graph read a second time), ADR 0084 (the one
  concurrency door of every runtime, whose proof is the next record), ADR 0120 (a lane run in parts).
- Requested by: a downstream consumer, 2026-11-26. Its protocol logic moves into SCE documents, generated for Rust
  and C11. It asks for the generated machines to be proved by the generated model and their runs to be read
  against it on both lanes. It also asks for the one concurrency door to be proved, and for the judges to be run
  on its own manifests at a pinned revision.

## Context

The judges of ADR 0115 are two Python programs, `scripts/formal/models.py` (levels 2, the bounded exploration and D9)
and `scripts/formal/proofs.py` (level 6). Each finds its manifest at a fixed place (`docs/formal/models.json`,
`docs/formal/proofs.json`) and its models beside it. Each manifest also pins the tool that judges it: TLC in
`models.json`, Verus and its Rust release in `proofs.json`. `bx.toml`, `.github/workflows/formal-*.yml` and
`tools/git-hooks/gate_registry.py` read the Verus pin from that file.

That serves this repository and no other:

- **A consumer cannot run them on its own models.** It can copy them, and the copy is a second judge that drifts
  from the first. Or it can run them from its vendored tree, where they read this repository's manifest. Pointing
  them elsewhere by their path makes the path, the manifest's keys and the output a contract that is written
  nowhere and that no gate holds.
- **The tools are pinned by what is judged, not by what judges.** A verdict is a function of the judge, the tools
  it runs and the input. The judge is tested with the tools it pins. A consumer that pins its own TLC gets a
  verdict from a pair nobody has tested.
- **The levels still to be built are in the generator.** D7's module is written by `sce-codegen model` from the
  tables the code is printed from. D4 and D8's trace specification is derived from the same tables, and ADR
  0119's D4.1 reads TLC's state graph (`-dump`) as the gate of D9 does. If levels 2 and 6 stay in Python, there are
  two programs that run TLC, parse its output and read its pin, one in each language. ADR 0115's own rule,
  "a generator is checked by checking each output", then has two readers of each output.

## The textbook structure

- **A verdict is reproducible only when the checker and its back-end tools are pinned as one unit.** A proof
  assistant's library pins the release of the assistant it is checked by, and a build that fetches a tool admits
  it by the digest of its content, whatever carried it (the fixed-output derivations of Nix, Dolstra 2006). The
  transport is not trusted; the digest is.
- **A tool's interface is its command, the schema of its input, its output and its exit status.** That interface is
  versioned and its stability is stated. In this repository, `SCE_WIRE_CONTRACTS.md` is the one registry of that
  statement, and a gate holds it to the tree.
- **The home repository is one user of the published interface.** A checker that its own repository runs through a
  path no other user has tests that path and nothing else. This repository's gates run the command a consumer
  runs, and a fixture shaped like a consumer is judged by it.
- **One semantics has one implementation where it can.** A TLC run, the reading of its exit status and of its count
  of states, and the reading of its graph are written once and used by every level that runs TLC.

## Decision

### D1. The judges are `sce-codegen formal`

`sce-codegen formal <judge> --manifest <file>` is the one entry, built in the `cli` feature as every other
subcommand is:

| Subcommand | Judges | Level of ADR 0115 |
|---|---|---|
| `formal models` | a models manifest: every module and configuration, its parts, its promise, its progress, what must stay possible, its inductive invariants | 2, bounded exploration, D9; 3, by Apalache (D3) |
| `formal proofs` | a proofs manifest: every file with a contract, verified whole, each mutant refused | 6 |
| `formal traces` | a traces manifest: every recorded run accepted by the model it names, each altered run refused (built with D4 and D8) | 5 |
| `formal toolchain` | nothing: prints the pins of D3 as JSON, for a cache key or an installer | — |
| `formal install` | nothing: fetches each pinned tool, admits it by its digest and puts it where the judges look | — |

A judge leaves through `sce-codegen`'s exit statuses (`SCE_ERROR_CONTRACT.md` §6), and every non-zero exit carries
a diagnostic record, as every other subcommand's does. The Python judges' own statuses (1 for a verdict, 3 for
"could not run") do not carry over, because §6 gives 3 to `validation/*`. A new stage, `formal`, has three codes:

| Code | Exit | Meaning |
|---|---|---|
| `formal/declaration` | 1 | the manifest is not as the tree is: a module, configuration or contracted file it does not declare, a part no run takes out, a property no run shows can fail, an unknown key, a manifest that is not one |
| `formal/verdict` | 1 | a run, a proof or a mutant answered otherwise than the manifest declares |
| `formal/tool-unavailable` | 15 | the judge could not run: a pinned tool or its runtime (Java, the Rust release Verus runs on) is missing, or is not the one pinned |

Exit 1 is the class §6 already gives `check-trace`: the inputs were read and the answer is no. Exit 15 is a new
row (`formal/tool-*`). A gate tells "could not run" from a verdict by it, as the Python judges' 3 let it do. A
judge never fetches a tool; `formal install` does. `proofs.py` already decided this for Verus, and TLC now
follows it.

The paths of a models manifest are read relative to the manifest's directory, which holds its modules and
configurations. The files of a proofs manifest are read relative to the root of the Git work tree that holds the
manifest. The proofs manifest is complete over the repository (every file with a contract is declared), so its
root is the repository's. `--shard k/N`, `--only`, `--record` and `--workers` keep their meaning.

### D2. Moved, not copied

The checks and the self-tests of `models.py` and `proofs.py` become `sce-build`'s module `formal` and its unit
tests, each check with the test that shows it can fail. The Python files are deleted in the change that switches
the gates to the command. No revision has two judges that answer.

### D3. The judge pins the tools

`sce-build/src/formal/toolchain.json` pins TLC, Verus and the Rust release Verus runs on, Apalache, and the Java
runtime TLC and Apalache run on: version, URL and sha256 of each. Apalache joined it on 2026-11-27 with ADR 0115 D3,
by the owner's approval; Storm joins it when
ADR 0119 brings it, with the owner's approval. The file
is compiled into the binary, so a binary of a revision knows its pins without a tree; it sits among the module's
sources, since it is one of them. `formal toolchain` prints
them. `bx.toml`, the workflows and `gate_registry.py` read this file, not a manifest. A manifest that names a tool
is refused as having an unknown key.

The runtime is pinned with the tools, since a tool pinned by its digest that runs on whatever virtual machine a
machine has is not pinned: a verdict is a function of the program run, and a JVM of another release runs another
program, or none (Apalache 0.63.0 is built for Java 21, class file 65). The pin is Eclipse Temurin's JRE for Linux on
x64. `formal install` installs it with TLC or Apalache, at `~/.local/jre/<version>`. TLC is run by that runtime's
`bin/java`, and Apalache's launcher, which runs the `java` its search path finds first, is given a search path led by
that runtime's `bin` and `JAVA_HOME` naming it. A runtime that is not there, or that cannot load the tool, is
`formal/tool-unavailable` (exit 15), never a verdict.

The transport is `curl`. The digest admits the bytes, as D3's source says. No network crate enters the default
build of `sce-build`, which keeps its one network client behind `remote-cache`.

### D4. One runner for every level

`formal::tlc` runs TLC: a module, a configuration, the workers, optionally a graph to write. It reads TLC's exit
status (0, 12, 13, otherwise a run that did not judge), its count of distinct states and the graph it wrote.
`formal::verus` runs Verus, with the jobserver handoff `proofs.py` has. `formal::apalache` runs Apalache (amended
2026-11-27): a module, the CONSTANTS of a configuration, a predicate to start from, the invariants and the number of
steps, in a directory of its own with an empty settings file in place of the home's; level 3 runs the base, the step
and the implication of an inductive invariant with it. TLC holds no module `Apalache`, which a model extends for
Apalache's types (`MkSeq`, `Gen`); TLC is given the pinned release's (SANY's `TLA-Library`), taken out of its jar by
`formal install`, so both tools read one release's module. Level 2 uses these runners now. Level 4
(D7's generated module) and level 5 (D4 and D8's trace modules) use the same runners when they are built, and so
does ADR 0119's comparison of graphs. None of them runs TLC in another way.

### D5. The manifests are wire surfaces

`schemas/sce-formal-models.v1.schema.json`, `schemas/sce-formal-proofs.v1.schema.json` and
`schemas/sce-formal-toolchain.v1.schema.json` (what `formal toolchain` prints) are added, each with its status constant
beside its reader in `sce-build/src/formal` and a row in `SCE_WIRE_CONTRACTS.md`, `pre-release`. Each manifest carries
`"v": 1`, so a reader refuses a version it does not know.
The traces manifest and the trace format follow when D4 and D8 build them. The judge is the only reader, and it
refuses an unknown key, as the Python judges do. Under the registry's policy, a field is added without a version
bump, and a change of meaning is a `v2`.

### D6. This repository is one user

The gates run `sce-codegen formal models --manifest docs/formal/models.json` and `sce-codegen formal proofs
--manifest docs/formal/proofs.json`, the command a consumer runs. A fixture shaped like a consumer,
`sce-build/tests/fixtures/formal_consumer/`, is judged by the same command. It is a root of its own with a manifest,
one model with one part, a run that holds and a run that takes the part out. Its declaration is judged in the unit
tests without Java. Its runs are judged by the models gate. A path that assumes this repository's `docs/formal`
fails here before it fails a consumer.

### D7. How a consumer runs it

A consumer builds `sce-codegen` from the SCE revision it pins, as it already does to generate. It runs `sce-codegen
formal install`, then `sce-codegen formal models|proofs|traces --manifest <its manifest>`. Its manifests and
models are in its own repository. It pins nothing but the revision: the binary carries the toolchain. D11's
refusals (a commit that changes what a check says and what it judges) are the judge's (order step 5), so a
consumer's pre-push runs them by the same command.

## Consequences

- One program judges every formal level, in the language of the generator that writes levels 4 and 5. A gate
  that judges a model from a document and one that judges a hand-written model share every line that runs TLC.
- The manifests become a contract that a consumer can rely on, with a registry row and a schema, and a change of
  their shape is seen in this repository first.
- A consumer's verdict is of the judge and the tools that this repository's gates ran together at that revision.
- `scripts/formal` is removed. A person runs `sce-codegen formal models` instead of `scripts/formal/check.sh`.

## Alternatives rejected

- **Run the Python judges by their path from a vendored tree.** This was the plan that the review replaced (Status).
  It gives two runners of TLC, a path and keys that are a contract written nowhere, and pins inside a manifest.
- **A second binary, `sce-formal`.** A consumer would build two entries from one revision. The judges of levels 4 and
  5 read the generator's tables, so they would link the generator anyway.
- **The consumer pins its own TLC and Verus.** Its verdict would then come from a judge and tools that nobody
  tested together.
- **A Rust HTTP client for `formal install`.** It would put a network crate in the default build. The digest
  carries the integrity, not the transport.
- **A copy of the judge in the consumer.** That is a second judge, and the drift is the fault this record removes.

## Order of work

Each step lands with a test that fails without it.

1. This record. `sce-build/src/formal/toolchain.json` (the pins moved out of the two manifests), `formal toolchain` and
   `formal install`. `bx.toml`, the workflows and `gate_registry.py` read the pins from it.
2. `formal models`: `models.py` and its self-test moved, its schema, status constant and registry row. The gate,
   the workflow and the README are switched, and `models.py` and `check.sh` are deleted.
3. `formal proofs`, the same way, with `proofs.py`.
4. The consumer fixture of D6, judged by the gate.
5. D11's judge (`tools/git-hooks/spec_isolation.py`) moved to `formal isolation`, which `pre-push` calls.
   As landed (2026-11-27): `pre-push` cannot run the generator before a gate has built it, so the judge is the gate
   `formal-isolation`, whose unit the push's plan hands the push's ranges (`commits: ["CHANGES"]`,
   `SCE_GATE_CHANGE_RANGES`) and which `formal-isolation.yml` runs over its own; a commit is judged where its parent
   holds the models or the proofs manifest, which `--models` and `--proofs` name for a consumer.
6. ADR 0115 continues on this runner: the proof of ADR 0084's door (ADR 0147), then D7, and `formal traces` with D4
   and D8 on every lane, the C11 lane among them.

## Sources

Dolstra 2006 (*The Purely Functional Software Deployment Model*, fixed-output derivations: content admitted by its
hash); `SCE_WIRE_CONTRACTS.md` (this repository's statement of a surface's stability). The rest is this
repository's own record (ADR 0115, D5, D13; `scripts/formal` as of 9b4c0d840c7).
