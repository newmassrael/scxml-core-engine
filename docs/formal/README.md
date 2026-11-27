# Formal models

What a protocol of SCE promises, how it keeps it, and what shows that. Decided by ADR 0115
(`docs/adr/0115-a-protocol-is-specified-by-what-it-promises-refined-to-what-it-does-traced-from-the-code-that-does-it-and-a-gate-runs-all-of-it.md`);
this file says what is here and how to read it.

## The levels, and what is built

| Level (ADR 0115) | Here | Built |
|---|---|---|
| 1. Promise | `AtMostOnceService.tla`, `JournalService.tla`, `PrincipalService.tla`, `InboxService.tla`, `OrderService.tla`, `ReceiptService.tla`: what a user may rely on, no mechanism; `CheckpointService.tla`: two of them over one server, and what ties them | yes, for seven protocols and the composition of two |
| 2. Protocol refinement | `Refines` in `OpenInstances.tla`, `EventJournal.tla`, `Principals.tla`, `BrowserNode.tla`, `EngineInbox.tla`, `OrderingBuffer.tla`, `DurableReceipt.tla` and `CheckpointWrite.tla` | yes, for seven protocols and the composition of two |
| Bounded exploration | every `.cfg`: one size, one part taken out or none | yes |
| Progress and what stays possible (D9) | `progress` and `possible` of `models.json` | yes |
| 3. Theorem for every size (D3) | `inductive` of `models.json`: an inductive invariant per theorem, judged by Apalache at a configuration's parameters | for every length, at fixed sizes, for three theorems; for every size, none (below) |
| 4. A model generated from a document (D7, D8) | `AppendixD.tla`: Appendix D once, over a document's tables; `W3CTest388.tla`, `W3CTest570.tla`, `W3CTest403c.tla`: three documents of the W3C corpus, written by `sce-codegen model` | the semantics, the generator for the constructs these three use, and their models |
| 5. A run read against a model (D4, D8) | `traces.json`: a run of each of the three W3C documents by the Rust, C11, C++, Go, Kotlin, Python and Swift lanes (`traces/`), each a behaviour of its model, and an altered run per kind of step that must be refused where it was altered | the Rust, C11, C++, Go, Kotlin, Python and Swift lanes, for three documents |
| 4q. Quantity (ADR 0119) | `quantities.json`: `W3CTest403c.jani`, the second encoding of `W3CTest403c.tla`, a JANI model Storm reads, written by `sce-codegen model --jani` from the same tables; its graph held to TLC's and the recorded runs of the module replayed over it (D4.1, D4.2) | the model and its two checks, for one document; no distribution and no property yet (D3) |
| 6. A function's contract (D10, D13) | `proofs.json`: the numbers of the `https` client, the Rust engine's drain of its inbox refining `EngineInbox` (ADR 0147, D4), the session store's lease and write rules refining the store's actions of `EventJournal` and its journal's positions refining `EventJournal`'s numbering, horizon and follower, a target's outbound side, the edge's rate limits, and what a pool decides of a session's slot under its locks (the admit and its bounds, an inbox's count and when it leaves the table, the key's lease), by Verus | yes, for seven modules |

## The models

| Module | What it is of |
|---|---|
| `SharedListConvergence`, `SharedListConflicts` | ADR 0034, the shared list |
| `BroadcastLog`, `BroadcastLogWrapped` | ADR 0062, the broadcast log of one producer and its readers |
| `OpenInstances` | ADR 0113, open instances: joining, the lease, the fence, requests at most once |
| `EventJournal` | ADR 0114, the event journal: the write, the horizon, a follower and a question as of a moment |
| `Principals` | ADR 0116, a person bound to an incarnation: the proof, the end, the write's condition |
| `BrowserNode` | ADR 0117, the pages of one browser as one node: the lock, the store, a cleared store, a change of person |
| `Freshness` | ADR 0117, a request signed ahead by one who was inside a page |
| `InboxService`, `EngineInbox` | ADR 0084 and ADR 0147, the one door of every runtime: an inbox per session, the drain at the boundary and before an event of the owner's own, the address resolved when posted, a post refused when posted (a session the tree does not hold, an ended one, a full inbox) |
| `SessionPool` | ADR 0115 D13 and ADR 0109 D1, a pool of sessions in one process: who holds the table, an inbox and a session's live lock, and when; a post and a removal under the table, an eviction that reads the inbox again, the bounds, a stop set and read under the table |
| `OrderService`, `OrderingBuffer` | SCE_MESH.md section 10.6, the receive half's order of one sender's stream (`backends/rust/mesh/src/inbound.rs`): an envelope held ahead of its number, the release of what follows on, a gap told lost once it has waited, and a durable stream, which is anchored at its first number and never told a gap |
| `OrderService`, `OrderingBuffer` | SCE_MESH.md section 10.6, the receive half's order of one sender's stream (`backends/rust/mesh/src/inbound.rs`): a stream expected from its first number whatever arrives first, an envelope held ahead of its number, the release of what follows on, a gap told lost once it has waited (the numbers before what a late listener heard first among them), and a durable stream, which is never told a gap |
| `ReceiptService`, `DurableReceipt` | ADR 0045 and SCE_MESH.md section 10.11, the router's durable receipt (`backends/rust/mesh/src/router.rs`): an envelope held until a durable checkpoint holds it, kept and sent again until acknowledged, a repeat told by its number, an acknowledgement of what a durable checkpoint holds, and either machine stopped and restored |
| `CheckpointService`, `CheckpointWrite` | ADR 0115 D12, the composition: `OpenInstances` and `EventJournal` over the one write in which a session's state, its outbox, its journal and a member's records are made durable together, and the time a lease is judged by |
| `AppendixD`, `W3CTest388`, `W3CTest570`, `W3CTest403c` | ADR 0115 D7, the SCXML recommendation's Appendix D over a document's tables, and three documents of the W3C corpus over it: history recorded and restored, the done events of a parallel state's regions and of the state, the removal of conflicting transitions |

`AppendixD` is declared `abstract`: it is the semantics a document's model instantiates, and is run only by those. A
document's module extends it with its tables and promises Appendix D with every part in (`Refines`). Its holding runs
keep that promise by construction; what shows a part is the run that takes it out, which must break an invariant of the
document and the promise both.

The three W3C modules are generated: `sce-codegen model <document> --module <name> --w3c-verdict` writes each from the
tables the generated code is printed from, row `r` of `DocTransitions` being the transition the code numbers `r - 1`,
with the corpus's verdict (`NeverFails`, `Passes`) as properties. Their configurations and their entries in
`models.json` are written by hand. Regenerate them with `scripts/regen_all_committed_trees.sh formal-models`, never edit
them; `regen-reproduces` holds them to their documents. A document carrying a construct the generator has no translation
for (an `<invoke>`, a `<log>`, a string, `In()`, ...: `Construct` in `sce-build/src/formal/document_model.rs` is the
list) is refused by name with `formal/no-translation` and has no model, never a model of part of it.

`CheckpointWrite` instantiates `OpenInstances` and `EventJournal` and copies neither: every step of each but the
run that takes an effect and the transaction, and in place of both the one write, which the store refuses whole by the
lease's fence or by the incarnation the run was made under. Each protocol's properties are checked there where the other
moves too, and a part of one is seen to carry a property of the other: without the lease's fence of `EventJournal`, a
request of `OpenInstances` takes effect twice. Its promise, `CheckpointService`, is the two promises and what ties
them: a request takes effect in the step that makes its entry, and an effect's entry is never passed over. Time is a
variable there, and each process's clock lags it: no property of either protocol is one of a clock (the run that lets a
clock run ahead keeps the promise), and the one property of a deadline, `TakenWhenDue`, is one of time.
A renewal, a lease given up and an owner that takes its own key again are steps of it, each guarded
as the session store guards it (the store's `rules/lease.rs`, proved by Verus to refine
`EventJournal`), and a run without each guard finds `TakenWhenDue` broken.

Each module opens with what it takes as given ("Taken as given"). The code must be shown to have it, by the test or the
tool that judges that level; the model shows nothing of it.

## A machine's properties (D15)

A document's author states what a machine promises in a document of its own, `sce:kind="properties"`, and the values a
search of it is made over in another, `sce:kind="exploration"`; neither is part of the machine's document, which a build
reads and an author edits. `sce-codegen formal properties --properties <document> --exploration <document>` writes the
machine's model as `sce-codegen model` does, and beside it a module of the properties
(`sce-build/src/formal/properties.rs`): an observation at the end of each macrostep (the event it took, the states the
machine is in and the data it publishes), each observer a statechart of its own over Appendix D that takes every
observation's event as its event, and each guarantee as the formula of its form. Appendix D is compiled into the
generator (`document_model::SEMANTICS_TEXT`), so the judge stages the semantics the model was written against.

- `<sce:always>` and `<sce:step>` are invariants of the machine with its observations; `<sce:recurs>` and
  `<sce:leads-to>` temporal properties under the interpreter's fairness and time's (a delayed send that is due comes);
  `<sce:possible>` is `AG EF` over the whole graph, as in `models.json`.
- An assumption reads only the event that comes (`_event.name`): an `<sce:always>` of it is a condition each event that
  comes meets, an `<sce:recurs>` one that keeps coming. One that reads the machine is refused by name
  (`formal/property-refused`), as is a guarantee that reads nothing of it.
- Each guarantee that holds is judged again with each part of it made stronger, false under an even count of negations
  and true elsewhere; one that still holds is vacuous, and refused.
- A verdict says no more than was judged: *holds of the whole space*, *holds within* the bounds the search reached (the
  most events from outside), *violated*, *vacuous* or *not judged*. A `<sce:possible>` that fails where a bound was
  reached is not judged, since the bound may be what cut the way. Every macrostep must end, under every document.
- `--record` writes the judgement with the digests of the machine, the properties and their observers, the exploration
  and the judge (`schemas/sce-formal-verdicts.v1.schema.json`); `--recorded` reports a recorded one instead of judging,
  and refuses it when a digest moved: a verdict of another digest is none.

- **A violation is a scenario** (D15, and D8's converse). The properties module names each kind of step by an action of
  its own (`PStart`, `PEventless`, `PInternal`, `PExternal`, `PStop`, `PArrive`, `PDueAny`), so the behaviour TLC
  reports says which step of Appendix D each state was reached by; `sce-build/src/formal/behaviour.rs` reads it back as
  a run in the run trace format, with the events the environment put on the external queue as its `inputs` (each an
  `arrive` or a delayed send come `due`, after the count of steps over when it came) and, for a run that ends in a
  cycle, the step the `cycle` starts at. The record holds it with the verdict; a violation whose behaviour is not read
  as a scenario is *not judged*, since no lane could show it is the code's. A `<sce:possible>` that holds has a
  witness, the run TLC finds to a state where its condition holds, held the same way. A `<sce:possible>` that fails
  has no run, and its scenario of the state from which it was lost is a step after this one. `--scenarios <dir>`
  writes each as `<req>.scenario.json` (`macrosteps.scenario.json` for a macrostep that does not end).
- **Every lane replays it.** A lane's harness gives the machine a scenario's inputs, on a manual clock, with its trace
  port on, and records the run the port is told as `<req>.<lane>.json` beside it; `sce-build/src/formal/replay.rs`
  judges each such run to be the scenario's, step for step, the configurations as sets. Every lane replays the
  fixture's scenarios, each with the fixture's machine generated in its test tree and its trace port on (D8), and
  `sce-build/tests/a_scenario_is_the_run_of_every_lane.rs` holds each lane's runs to them without TLC:
  Rust (`backends/rust/tests/tests/a_scenario_of_the_turnstile_is_its_run.rs`), C11
  (`backends/c/tests/integration/test_a_scenario_of_the_turnstile_is_its_run.c`), C++
  (`tests/w3c/run_traces/AScenarioOfTheTurnstileIsItsRunTest.cpp`), Go
  (`backends/go/tests/run_trace/a_scenario_of_the_turnstile_is_its_run_test.go`), Kotlin
  (`backends/kotlin/tests/src/test/kotlin/com/sce/formal/AScenarioOfTheTurnstileIsItsRunTest.kt`), Python
  (`backends/python/tests/trace_port/test_a_scenario_of_the_turnstile_is_its_run.py`) and Swift
  (`backends/swift/tests/Tests/EcmaScriptMachinesTests/AScenarioOfTheTurnstileIsItsRunTests.swift`). Each harness
  reads a scenario's `inputs`, gives each event by name and moves a clock it owns to the next deadline. A move makes
  every send of that deadline due at once, where the model's time (any pending send, at any moment) may make them due
  one at a time with steps between, so a `due` input that finds no send pending was given by the move before it, and
  gives nothing: the steps compared say whether it was. It gives an input once the machine has taken every step it
  can, so a scenario whose environment put an event on the queue in the middle of a macrostep that also sends to its
  own external queue, whose model made a later delayed send due before an earlier one, or made sends of one deadline
  due around another event, is one no harness replays as the model ran it, and the judge finds the runs part.
- **Coverage.** `--coverage` alters the machine in each way of a closed list (`sce-build/src/formal/coverage.rs`: a
  transition's condition made false, which takes it out, or made true; an action taken out; the event an action sends
  or raises changed to one no transition names; the value an `<assign>` writes changed, an integer by one more and a
  boolean to its negation) and judges every guarantee that held over each altered machine. A guarantee no alteration
  breaks is refused by name (`formal/property-refused`); an alteration no guarantee notices is listed, and the document
  has as yet no way to say why it may go unnoticed.

An observer reads the name of the event an observation took and its own data, and no more of the observation; a
property of a deployment (D12) and the watch beside a lane are the steps after this one. The formal-models gate judges
the fixture under `sce-build/tests/fixtures/formal_properties`: every form held, every form broken on purpose, a
vacuous guarantee, an assumption about the machine itself, the scenario of each violation and witness held to the one
committed under `scenarios/` (written with one worker, whose first violation of a level is one run), and a guarantee
no alteration breaks.

## Theorems, bounded and for every size (D3)

An inductive invariant `Inv` of a theorem `P` is three obligations, each checked by Apalache symbolically over every
state the predicate it starts from allows, not only the reachable ones: **base**, `Init` implies `Inv`; **step**, a step
of `Next` from a state that satisfies `Inv` ends in one that does; **implies**, `Inv` implies `P`. Then `P` holds after
executions of every length, at the parameters of the configuration the run names. Each invariant has runs that take a
part out, in which the obligation they name fails, so it shows what it stands on as every property does (D5).

| Theorem | Record | Invariant | Holds at | Fails without |
|---|---|---|---|---|
| a request takes effect at most once (`AtMostOnce`) | ADR 0113, D4 and D5 | `AtMostOnceInductive`, `OpenInstances.tla` | `OpenInstances.cfg` | `Records`, `GcDurable` (step) |
| a follower misses nothing (`NonePassed`, `FollowedOnce`) | ADR 0114, D3 and D4 | `FollowerInductive`, `EventJournal.tla` | `EventJournal.cfg` | `Horizon`, `CursorAtomic` (step) |
| each queue in order, once, nothing lost (`InOrder`, `AtMostOnce`, `NoLoss`) | ADR 0147, D2 | `QueueInductive`, `EngineInbox.tla` | `EngineInbox.cfg`, `EngineInbox_never_closes.cfg`, `EngineInbox_bounded.cfg` | `TakeOut`, `AddressAtPost`, `Lookup` (step) |

What each says, and what is **bounded**:

- **Every other property is bounded**: TLC explores every reachable state of a configuration, so it holds at that size
  after executions of every length, and says nothing of a larger size.
- **The three theorems above hold for every length at the sizes named**, which TLC's whole exploration of the same
  configurations also gives. What the invariant adds is the reason, checked over every state it allows: a hole in the
  argument is found even where no reachable state shows it, and a change to the protocol that keeps the theorem but
  breaks its reason is seen. The step's cost follows the size of one state and not the count of reachable ones, so a
  size TLC cannot explore is checked by naming a larger configuration in the invariant's runs.
- **None is proved for every size.** That is TLAPS's (D3), which is not a dependency: the owner approves it per theorem
  (ADR 0115, Questions decided at acceptance, 1). Decided per theorem, with the reason:
  - *At most once*: **not worth its upkeep.** The model has seventeen actions over sets of records, with a cardinality
    bound and a choice of the least number waited on, and it moves with ADR 0113's steps 4 and 5; each move would prove
    again obligations whose number grows with the actions. The argument is per request of one instance (one name, and
    what ran is kept), so a larger size costs Apalache time and no upkeep.
  - *A follower misses nothing*: **worth its upkeep, and the first to prove when TLAPS is approved.** It is what every
    follower of every deployment relies on, its argument is linear arithmetic over positions and one holder of a fence
    per session, which TLAPS's SMT back ends discharge, and the protocol of ADR 0114 has landed. Until then it is held at
    the size named.
  - *The inbox's order, once and no loss*: **not worth its upkeep.** The tree is two sessions by construction, and the
    sizes are the inputs and own events, which the argument treats one at a time (what a session linearized is its
    queue, then its inbox); a proof over sequences for every count costs more upkeep than Apalache at a larger count.

A model is written in the fragment both tools read. Apalache types every definition, so a model extends the standard
module `Apalache` where a sequence is built from a set (`MkSeq`) or a sequence variable is given a value (`Gen`), and
TLC reads that module from the pinned Apalache. Each constant and variable carries Apalache's type in a comment
(`@type:`), which TLC reads as a comment.

## Running them

`sce-codegen formal models --manifest docs/formal/models.json` (`--only <module>` for one, `--shard k/N` for a part)
checks every model as `models.json` declares it, with the TLC the judge pins by digest
(`sce-build/src/formal/toolchain.json`, printed by `sce-codegen formal toolchain`), run on the Java runtime pinned with it
(Eclipse Temurin's JRE 21, run by its path and never a `java` a search path finds). `sce-codegen formal install` fetches
the pinned tools and admits each by its digest, the runtime with TLC or Apalache; a judge never fetches. Exit 1 (`formal/declaration`,
`formal/verdict`) is a verdict about the models; exit 15 (`formal/tool-unavailable`) is "could not run". The judge is
one command of the code generator (docs/adr/0146), which another repository runs on its own manifest at the revision it
pins, as this one does; `sce-build/tests/fixtures/formal_consumer` is a manifest shaped like a consumer's, judged by the
same gate. The judge's own checks are the unit tests of `sce-build/src/formal`.

A configuration of its own is not enough: `models.json` must declare it, and the judge refuses

- a configuration or a module that is not declared, and a part of a model that no run takes out;
- a property a holding run checks and no run shows to fail: one nothing can break says nothing, however true it is;
- a run that holds with another number of states than is recorded (a model that shrank still reports "no error");
- a run that takes a part out and fails for another reason than it names;
- a model that states no progress and gives no reason, and a progress no run shows to fail without what it stands on;
- a temporal property beside a constraint, a symmetry or a view, under which it may hold for nothing;
- what must stay possible (`AG EF`, decided over the whole graph) and no run that loses it;
- a key it does not know, a tool named in the manifest (the judge pins its tools), and a manifest of another `v`;
- an inductive invariant that implies no property a holding run checks, that no run holds or no run shows to fail, a
  run that holds with a part out, and a violation that does not name the obligation it fails (base, step or implies),
  or fails an earlier one.

The proofs are `proofs.json`'s, judged by `sce-codegen formal proofs --manifest docs/formal/proofs.json` (the gate is
`formal-proofs`): each file verified whole, nothing taken on trust, and each change that breaks what it promises refused.

What a check says (a promise, a property, a contract, what a run must answer) is changed by a commit of its own with its
reason; `sce-codegen formal isolation <base>..<head>` refuses another (ADR 0115 D11), in the gate `formal-isolation`,
which a push's plan and the lane `formal-isolation.yml` hand the ranges they judge.

After a change that moves a count on purpose, `--record` writes the counts TLC found, and the diff is read.

The JANI models are `quantities.json`'s, judged by `sce-codegen formal quantities --manifest docs/formal/quantities.json`
(ADR 0119, D4), with the pinned TLC and the pinned Storm. A JANI model is the same document's model as its TLA+ module, in
a second encoding: JANI has no sets, sequences or recursion, so the generator evaluates Appendix D's control (the
configuration, the history, the queues, the phase, what a step told) over the document's tables and leaves the data to
the model, each step an edge under the conditions that select it. The judge refuses a model that is not what the
generator writes of its document now, and holds the rest to the module: the state graph Storm builds of it must be the
graph TLC writes (`-dump dot`) under the configuration's constants, state by state over the module's variables, and
every recorded run `traces.json` declares of the module must be a path of it, each altered run refused where it was
altered. Storm is published as a Debian package and no static build: `sce-codegen formal install --tool storm` unpacks
the pinned package and runs its program with the libraries it holds, and the system libraries it asks for (GMP, CLN,
GiNaC, GLPK, Z3, libarchive) are the host image's.

The recorded runs are `traces.json`'s, judged by `sce-codegen formal traces --manifest docs/formal/traces.json`
(ADR 0115, D4 and D8). A run is a file of the run trace format (`tests/forge/conformance/run_trace.schema.json`) a
lane's trace port wrote: each step the interpreter took (`start`, `eventless`, `internal`, `external`, `stop`) with
the event, the rows fired, the states exited and entered in order, and the configuration. The judge writes a module
that holds the run as a sequence and constrains the model's next-state relation to it (trace validation), and TLC
accepts the run or names the first step no behaviour explains. The environment's steps, an event from outside and a
delayed send that comes due, are not in a run and may come between any two steps. Every kind of step a run holds has an
altered run, one field given another value, which must be refused at that step: a trace that says too little is
accepted though the run was wrong.

Each lane records its runs with the command below, and the same tests hold each run to its recorded file:

- Rust: `UPDATE_EXPECT=1 cargo test -p sce-rust-tests --test a_run_is_a_behaviour_of_its_model` (the runtime's
  `trace-port` feature, `trace_port::RunTrace`).
- C11: `UPDATE_EXPECT=1 ctest -R c11_a_run_of_test_` (the machine's trace port, `sce/trace_port.h`, on by
  `SCE_TRACE_PORT=1`).
- C++: `UPDATE_EXPECT=1 ctest -R cpp_a_run_of_test_` (the engine's trace port, `core/TracePort.h`, on by
  `SCE_TRACE_PORT=1`).
- Go: `UPDATE_EXPECT=1 go test -tags sce_trace_port ./run_trace/` in `backends/go/tests` (the engine's trace port,
  compiled only with the build tag `sce_trace_port`).
- Kotlin: `UPDATE_EXPECT=1 ./gradlew :sce-kotlin-w3c:test --tests 'com.sce.formal.*'` (the engine's `tracePort`,
  written by `RunTrace`).
- Python: `UPDATE_EXPECT=1 python -m pytest backends/python/tests/trace_port/` (`Engine.set_trace_port`, written by
  `sce_runtime.trace_port`).
- Swift: `UPDATE_EXPECT=1 swift test --filter W3cMachinesTests.ARunIsABehaviourOfItsModel` in `backends/swift/tests`
  once `scripts/regen_swift_w3c.sh` has written the W3C machines (the runtime package's `TracePort` trait, which the
  tests package asks for).

A lane's run of a document is judged against the one model of that document, and these documents leave a run's steps
no choice, so the lanes' runs of one document differ only in their lane.

## What all of this stands on

- **TLC**, the release the judge pins, and the pinned Java runtime that runs it.
- **Apalache**, the pinned release, its Z3 and the pinned Java runtime that runs it; and the types a model names for it, read by a
  person: that they give each variable a value of its type and narrow nothing the invariant does not.
- **Verus**, the pinned release, its Z3 and the Rust release it runs on; that its reading of Rust is the compiler's.
- **Storm**, the pinned package, its libraries and the system's it is run with; that its reading of JANI is the
  format's. What it builds is held to TLC's graph, so a quantity it computes (ADR 0119, D3, when built) is of a model
  whose behaviour TLC judged.
- **The bounds** of a configuration: an error past them is not found. A theorem for every size (level 3, TLAPS) is what
  would say so; an inductive invariant says it for every length at the size named, and no more.
- **That the model is of the code.** No model reads the code. A generated model (level 4) is read from the tables the
  code is printed from, and that the code runs them as the model does is level 5's to show (D8). A proof (level 6) is of
  the code it is written on, and the model its preconditions name is read by a person.
- **That the promise is what was meant.** It is read by a person.
- **That `AppendixD.tla` is the recommendation's algorithm.** It is read by a person against the text, until the W3C
  corpus run by the Interpreter is accepted by it (ADR 0115, D8).
- What is judged elsewhere and not here: the memory order (GenMC), the bytes of a wire (the conformance kits), and
  cryptography (assumed).