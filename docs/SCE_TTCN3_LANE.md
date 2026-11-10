# SCE TTCN-3 conformance lane — design

- Status: design of ADR 0091 order of work step 5, written 2026-11-10. Nothing of the lane is built in SCE yet. The
  lane is split between SCE and a separate downstream repository, `sce-ttcn3-titan`, which exists and is private for
  now. Its `SCE_PIN` is `0d48bef3bd`. This document is SCE's side of the design. The SCE-side slices S1 and S2 come
  first, then the downstream slices D1 to D5 (§13).
- Downstream design: `docs/DESIGN.md` of https://github.com/newmassrael/sce-ttcn3-titan, cited as **DD §x**. It
  holds §6 to §9, §11 and §13.2 of this design under the same numbers, and this document keeps only their headings,
  so that a section has one text.
- Decision: `docs/adr/0091-a-test-is-a-document-of-the-test-profile-and-its-ttcn-3-export-runs-and-agrees-with-it.md`,
  answers 1, 3, 8 and 12, §6 (C1 to C3) and §7. The ADR was amended on 2026-11-10 with the repository split and the
  licensing record of §3.
- Semantics: `docs/SCE_TEST_PROFILE.md`, cited as **TP §x**: the scheduler (TP §3), the export (TP §9), what the
  lane requires of the tool (TP §10) and the native runtime (TP §12).
- Sources, pinned by ADR 0091 answer 2: ETSI ES 201 873-1 V4.17.1 (**-1 §x**) and -4 V4.6.1 (**-4 §x**); -5 V4.9.1,
  the TTCN-3 Runtime Interface (TRI), and -6 V4.14.1, the TTCN-3 Control Interface (TCI). The texts of -5 and -6 were
  not at hand when this design was written. Their operations are named in DD §7.2 as ADR 0091 and the native runtimes
  name them, which are the names TRI and TCI publish. Slice D1 reads each operation's name and parameters from the two
  texts before it writes a header, and corrects the downstream design where they differ.
- The split of §2, the licensing record of §3, the trace of §10 and the slices of §13, and the layout (DD §6), the
  Main Controller change (DD §9.2) and the lane (DD §11) of the downstream design, were decided by the main session on
  the owner's delegation of 2026-11-10. They are recorded with their reasons, and this document does not reopen them.
  Where they leave something open and this document chooses it, the choice is marked **design choice**, so that it can
  be told apart from what was decided.

## 1. What the lane is for

C1 asks for the export to run on a TTCN-3 tool that SCE did not write. C3 asks that two runs of one suite on one input
trace, the native run and the run of the export, agree per test case in the test verdict and in every decoded
message, and it makes a disagreement `error`. ADR 0091 §7 asks for a reference adapter set, so that both can be checked
without any consumer's test system. The lane is that check. It runs every test case of the profile's corpus natively and
through the export on Eclipse Titan, from the same recorded inputs, and compares the canonical logs of the two runs.

The comparison means something only under three conditions:

- **The two runs see the same inputs at the same instants**: the recorded input trace (§10) and the logical clock
  (DD §9.2).
- **The two runs take the same schedule**: the rounds of TP §3.2, carried in Titan by its Main Controller (DD §9.2).
- **The TTCN-3 side's codecs and logger are not SCE's**: the codec bindings (DD §8.4) and the logger plugin
  (DD §9.3.3) run on libraries that neither SCE nor Titan wrote, in a repository that holds no SCE code (§2).

## 2. Repository boundary

### 2.1 The split

Everything that touches Titan lives in a separate downstream repository, `sce-ttcn3-titan`, published under EPL-2.0:

- the adapter interface shaped after TRI and TCI (C++17, DD §7);
- the reference adapters: the trace player and recorder, and the logical clock (DD §8);
- the codec bindings, over libcbor (MIT) and libical (MPL-2.0) (DD §8.4);
- the Titan binding: test ports, the `enc_`/`dec_` external functions and the logger plugin (DD §9.3);
- the pinned Titan build and the Main Controller patches, which go upstream under the ECA (DD §9.1, §9.2, §9.4);
- the lane runner, which applies C3 (DD §11).

SCE keeps what it owns as a tool and as a specification:

- the exporter (`sce-codegen export-ttcn3`) and the validator (`sce-ttcn3-validate`);
- the wire surfaces the downstream reads and writes, with their schemas and their rows of `SCE_WIRE_CONTRACTS.md`
  (§4);
- the native runtimes and their runners, with trace replay (§10.5);
- the corpus, with its recorded traces and its expected logs (§12).

SCE holds no directory of the downstream's code, and no copy of it.

### 2.2 Why

- **The tool and its adapters belong together.** A test port is written to Titan's C++ Test Port API and compiled and
  linked with Titan's runtime, and the Main Controller patches are changes to Titan. They are kept, built and
  released with the Titan they are written for.
- **SCE is used, not combined.** The downstream runs SCE as a program and reads its published files, as any user of
  the generator does. No SCE file is part of the downstream, and no downstream file is part of SCE. The licensing
  question of combining SCE's code with Titan's therefore does not arise (§3).
- **The interface between the two is SCE's published surfaces.** These are versioned and their stability is stated
  in one registry (§4), so the downstream depends on a contract and not on SCE's internals. A second TTCN-3 tool, or a
  consumer's own harness, can depend on the same contract.

### 2.3 What crosses the boundary

| Artefact | Written by | Read by |
|---|---|---|
| `S.ttcn`, `S_Types.ttcn` | `sce-codegen export-ttcn3` (TP §9.5) | Titan's compiler, in the downstream |
| `S_Types.manifest.json` | the export | the downstream's generator (DD §9.3), value model (DD §8.3) and logger plugin |
| `S.witness.json` | the export | `sce-ttcn3-validate`, which the downstream runs as a process |
| a trace, `sce-test-trace.v1` (§10) | the corpus (SCE); the recorder (downstream, DD §8.1) | the native runners (SCE); the trace player (downstream) |
| a canonical log, `sce-test-log.v1` (ADR 0091 answer 8) | the native runners (SCE); the logger plugin (downstream) | the lane's comparison (downstream); SCE's corpus check (§12) |
| a codec document (`sce:kind="codec"`, `SCE_FORGE.md` §4.6) | the corpus and consumers | the downstream's codec bindings, for the format's parameters only (DD §8.4) |
| `sce-codegen`, `sce-ttcn3-validate`, each backend's runner (§10.5) | SCE | the downstream's lane, run as processes |

The downstream pins the SCE revision it runs: a commit of SCE, from which it builds `sce-codegen`, the validator and
the native runners (design choice: in a file `SCE_PIN` at its root). When SCE's CI runs the lane (DD §11.5), it runs
the downstream at a pinned revision of the downstream against SCE's own checkout, which then stands in for `SCE_PIN`.

### 2.4 How the boundary is kept

- **In the downstream.** Its slice D1 (DD §13.2) adds a check, run in its own CI, that every file carries
  `SPDX-License-Identifier: EPL-2.0` (but `LICENSE`, the verbatim text), and that no file includes, imports, links or
  copies a path of SCE. SCE is reached only through the pinned revision's programs and published files.
- **In SCE.** No build file, gate or source of SCE names a file of the downstream. SCE's CI fetches the downstream at
  a pinned revision and runs it as an external tool (DD §11.5), in the same way it fetches and runs mnemosyne-cli.

## 3. Licensing

This is the record ADR 0091 answer 1 carries, amended 2026-11-10. Each statement gives its basis.

- **The split removes the combination question.** No SCE file is relicensed. The downstream holds no SCE code and is
  a separate work: it runs SCE as a program and reads and writes SCE's published files (§2.3). Before the split,
  EPL-2.0 code (Titan's and a binding linked with it) and SCE's AGPL-3.0 code would have been combined in one
  executable. EPL-2.0 allows that only where the EPL file's notice names a GPL-family licence as a Secondary License,
  and Titan's notices name none (answer 1). After the split no such executable exists.
- **The exported modules are generated output.** The `.ttcn` modules are text `sce-codegen` writes from the suite's
  documents, as it writes any target language. They are governed by `LICENSE-GENERATED.md`, as every output of the
  generator is, and the downstream compiles them as any user of the generator compiles its output.
- **CI-only use distributes nothing.** EPL-2.0's obligations arise when the program is distributed. AGPL-3.0's arise
  when it is conveyed, or when users interact with a modified version over a network. Running Titan and the
  downstream in CI, or on a developer's machine, is neither: they are run, not handed to anyone. No SCE product
  contains them (answer 1).
- **The upstream patch is contributed under the ECA.** A contribution to an Eclipse project, here `eclipse/titan.core`,
  is made under the project's licence, EPL-2.0, by a contributor who has signed the Eclipse Contributor Agreement. The
  patches are EPL-2.0 from their first line, in a repository under EPL-2.0, so they need no relicensing on the way
  upstream. The signer is the patches' author.
- **The codec libraries keep their own licences.** libcbor is MIT. libical is offered under MPL-2.0 or LGPL-2.1, and
  the downstream takes MPL-2.0. Both are linked into the downstream's executable test suite with EPL-2.0 code. MIT is
  permissive, and MPL-2.0 is a file-level copyleft that stays on libical's own files. The downstream pins each library
  by version and by the SHA-256 of its source archive, and carries each licence text with the pin.
- **No legal review was made.** None is needed for the combination question, which no longer exists. The statements
  above are standard facts of the licences named, each with its basis. A consumer that distributes a binary built
  from the downstream takes on EPL-2.0's obligations for it, and those of MIT and MPL-2.0 for the libraries, as for
  any EPL-2.0 program.

`LICENSE-THIRD-PARTY.md` records, in its section "The TTCN-3 conformance lane", that the lane is a separate
repository, that nothing of it is vendored into SCE, and that SCE's CI runs it as a tool.

## 4. What SCE provides

The downstream depends on these SCE surfaces and on nothing else of SCE. Each has its row in `SCE_WIRE_CONTRACTS.md`,
which states its stability and how a change is announced.

| Surface | Schema | Status | Row today |
|---|---|---|---|
| the exported modules `S.ttcn` and `S_Types.ttcn`, under the name contract of TP §9.5 | the TTCN-3 the export writes (TP §9.5) | as the export's manifest and witness | the manifest's and the witness's rows describe them |
| the types manifest `S_Types.manifest.json` | `schemas/sce-ttcn3-types-manifest.v1.schema.json` | `pre-release` | yes |
| the correspondence witness `S.witness.json` | `schemas/sce-ttcn3-witness.v1.schema.json` | `pre-release` | yes |
| the recorded input trace `sce-test-trace.v1` | `schemas/sce-test-trace.v1.schema.json` (S1) | `pre-release` | planned (§10.4) |
| the canonical log `sce-test-log.v1` | `schemas/sce-test-log.v1.schema.json` (order of work step 6) | `pre-release` | planned (ADR 0091 answer 8) |
| the authoring grammar, of which the codec bindings read codec documents | `schemas/sce-forge.xsd`, `schemas/sce-forge-ext.xsd` | `pre-release` | yes |

Beside the files, the downstream runs SCE's programs. Their command lines are part of the contract, stated here and
in each program's usage. Design choice: a change to one is announced as a change to the surface it writes.

| Program | Contract |
|---|---|
| `sce-codegen export-ttcn3` | writes the two modules, the manifest and the witness of one suite, all or nothing (TP §9.5) |
| `sce-ttcn3-validate <suite.scxml> <export directory>` | exit 0 with no violation, 1 with one or more, 2 when an input is not read (TP §9.4) |
| `sce-codegen check-trace <suite> <trace>` (S1, design choice) | refuses a trace that does not fit the suite (§10.2) |
| each backend's runner (S1) | runs one test case of a generated suite against one trace and writes its result, the canonical log once step 6 lands (§10.5) |

While every surface above is `pre-release`, SCE may change one in a way that is not additive, under the policy of
`SCE_WIRE_CONTRACTS.md`. The downstream's `SCE_PIN` is what keeps it working across such a change: it moves to a new
SCE revision only together with its own adaptation.

## 5. What the downstream provides to SCE

The downstream provides one program to SCE's CI: the lane runner. Given an SCE checkout, a list of suites and the
directories of their traces, it runs the steps of DD §11.1 and writes a report. A test case whose two logs disagree,
or whose run failed on either side, is `error`, and the runner exits non-zero. SCE does not read any other downstream
file.

## 6. Layout of `sce-ttcn3-titan`

The downstream repository's directories and what each holds: the adapter interface, the reference adapter set, the
Titan binding and the lane runner. This section lives in `docs/DESIGN.md` of
https://github.com/newmassrael/sce-ttcn3-titan, under the same number.

## 7. The adapter interface (`adapter/`)

The C++17 interface whose operations are TRI's and TCI's under their names, where it departs from the two standards,
and its sketches. This section lives in `docs/DESIGN.md` of https://github.com/newmassrael/sce-ttcn3-titan, under the
same number.

## 8. The reference adapter set (`reference/`)

The trace player and recorder, the logical clock, the canonical values in JCS, and the codec bindings over libcbor and
libical. This section lives in `docs/DESIGN.md` of https://github.com/newmassrael/sce-ttcn3-titan, under the same
number.

## 9. The Titan side (`titan/`)

The pinned Titan build, the Main Controller change that carries the rounds of TP §3.2 and the logical clock of TP §3.3,
the binding (test ports, external functions and the logger plugin), and the change's way upstream. This section lives
in `docs/DESIGN.md` of https://github.com/newmassrael/sce-ttcn3-titan, under the same number.

## 10. The recorded input trace

### 10.1 The record

A trace is the inputs of one run of one test case from the SUT, each at its logical instant. It is a new SCE wire
surface, `sce-test-trace.v1`, `pre-release`. Each record holds:

| Field | Holds |
|---|---|
| `at` | the instant, in microseconds since the test case started, as a decimal string: digits only, no leading zero |
| `port` | the system port it arrives on: the `id` of an `<sce:system-port>` of the test case's configuration |
| `type` | the message's type: the record's name, as the types module and the manifest name it (TP §9.5, the name contract) |
| `value` | the message, in the canonical value form (§10.3) |

`at` is a string because the manifest writes every value that may pass 2^53 as the text of its digits, and an
instant is a 64-bit integer.

### 10.2 The file

A trace file holds one test case's inputs as one JCS text, so that a trace has exactly one spelling and two traces
are equal exactly when their bytes are. Design choice: the envelope is
`{"v": 1, "suite": <suite>, "test_case": <test case>, "inputs": [<record>, …]}`. The inputs are in non-decreasing
`at`, and the inputs of one port at one instant are in the order they are enqueued (TP §3.2 step 2). A test case may
have several traces. Design choice: they live beside the suite's document, as
`traces/<test case>/<name>.trace.json`, with the expected log of each beside it (§12).

A trace is refused before the run, on both sides, when:

- a record's `port` is not a system port of the test case's configuration;
- a record's `type` is not in the `in` list of that port's type;
- a record's `value` does not belong to its type;
- the records are not in the order above.

The native runners and the Titan system adapter both read the same file. So the two runs see the same inputs at the
same instants by construction, which is what C3's comparison assumes.

### 10.3 The canonical value form

The trace's values and the canonical log's values are one form. TP §12.5 names the log's records but does not yet fix
the form of their values: it says the log's schema is fixed with the equivalence law (order of work step 6). This
design states what the form must be. The first slice that lands a schema using it (S1, for the trace, or step 6, for
the log) writes it into TP §12.5, which then owns it.

| SCE type (TP §6.3) | Canonical value |
|---|---|
| `intN`, `uintN` | a string of decimal digits, with `-` before a negative, no leading zero |
| `bool` | `true` or `false` |
| `enum:X` | a string: the variant's name |
| `string[N]` | a JSON string |
| `bytes[N]` | a string of lower-case hexadecimal digits, two to an octet |
| `record:X`, an event schema | an object with one member for each field that is present, named as the field is declared. An absent optional field has no member |
| `list<T>[M..B]` | an array of the elements' values |
| `union:X` | an object with one member, named by the variant of the case, whose value is the case's record, or `{}` for a case that holds none |
| `float32`, `float64` | none: not in the profile (TP §8.1) |

The text is JCS: object members in its order, strings in its escaping, and no number anywhere, since every integer is
a string. JCS writes a number as the shortest form of an IEEE 754 double, which cannot hold every `int64` or `uint64`.
The value is always read against its type from the manifest, so a text, a byte string and a variant's name are never
mistaken for one another.

### 10.4 The surface, planned for slice S1

This design writes no schema. Slice S1 lands the surface as `SCE_WIRE_CONTRACTS.md` requires every surface to land:

- `schemas/sce-test-trace.v1.schema.json`, with `x-sce-schema-status: pre-release`;
- the const `TEST_TRACE_SCHEMA_STATUS` in `sce-build/src/test_trace.rs`. That module holds SCE's reader of a trace
  and its check of a trace against a suite, the refusals of §10.2, offered as a command of `sce-codegen` (design
  choice: `sce-codegen check-trace <suite> <trace>`);
- a row of `SCE_WIRE_CONTRACTS.md`:
  - Format owner: SCE;
  - Status source: `TEST_TRACE_SCHEMA_STATUS` (`sce-build/src/test_trace.rs`) and `x-sce-schema-status`;
  - Status: `pre-release`;
  - Shape governance: this document §10, and `sce-build/src/test_trace.rs`, the reader the native runners' checks
    share;
  - Producers: the corpus's traces, written by hand, and the downstream's recorder (DD §8.1);
  - Readers: the native trace players (§10.5) and the downstream's trace player.

### 10.5 Native trace replay

Every backend's test runtime already has a system adapter shaped after TRI (TP §12.2), and it is pulled by the run:
the instant of the next input, and the inputs due at an instant. Slice S1 adds to each of the seven runtimes a trace
player behind that interface, which reads a trace file, and generates for each record type of a suite the conversion
from the canonical value to the backend's record. Step 6's log writer needs the same conversion in the other
direction. The two are generated as one pair per type, so that the value form has one implementation per backend.
Slice S1 also adds, per backend, a runner program that runs one test case of a generated suite against one trace and
writes the result. When step 6 lands the canonical log, the runner writes that log. The downstream's lane calls the
runners as processes.

## 11. The lane

The lane runner's steps, why a disagreement between the two runs is never resolved, the canonical order of a log's
records, which backends take part, and where the lane runs, SCE's CI job among them. This section lives in
`docs/DESIGN.md` of https://github.com/newmassrael/sce-ttcn3-titan, under the same number.

## 12. The corpus

The corpus is SCE's: the suites, their recorded traces, their expected logs, and the codec documents of the message
types their mapped ports carry. ADR 0091 §7 asks it to cover every row of its §2 and every refusal of its §3. An
expected log is the canonical log a reviewed native run wrote for one trace. SCE's own tests hold every backend's run
to it, so a change of native behaviour shows in SCE before it reaches the lane. This is what the tree holds on
2026-11-10.

### 12.1 What runs today

- **`runtime_suite`** (`sce-build/tests/fixtures/test_profile/runtime/`) runs end to end on all seven backends: four
  test cases, an MTC alone and with PTCs, each verdict the profile has, and a guard that ends in `error`.
- **`job_suite`, `ping_suite`, `orders_suite` and `nested_suite`** (`sce-build/tests/fixtures/test_profile/`, its
  `assembly/`, and `sce-build/tests/fixtures/ttcn3_export/`) are exported and validated, and their documents are
  generated, but no suite of them runs end to end.
- **No suite has a trace or an expected log.** No mapped message type of any suite has a codec document (DD §8.4).

### 12.2 The rows of ADR 0091 §2

| Row | Runs end to end (`runtime_suite`) | Exported only | Missing from every run |
|---|---|---|---|
| Type definitions | records of `uint32` | enums, optional fields, texts, lists and records within records (`job_suite`, `orders_suite`); a union and a byte string (`orders_suite`) | every type but a record of `uint32`; signed and 64-bit integers are in no suite at all |
| Template | specific values, `?`, record matching, value parameters | every mechanism of TP §6.1 (`every_matcher`, in `job_suite`); union matching and template references (`orders_suite`) | every mechanism but the four that run |
| Port type | yes | | |
| Component type | yes | | |
| Test configuration | `connect` and `map` | | |
| Test case | yes | | |
| Module parameters | a feature (PICS), `<sce:module-parameter>` | | |
| Control part | the order, selection by `all` and `any`, guards | | |
| `setverdict` | `pass`, `inconclusive`, `fail` | | `none`, written by a document |
| `alt` with receive branches | yes | | |
| `timer` | start, stop, timeout, a default duration | a restart, a duration given at the start (`nested_suite`) | the restart and the given duration |
| `altstep` and `activate` (ancestor transitions) | | `nested_suite` | in every run |
| `send` | yes | | |
| PTC `create`, `start`, `done`, `kill` | all four | | |
| `log` | yes | | |

Also missing from every run:

- a test case that can never end, which is `error` with no instant pending (P13);
- a dynamic error, such as a send to a removed PTC's port (P6, P13).

### 12.3 The refusals of ADR 0091 §3

Every refusal has a refused fixture that `sce-build`'s tests hold to its diagnostic, with one exception:

| Refusal | Fixture (`refused_*.scxml`) |
|---|---|
| a native expression | `component_native_condition` |
| a construct that raises a script-engine cause | **none**. `test-profile/script-engine` is declared in `sce-build/src/forge/diagnostic.rs`, and no code outside that file raises it |
| a `datamodel` that is not `sce-static` | `component_datamodel`, `port_type_datamodel` |
| a `cond` that reads the received message | `component_cond_reads_message` |
| `<parallel>`, `<history>`, `<script>` | `component_parallel`, `component_history`, `component_script` |
| `<donedata>` as a verdict | `component_donedata` |
| a receive branch whose template is not an incoming type of its port | `component_receive_not_in` |
| a test case selected by an undeclared feature | `suite_feature_undeclared` |
| `<raise>`, and a `<send>` to `#_internal` | `component_raise`, `component_send_internal` |
| a `<send>` whose target is not a port of the component | `component_send_unknown_port` |
| a `<send>` with a delay, and `<cancel>` | `component_send_delay`, `component_cancel` |
| an `<invoke>` of anything but a test component | `component_invoke_not_component`, `component_invoke_type` |
| a transition that is not one branch kind | `component_branch_event`, `component_branch_two_kinds` |
| a done branch whose target keeps the invoking state active | `component_done_branch_stays` |

### 12.4 What the corpus must gain (slice S2)

- a trace for every test case of every runnable suite, and the expected log of each trace;
- a codec document for every message type a mapped port carries, in CBOR or content-line;
- runnable suites that cover the "missing" column of §12.2, including the two `error` cases;
- `job_suite`, `orders_suite` and `nested_suite` (or suites that replace them) made runnable end to end, so that
  their rows run as well as export;
- a fixture that raises `test-profile/script-engine`, or a recorded reason why no profile document can reach that
  refusal once `datamodel` is held to `sce-static`.

## 13. The slices

SCE's slices come first, because the downstream depends on the surfaces they land. Each of SCE's slices lists what it
builds, the tests it brings and the files it changes, so that merges can be planned. The downstream's slices are
listed in DD §13.2.

### 13.1 In SCE

**S1: the trace surface and native trace replay.**

- The schema, the const and the row of §10.4, `sce-codegen check-trace`, and the canonical value form written into TP
  §12.5 (§10.3), if step 6 has not written it first.
- The trace player and the generated value conversions of every backend, and each backend's runner (§10.5).
- Tests: per backend, a trace of `runtime_suite` replays to the verdicts its test cases expect, and `check-trace`
  refuses each case of §10.2.
- Files: `schemas/`, `sce-build/src/test_trace.rs`, the command in `sce-build/src/bin/sce_codegen.rs`,
  `SCE_WIRE_CONTRACTS.md`, `docs/SCE_TEST_PROFILE.md`, the seven test runtimes and their generators, and the
  generated test trees those generators write.

**S2: a corpus with traces and expected logs.**

- The additions of §12.4.
- Tests:
  - per backend, every trace of the corpus replays to its expected log, byte for byte;
  - a test of SCE holds the corpus to ADR 0091 §2 and §3: each row and each refusal names the suite or the fixture
    that covers it.
- Depends on the schema and the native writers of `sce-test-log.v1` (order of work step 6), which the expected logs
  are written in. The traces and the codec documents do not depend on it, and can land first.
- Files: the corpus fixtures, their traces, expected logs and codec documents, and the test that holds the coverage.

### 13.2 In the downstream

Slices D1 to D5: the skeleton, its licence and the adapter interface; the pinned Titan build and the Main Controller
patch; the reference adapters and the codecs; the binding and the logger; the lane and C3. This section lives in
`docs/DESIGN.md` of https://github.com/newmassrael/sce-ttcn3-titan, under the same number.

### 13.3 How the slices meet order of work step 6

Step 6 lands the schema of the canonical log, its row of `SCE_WIRE_CONTRACTS.md`, and the native runtimes' writers.
S2's expected logs, D4's logger and D5's comparison (DD §13.2) all need them. So the schema and the writers of step 6
land before S2's expected logs and before D4. Step 6 stays the step that holds the law over the whole corpus.

## 14. What this design leaves open

- **The names of -5 and -6.** DD §7.2 is checked against the two texts in slice D1 (DD §13.2).
- **Where the canonical value form is owned.** §10.3 states it, and TP §12.5 takes it over in S1 or in step 6.
- **SCE's own codecs in the lane.** In the lane the native side holds values and uses no codec (TP §12.2), so only the
  TTCN-3 side encodes. Comparing SCE's generated codecs with the reference bindings, byte for byte on the same values,
  would be a check of its own. D3's tests decode SCE's bytes for a few shapes. A check over the corpus is not part of
  this design.
- **Positional binary codecs.** They have no reference binding (DD §8.4).
- **A second TTCN-3 tool.** Running the suite on a second tool, by a binding of the same interface, remains the
  stronger form of C1 (answer 1). It is not planned.
