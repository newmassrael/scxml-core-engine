# SCE TTCN-3 conformance lane — design

- Status: design of ADR 0091 order of work step 5, written 2026-11-10. Of what SCE gives the lane, the canonical log
  the lane compares is in place (order of work step 6, TP §12.5): all seven native runtimes write it, and each is held
  to one expected log. Of the trace surface and native trace replay (S1), the surface, the shared walk and the Rust
  backend are in place (§10.4, §10.5); the other six backends and the corpus (S2) are not built yet. The
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
| a canonical log, `sce-test-log.v1` (ADR 0091 answer 8, TP §12.5) | the native runtimes, through their runners (SCE); the logger plugin (downstream) | the lane's comparison (downstream); SCE's corpus check (§12) |
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
| the recorded input trace `sce-test-trace.v1` | `schemas/sce-test-trace.v1.schema.json` | `pre-release` | yes (§10.4) |
| the canonical log `sce-test-log.v1`, under TP §12.5 | `schemas/sce-test-log.v1.schema.json` | `pre-release` | yes |
| the authoring grammar, of which the codec bindings read codec documents | `schemas/sce-forge.xsd`, `schemas/sce-forge-ext.xsd` | `pre-release` | yes |

Beside the files, the downstream runs SCE's programs. Their command lines are part of the contract, stated here and
in each program's usage. Design choice: a change to one is announced as a change to the surface it writes.

| Program | Contract |
|---|---|
| `sce-codegen export-ttcn3` | writes the two modules, the manifest and the witness of one suite, all or nothing (TP §9.5) |
| `sce-ttcn3-validate <suite.scxml> <export directory>` | exit 0 with no violation, 1 with one or more, 2 when an input is not read (TP §9.4) |
| `sce-codegen check-trace <suite> <trace>` (S1, design choice, not yet in place) | refuses a trace that does not fit the suite (§10.2) |
| each backend's runner of a generated suite (S1; in place on Rust, Kotlin and C++) | runs every test case a world selects against its trace and writes each one's canonical log (TP §12.5, §10.5), on the command line below |

Every backend's runner takes one command line, so the downstream's lane calls the seven alike (design choice):

```text
<runner> --trace-dir <dir> --log-dir <dir> [--world <feature>,...] [--module-parameter <id>=<value>]...
```

| Option | Meaning |
|---|---|
| `--trace-dir <dir>` | the directory of the traces, `<test case>.json` each (§10.2) |
| `--log-dir <dir>` | the directory the logs are written to, made where it is missing: `<test case>.log.json` each, the canonical log's text with nothing after it (TP §12.5) |
| `--world <feature>,...` | the features the world enables, by name, separated by commas; none where it is not given (ADR 0057 §2) |
| `--module-parameter <id>=<value>` | a module parameter's value in place of its default, a literal of its type as the `value` of a `<sce:module-parameter>` writes one (TP §11.4, §11.8); repeated, one per parameter |

The runner runs the test cases the world selects, in the suite's order, and prints `<test case> <verdict>` on a line
for each, and nothing else on the standard output; every diagnostic of the run, the engine's log lines among them,
goes to the standard error. A test case whose trace is missing or refused (§10.2) ends with `error` (P13) and its log
is written, the reason on the standard error. The exit status is 0 when every selected test case ran and its log was written, whatever
its verdict, since the verdict is the log's; it is 2 when the command line is not this one, a module parameter's value
is refused (a parameter the suite does not declare, or a literal that is no value of its type), or a log cannot be
written. The runner is generated per suite as the `main` of the suite's unit, which a crate makes a program with a
binary target of one line (Rust: `<suite>::main`, over `sce_rust_runtime::test::runner`).
On Kotlin the program is the class of the suite's unit, `<package>.<Suite>Kt`, whose `main` calls
`com.sce.runtime.test.runnerMain` (the tests module runs `runtime_suite`'s as `:sce-kotlin-tests:runRuntimeSuite`).
On C++ it is `SCE::Generated::<Suite>::main(argc, argv)`, over `sce/include/test/Runner.h`, which a one-line `main`
calls (`tests/integration/runtime_suite_runner.cpp`).
On Go it is the `Main` of the suite's package, over `testprofile.RunnerMain`, which a one-line `package main` calls
(`backends/go/tests/cmd/runtime_suite_runner`, run as `go run ./cmd/runtime_suite_runner` in `backends/go/tests`).

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
| `type` | the message's type: the record's name, as the types module and the manifest name it (TP §9.5, the name contract), which is the `type` a message record of the log carries (TP §12.5) |
| `value` | the message, in the value form of TP §12.5 (§10.3) |

`at` is a string because the manifest writes every value that may pass 2^53 as the text of its digits, and an
instant is a 64-bit integer.

### 10.2 The file

A trace file holds one test case's inputs as one JCS text, so that a trace has exactly one spelling and two traces
are equal exactly when their bytes are; the file may end in one line feed after it, as a text file does. Design
choice: the envelope is `{"schema": "sce-test-trace.v1", "suite": <suite>, "test_case": <test case>, "inputs":
[<record>, …]}`, named as the test log names its shape. The inputs are in non-decreasing `at`, and the inputs of one
instant are in the order they are enqueued (TP §3.2 step 2). Design choice: a trace is named by its test case,
`<test case>.json`, in a directory of the traces of one suite, which is what a runner reads (§4); the runtime suite's
live beside its expected logs, `sce-build/tests/fixtures/test_profile/runtime/traces/`. A test case with several
traces has them in several such directories, each with the expected log of each trace (§12).

A trace is refused before the run, on both sides, when:

- a record's `port` is not a system port of the test case's configuration;
- a record's `type` is not in the `in` list of that port's type;
- a record's `value` does not belong to its type;
- the records are not in the order above.

The native runners and the Titan system adapter both read the same file. So the two runs see the same inputs at the
same instants by construction, which is what C3's comparison assumes.

### 10.3 The canonical value form

A trace's values are in the value form of the canonical log, TP §12.5 ("The value form" and "The text"), unchanged.
The form has one definition, there, with the log's schema, `schemas/sce-test-log.v1.schema.json`, and this design
states none of its own. So a value a trace hands the run and the same value in the `message-received` record the run
writes are the same bytes, and a backend's trace player reads the form its log writer writes (§10.5).

A value is read against its type, which the record's `type` names in the manifest, so a text, a byte string and a
variant's name are never mistaken for one another.

### 10.4 The surface

Slice S1 landed the surface as `SCE_WIRE_CONTRACTS.md` requires every surface to land:

- `schemas/sce-test-trace.v1.schema.json`, with `x-sce-schema-status: pre-release`;
- the consts `TEST_TRACE_SCHEMA` and `TEST_TRACE_SCHEMA_STATUS` in `backends/rust/runtime/src/test/trace.rs`, the
  module of the Rust runtime that reads a trace (`Trace::parse`) and plays it (§10.5); a trace's reader is a part of
  each runtime, as the log's writer is, and the Rust runtime's is the authoritative one;
- a row of `SCE_WIRE_CONTRACTS.md`:
  - Format owner: SCE;
  - Status source: `TEST_TRACE_SCHEMA_STATUS` (`backends/rust/runtime/src/test/trace.rs`) and
    `x-sce-schema-status`;
  - Status: `pre-release`;
  - Shape governance: this document §10, and `backends/rust/runtime/src/test/trace.rs`, held by
    `sce-build/tests/a_test_trace_is_held_to_its_wire_schema.rs`;
  - Producers: the corpus's traces, written by hand, and the downstream's recorder (DD §8.1);
  - Readers: the native trace players (§10.5) and the downstream's trace player.

The reader refuses a text that is not JSON, not of the shape, out of the order of §10.2, or not the trace's one
canonical spelling; whether each input fits the test case, the other refusals of §10.2, is read by the test case's
configuration as the trace is played (§10.5). SCE's check of a trace against a suite without running it, offered as a
command of `sce-codegen` (design choice: `sce-codegen check-trace <suite> <trace>`, §4), is still to come.

### 10.5 Native trace replay

Every backend's test runtime already has a system adapter shaped after TRI (TP §12.2), and it is pulled by the run:
the instant of the next input, and the inputs due at an instant. Slice S1 adds to each of the seven runtimes a trace
player behind that interface, which reads a trace file, and generates for each record type of a suite the conversion
from the canonical value to the backend's record. The conversion in the other direction is in place: the unit of
every template over a record declares its record's type name and the writer of its value's form, over the walk of a
record's types that every backend's generator shares (`forge::generator::test_template_walk::canonical_value`, TP
§12.5). S1 generates each reader from the same walk, so that one walk decides a type's form in both directions on
every backend. Slice S1 also adds, per backend, a runner program that runs the test cases of a generated suite against
their traces and writes each one's canonical log, which every native runtime writes already (TP §12.5), on the
command line of §4. The downstream's lane calls the runners as processes.

What is in place, on Rust:

- The walk. `canonical_form` of `forge::generator::test_template_walk` is the one walk of the form, generic over a
  direction of it, `CanonicalForm`, which is told what a reader holds a value to and a writer does not read: an
  integer's width, a text's, a byte string's and a list's bounds, and the record a set of fields builds. A backend's
  writer, `CanonicalSpelling`, is one direction through the adapter `Writing`, and `canonical_value` is that walk, so
  the writers of the seven backends are unchanged. A backend's reader is the other direction: a sibling of its writer
  over the same walk (Rust: `RustReading`, beside `RustCanonical`).
- The readers. A record is an `event-schema` unit, which names no runtime (TP §12.2) and is compiled where the test
  profile's runtime is not, so the reader is not in it. A template over the record is no home either: a record may
  have several templates, or none, and the trace player needs one reader per record a system port takes. The unit of
  the test configuration is that home: it is on the SCE runtime, it declares the system ports, and the `in` list of
  each port's type names the records a trace may carry on it, which the configuration reads through its import of
  the port type (a port type writes no unit). So the configuration's unit declares, per record a system port takes,
  `read_<record>`, which reads the record from its canonical form or refuses it by name, and `canonical_<record>`,
  its writer, both over the walk, and `input(system_port, type_name, value)`, which refuses a port that is no system
  port of the configuration and a type the port's `in` list does not name, and otherwise reads the value into the
  message the run delivers. The test case's unit hands `input` to the runner (`TestCase::inputs`). The readers of each
  form are `sce_rust_runtime::test::from_canonical`, the inverse of `sce_rust_runtime::test::canonical`, each of
  which accepts the one spelling its writer gives.
- The player. `TraceAdapter` (`backends/rust/runtime/src/test/trace.rs`) is a system adapter: it reads every input of
  a trace through the configuration's `input` before the run, refusing the whole trace where one does not fit; at each
  instant it delivers the inputs due then, in the trace's order (TP §3.2 step 2); it announces the instant of its next
  input, which the runner's clock advances to when it is the earliest of that instant, the next timer expiry and the
  guard (TP §3.3, §12.4); and it records what the components send to the system ports.
- The runner. `run_suite_with_traces(suite, world, traces, overrides)` runs the test cases a world selects, each
  against the trace `traces` gives for its name, and returns each one's result and canonical log; a test case whose
  trace is not given, is of another test case, or does not fit ends with `error` (P13) before it starts, naming why.
  The program of §4 is the suite unit's `main`, over `sce_rust_runtime::test::runner`.
- The tests. The trace of `answers_the_job`, `sce-build/tests/fixtures/test_profile/runtime/traces/answers_the_job.json`,
  is the input the Rust suite runner's test adapter gives that test case, as a file; the generated `runtime_suite`
  replayed from it writes `expected_logs/answers_the_job.json` byte for byte, through `run_suite_with_traces` and
  through the program (`backends/rust/tests/tests/a_test_suite_runs_through_its_runner.rs`). The configuration of
  `sce-build/tests/fixtures/test_profile/trace_forms/` takes on its system port a record of every form of the
  canonical value (`order_placed`, `job_status` and `form_extras`), so its unit's reader and writer of each form are
  compiled; a value of each reads back from the text its writer gives, a malformed spelling of each is refused by
  name, and a trace of the three replays to a log that holds each value as the trace does
  (`backends/rust/tests/tests/a_record_reads_back_the_form_it_writes.rs`). The generated readers are written in
  shapes clippy's `-D warnings` does not refuse, with no `allow`: a value that may be absent is an `Option` mapped to
  its read and transposed, a union a `match` whose last arm is its last case, a list a `for` loop.

On Kotlin the same is in place: `com.sce.runtime.test.FromCanonical`, `Trace`, `TraceAdapter`, `runSuiteWithTraces`
and `runRunner` (`runnerMain` on the JVM) in the runtime, `KotlinReading` beside `KotlinCanonical`, and the Kotlin
twins of the Rust tests (`ATestSuiteRunsThroughItsRunnerTest.kt`, `ARecordReadsBackTheFormItWritesTest.kt`,
`TraceTest.kt`), which replay the same trace to the same log byte for byte; a reader throws a `TraceRefusal` in the
words of the Rust one.

On C++ the same is in place: `sce/include/test/` (`Trace.h`, `FromCanonical.h`, `runSuiteWithTraces` of `Suite.h`,
`Runner.h`) in the runtime, `CppReading` beside `CppCanonical`, and the C++ twins of the Rust tests
(`tests/test_runtime/TraceTest.cpp`, `RunnerTest.cpp`, and `ATestSuiteRunsEndToEndTest.cpp`,
`ARecordReadsBackTheFormItWritesTest.cpp` and `ARunnerProgramWritesTheLogOfEachTraceTest.cpp` under
`tests/integration/`), in the words of the Rust reader. Its configuration of every form,
`trace_forms/unionless_forms_configuration.scxml`, takes `order_taken` in `order_placed`'s place, since C++ lowers no
union.

On Go the same is in place: the readers of each form beside the writers in the package `testprofile/canonical`
(`from_canonical.go`, each answering an error in the words of the Rust reader), `ParseTrace`, `TraceAdapter`,
`RunSuiteWithTraces`, `RunRunner` and `RunnerMain` in `testprofile`, `GoReading` beside `GoCanonical`, and the Go twins
of the Rust tests (`trace_test.go`, `runner_test.go` and `canonical/from_canonical_test.go` in the runtime;
`a_test_suite_runs_through_its_runner_test.go` and `a_record_reads_back_the_form_it_writes_test.go` under
`backends/go/tests/integration/static_datamodel/`), which replay the same trace to the same log byte for byte, in
place and through the program as a process. A Go function literal names the types it answers, so each generated read
is an expression of the value and an error, a record, a list and a value that may be absent a function literal called
where it stands. Go lowers no union, so its configuration of every form is the one C++ reads,
`trace_forms/unionless_forms_configuration.scxml`.

The trace replay and the runner program are in place on Rust, Kotlin and C++. Go, Python, C11 and Swift follow the
same design: the reader of each form in the runtime beside its writers, the reader of each record and `input` in the
unit of a configuration over the same walk, a player behind the system adapter, the runner function and the program
of §4.

## 11. The lane

The lane runner's steps, why a disagreement between the two runs is never resolved, the canonical order of a log's
records, which backends take part, and where the lane runs, SCE's CI job among them. This section lives in
`docs/DESIGN.md` of https://github.com/newmassrael/sce-ttcn3-titan, under the same number. On SCE's side every one of
the seven native runtimes writes the log the lane compares, held to one expected log (§13.3), so each can take part
once its runner replays a trace (S1).

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
- **One test case has an expected log, and no suite has a trace.** The run of `answers_the_job` of `runtime_suite`,
  in a world with no feature and answered with success by the tests' system adapter, is held on all seven backends to
  `sce-build/tests/fixtures/test_profile/runtime/expected_logs/answers_the_job.json` (TP §12.5). Its input comes from
  that adapter, not from a trace. No mapped message type of any suite has a codec document (DD §8.4).

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

- The schema, the const and the row of §10.4, and `sce-codegen check-trace`. The trace's values take the form TP
  §12.5 defines (§10.3).
- The trace player and the generated value conversions of every backend, and each backend's runner (§10.5).
- Tests: per backend, a trace of `runtime_suite` replays to the verdicts its test cases expect, and `check-trace`
  refuses each case of §10.2.
- Files: `schemas/`, the command in `sce-build/src/bin/sce_codegen.rs`, `SCE_WIRE_CONTRACTS.md`,
  `docs/SCE_TEST_PROFILE.md`, the seven test runtimes and their generators, and the generated test trees those
  generators write.
- In place: the surface (§10.4), the shared walk of both directions and the Rust backend (§10.5): its readers, its
  player, its runner and its program, held by the trace of `answers_the_job` to the expected log. To come: the other
  six backends, after the Rust one, and `sce-codegen check-trace`.

**S2: a corpus with traces and expected logs.**

- The additions of §12.4.
- Tests:
  - per backend, every trace of the corpus replays to its expected log, byte for byte;
  - a test of SCE holds the corpus to ADR 0091 §2 and §3: each row and each refusal names the suite or the fixture
    that covers it.
- The expected logs are written in `sce-test-log.v1`, whose schema and native writers are in place (§13.3). The
  traces they are written for are replayed through S1. The codec documents depend on neither, and can land first.
- Files: the corpus fixtures, their traces, expected logs and codec documents, and the test that holds the coverage.

### 13.2 In the downstream

Slices D1 to D5: the skeleton, its licence and the adapter interface; the pinned Titan build and the Main Controller
patch; the reference adapters and the codecs; the binding and the logger; the lane and C3. This section lives in
`docs/DESIGN.md` of https://github.com/newmassrael/sce-ttcn3-titan, under the same number.

### 13.3 Order of work step 6's log is in place

Step 6's canonical log is in place: its schema, `schemas/sce-test-log.v1.schema.json` (`pre-release`), its row of
`SCE_WIRE_CONTRACTS.md`, and its writers in all seven native runtimes (Rust, C++, Kotlin, Go, Python, C11 and Swift),
each held to one expected log, `sce-build/tests/fixtures/test_profile/runtime/expected_logs/answers_the_job.json`
(TP §12.5). What S2's expected logs, D4's logger and D5's comparison (DD §13.2) need from step 6 is therefore there,
and none of them waits on it. What remains of C3 is the TTCN-3 side, D4's logger plugin writing the same log from
Titan's events and D5's comparison of the two logs, and the corpus they run over, S2's traces and expected logs
replayed through S1. Step 6 stays the step that holds the law over the whole corpus, which D5 applies to S2's corpus.

## 14. What this design leaves open

- **The names of -5 and -6.** DD §7.2 is checked against the two texts in slice D1 (DD §13.2).
- **SCE's own codecs in the lane.** In the lane the native side holds values and uses no codec (TP §12.2), so only the
  TTCN-3 side encodes. Comparing SCE's generated codecs with the reference bindings, byte for byte on the same values,
  would be a check of its own. D3's tests decode SCE's bytes for a few shapes. A check over the corpus is not part of
  this design.
- **Positional binary codecs.** They have no reference binding (DD §8.4).
- **A second TTCN-3 tool.** Running the suite on a second tool, by a binding of the same interface, remains the
  stronger form of C1 (answer 1). It is not planned.

Closed since the design was written: **where the canonical value form is owned.** TP §12.5 owns it, with the log's
schema, and §10.3 refers to it for the trace's values. The form there writes a byte string in upper-case hex. This
design's earlier statement of the form, which wrote it in lower-case, is withdrawn.
