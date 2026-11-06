# ADR 0091 — A test is a document of the test profile, and its TTCN-3 export runs and agrees with it

- Status: Proposed (2026-11-06). A draft for the owner. A downstream consumer session drafted it at the owner's request
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

The names in this table are placeholders chosen to match TTCN-3's terms (question 5). Three of them collide with names
the tree already uses, as the notes say.

| Concept (TTCN-3) | Profile form | Notes |
|---|---|---|
| Type definitions | existing `event-schema`, `enum` | exported as `record`, `union`, `enumerated`, with optional fields and value/length constraints |
| Template | new forge kind `template` over an `event-schema` type | specific values, `?`, `*`, ranges, length restrictions, `omit`, `ifpresent`, value lists, complement; parameterised templates. **Name collides** with the `<sce:template>` composition element (expanded by the parser before validation) and is close to the `text-template` kind |
| Port type | new forge kind `port-type` | message kinds per direction (`in`, `out`, `inout`) |
| Component type | new forge kind `test-component` | a statechart restricted per §3; declares its ports, timers and variables |
| Test configuration | new forge kind `test-configuration` | the MTC type, the test system interface, PTCs, `connect` and `map` of ports |
| Test case | new forge kind `test-case` | runs on a component type, names its configuration, names the features it requires |
| Module parameters (PICS, PIXIT) | existing `feature-model` (ADR 0057) for PICS booleans; new module-parameter declarations for numeric PICS items and PIXIT | a test case's feature requirement is its selection expression |
| Control part | new forge kind `test-suite` | the ordered list of test cases and their selection; exported as the module's `control` part |
| `setverdict` | new executable content `<sce:setverdict value="pass\|inconclusive\|fail\|none" reason="…"/>` | the document cannot set `error` (as in TTCN-3); only the runtime sets it |
| `alt` with receive branches | a state of a `test-component`; each `<transition sce:receive="port:template">` is a branch | branches are evaluated in document order over one snapshot (§5) |
| `timer` | `<sce:timer>` declarations; `<sce:start>`, `<sce:stop>`; `sce:timeout="t"` as a branch | **Name collides** with the existing `timer` forge kind (one periodic timer, `SCE_FORGE.md` §4.10) and with the `<data sce:timer=…>` attribute that kind replaced. A test timer is one-shot, which the `timer` kind by its own definition is not |
| `altstep` and `activate` | transitions of ancestor states | the innermost ancestor is the most recently activated default (TTCN-3 evaluates defaults in reverse order of activation) |
| `send` | `<sce:send port="p" template="…"/>` | the template must be specific (no matching symbols) |
| PTC `create`, `start`, `done`, `kill` | executable content over child sessions started by `<invoke type="scxml">` (§scxml-6.4) | a PTC is a child machine, and its local verdict is returned to the MTC. ADR 0043 applies only to a PTC that talks to Mesh peers |
| `log` | `<sce:log>` | goes to the TCI test logging interface on export. Whether SCXML's own `<log>` suffices is question 5 |

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

For each item, the specification of order of work step 1 says whether it is out by design or deferred. A deferred
item returns only through an amendment of this ADR that states its semantics and its export.

### 3. What the profile refuses

Each of the following is refused at build time with a `DiagnosticCode`. Each new variant lands with its acceptance-doc
row (`acceptance_doc_covers_every_code`):

- any native expression (`cpp:`, `kt:`, or a future prefix) anywhere in a profile document. ADR 0003 makes the escape
  hatch inadmissible where portability is the purpose;
- any construct that raises a script-engine cause. The generate manifest of a profile document must show
  `needs_script_engine = false`;
- `<parallel>` inside a `test-component`. A test expresses concurrency with PTCs, as in TTCN-3;
- `<history>`;
- `<script>`;
- `<donedata>` used as a verdict carrier;
- a receive branch whose template is not one of the port's declared incoming types;
- a `test-case` that requires a feature its suite's `feature-model` does not declare. This is the existing
  `validation/feature-requirement-undeclared` of ADR 0057, applied to a test case. It is not a new variant.

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
  its processor wrote. Whether a port is identified by that origin or by a field of its own is question 6.

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

- SCE ships a reference adapter set for the chosen TTCN-3 tool: a system adapter that connects ports to a trace player
  and recorder, a platform adapter with logical time, and codec bindings for the profile's own conformance corpus. With
  this set, §6 can be checked inside SCE without any consumer's test system.
- A new lane runs the profile corpus natively and through the export, replaying the same recorded input traces, and
  applies C3. This is a conformance law of the profile, held for each backend that emits the native test runtime, as
  other laws are held per lane.
- The corpus covers every row of §2 and every refusal of §3.

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
- SCE's CI gains a dependency on a TTCN-3 tool.
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
4. Build the exporter, deterministic, with the types module and its structural manifest (C2).
5. Build the reference adapter set and the conformance lane (C1, §7).
6. Add the equivalence law over the corpus (C3), with the canonical log's schema and its `SCE_WIRE_CONTRACTS.md` row.
7. Write migration guidance for consumers' existing test documents.

## Questions left open

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
