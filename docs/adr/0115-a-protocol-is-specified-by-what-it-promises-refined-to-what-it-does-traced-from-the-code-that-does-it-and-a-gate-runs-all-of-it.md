# ADR 0115 — A protocol is specified by what it promises, refined to what it does, traced from the code that does it, and a gate runs all of it

- Status: Accepted (2026-11-22, by the owner), in its third draft: D7 to D11 were added and D13 corrected at the owner's
  order, and the sources checked. D5's gate, and D1 and D2 for two models, were built before it was accepted, at the
  owner's order.
- Date: 2026-11-22
- Written under the owner's standing rule (the most long-term correct structure, textbook in its foundations, cost
  ignored) at the owner's question whether TLA+ can be designed and applied so, and whether SCE can meet what a survey
  of the field asks of it (reasonable.io, "The internet discovers TLA+. Now what?", 2026-09-25).
- Amends (each amended document gains an "Amended by: ADR 0115" line in the landing that moves it): `docs/formal` (a
  README that states the levels), `scripts/formal`, the gate registry, ADR 0113 and ADR 0114 (their "Order of work" gain
  the levels below), `docs/SCE_TRANSITION_TABLE.md` (a run read against a model), `SCE_FORGE.md` section 4.12 (an
  algorithm's contract, D10), `docs/SCE_TEST_PROFILE.md` (a trace is evidence).
- Scope: the models of `docs/formal`, the Rust host core's protocols first, and the `sce-static` documents. A generated
  machine's output does not change.
- Related: ADR 0034, ADR 0062, ADR 0064, ADR 0089, ADR 0091, ADR 0113, ADR 0114, RFC §synth-5-P (GenMC, which judges the
  atomics of the code itself).

## Context

Six models were under `docs/formal`. They found real defects, and each holds small configurations explored whole, with
others in which one part is taken out and TLC must find the violation. What they are not:

1. **Nothing ran them.** No workflow, gate or declaration of bx called `scripts/formal/check.sh`. And its pin was of a
   rolling pre-release whose asset is replaced, so a machine that fetched TLC later was refused: found on the first run
   of the gate on another machine.
2. **They are bounded**, and all but one state no liveness.
3. **They state invariants and no promise.**
4. **They do not judge the code.** A model that holds and a program that differs are both green, and the two drift as
   each is edited.

The fourth has an answer here that other systems have not. A protocol is a hand model because the program is a hand
program. In SCE the document is the source of both: the machine is generated from it in seven languages, so a **model
generated from the same document** cannot drift from the code by editing one of them. What remains is the question every
compiler has, whether the generator was right, and it has a textbook check (translation validation, Pnueli, Siegel and
Singerman 1998), which ADR 0091 applies to its exports.

## The textbook structure

- **A specification is what a user may rely on**, and an implementation is another that refines it (Lamport 2002; Abadi
  and Lamport 1991).
- **Bounded exploration finds errors; an inductive invariant shows their absence for executions of any length, and a
  proof of it for any size.** Apalache checks the step of an invariant symbolically and, as its authors say, with every
  parameter fixed and every state finite (Konnov et al., OOPSLA 2019); TLAPS proves it for every size.
- **A trace accepted by the specification ties a program to a model** (Cirstea, Kuppe, Loillier and Merz, SEFM 2024).
- **A generator is checked by checking each output**, not by proving the generator (Pnueli et al. 1998).
- **A semantics is written once** and a program of the language is data for it (an operational semantics; ADR 0064).
- **Safety alone is met by doing nothing**, so progress is stated under fairness (Lamport 1977; Lamport 2002, chapter
  8).
- **Linear time is enough for what must hold of every execution, and cannot say that an option stays open**; branching
  time can (Emerson and Halpern 1986). Lamport (2002,
  8.9.3) holds that such a property is never a useful assertion about a system, and may be one about a specification:
  that what its environment may do has not been shut out.
- **A contract is a precondition and what follows from it** (Hoare 1969, who proves it of a program that ends and leaves
  its ending to another proof); a language can check it as the program runs (Meyer 1992, Eiffel) or prove it before,
  with its ending (Lattuada et al., Verus, SOSP 2024).
- **A check that cannot fail shows nothing**, and a check nobody runs decides nothing.

## The levels

| Level | Holds | Verdict |
|---|---|---|
| 1. Promise | what a user may rely on, no mechanism | read by a person (D1) |
| 2. Protocol | refines the promise, parts each needed | TLC, whole, small (D2, D5) |
| 3. Theorem | the few that matter, for every size | Apalache, TLAPS (D3) |
| 4. Document | the model generated from the document | TLC over the generated module (D7) |
| 5. Run | a run of the code is a behaviour of the model | TLC over the trace (D4, D8) |
| 6. Function | an algorithm meets its contract; the core's decisions refine the protocol | Verus (D10, D13) |

## Decision

### D1. The promise is a module

Each protocol has an **abstract module** that says what its user may rely on and no mechanism: `AtMostOnceService` and
`JournalService`. It is short enough for a reader to judge by eye and is the sentence a document quotes.

### D2. The protocol refines it

Each protocol module instantiates its abstract module with state functions and TLC checks the abstract `Spec` as a
property of every holding configuration. Where the mapping needs history or prophecy, the variable is added and named as
one. The refinement was written for both protocols on 2026-11-22 and checked; with each part taken out in turn, TLC
finds the promise broken for 12 of 13, and the thirteenth (a clock in place of a position) breaks an answer the promise
does not speak of, which its run says. It also changed the promise: the first `AtMostOnceService` let a request end
unanswered without a fault, so a protocol without its handshake refined it, and the promise now says that nothing the
network does ends a request.

### D3. The few theorems that matter are proved for every size

An **inductive invariant** is written for at most once (ADR 0113, D4 and D5) and for a follower that misses nothing (ADR
0114, D3 and D4). Apalache checks the base case and the step at fixed parameters in the gate, which is any length of
execution at that size; TLAPS proves the same invariant for every size where the proof is worth its upkeep, decided per
theorem and written with its reason in `docs/formal/README.md`. A theorem not proved is named as bounded, and the README
is the one place that says which.

### D4. A run of the protocol code is a behaviour of the protocol model

`backends/rust/world` runs the Rust host core under virtual time over a network it owns and keeps a trace, and the
server suite kills and restarts processes at chosen points. The code reports the events its module names through a
**trace port** of the core, empty unless a test installs one, never on a release path. A run is written as a TLA+ module
holding its sequence, and a trace specification constrains the protocol's next-state relation to it. TLC accepts the run
or names the event it cannot explain. An event may leave a variable unsaid, and that is a danger and not only a
convenience: the authors of trace validation report that an incomplete trace may be accepted though the run was wrong,
in the extreme a trace that says only its length. So a trace declares, per event, the variables it must say; a run that
leaves one unsaid is refused; and the check has an altered run per event kind that it must reject.

### D5. A gate runs them, and the manifest is complete

`docs/formal/models.json` declares every module and configuration, the **parts** of each protocol, what each run must
answer and what each module refines; `scripts/formal/models.py` judges them and `scripts/gates/formal-models.sh` is the
gate, with a workflow of its own. Refused as a verdict: a module or configuration not declared; a part no run takes out;
a violating run that takes out something else or finds another invariant; a holding run that does not check the promise,
or whose count of distinct states moved; a run that takes a part out and does not say whether the promise is broken
without it. "Could not run" (exit 3) is the answer without Java or TLC. TLC is a stable release pinned by its digest.

### D6. Assumptions are named

Each module opens with what it takes as given and the code must be shown to have: a transaction is atomic and durable; a
fence is judged where the data is; a counter is never given twice; a clock may lag and never runs back for one process;
authentication proves a name. An assumption is a named constant or conjunct, so a run that takes it out is a run of D5.
Cryptography is assumed. The README lists the trusted base of every level: TLC, the pinned toolchain, the generator of
D7 as far as D8 does not check it, the compilers, the operating system.

### D7. A document's model is generated from the document

- **One semantics.** `docs/formal/scxml/AppendixD.tla` is Appendix D once, over tables: configuration, the selection of
  transitions, entry and exit sets, microstep and macrostep, the two queues, the delayed sends and their cancellation,
  as ADR 0064 runs it once in the runtime. It is written in the order and the words of the recommendation, each operator
  citing its section, and is the model's only place of SCXML.
- **A document brings tables and behaviour.** `sce-codegen model` writes `<machine>.tla` from a `sce-static` document:
  its states, its transitions by the row of its transition table, its variables with their types, and each `cond`,
  `<assign>`, `<sce:call>` and algorithm body as operators. The numbers of a build (bounds, named limits) are constants
  of the module. The data of a document are unbounded in the model only where they are in the document, so a finite
  exploration needs **domains**: a model configuration file beside the document names the values each input may take and
  the most events, and is no part of the document, which a build never reads.
- **The environment is a module too.** An event from outside is any event of the document's alphabet the domains allow.
  A peer is the Mesh contract, `MeshChannel.tla`, with the attributes `deploy.yaml` gives the binding (order,
  duplication, receipt, loss of a link), so a deployment's machines are composed into one model and checked together.
- **A kind with no translation is refused by name**, as a lane refuses a construct it does not lower. A refused document
  is recorded and has no model, never a partial one.
- A document's properties are `<machine>.properties.tla`, named by the `sce:req` tokens they state, so a requirement,
  the property that judges it and the verdict are one thread, and a requirement with no property is listed as one.

### D8. A run of every backend is a behaviour of the generated model

The scenarios every lane replays are runs of the code whose transitions are told by row (ADR 0089) and whose snapshots
are recorded. Each run of a document that has a model is written as a trace and accepted by the model, lane by lane:
this is translation validation of the machine generator, by the runs and not by the generator's proof. The theorem held
is:

> for every document with a model, every lane and every scenario, the lane's run is a behaviour of the model.

- The same check binds `AppendixD.tla` to the recommendation: the W3C corpus run by the Interpreter is accepted by the
  semantics, and a semantics altered in one operator rejects a run of it (the part-taken-out rule of D5).
- A lane whose run is refused is a defect of the lane, the model generator or the semantics, and which is settled before
  any is edited.
- It judges executions; D3 and the exploration of D7 judge the rest.

### D9. Every model says what must happen, and what must stay possible

- **Progress.** Every protocol and every generated model states liveness under named fairness, or the manifest says why
  it has none. A model whose safety holds because nothing moves fails its own liveness.
- **A liveness that cannot fail shows nothing.** A behaviour cut off by a constraint ends in stuttering, where the
  fairness it would have met is false, so any progress holds of it (Lamport 2002, 14.3.5). A run that checks a temporal
  property has no constraint, symmetry or view, and every progress property has a run that takes its fairness away and
  must violate it. Fairness is asked only of steps the protocol itself may take, so it adds no safety of its own
  (machine closure, 8.9.2).
- **Branching time where a specification must leave something open.** "From any state a new election can still be
  started" is `AG EF p`, which linear time cannot say. TLC explores the whole graph and writes it (`-dump`), and
  `scripts/formal/models.py` decides `AG EF p` over it: every reachable state is in the backward closure of the states
  that satisfy `p`. It needs no other tool and it decides exactly that form and no more. It is a statement about the
  specification, of what its environment may still do; a thing the system must do is progress, and is stated as that. A
  module states each with a name, and the manifest declares it.
- Strategic properties (`ATL`) have no place: no document here states one.

### D10. An algorithm's contract is checked by Verus

An `algorithm` is a pure, bounded, total function: the shape a deductive checker is best at, and a construct of SCE's
own.
- **The contract is the document's.** `<sce:require>` is its precondition; `<sce:ensure>` and, on a loop,
  `<sce:invariant>` and `<sce:decreases>` are added to the kind, in the algorithm's own expression language, so a
  contract is one expression of the document and every lane's generator reads it (a lane that has no checker keeps it as
  a documented contract and refuses nothing).
- **The proof is of the code that runs.** The generator writes the Rust of an algorithm that carries a contract in a
  form Verus checks and a Rust compiler builds, so there is one rendering and no second to drift from it. Where the
  pinned Verus cannot check a construct the algorithm uses, the contract is refused by name for that algorithm. The lane
  that proves it is declared, and its proof is in the gate.
- **First:** one algorithm of the standard library whose contract is not trivial (`sce:std/rate/gcra_admit`
  is the candidate: its arithmetic must never overflow and its decision must be monotone).
  What it costs, and what it finds, decide how far this goes.

### D11. The specification cannot be edited to make a check pass

An agent or a person that is asked to make a check green must not be able to do it by changing what is checked.
- The promise modules, the abstract modules of D1 and the contracts of D10 are changed by a commit that changes nothing
  else and names the reason; a gate refuses a commit that edits one and the code it judges together.
- A proof or a check that contains `assume`, `admit`, an `OMITTED` step, a `PROOF OMITTED`, or a model constant that
  makes a guard true is refused, by name, in the same gate.
- The manifest pins the numbers of states, so a model narrowed until it holds is seen, and every part has a run that
  fails without it.
- A check judged by an agent is judged again by another that reads the specification and the verdict and not the first
  agent's account.

### D12. The composition

The modules meet in the checkpoint: a session's state, its outbox, its journal and a member's records are one write. A
module `CheckpointWrite` states that one write, and the two protocols are instantiated over it and checked together, so
that a property of each is not broken by the other. Time is an input of it where a lease or a retention is judged, as a
clock that may lag, so that a property of a deadline is said to be one of time.

### D13. The decisions of the host core are a synchronous core, proved; the shell is thin and traced

Read from the tree on 2026-11-22: the Mesh core's decisions (`router.rs`, `inbound.rs`, `outbound.rs`, `checkpoint.rs`,
`causal.rs`, `rpc.rs`, `https/rate.rs`, `https/signature.rs`) and the session store's rules use no `async`, tokio or
lock, and `endpoint.rs` and `pool.rs` use no `async` or tokio either; the asynchronous code is in the shell
(`https/server.rs`, `https/adapter.rs`, `wss.rs`); and `pool.rs` holds most of the locks and threads. The structure is
the one verified distributed systems use (IronFleet, SOSP 2015): each host a state machine whose steps are proved to
refine the protocol, and a small main-event loop that moves packets and is trusted, with the network's own behaviour
assumed. Its checker considers one single-threaded program, which the split below also asks for.

- **The core is proved by Verus** to refine the protocol module of D2, one module at a time, from the smallest that
  decides something. Where Verus cannot check a construct the core uses, the core is written so that it can, and the
  change is judged by what D4 and the suites see, not by the proof.
- **The lock's guarded state becomes a value with a transition function**, taken out of `pool.rs`, and that function is
  proved. What is left of `pool.rs` is who holds which lock and when, judged by a model and, for its atomics, by GenMC's
  method (RFC §synth-5-P).
- **The shell is kept thin and is not proved now.** Verus has had `async` functions and `.await` since 2026
  (`vstd/future.rs` is on its main branch, and issues on closures and `Send` futures are open), where its feature list
  of 2026-05-13 says it does not; the runtime, wakers and `Pin` polling are not covered. Verifying one costs: Lion, a
  clean-slate runtime with a Tokio-compatible interface, has about 40,000 lines of specification and proof for 3,400 of
  code. The shell moves bytes and readiness; what it may not do is decide. D4's traces judge it, and it has tests that
  drop a task at each `await`, which hold that a cancellation loses nothing a `200` acknowledged.
- The first module is chosen by its size and its weight: `https/rate.rs`, then `outbound.rs`. What the first costs, and
  what it finds, decide the rest.

### D14. What stays assumed, and of what kind

A level names what it takes as given. Each item is of one of four kinds, and only the first two are limits that no work
removes.

| Kind | What | What we do |
|---|---|---|
| Mathematical | no procedure decides every program (Rice); asynchronous consensus with one crash cannot promise to end (Fischer, Lynch and Paterson 1985); two parties over a channel that loses cannot both know the outcome (Two Generals); a client whose answer was lost cannot know whether it was done | a theorem is stated under the assumption that makes it true (a fairness, a partial synchrony), and "outcome unknown" is a result the document handles, never hidden |
| Physical | a signal takes time and clocks disagree; a medium can report a write it did not make; a bit can flip; a machine stops | the clock may lag and never orders; a store's durability is an assumption with a name and a test that cuts the power in a virtual machine; checksums detect, and do not cure |
| Trust | the compiler, the checker, the runtime's scheduler, the TLS library, the database, the operating system | named in the README; each is a program somebody can verify (a verified compiler, TLS library, runtime or store exists for some), so it is a cost and not a wall |
| Judgement | whether the promise says what the user meant; whether a model is of this system | no tool reads a wish. The promise is short and read by a person, and D4, D7 and D8 tie the model to the code |

Two more are often called limits and are costs: a **state space** that grows faster than any machine (met by a proof,
which does not enumerate) and an **inductive invariant** that no program finds in general (met by a person, or an agent,
and checked by a program). Interleavings at `await`, an executor and cancellation are in this group: they are hard, they
are verified elsewhere, and D13 arranges that they decide little.

## Sources

The passages these claims rest on were checked against the papers' own text on 2026-11-22; the papers were not read
through. Checked: Abadi and Lamport (refinement mappings; history and prophecy variables); Lamport 1977 (safety and
liveness; a schedule that always runs one process has to be ruled out to prove progress and does not touch safety);
Lamport 2002 (8.9.2, 8.9.3, 8.9.5, 14.3.5); Hoare 1969 (partial correctness); Meyer 1992 (assertions may be monitored as
the program runs); Emerson and Halpern 1986; Pnueli, Siegel and Singerman 1998 (each run of a translator is validated,
by refinement, and the translator is not proved); Liskov, Shrira and Wroclawski, the technical report of April 1990
(which corrected what ADR 0113 had said of it); Kreps 2013 (a log is totally ordered, its position is the timestamp, and
order holds within a partition and not across them); Jensen and Snodgrass, the technical report of June 1997 (the two
times); Birrell and Nelson 1984 (a call that ends in an exception ran "either once or not at all", and the caller is not
told which: the "outcome unknown" of ADR 0113); Gray and Cheriton 1989 (a lease's definition); Gray 1978 (the generals
paradox); Fischer, Lynch and Paterson 1985 (every protocol may not terminate with one faulty process); Saltzer, Reed and
Clark 1984 (its abstract); Bernstein et al. (Orleans) 2014 (an identity is a type and a key); RFC 1498 (names and
binding); RFC 9110 (a server must not perform a method whose `If-Match` is false, and may answer `412`; `410` is
heuristically cacheable and `412` is not); Snodgrass 1999 (valid and transaction time); Helland 2015; Lee et al. (RIFL)
2015 (completion records, low-water mark, leases); Hawblitzel et al. (IronFleet) 2015 (its assumptions, section 2.5);
Konnov et al. 2019; Lattuada et al. (Verus) 2024 (its trusted base); Sun et al. (Anvil) 2024; Cirstea et al. 2024; Li et
al. 2026 (for the figure about Lion); the Verus guide's feature list and the `vstd/future.rs` of its main branch.

**Read in another edition than the one cited:** the TKDE 1999 text of Jensen and Snodgrass (ADR 0114) and the conference
text of Liskov, Shrira and Wroclawski were not reached, and the reports above were read for them.
**Not reached:** Tomlinson 1975, for which ADR 0113 now cites RFC 9293, and the paper on Lion itself.

## Not in scope

Models of cryptography, of a transport's bytes (their conformance kits judge those), of the C++ Interpreter beyond D8's
use of it, a proof of the asynchronous shell (D13 says what judges it), strategic logics, a search of protocol variants
(the parts of D5 are the variants a person means), and a playground for learning.

## Consequences

- A protocol change edits its promise, its model, its refinement and its code, and the gate says which moved. A document
  change regenerates its model with its code.
- Proof has a price in upkeep, so D3, D10 and D13 name what pays it and no more.
- New tools (Apalache, TLAPS, Verus) are dependencies, pinned by digest as TLC is, and added only when their level is
  built. A lane without one says so and judges nothing in silence.
- The generator gains a second output to keep right, and D8 is what keeps it so.

## Alternatives rejected

- **A hand model of every document.** It drifts from the document, which is the fault this record is about.
- **A semantics per document.** Appendix D would be written as often as there are documents.
- **Proving the generator.** It moves with every construct; translation validation judges each output.
- **Trace validation alone**, or **bounded exploration alone**: each judges what the other does not.
- **More invariants and larger bounds** in place of a promise, which no size and no count of invariants makes.
- **nuXmv for branching time.** A second checker and a second semantics for one form of property.
- **Proving the asynchronous shell of the host core first** (the socket adapters, the HTTPS server). It is where
  interleaving at every `await`, cancellation and an executor meet, and it decides nothing a smaller shell would not
  leave to the core (D13).
- **Translating documents to Lean or Veil.** A second target with no second use; a theorem of D3 is TLA+'s.

## Order of work

Each step lands with a test that fails without it.

1. (2026-11-22) D5: the gate, its manifest and its self-test. D1 and D2 for `OpenInstances` and `EventJournal`.
2. (2026-11-22) D9's progress for every model of today, and `AG EF` with its first use (a journal's follower can always
   come to have taken in all there is); D6's README and the assumptions. The first use found a condition ADR 0114 had
   not stated: a store ends the transactions of a writer that is gone.
3. D11's refusals in the gate.
4. D7: `AppendixD.tla`, accepted against the W3C corpus by the Interpreter's traces (D8's second bullet); then
   `sce-codegen model` for one machine, then for every document that has no refused construct, and D8 lane by lane.
5. D4, with the code of ADR 0113 steps 4 and 5 and of ADR 0114 step 4, so that each lands traced.
6. D10's first algorithm. D13's first module. D3 with Apalache. D12.

## Questions decided at acceptance

1. Apalache, TLAPS and Verus as dependencies: Verus with D10's first algorithm and D13's first module, Apalache with D3,
   TLAPS per theorem. Acceptance of this record adds none of them; each is the owner's to approve when its step comes.
2. Whether a trace is kept by a failing run only, or always: always, in the world tests, the server suite
   and the scenarios of a lane, since a run with no trace has judged nothing of the model.
3. Whether `<sce:ensure>` is a change every lane reads at once: yes, as a documented contract, and checked
   on the Rust lane alone.
