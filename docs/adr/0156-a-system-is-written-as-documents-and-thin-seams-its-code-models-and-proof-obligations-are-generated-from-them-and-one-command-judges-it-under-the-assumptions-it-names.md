# ADR 0156 — A system is written as documents and thin seams, its code, models and proof obligations are generated from them, and one command judges it under the assumptions it names

- Status: Accepted (2026-11-28, by the owner): "비용무시하고 가장장기적으로 올바르고, 학술적으로 교과서적이고,
  궁극적인 구조로 범용적으로 adr받고 도구 설치도 승인" (cost ignored, the most long-term correct structure, textbook in
  its academic foundations, ultimate and general: the record is accepted, and the installation of the tools is
  approved). The tools that sentence approves, and what each is for, are D13's. Pinning and installing them is
  another change's; this record names the decision only.
- Amended 2026-11-28 by the owner: "에이전트가 만드는 시험에 flaky 가 많으니 제품 밖에도 북극성을 적용" (the tests
  agents write are often flaky, so apply the north star outside the product too), relayed by the same consumer's
  session with two needs, both general: a simulation host that runs generated machines from one seed (D15), and the
  paths of a generated model exported as tests (D16). The Order of work places them.
- Amended (2026-11-29): the `sce-static` data model is translated (D17, and order steps 2a to 2c). Four independent
  reviews found that ADR 0151 D7 (a model per document, `<sce:noninterference>`), accepted, and the proposed ADR 0162
  (a decimal), ADR 0164 (recursive types), ADR 0165 (graphs) and ADR 0167 (a value as an internal event's data, an
  effect per element) each rely on a model of `sce-static` documents that no record and no step scheduled, while the
  model generator refuses every such document whole. D17 decides the translation, and the order places it before every
  step that reads such a model. Written under the owner's standing rule (the most long-term correct structure,
  textbook in its foundations, cost ignored).
- Date: 2026-11-28
- Amended by: ADR 0167 (accepted 2026-11-29), D3: the list gains, before the finite domains, the step that has the
  model read a `sce-static` document, lifting `Construct::Datamodel`; ADR 0167's model of an event's data, a branch, a
  loop and a host call depends on it.
- Requested by: the owner, relayed the same day by a session of a downstream consumer, which is the first reference
  case and not the scope. The owner's words: "sce와 얇은 seam을 이용해 만드는걸 산업표준으로 만들고 싶어, 증명도
  최대한 쉽고 기계적으로 될수있게해야해" (I want building with SCE and thin seams to become an industry standard, and
  proof has to be made as easy and as mechanical as it can be) and "이게 북극성이 되어야해" (this has to be the north
  star). The request listed the needs it saw, their state as it read this repository, the limits and a priority; this
  record decides each from the tree at `3fbffb5110d`, and corrects the request where the tree says otherwise
  (Context, "What the request read otherwise").
- Amends, as each step lands (each amended document gains an "Amended by: ADR 0156" line in the landing that moves
  it):
  - ADR 0115: D3 (TLAPS approved, and candidates pruned to an inductive invariant, D9), D7 (the semantics a public
    surface, D2; the composition of a deployment, D8), D8 (each output's tables held to the model's, D11), D10 (a
    seam's contract in the same expression language, D7; D8's converse, paths that cover the model and a run of none
    refused, D16), D15 (the paragraph "A bound is not an assumption", D5; a
    refinement declared by the properties document, D6), "Questions decided at acceptance", 1 (D13);
  - ADR 0146, the formal judge: D3 (the toolchain gains the tools of D13 as their steps land) and D4 (a runner per
    tool);
  - ADR 0111, once its amendment of 2026-11-28 lands: `LinkChannel.tla` is written as a promise module with its
    refinement (D8);
  - `backends/rust/world`: its virtual time, its seed and its counted guards become the Rust lane's simulation host
    (D15);
  - `SCE_WIRE_CONTRACTS.md` (the surfaces of D2, D7, D10 and D12) and `docs/formal/README.md` (what is built);
  - by D17 (2026-11-29): ADR 0115 D7 (a document's typed values, its events' data and its host calls), D8 (a run
    carries its datamodel at every step) and D15 (an exploration names a text's values and a host's answers; the bound
    `integer-range`); ADR 0119 D2 (what the second encoding holds); ADR 0151 D7 (its model per document rests on
    D17); `docs/formal/AppendixD.tla` (an event carries its data, `Execute` reads the configuration, an effect carries
    its host calls); the run trace's row of `SCE_WIRE_CONTRACTS.md` (its version moves).
- Scope: this record states the goal, the status of each part of it, the order and the tools. It changes no code and
  no generated output. Each part lands by the record or the amendment its decision names, with a test that fails
  without it.
- Related: ADR 0064 and ADR 0135 (Appendix D runs once over a document's tables), ADR 0084 (one door), ADR 0091
  (translation validation of an export), ADR 0110 (the C11 core), ADR 0111 (links), ADR 0115 (the levels), ADR 0118
  (cost), ADR 0119 (quantity), ADR 0146 (the formal judge), ADR 0147 (the door proved); for D17, ADR 0055 (optional
  values), ADR 0092 (unions), ADR 0108 (a map's order), ADR 0133 (a statement total where the build proves it) and ADR
  0151 (a view, and the one comparison of every type).
- When this record was written two records carried the number 0146 (the formal judge; a bounded FIFO's full policy)
  and two carried 0148 (a reactive turn; a message as a case of a union). On 2026-11-28 the formal judge and the
  reactive turn kept their numbers, the bounded FIFO's full policy became ADR 0157 and the message as a case of a
  union ADR 0158. The subjects this record names beside a number stay, as a reader's check.

## Context

SCE already is most of the shape the owner names. A `sce-static` document is generated into seven languages. The
model of the same document is generated from the tables its code is printed from (ADR 0115 D7), the runs of every lane
are read against that model (D8), and one command of the code generator judges every level on a manifest any
repository gives it (ADR 0146, the formal judge). What a reader cannot find in one place is the goal those records
serve, how far each part of it stands, and what is still missing for the proof of a whole system to be a matter of
running tools. This record is that place.

### What is built, read from the tree

| Part | Where | State at `3fbffb5110d` |
|---|---|---|
| Every resource bounded | ADR 0005, 0050, 0076, 0077, 0132 | built: a document's state is finite, and a bare-metal build allocates nothing |
| One semantics | `docs/formal/AppendixD.tla` | built for the core of the recommendation over tables, the integers and the booleans |
| A model generated from a document (0115 D7) | `sce-codegen model`, `sce-build/src/formal/document_model.rs` | built for the fragment above; `W3CTest388`, `W3CTest570`, `W3CTest403c` generated; every other construct refused by name (`Construct`: `<invoke>`, `<donedata>`, strings, a nested `<datamodel>`, ...) |
| A run read against the model (0115 D8) | `sce-codegen formal traces`, `docs/formal/traces.json` | built: the three documents run by the Rust, C11, C++, Go, Kotlin, Python and Swift lanes, an altered run per kind of step refused |
| An author's properties (0115 D15) | `sce-codegen formal properties`, `properties.rs`, `verdicts.rs`, `coverage.rs`, `behaviour.rs`, `replay.rs` | built for the fixture `sce-build/tests/fixtures/formal_properties`: every form, vacuity, coverage, verdicts with digests, each violation and witness a scenario replayed on all seven lanes |
| Protocol promises and refinements (0115 D1, D2, D12) | `docs/formal/*Service.tla`, `Refines` in seven protocols, `CheckpointWrite.tla` | built |
| Theorems for every length (0115 D3) | `inductive` in `docs/formal/models.json`, `sce-build/src/formal/inductive.rs`, Apalache | built for three hand-written invariants; for every size, none |
| Functions proved (0115 D13) | `docs/formal/proofs.json`, Verus | built for eight Rust files of the host core and the runtime's drain |
| The door proved (ADR 0147) | `InboxService.tla`, `EngineInbox.tla`, `sce-build/src/inbox_history.rs`, `backends/rust/runtime/src/inbox/drain.rs`, the GenMC harnesses of the C11 and C++ inboxes | built |
| A second encoding (ADR 0119) | `sce-codegen model --jani`, `quantities.rs`, Storm | built for one document, its graph held to TLC's and the runs replayed; no distribution and no property yet |
| One judge (ADR 0146, the formal judge) | `sce-codegen formal`, `sce-build/src/formal/toolchain.json`, `sce-build/tests/fixtures/formal_consumer` | built; the manifests and the verdicts record are `pre-release` surfaces |
| What a check says is not edited with what it judges (0115 D11) | `sce-codegen formal isolation` | built |

### What the request read otherwise

- **0115 D7 is not absent.** The semantics, the generator and three generated models have landed, with D8 on every
  lane and D15 for one fixture. What is missing is breadth: the constructs `Construct` refuses, the connectors
  (`MeshChannel.tla`, `LinkChannel.tla`, the queue and the log as promise modules), a codec's refusals
  (`Refuses_C`), the faults of a channel and a deployment's composition.
- **0115 D4 and D8 are not one state.** D8 (a lane's run of a document) is built; D4 (a run of the protocol code under
  `backends/rust/world`) is not, and `traces.json` holds no such run.
- **0115 D13 is further than the request says** (eight files proved) and **D10 is not started**: `<sce:require>`
  is in the grammar, `<sce:ensure>` is in neither the grammar nor the parser, and no algorithm is in `proofs.json`.
- **ADR 0118 is Proposed**, and no module computes a cost; the bounds the build computes (`microstep_bounds.rs`,
  `internal_queue_bound.rs`, `scheduler_bound.rs`, `send_id_bound.rs`) are sizes, not times.
- **ADR 0119 is Accepted** (2026-11-28) and Storm is pinned; its D2 and D4 are built for one document, its D3 is not.
- **ADR 0146, the formal judge, is built**, and ADR 0147's four steps have landed.
- **ADR 0111's amendment of 2026-11-28** (an asked dial, the host's explicit end, the contract observer seam its review
  calls 11(b)) is on a branch (`ac7a1567374`) and not on main; the seam that reports a violation at no cost is not yet
  in the tree.
- **TLAPS was not approved** (ADR 0115, Questions decided at acceptance, 1); `docs/formal/README.md` names the theorem
  it would prove first. It is approved by this record's acceptance (D13).
- **Kani, CBMC and a thinness rule** appear nowhere in the tree.

## The textbook structure

- **A verdict is only as trusted as what it rests on**, so the base it rests on is made small and every part of it is
  named (the trusted computing base; Hawblitzel et al., IronFleet, SOSP 2015, section 2.5, which ADR 0115 D6 follows).
- **A specification is what a user may rely on, and a system is correct when it refines it** (Abadi and Lamport 1991;
  Lamport 2002). A refinement mapping is a function a person writes; checking it is a program's.
- **A generator is trusted by checking each output, not by proving the generator** (Pnueli, Siegel and Singerman 1998;
  Necula, PLDI 2000). Where the output is data a fixed interpreter runs, checking the output is comparing data.
- **Composition is by assumption and guarantee** (Abadi and Lamport 1995): each component is proved under what it
  assumes of its environment, and the components' guarantees discharge each other's assumptions. Circular discharge is
  sound for safety and not for progress.
- **A few values can stand for all** when a system only copies and compares them (data independence, Wolper 1986;
  Lazić, Newcomb and Roscoe 2003), and **a few instances for many** when they are interchangeable (symmetry, Ip and Dill
  1996; cutoffs, Emerson and Kahlon 2000). Each is a theorem with conditions, and the conditions are decidable from the
  text of the system.
- **An inductive invariant is found by no procedure in general**; the largest inductive subset of a finite set of
  candidates is computed by a fixed point of step checks (Houdini; Flanagan and Leino, FME 2001), and what remains is a
  counterexample to induction a person reads.
- **A function's contract is a precondition and a postcondition** (Hoare 1969), proved deductively (Verus) or, for code
  whose loops are bounded, by bounded model checking that also proves its bounds sufficient (CBMC, Clarke, Kroening
  and Lerda, TACAS 2004; Kani, its front end for Rust).
- **A tool whose output is verified need not be qualified as a development tool; the tool that verifies it is
  qualified as a verification tool.** That is how DO-178C section 12.2 with DO-330, ISO 26262-8 clause 11 (where
  detecting a tool's errors lowers its confidence level) and IEC 61508-3 7.4.4 are commonly read; this record did not
  read them (Sources).
- **A standard is a specification that a second, independent implementation can be held to**, not a program.
- **A test is a function of its script and its seed.** Every choice that is not the program's own (the time, the
  order of messages and of threads, a loss) is drawn from one seed or given by a script, so a failing run is
  reproduced by its seed, and a test that sometimes fails is a defect of its structure, not a rate to be lowered by
  retries, longer timeouts or sleeps (deterministic simulation testing, as FoundationDB and TigerBeetle describe it).
- **A test derived from the specification is sound**: it fails only where the implementation does not conform
  (Tretmans 1996, checked by ADR 0115), and its expected values are the specification's, never a second statement of
  them written by hand.

## Decision

### D1. The goal

A system is written as **documents**, which hold every decision it makes, **seams**, which move bytes, readiness and
time between the documents and the world and decide nothing, and a **deployment**, which names the documents, their
connectors and their seams. From these alone the code of every lane, the model of every document and of the
deployment, the proof obligations, and the traces are generated, and `sce-codegen formal` judges all of it at the SCE
revision a repository pins, on that repository's own manifests (ADR 0146, the formal judge, D7). The theorem it
judges is:

> the deployment refines its promise, under the assumptions it names.

It is the conjunction of four judgements, each of one level and each mechanical:

1. **The model refines the promise.** The promise is a short module a person reads (ADR 0115 D1). That the model of
   the deployment refines it is TLC's at a size, Apalache's for every length, and for every size by D5 or by TLAPS.
2. **The code is the model.** Each lane's run is a behaviour of the model and each behaviour the model reports is a run
   of each lane (ADR 0115 D8 and its converse), and each output's tables are the model's (D11).
3. **Each seam meets its contract**, and is thin enough that a bounded proof of it is complete (D7).
4. **What lies below is assumed, by name**: the compilers, the operating system, the drivers and interrupts, the
   hardware and the memory model its atomics are judged under, cryptography, and the checkers themselves (D14).

What a person writes is the promise, the documents, the seam bodies, the refinement mapping and, for the few theorems
that need one, an inductive invariant. Everything else is generated, and every judgement is a program's. The structure
is any repository's: this repository is one user of it (ADR 0146, the formal judge, D6), and a downstream consumer is
the first reference case.

### D2. The semantics of the dialect is one public, versioned specification

- **Status.** `AppendixD.tla` is the core of the recommendation over tables, the integers and the booleans, and is
  the model generator's only place of SCXML (ADR 0115 D7). The rest of the dialect is specified in prose
  (`docs/SCE_ACCEPTED_SUBSET.md`) and by grammar (`schemas/sce-forge.xsd`, `schemas/sce-forge-ext.xsd`,
  `pre-release`). SCE's own constructs (`<sce:call>`, an `algorithm`, a codec's values and refusals, a link, a
  `queue`, a map, a `broadcast-log`) have no module, and `AppendixD.tla` is no registered surface.
- **Decision.** The semantics is `AppendixD.tla` and one module per construct of SCE and per kind of connector beside
  it, each written once, citing the text it formalises, and the only definition the model generator reads. Together
  they are registered in `SCE_WIRE_CONTRACTS.md` as one spec-bearing surface with a version; a prose document that
  states the same rule points at it. A construct with no module has no model and is refused by name, as today.
- **Amends** ADR 0115 D7 ("One semantics"). **Depends on** nothing.

### D3. The model of every document, and of a deployment

- **Status.** As in the Context: built for a fragment.
- **Decision.** ADR 0115 D7 as written, in this order: the finite domains an exploration names for a string or a
  number; `<invoke>` and `<donedata>`; `Refuses_C` and `StoreRefuses_{C,n}`; the promise modules of the connectors
  (D2); the faults of a channel; the composition of a deployment (D8 changes how it is judged).
- **Amended 2026-11-29.** Before the first item of that order stands D17, the typed values of a `sce-static`
  document, which every later item reads; "the finite domains an exploration names for a string or a number" is
  D17's, for a text and an integer of any width.
- **No new record**: ADR 0115 D7 and its order step 4. **Depends on** D2, and from 2026-11-29 on D17.

### D4. The proof obligations of a document are generated

- **Status.** None is generated. `AppendixD.tla` states a legal configuration once (`LegalConfiguration`); the three
  inductive invariants are hand-written.
- **Decision.** For every document with a model the generator writes: `TypeOK`, each variable in the type and bounds
  the document declares, which it already must (ADR 0050, ADR 0077); one bound invariant per bounded structure (each
  queue's and list's length at most its bound, each counter in its range); the configuration invariants instantiated
  for the document; and, per row of the transition table, the obligation that its step keeps them. They are checked as
  invariants by TLC and are D9's first candidates.
- **Amends** ADR 0115 D7 (what the module holds) and D3. **Depends on** D3.

### D5. For every size, by conditions the generator decides

- **Status.** ADR 0115 D15 says that a document that only copies and compares a type is data independent in it, and
  that "no verdict uses it before it is read and stated here". It has not been read; Wolper 1986 is among the sources
  ADR 0115 did not reach. No symmetry is declared by any generated configuration.
- **Decision.** The generator decides, per type of a document, whether the document only copies it and compares it for
  equality, and, per set of instances in a deployment, whether they are interchangeable (one document, no behaviour
  that reads an instance's identity but to compare it). Where the theorem's conditions hold, as read from its text
  and stated in this repository with the threshold it gives, the exploration is generated at that threshold and its
  verdict is *holds for every size* by the theorem, beside *proved* (TLAPS) in ADR 0115 D15's table. A symmetry is
  used only for safety: ADR 0115 D9 refuses one beside a temporal property.
- **Amends** ADR 0115 D15 (the paragraph "A bound is not an assumption" and the verdict table). **Depends on** D4.

### D6. A refinement is declared, and its check generated

- **Status.** `Refines` exists for seven hand-written protocols, and every generated module refines `AppendixD`
  with all its parts. Nothing declares that a document refines a promise written by its user.
- **Decision.** A properties document (ADR 0115 D15) may name a promise module and map each of its variables to an
  expression over the machine's observations and its observers' states (an observer is a history variable, D15; a
  prophecy, where one is needed, is named as one). The generator writes the instance and `Refines`, TLC checks it at
  the exploration, and D5 and D9 extend it.
- **Amends** ADR 0115 D15 and D1. **Depends on** D3.

### D7. A seam declares its contract, a harness proves it, and a rule keeps it thin

- **Status.** None of it exists. Verus is pinned and proves Rust files a person separated from the shell (ADR 0115
  D13); an algorithm's contract (D10) has its precondition and no postcondition; ADR 0111's unlanded amendment adds a
  seam that reports a contract's violation.
- **Decision.**
  - **A seam is declared where the document or the deployment already names it** (a host operation of `<sce:call>`, a
    host invocation, a link's driver, a C11 seam the runtime declares and the deployment defines), with a
    precondition and a postcondition in the expression language of ADR 0115 D10, so the contract is one text every
    lane reads.
  - **The generator writes the harness.** For Rust, a Kani harness per seam: inputs any value the precondition admits,
    the postcondition asserted, unwinding assertions on; and a Verus specification where the seam is a pure function.
    For C11, a CBMC harness of the same contract. One contract has its back ends; a seam has no hand-written harness.
  - **Thin is a rule a program checks.** A seam body is refused when it keeps state across calls other than the handles
    it declares, holds a loop whose bound is not a constant, or branches other than to turn an outcome of the world into
    a value of an enumeration the contract declares. A body that keeps the rule is one whose bounded proof is complete,
    which is why the rule is what it is.
- **New record**, which amends ADR 0115 D10 and D13. **Depends on** D2 (what a host call means) and on D10's
  expression language. Its first seam is the link's lending operation of ADR 0111 order step 3, written before the
  contract form exists and given one after (section "Order of work").

### D8. A machine is proved alone against the promises of its connectors, and a deployment by composition

- **Status.** ADR 0115 D15 states an assumption and a guarantee with Abadi and Lamport's meaning, and discharges an
  assumption by checking the composition whole; D7 composes a deployment into one model. None is built for a
  deployment. ADR 0111 D3 gives a link's outcomes, and `LinkChannel.tla` is in its unlanded amendment.
- **Decision.** Each machine is checked once against the promise modules of the connectors it uses and its
  environment's assumption. A deployment is judged by the composition theorem: each connector's obligation that one
  side's guarantee implies the other side's assumption is checked over the connector's promise, never over the product
  of the machines. Discharge may be circular for safety and must not be for progress. The whole product is still
  checked at one small size, so the compositional verdict is shown not to hold for nothing (ADR 0115 D5's rule).
  `LinkChannel.tla` is written as a promise module, with the link's implementation refining it.
- **Amends** ADR 0115 D7 (the paragraph that composes a deployment into one model) and D12, and ADR 0111's amendment.
  **Depends on** D3 (connectors) and D6.

### D9. Candidates are pruned to an inductive invariant, and TLAPS proves what needs every size

- **Status.** Apalache is approved and pinned and judges three hand-written invariants for every length at fixed
  sizes. TLAPS is not a dependency; `docs/formal/README.md` decides per theorem and names the follower of ADR 0114 as
  the first worth proving.
- **Decision.** D4's obligations and a property itself are the candidates. The judge computes their largest inductive
  subset by repeated Apalache step checks (Houdini) and reports a property it implies as holding for every length at
  the size named, and a property it does not as not judged, with the counterexample to induction. A person
  strengthens it by a candidate, which is then checked as any is. TLAPS proves a theorem for every size where
  `docs/formal/README.md` says it is worth its upkeep, first the follower; a proof is refused with an `OMITTED` step or
  an axiom, as the models judge (`models.rs`) already refuses them (ADR 0115 D11).
- **Amends** ADR 0115 D3 and ADR 0146, the formal judge, D4 (a TLAPS runner). **Depends on** D4.

### D10. Traceability is generated from the surfaces that exist

- **Status.** Each link exists and no chain does: a requirement on a node (`sce:req`, `sce-codegen requirements`,
  the transition table), a node to the code of a lane (the sourcemap, `addr2sce`), a property named by the
  requirement it states (ADR 0115 D15), a verdict with the digests it is of (the verdicts record), an acceptance (ADR
  0008), and the bindings of a ledger.
- **Decision.** A matrix of requirement, promise clause, document node, generated code per lane, scenario, run and
  verdict or proof, each with its digest, generated from those surfaces and from no annotation of its own, and
  registered as a `pre-release` surface.
- **New record. Depends on** D6 (the promise clauses) and on every verdict surface.

### D11. The generator stays outside the trusted base by checking each output

- **Status.** ADR 0115 rejects proving the generator; D8 checks its outputs by runs, which show agreement on the
  behaviours run and no further.
- **Decision.**
  - Where a lane runs Appendix D once over a document's tables (ADR 0064, ADR 0135), the tables its code holds are
    compared with the model's tables, entry by entry, for every output: total, for the control of the machine. The
    walker of a lane is one program, judged once: by D8's runs now, and by a proof of it against `AppendixD.tla` where
    the walker's language has a checker.
  - The operators a document brings (its conditions, its actions, its algorithms) stay judged by D8's runs and by D7's
    contracts where they are seams.
  - No claim of qualification is made here. The structure is chosen so that, under DO-178C and DO-330 and under ISO
    26262-8, the generator is a tool whose output is verified and the tools that qualify are the checkers (TLC,
    Apalache, TLAPS, Verus, Kani, CBMC, Storm and SCE's judges), each pinned and each with runs that show it can refuse
    (ADR 0115 D5). The reading of the standards is not checked against their texts in this record, and is checked
    before anything is claimed to an authority.
- **Amends** ADR 0115 D8. **Depends on** D3.

### D12. The standard is the specification, and a second checker must be able to recompute every verdict

- **Status.** `SCE_WIRE_CONTRACTS.md` registers the grammar, the formal manifests, the toolchain and the verdicts
  record, all `pre-release`, with a flip procedure. The semantics (D2), the seam contract form and its ABI (D7) and
  the matrix (D10) are not registered.
- **Decision.** What a standard would name is the set: the grammar, the semantics, the connector promises, the seam
  contract form and ABI, the manifests and the verdict formats, each a registered surface with a version, flipped to
  `stable` by the registry's procedure. That set must suffice for an implementation that shares no code with SCE to
  recompute every verdict; a corpus of documents, manifests and expected verdicts, as the W3C corpus is for SCXML,
  holds this repository to it.
- **New record** for the corpus. **Depends on** D2 to D10 having settled their formats.

### D13. The tools, and the owner's approval

The owner's sentence that accepted this record approves the installation of the tools below that were not yet
approved, for the use named. A tool joins `sce-build/src/formal/toolchain.json`, pinned by digest with what it runs
on (ADR 0146, the formal judge, D3), in the change that builds its step, as ADR 0115's Consequences say.

| Tool | Approval | Pinned | For |
|---|---|---|---|
| TLC | ADR 0115 D5 | 1.7.4 | every bounded level, D3 to D6, D8 |
| Java runtime (Eclipse Temurin JRE 21) | ADR 0146, the formal judge, amendment of 2026-11-27 | yes | TLC and Apalache |
| Apalache | 2026-11-27 (ADR 0115) | 0.63.0 | D9's step checks; ADR 0115 D3 |
| Verus | 2026-11-23 (ADR 0115 D13) | yes, with its Rust release | ADR 0115 D10 and D13; D7's specification of a pure seam; D11's proof of the Rust walker |
| Storm | 2026-11-28 (ADR 0119) | 2.0.0 | ADR 0119 |
| TLAPS, with the back ends its release carries | 2026-11-28, by this record | not yet | D9: theorems for every size, the follower of ADR 0114 first; the candidates of ADR 0157 (the full policy), its order step 7 |
| Kani, with the Rust toolchain and CBMC it pins | 2026-11-28, by this record | not yet | D7: the harness of each Rust seam |
| CBMC | 2026-11-28, by this record | not yet | D7: the harness of each C11 seam, the same contract |

GenMC judges atomics in its own lane (ADR 0147 D5) and is outside the judge's toolchain; this record changes nothing of
it. Not approved, and not asked for: a verified or qualified compiler (CompCert for C11, Ferrocene for Rust). A
compiler is below the seam and is a deployment's choice under its own licence (D14).

### D14. The limits

- **Mathematical.** No procedure finds an inductive invariant in general, so D9 automates a fixed point over
  candidates and leaves a counterexample to a person. Progress holds only under a fairness or a partial synchrony that
  is named, and consensus with one crash cannot promise to end (Fischer, Lynch and Paterson 1985); a liveness verdict
  is not reduced by symmetry, not discharged circularly and not given where a bound was reached. A proof is as right as
  its promise, which a person reads (ADR 0115 D14).
- **Below the seam.** The operating system, the drivers, interrupts, the hardware, timing and the memory model the
  atomics are judged under are assumed. Compression, cryptography and numerics are no part of a state machine: they
  are seams or verified libraries.
- **Compilers.** The generated code is compiled by compilers that are not verified. A C11 deployment can narrow that
  with CompCert; Rust has no verified compiler, and a qualified one (Ferrocene) is qualified, not proved.
- **Checkers.** TLC, Apalache, TLAPS's back ends, Verus, Kani, CBMC, Storm, Z3 and SCE's judges are trusted. A second
  encoding (ADR 0119 D4) and a second checker (D12) make their errors detectable; nothing makes them impossible.
- **Simulation.** A run of D15's host shows what the code does under the behaviours a promise allows and a seed
  chose, and D16's paths what it does on the behaviours chosen; neither shows that a real transport, scheduler or
  clock keeps the promise, which its conformance kit, GenMC and the assumptions of D1 answer.
- **Cost of the code.** A construct whose generated code is slower than hand code limits adoption and not proof; it
  is the business of the records that measure the generated code.
- **Standardisation.** A tool from one repository is not a standard. A public specification, an implementation or a
  checker independent of SCE, a community that keeps it, and an authority that accepts its evidence are needed, and
  the last is a matter of years. This record decides what this repository can make true: a specification sufficient
  for an independent checker (D12), and the evidence an authority would read (D10, D11).

### D15. A simulation host runs generated machines from one seed, behind seams that keep their contracts

Added by the amendment of 2026-11-28.

- **Status.** Part of it exists, for one lane and one kind of connector. `backends/rust/world` runs several generated
  Rust machines on Mesh endpoints over a network it owns: every engine on `SceClock::Manual` and one shared clock that
  only the world moves, every random choice (a delay, a loss, a duplicate, a reordering, an endpoint's ids and jitter)
  drawn from one seed by SplitMix64, guards that count steps (`PUMP_LIMIT`, `SETTLE_TICKS`) and never wait, and a run
  read back as the replay scenarios of every backend. Each lane has a manual clock (the Rust `SceClock::Manual`, C11's
  `sce_clock_manual`, Go's `clock.go`), and the C11 tests link `no_wall_clock.c`, which ends a test whose machine
  reads the wall clock. What is missing: the world runs every machine on one thread, so the door's interleavings (ADR
  0084) are not among its choices, and the inbox histories of ADR 0147 come from stress runs of real threads, judged by
  linearizability for whatever schedule occurred; there is no in-memory link of ADR 0111, whose transmit side has not
  landed; a host invocation is completed by each test's own code; nothing holds a simulated seam to the contract of
  the seam it stands for; and no lane but Rust has a host of this kind.
- **Decision.**
  - **One host per lane, for any deployment.** The simulation host is a product surface, generated or written once per
    lane runtime, and runs the machines of any deployment a repository gives it. It owns the clock, which only it
    moves, the in-memory implementation of every connector and seam the deployment uses (a link, ADR 0111, with its
    lending and the drive's wait; the door, ADR 0084; a Mesh binding; a host invocation, completed by a script), and
    the schedule: which machine or poster takes the next step, the order of each queue, and every loss, duplicate,
    reordering and delay.
  - **Its choices are exactly the promise's.** Each choice is one the connector's promise module allows (D2, D8) and
    nothing else, so a simulated link loses only where its promise lets a link lose, and a run of the host is a
    behaviour the composed model can take. The choices come from a seed, drawn as the world draws them now, or from a
    script, which is how D16 replays a path of the model. A failing run prints its seed, and the seed reproduces it on
    every run.
  - **A guard is a count.** A test waits for a number of steps, deliveries or ticks, never for a duration, and a
    machine in the host that reads the wall clock ends the test, as `no_wall_clock.c` makes it do in C11.
  - **One body with D7.** A simulated seam is held to the contract of the seam it stands for by the same generated
    harnesses (Kani, CBMC) and its runs by trace validation against the connector's promise (ADR 0115 D4), so what a
    test learns in simulation is what the shipped seam is proved to do. A simulated seam that models less than the
    engine relies on (a link with no wait at the drive) passes defects through, which this rule refuses.
- **Record.** The seam-contract record of D7, which takes the simulation host as its second half, and amends
  `backends/rust/world`, whose rules become the Rust lane's host. **Depends on** D7 (the contracts), D2 (the
  connectors' promises the choices are drawn under) and, for the link, ADR 0111 steps 3 and 3a.

### D16. The paths of a generated model are exported as tests, and a test that runs none fails

Added by the amendment of 2026-11-28.

- **Status.** The converse of ADR 0115 D8 is built for the behaviours a property picks: a violation and a witness
  of ADR 0115 D15 are written as scenarios in the run trace format (`behaviour.rs`) and each of the seven lanes
  replays them, judged step for step (`replay.rs`,
  `sce-build/tests/a_scenario_is_the_run_of_every_lane.rs`). The behaviours drawn at random from the model that the
  converse also names (TLC's simulation) are not built, no criterion chooses paths for what they cover, and nothing
  refuses a test that replayed nothing.
- **Decision.**
  - **A test's expected values are the model's.** The generator chooses paths of a document's model (ADR 0115 D7) by a
    stated criterion: every row of the transition table taken, and every clause of the promise a refinement declares
    (D6) brought into play. It writes each in the run trace format with its inputs, as ADR 0115 D15's scenarios are, so
    the same harness of each lane replays it and the same judge reads it. No expected value is written by hand.
  - **What cannot be covered is named.** A row or a clause no path within the exploration reaches is listed with the
    bound that cut it, as a verdict names a bound (ADR 0115 D15).
  - **A test that runs zero paths fails**, as a check that cannot fail shows nothing (ADR 0115 D5): a suite whose
    selection came back empty, or whose lane replayed none of it, is refused by name.
  - **Where it runs.** A path of one machine runs on a lane's scenario harness as it is. A path of a deployment fixes
    the choices of its connectors, and runs on this record's D15 host, given as a script.
- **Amends** ADR 0115 D8 (its converse, "What is replayed"). **Depends on** D3 (what has no model has no paths), D6
  (for the clauses) and, for a deployment, D15.

### D17. The `sce-static` data model is translated

Added by the amendment of 2026-11-29.

- **Status.** `sce-codegen model` writes a module for a document whose `<data>` start as integers and booleans, and
  refuses every `sce-static` document whole before it reads a construct of it (`whole_document`,
  `Construct::Datamodel("sce-static")`, `sce-build/src/formal/document_model.rs`). Its terms are integers, booleans
  and event names (`Ty`); a literal past `LARGEST_INTEGER` (`i32::MAX`, the largest integer TLC holds) is refused; its
  blocks are an `<assign>` of a whole variable, a `<raise>` and a `<send>` with no data to the session's own queue; and
  the events of `AppendixD.tla` carry names and no data. A lane's run trace carries no value of the datamodel
  (`sce-build/src/run_trace.rs`, version 1), so D8 judges a run's control and none of its data. ADR 0115 D7 names the
  generator's input as a `sce-static` document with its typed variables, its `<sce:call>` and its algorithm bodies;
  D3 here orders breadth after that and never names the typed values themselves, so the records that read such a
  model (ADR 0151 D7; the proposed ADR 0162, 0164, 0165 and 0167) wait for a step that did not exist.
- **Decision.** The model of a `sce-static` document holds every value its `<data>`, its events' data and its
  algorithms' slots may hold, one canonical form per kind; computes every expression and statement as every lane
  computes it, failures included; and refuses by name what it does not translate. A document so refused has no model,
  never a model of part of it (ADR 0115 D7). The parts follow.

**The values.** Each kind has one form, so two values of the model are equal exactly when the lanes' values are, and
one predicate, which is the kind's part of D4's `TypeOK` and its bound invariant.

| Kind | In TLA+ | Its predicate | Why this form |
|---|---|---|---|
| an integer of any width, `int8` to `uint64` | an integer | in the width's range | a lane computes the mathematical integer and refuses what leaves the width (SCE_FORGE.md section 3.4.1), so the model holds the number, never a wrapped one |
| `bool` | `TRUE`, `FALSE` | `BOOLEAN` | |
| a value of an enum | the variant's name, a string; the enum's declared order a sequence of its names | one of its names | a variant is compared by equality, and by its declared order where ADR 0151 D2 orders it |
| a record of an event schema | a record of the schema's fields | each field of its type | |
| a union (ADR 0092) | a variant, written with Apalache's `Variants` module (`Variant`, `VariantTag`, `VariantGetUnsafe`), whose definitions TLC evaluates as the records they are | its tag one of the union's cases, its value that case's record | Apalache types a variant; a record with a field per case would type no union |
| an optional value (ADR 0055) | a variant of two cases, `None` with no value and `Some` with the value | absent, or `Some` of a value of `T` | the sum `none \| some(T)` that ADR 0151 D2 orders |
| a list `list<T>[B]` | a sequence | its elements of `T`, its length at most `B` | |
| a text `string[N]` | a sequence of Unicode scalar values, as integers | each a scalar value, its UTF-8 length (`Utf8Length`) at most `N` | the bound counts UTF-8 bytes on every lane (docs/SCE_ACCEPTED_SUBSET.md section 2.15, "String variables") and a comparison is by code point (ADR 0151 D2); TLC holds a TLA+ string as an atom with no code points, and could measure neither |
| a byte string `bytes[N]` | a sequence of integers in `0..255` | its length at most `N` | |
| a map `map<K, V>[B]` | a sequence of pairs `<<k, v>>`, in the order their keys were first put (ADR 0108 D2) | no two keys equal, at most `B` pairs, keys of `K`, values of `V` | a walk, the saved form and the snapshot read that order on every lane, so a model that forgot it would accept a run walked in another |
| a set `set<K>[B]` | a sequence of keys in the order they were first put | as a map's keys | a set is a map whose entries carry no value (ADR 0108 D1) |
| a map of maps, lists or records that hold a list (ADR 0108 D12) | the forms above, composed | each level its own predicate | |

- **Two maps are never compared** (ADR 0108 D2), so the order a map holds is observed only through a walk. A record
  that reads a map by its contents (a view, a graph, an order of keys) reads it through `docs/formal/Compare.tla`,
  ADR 0151 D2's one total comparison of every type written once, which takes a map as its entries in the order of
  their keys and never in the order they were put.
- **A text that is only copied and compared** keeps this form here. Whether a few symbolic values may stand for all of
  its values is D5's question, decided by the generator from the document's text, and not this decision's.
- **Integers past TLC's range.** TLC holds an integer in 32 bits; Apalache's integers are its solver's, unbounded. The
  model is written over the integers, one text for both checkers:
  - TLC computes no value outside its range. `Checked.tla` tests each operation's operands against TLC's range before
    it computes, as it tests the result against the width's, and a step that would compute past it is not taken: TLC
    records the bound `integer-range` as *reached*, by name, as ADR 0115 D15 records every bound that refuses a step
    the document would take. Its verdict on such a document is *holds within*: never a wrapped value, never an
    `error.execution` the lanes would not raise, and never an error of TLC that ends the search. A width whose limit
    passes TLC's range is not compared with it under TLC, since every value TLC holds is inside it already.
  - A literal TLC cannot write (a `uint64` constant past `i32::MAX`) refuses TLC's run of the module by name, naming
    the literal; the model itself is not refused.
  - Apalache judges the same module with no cut, so its verdicts, and D9's, are of each width's whole range. It is the
    route to a verdict on a value TLC cannot hold, and a lane's run that holds such a value is read against the model
    by Apalache, one step check per recorded step, with `docs/formal/traces.json` saying which checker read it.

**The expressions** are the forge expressions docs/SCE_ACCEPTED_SUBSET.md section 2.15 admits, typed by the one scope
every lane reads (`crate::forge::type_ctx::StaticScope`) and written by the one translation (`Term`, which gains the
kinds above), which prints TLA+ for this module and JANI for ADR 0119's:

- literals of every kind above; a read of a variable, of a field at any depth, of an element (`xs[i]`), of a map's
  value (`m[k]`, `m[k].f`), `len` (a text's in UTF-8 bytes, a byte string's in bytes, a list's and a map's in
  entries), `has`, the payload (`_event.data.f`), a variant of an enum, a union's tag and a field its tests prove, and
  `In(id)`, in a condition and in executable content alike (`Execute` is handed the configuration, below);
- `+ - * / %` and unary `-` at the width SCE_FORGE.md section 3.4 types them at, with section 3.4.1's results: a
  quotient truncated toward zero and a remainder of the dividend's sign, written once over TLA+'s `\div` and `%`,
  which round toward minus infinity; the eight wrapping and saturating names of section 3.4.2; the bitwise operations
  and the shifts at the width section 3.4.1 gives them, defined arithmetically by a fold over the width's bits, since
  TLA+ has none; the comparisons; `===` and `!==` as equality of canonical forms; `&&`, `||` and `? :`, each
  evaluating only the side that is reached; a text joined to a text or to an integer's decimal digits; and the other
  names of the expression vocabulary (docs/SCE_ACCEPTED_SUBSET.md section 3.4.1);
- **each checked as the lanes check it** (ADR 0133). An expression's value is a value or a failure, named as every
  lane names it (`overflow`, `divide-by-zero`, `out-of-range`, `capacity-exceeded`, `precondition`). A failure ends
  the computation where it occurs, and what the statement around it then does is section 2.15's rule ("Integer
  operations", "List variables"), written once in `Effects.tla`: an `<assign>`, a `<log>`, an `<sce:append>` or a host
  action whose expression fails does nothing, a condition that fails is false, `error.execution` is raised on the
  internal queue where the document declares it, and the block ends at the element that raised. The model writes
  every operation checked, those the build proves total included, and for each statement the build classifies as
  total (`Action::native_fails`, ADR 0133 D2) states the obligation that it never fails, beside D4's: TLC judges the
  interval analysis on every state it explores, and a failure the build said could not occur is a defect of the
  analysis, reported with the run that reaches it.

**The statements.**

- `<assign location>` into a variable or a place below it (a record's field at any depth, a union's field written in
  place, ADR 0133 D1), checked as a store is (an integer into a narrower width, a text past its bound).
- `<if>`, `<elseif>` and `<else>` as a chain of conditions in document order.
- `<foreach>` over a list, a map or a set: a fold over the copy the loop began with, in its order (put order for a map
  or a set), binding the item, the key (`sce:key`) and the index, a `uint32`; each pass's effect is composed onto the
  one before, its writes in sequence and its events and calls appended, so a loop's model is its sequence of effects.
- `<sce:append>` and `<sce:clear>` of a list, an append to a full list failing `capacity-exceeded`; `<sce:put>`,
  `<sce:remove>` and `<sce:clear>` of a map or a set, at a nested target too (ADR 0108 D12), each an operator of
  `Map.tla`: a put inserts its key last or replaces its value where it stands, and answers `inserted`, `replaced` or
  `full`; a remove answers `removed` or `absent`; the answer is written to the `outcome` variable as the variant of
  `sce:std/collection/put_outcome.scxml` or `remove_outcome.scxml`; a put with no `outcome` that finds its map full
  fails, and so does a nested target whose outer key is absent (`out-of-range`).
- `<raise>`; a `<send>` to the session's own external queue or to `#_internal`, with its `<param>`s and its
  `namelist` as the event's data, a record of the fields its schema declares; a `<log>`, computed for its failure and
  shown to no one.
- **An event is a name and its data.** An entry of either queue is a record of the event's name and its data. An event
  from outside takes its data from the values the exploration names for its schema's fields (ADR 0115 D7 and D15); an
  event the document raises or sends carries what its parameters computed; `<raise>`'s data, and those of an event
  whose schema declares no field, are the empty record.

**Host calls.**

- A `<sce:action>` is an output. A block's effect gains the sequence of the calls it makes, each the action's name and
  its arguments, in the order run; a step's calls are what a lane's run reports, and D8 compares them call by call.
- **Its answer is the environment's.** Whether the host performed the operation (docs/SCE_ACCEPTED_SUBSET.md section
  2.11), and the value of an operation that answers one (ADR 0109 D5 and D6), are choices of the environment, drawn
  from the answers the exploration names for that operation (ADR 0115 D15). An answer that the operation was not
  performed raises `error.execution`, as every lane raises it. That a host performs its duties is an assumption,
  named with every verdict (D1, item 4; ADR 0038's `performs`).
- A host operation whose answers the exploration does not name is refused by name (ADR 0115 D15), and the document has
  no model.

**Algorithms.** An algorithm a document calls (`<sce:call>`, or a call in an expression) is translated once, into a
module named by it, as an operator from its arguments to its value or, where it declares `may-fail`, its failure. Its
body is written by the statements above and its own (`<sce:var>`, `<sce:assign>`, `<sce:append>`, `<sce:if>`,
`<sce:foreach>`, `<sce:require>`, `<sce:return>`, an `inout` map, ADR 0108 D6), and every document that calls it
extends that module. A `<sce:while>` is a recursive operator, which TLC evaluates. Apalache takes no recursive
operator, and reads a loop only as a fold of the length its `<sce:decreases>` measure gives (ADR 0115 D10); an
Apalache run of a document that calls a loop with no measure is refused by name, and TLC's stands.

**The modules written once**, beside `docs/formal/AppendixD.tla`, each citing the text it formalises and the only
definition the generator reads. They are part of D2's surface: D2 registers them with `AppendixD.tla`, and where they
land first, the step that lands D2 registers them.

| Module | Holds |
|---|---|
| `AppendixD.tla`, amended by a commit of its own (ADR 0115 D11) | an event as a record of its name and its data; `Execute` handed the configuration at the moment the block runs, as `In()` reads it there; an `Effect` that carries its host calls |
| `docs/formal/Values.tla` | each kind's form and predicate: the widths' ranges, a text and its `Utf8Length`, a byte string, a bounded sequence, a variant |
| `docs/formal/Checked.tla` | every operation of SCE_FORGE.md sections 3.4.1 and 3.4.2, each answering a value or the name of a failure, with TLC's range cut as above |
| `docs/formal/Map.tla` | a map and a set in put order: `Has`, `Get`, `Put` and its outcome, `Remove` and its outcome, `Clear`, `Len` and the walk; the module of a map that ADR 0165 (graphs, proposed) D9 waits for |
| `docs/formal/Compare.tla` | ADR 0151 D2's one total comparison of every type, read by ADR 0151 D7 and by every record that orders values; built by whichever of order step 2a here and ADR 0165's step 2 lands first, the other citing it |
| `docs/formal/Effects.tla` | a block as the composition of its statements' effects, section 2.15's rule of a failure, the fold of a `<foreach>`, a write at a place, a call appended |

Each has its laws checked by TLC over small domains, and parts taken out that TLC must find (ADR 0115 D5): a `Put`
that appends a present key again, a quotient rounded toward minus infinity, a text's bound counted in code points, a
walk in the order of the keys, an addition that wraps, a comparison decided by the last field.

**What each document generates.**

- `<machine>.tla`, as now (its tables, `Data0`, `Guard`, `Execute`), with each variable's type as a predicate of
  `Values.tla`, the declared order of each enum it imports, its events' schemas, each condition and block over the
  modules above, and the obligation of each statement the build classifies as total.
- `<algorithm>.tla`, once per algorithm a document set reaches, shared by every document that calls it.
- `<machine>.cfg`, from the exploration (ADR 0115 D15), which names each field's values (an integer's within its
  width, a text's as texts read into code points, a record's, a union's and a list's built from them), each host
  operation's answers and the counts. A value the field's type does not hold is refused, as ADR 0115 D7 refuses one.
- **The second encoding** (ADR 0119 D2) holds what JANI's core holds: a bounded integer of its width's range, a
  boolean, and an enum as the position of its variant. A document whose data hold another kind has a TLA+ model and
  no JANI model, refused for JANI by the name of the kind, until an amendment of ADR 0119 writes the others through
  JANI's `arrays` and `datatypes` extensions; ADR 0119 D4's checks are of the documents both encodings hold.

**The formal level.** TLC checks the translation's agreement with the lanes through D8's replay: translation
validation of each output, never a proof of the generator (D11).

- **A run carries its data.** The run trace gains, at every step, the datamodel after it in the saved form every lane
  already writes (a map as the array of its entries in put order, ADR 0108 D8; a 64-bit integer as the text of its
  digits), the data of the event the step took, and the host calls it made with the answers given. It is a new
  `TRACE_FORMAT_VERSION`, written by each lane's trace port. The judge reads each value into its canonical form,
  holds it equal to the model's at every step, and refuses a run whose datamodel the model does not reach, naming the
  first variable that differs.
- **Each module can refuse a run.** Fixture documents are entered in `docs/formal/models.json` with a run that holds,
  and in `docs/formal/traces.json` with every lane's runs of their scenarios, and each part taken out of a module
  makes a lane's recorded run refused: of `static_overflow.scxml`, an addition that wraps; of
  `static_wrapping_counter.scxml`, a wrapping addition that fails; of `static_string_capacity.scxml`, a bound counted
  in code points; of `map_faces.scxml`, a walk in the order of the keys and a put that appends a present key; of
  `static_host_call.scxml`, a call dropped.
- **The verdicts** are ADR 0115 D15's: *holds within* the exploration's bounds, `integer-range` among them where TLC
  cut a width; for every length, Apalache's (D9).

**What is refused, by name, and what lifts it.**

| Refusal | Of | Lifted by |
|---|---|---|
| `Construct::Real`, new | a `float32` or `float64` value or operation | no record: neither checker holds a binary floating number, ADR 0115 D15 refuses a real by name, and a model of IEEE 754 arithmetic would be a decision of its own; an exact number is the decimal of ADR 0162 (proposed) |
| `Construct::Decimal`, new | a decimal | ADR 0162 (a decimal, proposed), its step 9 |
| `Construct::Inductive`, new | a value of a recursive type | ADR 0164 (recursive types, proposed), its step 5 |
| `Construct::View`, new | a variable that holds a view's value | ADR 0151, its step 4 |
| `Construct::Action`, naming the statement | a graph statement | ADR 0165 (graphs, proposed), its step 7 |
| `Construct::HostAnswer`, new | a host operation whose answers the exploration does not name | the exploration that names them |
| `Construct::Invoke`, `Construct::DoneData`, and `Construct::Send` for a target other than the session's own queues | a child session, its done data, a send to another session | D3, its second item, and for a send between machines D3's composition of a deployment |
| `Construct::OnSample` | a link's sample | the link's promise module (D2, D8; ADR 0111's amendment) |
| `Construct::Send` for a `delayexpr`, an `idlocation` or a `sendidexpr`, and `Construct::Action` for `<cancel>` | a computed delay, a send's id and its cancellation | ADR 0115 D7 and D15 (time as a variable of the model, and the cancellation of a delayed send); `AppendixD.tla` makes a delayed send due at any moment and cancels none, and no step of this order schedules time yet |
| `Construct::Send` for an `eventexpr`, a `typeexpr` or a `<content>` | a computed name or type, a body as content | no record |
| `Construct::Action` for a native `<script>` block | code of a backend's language | none: native code has no model; what it does belongs in a host action or in a seam with a contract (D7) |
| `Construct::LateBinding` | `binding="late"` | no record: `AppendixD.tla` binds early |
| `Construct::FeatureConditions`, `Construct::TestComponent` | a product line, a test component | none, by design: a variant is projected to a plain document first, and a test component is judged by its profile (ADR 0091) |
| `Construct::Datamodel` | a data model other than `null`, `ecmascript` (whose integers and booleans stay as they are) and `sce-static` | no record |
| `Construct::Unspellable`, `Construct::Name`, `Construct::DottedStateId`, `Construct::EventName` | an id or a name TLA+ cannot spell | none; a text value is a sequence of code points and no longer meets `Unspellable` |

Lifted by this decision: `Construct::Datamodel("sce-static")`; `Construct::StateDatamodel`, since under early binding
every `<data>` is bound when the session starts, in document order, wherever it is written; `Construct::Send` for a
`<param>`, a `namelist` and the target `#_internal`; and `In()` in executable content. Each new arm joins the closed
list of `Construct` with its message, and a construct that is neither translated nor named stays refused as an action
or an expression with no translation.

- **Amends** ADR 0115 D7, D8 and D15 and ADR 0119 D2 (header, "Amends"), and is what ADR 0151 D7's model per document
  rests on. **Depends on** nothing but `AppendixD.tla`. Every step that reads a model of a `sce-static` document
  depends on it: D3's later items, D4, D5, D6 and D16 for such a document, ADR 0151's step 4, and the steps the
  proposed ADR 0162, 0164, 0165 and 0167 give their models.

## Consequences

- Every formal record names the part of D1 it serves, and a request for proof is answered by the step of the order it
  belongs to.
- The proof of a system becomes four kinds of judgement a program runs, and what a person writes is listed (D1).
- The trusted base is the list of D14 and nothing else; a verdict that rests on more says so.
- Three tools join the toolchain as their steps land (D13), each with its runner in the one judge and its runs that
  show it can refuse.
- The generator gains outputs (obligations, harnesses, a matrix, paths) to keep right, and D8, D11 and the judges are
  what keep them so.
- A test of a generated machine is a function of its script and its seed (D15), and its expected values are the
  model's (D16), so a test that sometimes fails points at a defect of the structure, which is fixed there.
- (2026-11-29) A `sce-static` document, the class every lane generates, has a model and its runs are read against
  it value by value (D17), so a property of its data, and every record that reads its model, can be judged; what is
  refused is refused by name, with the record that lifts it.

## Alternatives rejected

- **Amending ADR 0115 in place.** ADR 0115 is the levels of one check; this record is the goal of every check and the
  order across ADR 0111, 0115, 0118, 0119, 0146 and 0147. Each of its decisions amends the record it belongs to.
- **TLAPS for every theorem.** A proof of every theorem for every size costs upkeep at every change; D5 and D9 give most
  properties every size or every length mechanically, and TLAPS proves only what they cannot.
- **Proving the generator, or qualifying it as a development tool.** It moves with every construct; D11 checks each
  output and leaves the qualification to the checkers, which move less.
- **A deployment checked only as one product.** Its state grows with the product of its machines; D8 checks each
  machine once and keeps the product for one small size.
- **Seams proved by hand-written harnesses.** A harness would be a second statement of the contract, written by whoever
  writes the seam.
- **A seam rule by convention.** Thinness that no program checks is the boundary the request says must be the
  machine's to keep.
- **Lowering a flaky test's rate** by retries, longer timeouts or a sleep before a check. It hides the defect a
  schedule exposed; a seed that reproduces it shows it (D15).
- **A simulated seam written per test.** It models what its author thought of, and a seam that models less than the
  engine relies on lets a defect through; D15 holds it to the shipped seam's contract.
- **Expected values written by hand beside a model.** They are a second statement of the model, written by whoever
  writes the test; D16 takes them from the model.
- **Naming the first consumer's system as the scope.** The goal is any repository's (the owner's "범용적으로",
  generally); a consumer is a case that tests it.
- **(2026-11-29) A text as a TLA+ string.** TLC holds a string as an atom: it has no length in UTF-8 bytes and no
  code points, so neither a text's bound nor ADR 0151 D2's comparison could be stated (D17).
- **(2026-11-29) A map as a function from its keys.** It forgets the order a walk reads on every lane (ADR 0108 D2),
  and a model that forgot it would accept a run walked in another order (D17).
- **(2026-11-29) Refusing a document whose integers pass TLC's range.** Every `uint64` and `int64` variable would
  refuse its document though most never leave 32 bits; D17 cuts the step TLC cannot compute, names the bound, and
  leaves the whole range to Apalache.

## Order of work

Two tracks, each step landing with a test that fails without it. Track A is the model's, track B the code's; they
share only ADR 0115 D10's expression language.

1. This record.
2. (A) D2: the semantics registered, and the modules of the connectors as promises. A specification precedes what
   refines it, and every later step of A reads the model; it unblocks D3 to D10.
2a. (A) D17's modules, amended 2026-11-29: `Values.tla`, `Checked.tla`, `Map.tla`, `Compare.tla` and `Effects.tla`,
   each entered in `docs/formal/models.json` with its laws checked by TLC and each part D17 names taken out, which TLC
   must find; and `AppendixD.tla` amended by a commit of its own (an event with its data, `Execute` with the
   configuration, an effect with its calls), under which `W3CTest388`, `W3CTest570` and `W3CTest403c` still hold and
   their recorded runs are still accepted. It needs nothing of step 2 and may land before it. Red without it: the
   modules do not exist.
2b. (A) D17's translation in `sce-codegen model`: the values, the expressions checked, the statements, the host calls,
   the algorithms, the second encoding's refusal by kind, and every refusal of D17's table by name, held by the tests
   of `sce-build/src/formal/document_model.rs`, one per kind, per statement and per refusal. Red without it: a
   `sce-static` document is refused as `Construct::Datamodel("sce-static")`.
2c. (A) D17's replay: the run trace's new version, written by the trace port of each of the seven lanes; the judge
   that holds every step's datamodel to the model's; the fixtures D17 names entered in `docs/formal/models.json` and
   `docs/formal/traces.json`, each part taken out refusing a recorded run. Red without it: a lane whose datamodel
   differs from the model's is accepted. Every later step that reads a model of a `sce-static` document comes after
   2c: step 3's later items, 3a, 4, 5 and 6 for such a document, ADR 0151's step 4, and the model steps of the
   proposed ADR 0162 (step 9), 0164 (step 5), 0165 (step 7) and 0167 (step 5).
3. (A) D3: the breadth of ADR 0115 D7 in its order. What has no model has no verdict of any later step.
3a. (A) D16 for one machine at a time, as each document gains its model. It needs nothing but D3 and the harnesses
   and judge that replay ADR 0115 D15's scenarios now, and it makes every expected value of a machine's tests
   mechanical. The clauses of a promise join it with step 5, and a deployment's paths with step 10a.
4. (A) D4: the obligations. They are mechanical once every bound is declared, which it is, and they are D5's types and
   D9's candidates.
5. (A) D6: the refinement declared. It ties a document to the promise a user reads, which D8 composes by and D10 cites.
6. (A) D5: every size by data independence and symmetry, which takes most safety properties to every size with no
   person's proof.
7. (A) D8: a deployment by composition. It needs the connectors of step 2 and the refinements of step 5, and is what
   keeps a large system's check finite.
8. (A) D9: Houdini over D4's candidates, and the TLAPS lane, whose first theorem (the follower) needs nothing of this
   order and may land as soon as TLAPS is pinned.
9. (B) D11: the tables of each output held to the model's. It needs only what exists, and closes the generator's part
   of the trusted base for the control of every machine.
10. (B) D7: the seam contract, the harnesses and the thinness rule, after ADR 0115 D10's postcondition.
10a. (B) D15, in the same record as D7: the host's rules (a clock only it moves, one seed, counted guards) on every
   lane for the seams that exist (the clock, the door, a Mesh binding, a host invocation), starting from
   `backends/rust/world`; then each simulated seam held to its contract as D7 gives the real one its own; the link's
   once ADR 0111 steps 3 and 3a land. It makes a test's schedule mechanical, and is what D16 runs a deployment on.
11. D10, then D12's corpus and the flips to `stable`, once the formats of the steps above have stopped moving.

**Where this meets ADR 0111's order** (its order of work, with its amendment of 2026-11-28 on a branch):

- Its step 3 lands a seam, the link's lending operation, before D7 gives seams a contract. This record does not
  reorder ADR 0111, which was agreed with a consumer and confirmed by the owner: that operation is D7's first seam, and
  is given its contract when D7 lands.
- Its amendment writes the steps of `LinkChannel.tla` with step 3. D8 needs that module as the link's promise, with
  the implementation refining it; written so, the two orders agree, and written as a mechanism it is written twice.
- Its steps 3 and 3a give the link a lending driver on the loopback fixtures of `tests/forge/conformance/link_events/`.
  D15's in-memory link is that driver with its choices drawn from a seed, held to the same contract, and comes after
  them; it reorders nothing of ADR 0111.
- Its step 7 (the drive and the ask) changes ADR 0084's door, which ADR 0147 proved: `InboxService.tla` changes by a
  commit of its own (ADR 0115 D11), and the Verus proof of the Rust drain and the inbox histories follow. D8 reads the
  door as a connector, so step 7 comes before D8 treats it as one.

ADR 0115's own order (D7, then D4, then D10, D13, D3 and D12) is kept; this record orders what comes after it and
widens D7 first.

## Sources

Abadi and Lamport 1991 (refinement mappings) and 1995 (conjoining specifications); Lamport 2002 (*Specifying
Systems*); Pnueli, Siegel and Singerman 1998; Hawblitzel et al., SOSP 2015; Fischer, Lynch and Paterson 1985; Hoare
1969; and ADR 0115's checks of them, which this record relies on as ADR 0115 does. **Not checked against their texts
for this record, and to be checked before a step relies on them:** Necula, PLDI 2000 (translation validation for an
optimising compiler); Wolper 1986 (data independence); Lazić, Newcomb and Roscoe 2003 (its thresholds); Ip and Dill
1996 (symmetry); Emerson and Kahlon 2000 (cutoffs); Flanagan and Leino 2001 (Houdini); Clarke, Kroening and Lerda 2004
(CBMC); the Kani documentation; what FoundationDB and TigerBeetle publish of their deterministic simulation
testing; DO-178C, DO-330, ISO 26262-8 and IEC 61508-3, which were not read for this record.
