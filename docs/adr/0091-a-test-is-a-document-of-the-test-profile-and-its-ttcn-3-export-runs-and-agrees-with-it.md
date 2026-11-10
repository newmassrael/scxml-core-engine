# ADR 0091 — A test is a document of the test profile, and its TTCN-3 export runs and agrees with it

- Status: Accepted (2026-11-06), with each question answered in "Answers (2026-11-06)". The owner ordered the
  questions decided by the rule of 2026-10-26 and 2026-10-27 (ignore the cost; decide for the structure that is the
  most correct in the long run, the ultimate one, and textbook in its academic foundations); the answers below are the
  decisions taken under it, each with its basis.
  History: a downstream consumer session drafted it at the owner's request
  under the owner's rule of 2026-10-26 and 2026-10-27: the most long-term correct structure, textbook in its
  foundations, cost ignored. The drafter wrote it as 0083 against a checkout 681 commits behind `origin/main`. It was
  renumbered 0091 because ADRs 0083 to 0090 had already landed, and it was reviewed against `origin/main` at
  `fc84cf9227`. The review changed what it said about the tree: child sessions, the backend count, existing names,
  and the existing diagnostic for an undeclared feature. It also added the constraints that ADRs 0084 and 0090 place
  on ports, and five questions (6 to 10). The decision is the drafter's and is unchanged.
  Amended the same day, at the owner's request, after the owner asked whether the design is the textbook one. It
  was not yet. §1 now makes the syntax-translation claim a theorem the work must state and check, rather than an
  assertion. §2.1 states what the subset leaves out. Alternatives F (the UML Testing Profile) and G (tests derived
  from the specification model under ioco) are recorded, and questions 11 and 12 added.
  Answers 1, 3 and 8 amended the same day. As first accepted, answer 1 required a tool that implements the
  standard TRI and TCI, and the one such tool found is commercial. The owner stated that no licence for a TRI/TCI
  tool can be obtained, now or later. The lane therefore runs on Eclipse Titan, an open-source implementation of
  TTCN-3 independent of SCE, and C1's meaning is kept by the adapter interface of answer 1, not by the tool's own
  interfaces. Answer 3 now says how the logical clock is supplied inside Titan; answer 8 says where Titan's log
  comes from.
  Amended again the same day after a probe on Eclipse Titan 11.2.1, built from source. A hand-written suite
  using the profile's constructs ran in Titan's parallel mode with the expected verdicts. The probe corrected
  two statements of the previous amendment: Titan's own encoding engines cannot write either of SCE's codec
  formats (answer 1), and Titan reads the time at two functions, not one (answer 3). It also showed what
  Titan's logger passes for a message value (answer 8). Answer 1 now lists the licence facts that were checked.
  Amended once more when the operational semantics was written (`docs/SCE_TEST_PROFILE.md`, order of work step 1).
  That specification found two things the decision needs:
  - a scheduler: -4 leaves the order of components free, and C3 needs one run (answer 3);
  - refusals for SCXML constructs that have no TTCN-3 meaning in a test component (§3).

  Amended when the document forms were written (`docs/SCE_TEST_PROFILE.md` §11, order of work step 2). A test case's
  feature selection is written on its entry in the suite, where ADR 0057 lets an `sce:` element carry `requires` and
  where TTCN-3's control part writes it. Module parameters are declared by the suite (§2). The type grammar the
  templates match now includes the lists of ADR 0086 and the unions of ADR 0092; `float32` and `float64` are left out
  of the profile until a rule matches IEEE 754 values against -1's `float`.

  Amended 2026-11-09 when the exporter's types module and manifest were written (`docs/SCE_TEST_PROFILE.md` §9.5,
  order of work step 4). C2's stable name contract is identity: every name the export writes is the documents' own,
  and one TTCN-3 cannot take is refused (`test-profile/export-name`), never renamed. The external codec functions
  have one stated form, and the manifest is a wire surface of its own.

  Amended 2026-11-10 when the design of order of work step 5 was written (`docs/SCE_TTCN3_LANE.md`), on the owner's
  delegation of that day to the main session. Everything that touches Titan moves to a separate downstream
  repository under EPL-2.0, `sce-ttcn3-titan`: the adapter interface, the reference adapter set and its codec
  bindings, the Titan binding, the pinned Titan with its Main Controller patches, and the lane runner. It uses SCE
  only as a tool, through SCE's programs and published wire surfaces. SCE keeps the export, the validator, those
  surfaces, the native runners with trace replay, and the corpus with its traces and expected logs. Answer 1's
  licence question is answered by that split (answer 1), §7 says which repository ships what, and step 5 is split
  into slices.
- Date: 2026-11-06
- Scope:
  - a test profile of SCE documents: new forge kinds, new executable content, a verdict model, port and timer
    semantics, test components;
  - its validation rules and `DiagnosticCode` variants (`SCE_ERROR_CONTRACT.md`, the acceptance-doc appendix of
    `docs/SCE_ACCEPTED_SUBSET.md`);
  - a native test runtime, emitted per backend case by case;
  - a TTCN-3 exporter that is not a backend;
  - a reference adapter set and a conformance lane that executes the export;
  - an equivalence law between the native run and the TTCN-3 run, and the canonical log it compares, a new wire
    surface registered in `SCE_WIRE_CONTRACTS.md`.
- Related:
  - ADR 0003 (one portable document, compiled per target);
  - ADR 0005 (`sce-static` closes its gaps by bounding and declaring);
  - ADR 0043 (a child session that talks to Mesh peers is a machine of the deploy), only for a test component that
    talks to Mesh peers;
  - ADR 0057 (an element may require a feature, and every world of the features is a valid machine);
  - ADR 0064 (Appendix D runs once per program);
  - ADR 0081 (a codec's encode is planned once);
  - ADR 0084 (what reaches an engine from outside enters through one inbox and one processor door);
  - ADR 0089 (a run tells each transition it takes by its row);
  - ADR 0090 (an event origin is the reply address its processor writes);
  - ADRs 0086 and 0092 (an event field may hold a bounded list, or one of several record shapes chosen by its tag);
  - `ARCHITECTURE.md` Key Principles 1, 2, 5, 7, 8, 9;
  - `docs/SCE_CODEGEN_DETERMINISM.md`, `docs/SCE_ACCEPTED_SUBSET.md` (§2.18, the scenario set),
    `SCE_WIRE_CONTRACTS.md`, `SCE_ERROR_CONTRACT.md`.

## Context

Downstream consumers already write conformance tests as SCE documents. ADR 0003 measured the shape they take: one
harness holds 432 documents that pair `cond="cpp:…"` with `datamodel="null"` and return their verdict through inline
`<donedata><content>` text such as `{"verdict":"pass"}`. The case's message types are C++ types brought in through
`cpp:type` contexts. Its stimulus and expected values live in a C++ traits header beside the document.

That shape works, but it has five properties a test notation must not have:

1. **The verdict is a convention, not a language element.** Nothing checks that every final state carries a verdict,
   that a verdict comes from a closed set, or how the verdicts of two components combine.
2. **Matching is host code.** The condition that decides whether a received message is the expected one is a C++
   expression. The expected values therefore live outside the document, and the test cannot be read or reviewed as one
   artefact.
3. **Message types are host types.** The document depends on the test system's implementation language.
4. **Every point of control and observation shares one queue.** SCXML has one external event queue per session, so the
   order in which events from two interfaces are seen is whatever order an adapter enqueued them in.
5. **Only SCE can read or run it.** No conformance laboratory, customer or second implementer can review the suite in a
   standard notation or execute it on another tool. ISO/IEC 9646 asks for an abstract test suite to be validated by an
   independent party, and that is not possible.

A further problem follows from SCE's own reach. Consumers also generate the codecs of the implementation under test
(IUT) with SCE (`sce:kind="codec"`). If the same generator also produces the tester's codec, a generator defect is
present on both sides of the wire and is invisible to the test. A tester whose every codec comes from SCE cannot
remove that common cause.

The field already has a standard answer to all six problems: TTCN-3 (ETSI ES 201 873). Its core language (part 1) has
a verdict model, templates, typed ports with their own queues, timers, defaults and parallel test components. Its
operational semantics (part 4) define exactly when a receive matches. Its runtime interface (TRI, part 5) and control
interface (TCI, part 6) separate the test behaviour from adapters and codecs. ISO/IEC 9646 gives the surrounding
method: conformance requirements, test purposes, abstract and executable test suites, PICS and PIXIT, and the verdicts
pass, fail and inconclusive.

### What SCE already has, and how this differs

The tree already has one surface that judges behaviour: the scenario set (`docs/SCE_ACCEPTED_SUBSET.md` §2.18,
`schemas/sce-scenario-set.v1.schema.json`). It works with the observation trace
(`schemas/sce-observation-trace.v1.schema.json`) and `sce-codegen judge-scenarios`, whose `Verdict` is `pass`, `fail`,
`not-judged`, `blocked` or `awaiting-decision` (`sce-build/src/scenario_judge.rs`). A scenario set is a
specification owner's examples, judged against a *design*, and SCE never runs it. The test profile is an executable
*tester* of an IUT. The two are different artefacts with different verdict vocabularies. This ADR does not merge them.
The profile's verdicts are TTCN-3's (§4) and are always called test verdicts, so that the word "verdict" does not
silently mean two things. Whether a scenario set can be lowered into test cases is question 7.

## Decision

**SCE gains a test profile whose semantics are TTCN-3's, restricted to a subset that is idiomatic in both languages.
A document of the profile is the single source. SCE runs it natively and exports it deterministically to TTCN-3, and
the export is executed, not only read. The two runs must agree.**

### 1. Semantics are adopted, not invented

The behaviour semantics of the profile are the operational semantics of ETSI ES 201 873-4, restricted to the constructs
listed in §2. Where SCXML and TTCN-3 differ, the profile follows TTCN-3 and the validator refuses the SCXML construct
that would contradict it. The profile is therefore meant to be a second syntax for a subset of TTCN-3 behaviour, with
an exporter that translates syntax, not semantics.

That sentence is a claim about two semantics, and nothing makes it true by being written. The decision therefore
includes its proof obligation:

- **The profile has a written operational semantics.** It is defined over the same labelled transition system that
  ES 201 873-4 uses for the adopted subset, not in prose alone. Every row of §2 is a rule of that semantics and
  cites the TTCN-3 clause it adopts.
- **The exporter is a function from profile documents to TTCN-3 modules, and its correctness is a stated theorem:**
  for every profile document, the exported module and the document have the same observable traces. Those traces are
  the messages sent and received on each port, the timeouts, and the final test verdict per test case.
- **The theorem is checked per export (translation validation).** The textbook check for a code generator that has no
  machine-checked proof is translation validation: each run of the translator is followed by a check that the output
  agrees with the input (Pnueli, Siegel, Singerman, *Translation Validation*, TACAS 1998, LNCS 1384). C3 (§6) is the
  executed half of that check, over recorded traces. It is evidence, not proof, and is not counted as proof. Whether
  a static check of the module against the document's semantics is added, or the theorem is proved once for the
  exporter, is question 12.

The equivalence law of §6 can be decided only on these terms. Without them it is merely hoped for.

### 2. What the profile contains

The names are those of answer 5: every new kind carries the prefix `test-`, and a TTCN-3 operation that SCXML already
has is written with the SCXML element.

| Concept (TTCN-3) | Profile form | Notes |
|---|---|---|
| Type definitions | existing `event-schema`, `enum` | exported as `record`, `union`, `enumerated`, with optional fields and value/length constraints |
| Template | new forge kind `test-template` over an `event-schema` type | specific values, `?`, `*`, ranges, length restrictions, `omit`, `ifpresent`, value lists, complement; parameterised templates |
| Port type | new forge kind `test-port-type` | message kinds per direction (`in`, `out`, `inout`) |
| Component type | new forge kind `test-component` | a statechart restricted per §3; declares its ports, timers and variables |
| Test configuration | new forge kind `test-configuration` | the MTC type, the test system interface, PTCs, `connect` and `map` of ports (answer 10) |
| Test case | new forge kind `test-case` | names its configuration and runs on its MTC type; the features it requires are stated by its entry in the `test-suite` |
| Module parameters (PICS, PIXIT) | existing `feature-model` (ADR 0057) for PICS booleans; `<sce:module-parameter>` declarations of the `test-suite` for numeric PICS items and PIXIT | a test case's feature requirement is its selection expression, written on its `<sce:execute>` as TTCN-3 writes it in the control part |
| Control part | new forge kind `test-suite` | the ordered list of test cases, their selection and each one's guard timer (answer 4); exported as the module's `control` part |
| `setverdict` | new executable content `<sce:setverdict value="pass\|inconclusive\|fail\|none" reason="…"/>` | the document cannot set `error` (as in TTCN-3); only the runtime sets it |
| `alt` with receive branches | a state of a `test-component`; each `<transition sce:receive="port" sce:test-template="t">` is a branch | branches are evaluated in document order over one snapshot (§5) |
| `timer` | `<sce:test-timer>` declarations; `<sce:start-timer>`, `<sce:stop-timer>`; `sce:timeout="t"` as a branch | one-shot, unlike the periodic `timer` kind (`SCE_FORGE.md` §4.10) |
| `altstep` and `activate` | transitions of ancestor states | the innermost ancestor is the most recently activated default (TTCN-3 evaluates defaults in reverse order of activation; §2.1 states what this leaves out) |
| `send` | SCXML `<send>` whose `type` is the test-port processor and whose `target` is the port, with `sce:test-template="t"` | the template must be specific (no matching symbols); the send leaves through ADR 0084's processor door (answer 6) |
| PTC `create`, `start`, `done`, `kill` | SCXML `<invoke type="scxml">` (§scxml-6.4): the invocation creates and starts the PTC, `done.invoke.<id>` is `done`, and leaving the invoking state is `kill` | a PTC is a child machine; the runtime, not the document, returns its local verdict to the MTC. `create` and `start` are not separable (§2.1). ADR 0043 applies only to a PTC that talks to Mesh peers |
| `log` | SCXML `<log>` | exported as TTCN-3 `log` and recorded through the test logging interface |

### 2.1 What the subset leaves out, and why

A restriction that is not written down reads as an omission. Each of the following is outside the profile, and a
document cannot express it.

- **Procedure-based communication.** TTCN-3 ports are message-based or procedure-based (`call`, `getcall`, `reply`,
  `getreply`, `raise`, `catch`). The profile has message-based ports only. A request and its reply are two messages
  of an `inout` port type, and a test that needs the blocking-call semantics of a procedure port cannot be written.
- **Defaults activated anywhere.** TTCN-3 keeps a list of defaults in order of activation. Any statement may
  `activate` an altstep or `deactivate` a default, and the alternatives of the list are tried in reverse order of
  activation. The profile has only defaults whose activation follows the nesting of states. A default is the
  transitions of an ancestor state, active while that state is, and the innermost is tried first. A default that
  stays active after the state that activated it is exited, or one deactivated in the middle of a state, cannot be
  expressed.
- **`interleave`.** An interleaved statement is a set of alternatives taken in every order. In a statechart without
  `<parallel>` it has no direct form, and its expansion grows factorially with the number of alternatives.
- **Concurrency inside a component.** The profile refuses `<parallel>` (§3); TTCN-3 has none inside a component
  either.
- **A PTC created and started apart.** TTCN-3 may `create` a component and `start` a function on it later, or start
  several functions on it in turn. An SCXML invocation does both at once and runs one machine, so a PTC runs one
  behaviour from its creation to its end.

For each item, the specification of order of work step 1 says whether it is out by design or deferred. A deferred
item returns only through an amendment of this ADR that states its semantics and its export.

### 3. What the profile refuses

Each of the following is refused at build time with a `DiagnosticCode`. Each new variant lands with its acceptance-doc
row (`acceptance_doc_covers_every_code`):

- any native expression (`cpp:`, `kt:`, or a future prefix) anywhere in a profile document. ADR 0003 makes the escape
  hatch inadmissible where portability is the purpose;
- any construct that raises a script-engine cause. The generate manifest of a profile document must show
  `needs_script_engine = false`;
- a profile document whose `datamodel` is not `sce-static` (answer 9);
- a `cond` that reads a received message. Whether a message matches is decided by a `test-template` and nothing else
  (answer 9);
- `<parallel>` inside a `test-component`. A test expresses concurrency with PTCs, as in TTCN-3;
- `<history>`;
- `<script>`;
- `<donedata>` used as a verdict carrier;
- a receive branch whose template is not one of the port's declared incoming types;
- a test case selected by a feature its suite's `feature-model` does not declare. This is the existing
  `validation/feature-undeclared` of ADR 0057 §5, applied to the suite's `<sce:execute>`. It is not a new
  variant;
- the constructs the operational semantics needs refused in a `test-component` (`docs/SCE_TEST_PROFILE.md` §8.2):
  - `<raise>` and a `<send>` to `#_internal`;
  - a `<send>` whose target is not one of the component's ports;
  - a `<send>` with a delay, and `<cancel>`;
  - an `<invoke>` of anything but a test component;
  - a transition that is not one branch kind;
  - a done branch whose target keeps the invoking state active.

As ADR 0057 requires, every rule is classified by how it behaves under feature derivation, so that every world of the
features of a suite is itself a valid suite.

### 4. Test verdicts

- Each component has a local test verdict, initially `none`.
- `setverdict` follows TTCN-3's overwriting rule: the new local verdict is the greater of the old one and the given
  value, in the order `none < pass < inconclusive < fail`.
- `error` dominates every other value, and only the runtime sets it. It records a test-system fault: an adapter or
  codec failure, a dynamic error, a component that does not terminate within its guard timer, or an equivalence or
  codec disagreement under §6.
- The test case verdict is the greatest over the MTC and all PTCs. The runtime computes it, not the document.

### 5. Ports, snapshots and receive

- Each port has its own FIFO queue. Messages from different ports are never merged into one queue.
- On entering an alt state, the runtime takes a snapshot of all port queues and timer states. It evaluates the branches
  in document order and then the defaults (§2), and fires the first branch that matches.
- A receive examines only the head of its port's queue, and removes the message only when it matches (TTCN-3 receive
  semantics). A head that does not match blocks that branch. Unexpected messages are handled by a default.
- If neither a branch nor a default matches, the runtime waits for the next change of a queue or a timer and takes a
  new snapshot.
- Timers run on the platform adapter's time. In the conformance lane (§7) time is logical, so that a run is a
  function of its input trace.

Two accepted ADRs constrain how this sits in an engine. The profile does not reopen either.

- **Ports are not a second input path (ADR 0084).** Everything that reaches a session from outside enters through its
  one inbox and is drained at a macrostep boundary, on the owner thread. The ports' queues are a distribution of what
  the inbox delivers, not channels that bypass it. Run-to-completion is unchanged: a snapshot is taken between
  macrosteps, never inside one.
- **Each event carries the identity of its port (ADR 0090).** An event that reaches a port carries the `EventOrigin`
  its processor wrote, and that origin names the port (answer 6).

The profile changes Appendix D (ADR 0064) in one place only: how a macrostep picks the event it takes. SCXML takes the
head of one external queue and discards an event that enables no transition. The profile takes the first matching
branch over a snapshot and leaves a message that does not match where it is. Transition selection within a state,
exit and entry order, executable content, the microstep and eventless transitions are W3C SCXML's, and they reuse the
shared core of each engine (Key Principle 5). The runtime code is new only for the event-picking step, the port
queues, the timers and the verdicts.

### 6. Two runs, one test: the three conditions

The export exists so that the suite can be reviewed and executed outside SCE, and so that the tester is no longer a
single version. It achieves that only under three conditions, and all three are part of this decision.

**C1: The export is executed.** The TTCN-3 output must compile and run on a named TTCN-3 tool in SCE's CI (§7). SCE
provides no review-only export: what a reviewer reads must be what runs.

**C2: The codec on the TTCN-3 side is independent.** The exporter emits the suite's types into their own module
(`<Suite>_Types`) under a stable name contract, together with a canonical structural manifest of those types. It does
**not** transliterate SCE's generated codec code into TTCN-3. Encoding is expressed either by the tool's declarative
encoding attributes, or by external encoding functions that the exporter declares and does not implement. A consumer
can therefore replace the module with an independently written types-and-codec module of the same names, and compare
it against the manifest. Consumers are not required to do this, but the exporter must never make it impossible.

The scope of independence is stated, not assumed:

- An independent **encoding engine** removes defects of SCE's codec generator from the comparison.
- An independent **protocol reading** also removes errors of interpretation. It means types written separately from the
  specification rather than exported from the same `event-schema`. Only this second kind removes the common cause
  described in Context completely; the first removes the generator's share of it.
- Neither kind makes the failures of the two sides independent. Knight and Leveson found that programs written
  independently from one specification failed together far more often than independence predicts (*An Experimental
  Evaluation of the Assumption of Independence in Multiversion Programming*, IEEE TSE 12(1), 1986). Agreement of the
  two runs is therefore evidence that both are right only to the extent that their failures are independent. The
  profile never reports it as more than that.

**C3: Disagreement is a gate.** Both runtimes log, per test case, the test verdict and every decoded message in a
canonical form. A run of the same suite against the same input trace must give the same verdict per test case and the
same decoded values per message on both sides. A disagreement is reported as `error` for that test case, and it is
never resolved by choosing one side. The canonical log is a wire surface. It lands with a schema, a row in
`SCE_WIRE_CONTRACTS.md` and the status `pre-release`. Whether it extends the observation trace and the transition
observer of ADR 0089 or stands alone is question 8.

### 7. The conformance lane and its reference adapters

- SCE ships a reference adapter set: a system adapter that connects ports to a trace player and recorder, a platform
  adapter with logical time, and codec bindings for the profile's own conformance corpus. It is written to the
  adapter interface of answer 1 and reaches the lane's tool through that tool's binding. With this set, §6 can be
  checked inside SCE without any consumer's test system.
- A new lane runs the profile corpus natively and through the export, replaying the same recorded input traces, and
  applies C3. This is a conformance law of the profile, held for each backend that emits the native test runtime, as
  other laws are held per lane.
- The corpus covers every row of §2 and every refusal of §3.
- Amended 2026-11-10 (answer 1): the reference adapter set, the Titan binding and the lane runner are shipped by the
  downstream repository `sce-ttcn3-titan`, not by SCE. SCE ships the export, the validator, the wire surfaces both
  read and write, the native runners with trace replay, and the corpus with its recorded traces and expected logs
  (`docs/SCE_TTCN3_LANE.md` §2, §4). SCE's CI runs the downstream lane as an external tool at a pinned revision.

### 8. Where the profile stands among SCE's principles

- The exporter is **not a backend**. It is not in `Language::ALL`, it does not carry Key Principle 1's W3C
  obligation, and Key Principle 2 (never refuse W3C generation) does not apply to it, because it accepts only profile
  documents. `Language::ALL` already has seven entries: the six emitting backends of Key Principle 7, and Swift,
  which writes a statechart in part. Making TTCN-3 an eighth would force the export to embed a statechart interpreter
  in order to pass the W3C suite. The output would then no longer be readable TTCN-3, and there would be no reason left
  to export.
- The profile is an SCE-specific capability outside the W3C parity obligation, as Key Principle 8 places Mesh. Backends
  emit the native test runtime case by case, and a backend without it refuses a profile document at build time, naming
  the backends that serve it (Key Principle 2's scoping).
- The source is the profile document (Key Principle 9). The TTCN-3 export is a derived artefact and obeys
  `docs/SCE_CODEGEN_DETERMINISM.md`. A finding from an external review is fixed in the document, never in the export.

## Consequences

- A test becomes one artefact that can be reviewed: types, templates, behaviour, test verdicts and selection are all
  in documents.
- A conformance suite can be handed to a third party as TTCN-3 and run there.
- The tester can be built as two independent versions (C2). That is the only way to see defects of SCE's own codec
  generator in a test of an implementation whose codecs SCE also generated.
- Each backend that emits the native runtime gains snapshot event selection, per-port queues, test timers and component
  verdicts. It reuses its shared core for everything else (§5).
- Existing test documents that use native conditions and donedata verdicts are not profile documents. They keep
  working under ADR 0003's accounting of the escape hatch, and the existing census counts them. Moving them is
  migration work for consumers.
- SCE's CI gains a dependency on a TTCN-3 tool: Eclipse Titan, built from a pinned release together with the
  logical clock of answer 3 until that clock is part of a Titan release. Since 2026-11-10 it reaches Titan only
  through the downstream repository of answer 1, fetched at a pinned revision and run as a tool.
- The tree gains a second meaning of "verdict" next to the scenario judge's. The profile always calls its own a test
  verdict (Context).

## Alternatives considered

### A — TTCN-3 as an eighth entry of `Language::ALL`

Rejected. Key Principles 1, 2 and 7 would require the TTCN-3 output to pass the W3C suite and to accept every W3C
construct. The output would have to carry a statechart interpreter and would stop being readable TTCN-3.

### B — Author the abstract test suite directly in TTCN-3, outside SCE

Rejected. It gives up three things the profile keeps:

- ADR 0057's check that every world of the features is valid. A TTCN-3 control part selects test cases, but nothing
  checks every selection.
- Generation of an embedded tester from the same source.
- Continuity with the existing corpus of SCE test documents.

### C — Publish the profile as a specification, export nothing

Rejected. It answers review (Context 5) but not execution elsewhere. It gives no validation of the suite by a second
executor, and it cannot remove the codec common cause.

### D — Export for review only

Rejected under C1. A reviewer would read an artefact that nothing runs, and nothing would show when it drifts from what
does run.

### E — Keep the present conventions (native conditions, donedata verdicts) and document them

Rejected. It keeps every property listed in Context 1 to 5, and ADR 0003 already places native expressions as a
counted escape hatch, not a path.

### F — Adopt the UML Testing Profile (OMG UTP 2) as the profile

The UML Testing Profile is the standard answer to "a test profile over a modelling language". Its concepts include a
test context grouping test cases and an arbiter that computes the final verdict from the components' verdicts. Its
relation to TTCN-3 was worked out when it was written (Schieferdecker, Dai, Grabowski, Rennoch, *The UML 2.0
Testing Profile and its Relation to TTCN-3*, TestCom 2003). UTP 2 is formal at 2.0 (December 2018); the current
formal version is 2.2 (August 2024).

Not adopted as the profile. UTP is a profile of UML, and SCE's documents are SCXML. Taking UTP would mean
taking the UML metamodel it stereotypes, or translating its concepts into SCXML anyway. In UTP the behaviour of a
test is written in the UML behaviour it annotates. The profile needs one execution semantics that the export
preserves (§1), and it takes TTCN-3's for that reason. UTP remains the reference for naming: where a concept of the
profile is a UTP concept, question 5 is answered with UTP's name unless a recorded reason says otherwise. §4's
computation of the test case verdict is the UTP arbiter's job.

### G — Derive test cases from the specification model instead of authoring them

Model-based testing gives the textbook answer to what a test case is for. Conformance is a relation between an
implementation and a specification model. In the ioco theory, an implementation conforms when, after every
suspension trace of the specification, its outputs (quiescence included) are among those the specification allows.
Test cases are generated from the specification by an algorithm that is proved to produce *sound* suites (a
conforming implementation never fails) and *exhaustive* ones (every non-conforming implementation is failed by some
generated test). See Tretmans, *Model Based Testing with Labelled Transition Systems*, in *Formal Methods and
Testing*, LNCS 4949, 2008. Test purposes select which part of the specification to test (Jard, Jéron, *TGV: theory,
principles and algorithms*, STTT 7(4), 2005).

Not taken as a replacement, because it does not replace the profile; it sits above it. A derived test case still
needs a notation that runs, exports and reports a verdict, and the profile is that notation. What hand-authoring
alone lacks is soundness. Nothing makes a `fail` written by an author mean that the implementation is
non-conforming. SCE already holds the specification statecharts of the implementations whose tests are written in
it, so both derivation and a soundness check are within reach. They are question 11 and a later ADR, not part of this
decision. This ADR is written so that they do not need to change it: a test case's identity, verdicts and
traces are those a generator would also produce.

## Order of work

1. Write the profile's operational semantics against ES 201 873-1 and -4 (§1): the verdict order, receive and snapshot
   semantics, defaults, timers, components. Every row of §2 cites the TTCN-3 clause it adopts. §5's change to
   Appendix D is written as a change to named steps of W3C SCXML Appendix D. Each item of §2.1 is marked out by
   design or deferred. The equivalence theorem of §1 is stated over these semantics before any exporter code exists.
2. Write the schemas for the new kinds and elements. Extend the parser, model and validator. Add the refusals of §3 with
   their diagnostics and acceptance-doc rows, and classify each rule under feature derivation.
3. Build the native test runtime in one backend first (the one the existing corpus uses), then in the others case by
   case.
4. Build the exporter, deterministic, with the types module and its structural manifest (C2), the correspondence
   witness, and the separate validator that checks it (answer 12).
5. Build the reference adapter set, its Titan binding and the conformance lane (C1, §7). Contribute the logical
   clock of answer 3 to Eclipse Titan, and build the lane's Titan from a pinned release with that change until a
   release contains it. Settle the licence question of answer 1 before the binding is written.
   **Status (2026-11-10):** the design is written (`docs/SCE_TTCN3_LANE.md`), and the licence question is answered by
   the repository split (answer 1). The downstream repository `sce-ttcn3-titan` is to be created. The SCE-side slices
   come first: S1, the trace surface and native trace replay; S2, a corpus with traces and expected logs. The
   downstream slices follow: D1, the skeleton, its licence and the adapter interface; D2, the pinned Titan build and
   the Main Controller patch; D3, the reference adapters and codecs; D4, the binding and the logger; D5, the lane
   and C3.
6. Add the equivalence law over the corpus (C3), with the canonical log's schema and its `SCE_WIRE_CONTRACTS.md` row.
7. Write migration guidance for consumers' existing test documents.

## Questions (answered below)

1. **Which TTCN-3 tool the lane uses**, and whether its adapter interface is the standard TRI/TCI or a tool-specific
   one. Eclipse Titan is the obvious open-source candidate. Nobody has checked whether it implements the standard TRI
   and TCI mappings or only its own test-port API. If the standard interfaces are required, the choice of tool follows
   from that.
2. **Which edition of ES 201 873** the profile targets.
3. **Logical time in the lane**: how a recorded trace fixes timer outcomes so that both runtimes see the same timeouts.
4. **Whether `error` from a guard timer** belongs to the runtime or to the configuration.
5. **Names of the new kinds and elements.** The names in §2 are placeholders that match TTCN-3's terms. `template`
   collides with `<sce:template>` and is close to `text-template`. `<sce:timer>` collides with the `timer` kind. Also
   open: whether `<sce:log>` and `<sce:send>` are needed beside SCXML's `<log>` and `<send>`, or whether the standard
   elements, given a port, are enough.
6. **What names a port in an engine**: the `EventOrigin` of ADR 0090 (a port as an Event I/O Processor address,
   §scxml-C), or a field of its own on the event.
7. **Whether a scenario set (§2.18) can be lowered into test cases**, so that a specification owner's examples become
   executable tests of an implementation. If so, how its five judge verdicts map onto test verdicts.
8. **What the canonical log of C3 is**: an extension of the observation trace and of the transition observer of
   ADR 0089, or a surface of its own.
9. **Which data model a profile document declares.** The profile refuses native expressions and script-engine causes,
   but a component holds variables and templates match values. `datamodel="sce-static"` (ADR 0005) is the candidate
   that needs no engine. Whether its expression language can express template matching and verdict reasons, or the
   profile needs constructs of its own, has not been checked.
10. **How a PTC's ports reach the MTC's.** `connect` between components runs over the invoke channel (`#_parent`,
    `#_<invokeid>`). Each side's port queues then distribute what that channel delivers (§5).
11. **Whether SCE derives test cases from a specification statechart under ioco (alternative G).** Derivation would
    use quiescence and test purposes. A weaker step is to check a hand-written test case for soundness against the
    specification: a `fail` must be reachable only on a trace the specification does not allow. Either step needs a
    reading of an SCE statechart as an input-output labelled transition system, with its inputs and outputs and its
    quiescence defined. That reading does not exist today and would be a decision of its own.
12. **How the equivalence theorem of §1 is held.** The options are: by C3 alone (executed agreement over recorded
    traces); by a static translation validation of each exported module against its document; or by a proof made
    once for the exporter. The recommendation is the second together with C3, because neither alone covers what the
    other misses: a static check sees every trace of the document but not the tool, and an executed run sees the tool
    but only the traces it replays.

## Answers (2026-11-06)

Each question was decided by the owner's rule: the most long-term correct structure, textbook in its foundations,
cost ignored. The facts each answer rests on were checked on 2026-11-06 and are cited with it.

1. **The lane runs on Eclipse Titan. SCE's adapters are written to an adapter interface SCE owns, whose operations
   are those of TRI (ES 201 873-5) and TCI (ES 201 873-6) one for one and under their names, and each tool is
   reached through a binding of that interface.** C1 asks for two things: an executor of TTCN-3 that SCE did not
   write, and a suite that is not tied to one executor. The tool supplies the first. The second is supplied by
   where the adapters' semantics live: in the standard's operations, not in one tool's API.
   - **The tool.** Eclipse Titan (EPL-2.0) is a TTCN-3 compiler and runtime written independently of SCE, so a
     run on it is a second executor in the sense of C1. Its "TTCN-3 Limitations" chapter (Reference Guide) was
     checked against the profile: it restricts none of templates, message ports, timers, altsteps used as
     defaults, PTCs or verdicts. It requires a component type's compatibility and a port type's message list to be
     stated explicitly; the exporter always states both. A probe on Titan 11.2.1 executed the constructs in
     parallel mode, the mode a suite with PTCs runs in:
     - PTC creation, `connect` and timers;
     - a default that fails on an unexpected message;
     - templates with `?`, `*`, `omit`, `ifpresent`, ranges, value lists, `complement` and `length`;
     - `map` through a test port;
     - `execute` with a time limit, which ended in `error` assigned by the runtime (answer 4).

     Every verdict was the expected one.
   - **Why an interface of SCE's own.** Titan does not implement TRI or TCI. Its adapters are test ports written to
     its own C++ Test Port API, and its Programmers' Technical Reference (11.2.1, 2026-08-05) cites parts 1, 4, 7
     and 9 of ES 201 873 but not parts 5 or 6. Writing the adapters to that API would make "the suite runs
     elsewhere" mean "the suite runs on Titan". The interface keeps TRI's system and platform adapter operations
     and TCI's codec, test logging and test control operations, so the adapters carry the standard's meaning and
     a binding carries only the translation to one tool.
   - **The Titan binding.** The system adapter's send and enqueue operations become a Titan test port per mapped
     port type (outgoing send, incoming message). A `connect` between components (answer 10) is Titan's own
     component-to-component connection and does not pass through the binding. The platform adapter's timer
     operations have no binding in Titan, because Titan keeps timers inside its runtime; the logical clock of
     answer 3 is supplied there instead. Codecs are external functions (C2, below). The test logging operations
     are a Titan logger plugin (answer 8).
   - **C2's codecs are independent libraries, not Titan's engines.** Titan's own encoding engines (RAW, TEXT,
     BER, XER, JSON, OER) cannot write either format SCE generates codecs for:
     - **CBOR.** SCE writes maps with integer keys and byte strings (`SCE_FORGE.md` §4.6.1). Titan reaches CBOR
       only through JSON (`json2cbor`), which writes every key as a text string and a byte string as hex text.
       The probe encoded the shape of `codec_cbor_map` and got `A3 62 "id" 78 20 "0001…"`, where SCE writes
       the key as `00` and the value as `50` followed by sixteen bytes.
     - **content-line.** SCE's content-line codec is one RFC 5545 component (`SCE_FORGE.md` §4.6.4). It folds
       lines at 75 octets and escapes TEXT values, and Titan's TEXT engine does neither.

     The types module therefore declares external encoding and decoding functions, and the binding implements
     them on codec libraries that neither SCE nor Titan wrote: a CBOR library and an iCalendar library. Their
     authors share nothing with SCE's codec generator, so this is C2's first kind of independence, an
     independent encoding engine. The choice of library is pinned with the lane: libcbor and libical
     (`docs/SCE_TTCN3_LANE.md` §8.4).
   - **A tool that implements TRI and TCI natively** is admitted by the same adapters through a binding that is
     the identity. Elvior TestCast states TRI and TCI mappings for C, C++, C# and Java; it was the first answer's
     tool and is not used because its licence cannot be obtained. Running the suite on a second tool remains the
     stronger form of C1.
   - **Rejected for this answer:** an executor of TTCN-3 written by SCE, because it would share its author and its
     reading of the standard with the native runtime and bring back the common cause that C2 and C3 remove; and
     export without execution, which is alternative D.
   - **Licence: the facts checked, and what is still to be decided.** No legal review has been made. The facts:
     - Titan's files carry EPL-2.0 alone, with no Exhibit A notice naming a Secondary License, so EPL-2.0 is not
       made compatible with the GPL family for Titan.
     - SCE's code is AGPL-3.0 with the SCE Linking Exception, or the commercial licence, and has one copyright
       holder.
     - EPL-2.0's obligations arise when the program is distributed. AGPL-3.0's arise when it is conveyed or
       offered to users over a network.
     - Titan is used only by the conformance lane: in SCE's CI, and by a developer who runs the lane. No SCE
       product contains Titan. An exported TTCN-3 module is text SCE writes and contains no Titan code.

     So the lane itself distributes nothing. One case remained: SCE publishes the binding's source, and a consumer
     builds it with Titan and distributes the result. As first accepted, this answer left open the licence of the
     binding and the reference adapter set, to be settled before the binding is written (order of work step 5).

     **Decided 2026-11-10: the lane is a separate repository, and no SCE file is relicensed.** Everything that
     touches Titan lives in a downstream repository, `sce-ttcn3-titan`, published under EPL-2.0: the adapter
     interface, the reference adapter set, the codec bindings, the Titan binding, the pinned Titan build with its
     Main Controller patches, and the lane runner. It never includes, links or copies SCE code. It uses SCE only as
     a tool: it runs `sce-codegen export-ttcn3`, the validator and the native runners as processes, and reads and
     writes SCE's published wire surfaces (the exported modules, the types manifest, the witness, the recorded input
     trace `sce-test-trace.v1` and the canonical log `sce-test-log.v1`). It pins the SCE revision it runs. SCE's CI
     may run the downstream lane as an external tool fetched at a pinned revision; that is use, not combination.
     The record, each statement with its basis (`docs/SCE_TTCN3_LANE.md` §3):
     - **The split removes the combination question.** No SCE file is relicensed. The downstream holds no SCE code
       and is a separate work. No executable combines SCE's AGPL-3.0 code with Titan's EPL-2.0 code, so the
       Secondary License that Titan's notices do not name is never needed. The exported modules are the
       generator's output, governed by `LICENSE-GENERATED.md` as all its output is.
     - **CI-only use distributes nothing.** EPL-2.0's obligations arise on distribution, and AGPL-3.0's on
       conveying or on network interaction with a modified version. Running Titan and the downstream in CI, or on
       a developer's machine, is neither, and no SCE product contains them.
     - **The upstream patch is contributed under the ECA.** A contribution to `eclipse/titan.core` is made under
       EPL-2.0 by a signer of the Eclipse Contributor Agreement. The patches are EPL-2.0 from their first line, in
       an EPL-2.0 repository.
     - **The codec libraries keep their own licences.** libcbor is MIT; libical is offered under MPL-2.0 or
       LGPL-2.1, and the downstream takes MPL-2.0. Each is pinned by version and source digest with its licence
       text.
     - **No legal review was made.** None is needed for the combination question, which no longer exists. The
       statements above are standard facts of the licences named.
2. **The profile targets one pinned set of parts:** ES 201 873-1 V4.17.1 (2025), -4 V4.6.1 (2017), -5 V4.9.1
   (2022) and -6 V4.14.1 (2023), the latest of each on ETSI's TTCN-3 download page on 2026-11-06. Part 4 is eight
   years older than part 1, and the profile takes its semantics from part 4 (§1). So the profile admits only
   constructs whose semantics part 4 V4.6.1 defines, even where part 1 V4.17.1 adds to them. Moving to a later
   edition is an amendment of this ADR, made for all four parts at once.
3. **Time in the lane is a logical clock, and everything due at one instant is in the same snapshot.**
   - A recorded trace gives each input a logical timestamp. TRI's timer operations (start, stop, read, running,
     timeout) run on a logical clock: in the platform adapter, where a tool delegates timers to it, and inside the
     runtime where the tool keeps them, as Titan does (below).
   - The clock advances only when every component of the run is blocked (no branch can fire on the present
     snapshot). It advances to the earliest of the next timer expiry and the next input of the trace.
   - At an instant, every input of the trace and every timer expiry due then is delivered before the next snapshot
     is taken. Which of them a component handles first is then decided by §5's document order, not by the order of
     delivery. The native runtime and the TTCN-3 side therefore see the same timeouts by construction: their order
     is never a race.
   - The same clock is the time domain of answer 11's timed conformance relation.
   - **Time alone does not make a run unique.** -4's selection phase may run components in any order, so with PTCs
     one input trace has many runs, and C3 could not tell a defect from a different legal schedule. The profile
     therefore runs in rounds (`docs/SCE_TEST_PROFILE.md` §3):
     - at quiescence, every waiting component takes its snapshot at once;
     - each component then runs the branch it selects to the end of its macrostep;
     - messages sent in a round are seen in the next.

     Every such run is one -4 permits, so the profile stays a subset of TTCN-3, and the run is a function of the
     input trace. Titan's Main Controller grants rounds by the same quiescence condition by which it advances the
     clock; one protocol carries both (below).
   - **In Titan the clock is part of the runtime.** Titan has wall-clock time only. Its runtime reads the time,
     from `gettimeofday`, at two functions:
     - `TTCN_Snapshot::time_now()` (`core/Snapshot.cc`), which the timers, the test case's guard timer, the
       snapshot wait and the test ports' timed handlers use;
     - `TTCN_Runtime::now()` (`core/Runtime.cc`), the TTCN-3 `now` operation, measured from the start of the
       test case.

     The profile does not use `now`, but both functions read the one replaceable clock. The other reads of the
     time (log timestamps, the profiler, the debugger) do not decide behaviour. The snapshot wait
     (`TTCN_Snapshot::take_new`) already takes the earliest of the next timer expiry and the next handler
     timeout before it polls. When nothing is ready at a poll that does not block, the logical clock advances
     to that instant instead of sleeping. A trace player in the binding schedules each recorded input as a
     handler timeout, so the inputs run on the same clock.
   - **With PTCs the clock is distributed.** In parallel mode Titan runs each component in its own process, under
     its Main Controller. "Every component is blocked" is then a global condition, and it is true only if no port
     message is still in transit, since a component waiting for a message in flight looks blocked. Connected
     ports exchange messages over sockets between the component processes, not through the Main Controller,
     so the Main Controller cannot see such a message. It therefore grants an advance only when three things
     hold:
     - every component has reported itself blocked, with its next due instant, in the current epoch;
     - the totals of port messages sent and received in those reports are equal;
     - no operation the Main Controller itself carries (create, connect, map, done) is pending.

     Each grant starts a new epoch, and a report from an earlier epoch does not count. This is conservative synchronisation of
     a discrete-event simulation (Chandy, Misra, *Distributed simulation: a case study in design and verification
     of distributed programs*, IEEE TSE 5(5), 1979) with termination detection by message counting (Mattern,
     *Algorithms for distributed termination detection*, Distributed Computing 2(3), 1987). The advance goes to
     the least due instant reported.
   - **The change is contributed to Eclipse Titan**, so that the logical clock is a property of the tool and not
     of SCE's copy of it. Until a Titan release contains it, the lane builds Titan from a pinned release with that
     change applied.
   - **Rejected:** running the lane on wall-clock time with generous slack on every timer. It makes the order of a
     timeout and an input a race, which is what this answer exists to remove, and a slack large enough to hide the
     race still decides nothing at the boundary.
4. **The guard timer belongs to the control part, and the `error` it causes belongs to the runtime.** TTCN-3's
   `execute` statement takes an optional time limit for the test case. If the test case has not ended when the
   limit expires, it is stopped and its verdict is `error`. The `test-suite` therefore declares each test case's
   guard, exported as the second argument of `execute`. The runtime alone assigns `error` (§4).
5. **Names.** The profile stays in SCE's one extension namespace (`http://sce.dev/ext`), the one authoring grammar
   `SCE_WIRE_CONTRACTS.md` lists as a single surface.
   - Every new kind carries the prefix `test-`, so that a kind's name says it belongs to the profile, and the
     collisions found in review cannot recur: `test-template`, `test-port-type`, `test-component`,
     `test-configuration`, `test-case` and `test-suite`.
   - The new elements and attributes carry no name the tree already uses: `<sce:setverdict>`,
     `<sce:test-timer>`, `<sce:start-timer>`, `<sce:stop-timer>`, `sce:timeout`, `sce:receive` and
     `sce:test-template`.
   - Where SCXML already has the operation, the profile uses the SCXML element: `<send>` (answer 6), `<log>`, and
     `<invoke>` for a PTC's lifecycle (§2). Inventing `<sce:send>` or `<sce:log>` would put a second element beside a
     standard one for the same act.
   - UTP's name is used where the concept is UTP's: the runtime's computation of §4 is the arbiter.
6. **A port is an address of the test-port Event I/O Processor, and the `EventOrigin` of ADR 0090 names it.**
   §scxml-C is SCXML's extension point for a new way of communicating, and a port is one. The profile defines one
   processor type URI. Its addresses name a component instance and one of its ports.
   - A `<send>` to a port leaves through ADR 0084's processor door.
   - An event delivered to a port arrives through the receiving session's inbox, carrying
     `EventOrigin { processor: <test-port URI>, address: <sending port> }` and the receiving port.
   - The runtime distributes the inbox into port queues by that pair (§5).
   - Nothing new is added to the event: the identity of a port is the value SCXML and ADR 0090 already define for
     the origin of an event.
7. **A scenario set is not lowered to test cases. Its runnable scenarios are the test purposes of answer 11.**
   - A scenario states what a specification says should be observed. In model-based testing that is a test
     purpose: it selects which behaviour of the specification a derived test case exercises (Jard and Jéron, TGV).
     It is not a test case, because it does not say what the tester does with every other output.
   - A scenario judged `not-judged`, `blocked` or `awaiting-decision` is not a test purpose yet. None of the
     judge's verdicts maps onto a test verdict: they are judgements of a design against examples, and §4's verdicts
     are judgements of an implementation by a tester.
8. **The canonical log of C3 is a surface of its own, `schemas/sce-test-log.v1.schema.json`, whose record kinds are
   the events of TCI's test logging interface (ES 201 873-6).** These are message sent, message received and matched,
   timer started, stopped and timed out, verdict set, component created and done, and test case started and ended.
   - The TTCN-3 side writes the log from the test logging operations of the adapter interface (answer 1). On
     Titan these are a logger plugin, Titan's interface for a user-written logger, which receives Titan's log
     events and writes the records whose TCI kinds they correspond to. Titan passes a message's value to the
     plugin as text in TTCN-3 value notation, not as a structured value. The plugin parses that notation into
     the log's canonical value form; the notation is ES 201 873-1's, a standard grammar. The native runtime
     writes the same record kinds at the same semantic points.
   - The record kinds stay TCI's even though Titan does not implement TCI, so that the log's vocabulary is the
     standard's and does not change with the tool.
   - It does not extend the observation trace, whose subject is a design judged against examples (§2.18).
   - It does not extend ADR 0089's transition observer, because the TTCN-3 side has no transitions to report.
   - It lands `pre-release`, with its row in `SCE_WIRE_CONTRACTS.md`.
9. **A profile document declares `datamodel="sce-static"`, and matching is never an expression.** `sce-static` is
   SCE's statically typed data model, run by every engine without a script engine (ADR 0005). A component's
   variables, assignments and verdict reasons are `sce-static` expressions. Whether a received message is the
   expected one is a TTCN-3 template's question, answered by a `test-template` (§2). A `cond` that reads the message
   is refused (§3), so matching has one form and the exporter translates one form.
10. **Ports reach each other through the test-port processor alone.** The `test-configuration` declares each
    `connect` (component port to component port) and `map` (component port to a port of the test system interface).
    - The processor of answer 6 routes by that table. A connected send is delivered into the inbox of the
      connected component's session. A mapped send goes to the system adapter (TRI).
    - The invoke channel (`#_parent`, `#_<invokeid>`) carries a PTC's lifecycle and nothing else.
    - One mechanism carries every port message, so there is one rule to translate, one to log and one to check.
11. **SCE adopts a timed ioco relation as its conformance relation, and the work follows in three later ADRs.**
    - SCE's statecharts have delayed sends and the profile has timers, so the relation must be timed. The relation
      is tioco: Krichen and Tripakis, *Conformance testing for real-time systems*, Formal Methods in System Design
      34(3), 2009, over timed automata with inputs and outputs. It extends Tretmans' ioco (alternative G), and its
      digital-clock tests fit the logical clock of answer 3.
    - The three ADRs, in order:
      1. the reading of an SCE statechart as a timed automaton with inputs, outputs and quiescence;
      2. a soundness check of authored test cases against the specification under that reading: a `fail` is
         reachable only on a trace the specification does not allow;
      3. derivation of test cases from the specification, with answer 7's scenarios as test purposes.
    - This ADR fixes only the target that work produces into: test cases of this profile.
12. **The equivalence theorem of §1 is held by translation validation of every export, together with C3.**
    - The exporter emits, beside each module, a correspondence witness that relates each node of the document to
      the TTCN-3 construct it became.
    - A validator, separate from the exporter and kept small, checks that the witness relates every construct by a
      rule of the specification (order of work step 1), and that each rule's semantic correspondence is proved in
      that specification. Checking the witness is local and decidable, where comparing the trace sets of two
      programs with data is not.
    - C3 then checks what the static validation cannot see: the TTCN-3 tool itself.
    - A proof made once for the whole exporter is not chosen. It would have to be redone at every change of the
      exporter, and it says nothing about the tool. Translation validation checks every output that actually exists
      (Pnueli, Siegel, Singerman 1998), and with C3 it covers both sides.
