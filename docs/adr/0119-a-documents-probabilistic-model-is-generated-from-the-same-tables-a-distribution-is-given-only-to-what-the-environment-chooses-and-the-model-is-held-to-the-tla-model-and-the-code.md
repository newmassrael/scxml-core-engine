# ADR 0119 — A document's probabilistic model is generated from the same tables, a distribution is given only to what the environment chooses, and the model is held to the TLA+ model and to the code

- Status: Proposed (2026-11-23). The owner ordered it written on 2026-11-23, relayed by a session of a downstream
  consumer. It comes after ADR 0118, ordered the same day. It is written under the owner's standing
  rule: the most long-term correct structure, textbook in its foundations, cost ignored.
- Date: 2026-11-23
- Amends ADR 0115 once accepted (its owner adds the "Amended by: ADR 0119" line then):
  - "The levels" gains a level of quantity (D1);
  - D15's properties document gains forms of quantity, and a distribution over an environment choice as one more kind
    of assumption (D2, D3).
  The text of ADR 0115 is otherwise unchanged. Its rejection of nuXmv stands: that rejection is of a second checker
  for a form of property that TLA+ can partly state (D9). A probability cannot be stated in TLA+ at all. An earlier
  reading by this repository took that rejection to cover a probabilistic checker, and that reading was wrong.
- Scope: `sce-build` (a second output of ADR 0115 D7's model generator), `scripts/formal` and the gate of ADR 0115
  D5. Also the Rust world tests for D6. A generated machine's output does not change.
- Related: ADR 0115 (D5, D6, D7, D8, D11, D15), ADR 0118 (sizes as rewards), ADR 0111 (links), ADR 0089 (a run told by
  the rows of its table).

## Context

A design claim about a protocol on a network that loses and delays packets is often a claim of quantity:

- the probability that a request is answered within a deadline;
- the expected number of retransmissions or bytes before a state is reached;
- the probability that a session is lost under a given loss rate.

ADR 0115 decides behaviour: what must hold of every run, and what must stay possible. Neither TLC nor TLAPS states a
probability or an expectation. A downstream project means to prove at design level that one protocol fits a closed
system better than another (ADR 0118, Context), and its objective is such a quantity.

## The textbook structure

- **A system that is deterministic, put in an environment that chooses with probabilities and without them, is a
  Markov decision process.** A property is the least or greatest probability or expectation over the choices that have
  no distribution (Baier and Katoen 2008, chapter 10; PCTL, Hansson and Jonsson 1994).
- **A machine of SCXML is deterministic.** Given the sequence of events it takes, Appendix D selects transitions in
  document order, and its run is one. All the nondeterminism of a system of machines is the environment's: which event
  arrives, when, and what the network does to it.
- **One semantics, two encodings, held equal.** When a second encoding of a semantics cannot be avoided (a checker
  reads its own language), the two are held equal by an exact comparison of what they generate (ADR 0115, "A generator
  is checked by checking each output").
- **A numeric verdict must be sound.** Value iteration that stops when two iterates are close can be arbitrarily wrong
  (Haddad and Monmege 2018). A gate's number is computed in exact arithmetic or by a method that bounds its error from
  both sides (interval iteration).
- **A sample is an independent check, not a proof.** Statistical model checking estimates a probability from runs,
  with a confidence stated in advance (Younes and Simmons 2002; Wald's sequential test).

## Decision

### D1. A level of quantity

ADR 0115's levels table gains:

| Level | Holds | Verdict |
|---|---|---|
| 4q. Quantity | probabilities and expectations of the document's generated model, under named distributions | Storm, exact or sound (D3) |

It sits beside level 4 and uses its model. It replaces none of the others: a quantity is stated only of a model whose
safety and progress the levels before it hold.

### D2. The machine stays deterministic; distributions are given only to the environment's choices

`sce-codegen model` (ADR 0115 D7) writes, beside `<machine>.tla`, a JANI model (Budde et al., TACAS 2017) of the same
document. It is generated from the same tables and operators: states, transitions by row, variables, guards, actions,
the two queues and the delayed sends. The choices it holds are the environment's only:

- the faults of each connector as ADR 0115 D7 holds them (loss, duplication, reordering, delay within its bounds,
  corruption);
- which event of the alphabet arrives from outside, and the value of its data, within the domains the exploration
  document names (ADR 0115 D15).

**A distribution is an assumption.** It restricts and weighs what the environment does, so it is a claim about the
environment and not a value to search over. It is written in the properties document (`sce:kind="properties"`, ADR
0115 D15) beside the other assumptions: a loss probability, a distribution of delay in ticks, weights on the events of
the alphabet. It changes by a commit of its own (ADR 0115 D11), and every quantitative verdict is reported with the
distributions it assumed, listed as assumptions not discharged. The exploration document names the finite domains and
counts and the granularity of a tick, and claims nothing. A choice given no distribution stays nondeterministic, and a
property states whether it means the least or the greatest value over those choices. A distribution is never given to a
choice of the machine, since the machine makes none.

Time is discrete in the first form: a delay is a whole number of ticks of a granularity the configuration names, so
the model is a Markov decision process. A deadline is a number of ticks. A model in continuous time (a continuous-time
Markov chain, a Markov automaton, a probabilistic timed automaton) is a decision of its own, taken when a property needs
it.

### D3. Properties are named, and the checker is Storm

A document's quantities are guarantees of its properties document (ADR 0115 D15), in two forms added to its five:
`<sce:probability>` (the least or greatest probability of reaching a condition of the observations, within a deadline
or not, bounded from below or above) and `<sce:expected>` (the least or greatest expected reward until a condition).
Each is named by the `sce:req` token it states, holds under the document's assumptions, and is translated into the
property language JANI carries. A reward may be ADR 0118's size of the event sent, so that expected bytes are stated in
the sizes the build computes.

The checker is Storm, pinned by digest as TLC is. A verdict that the gate holds is computed in exact rational
arithmetic, or by interval iteration with its bound stated. A plain value iteration is not a verdict.

### D4. The second encoding is held to the first

The JANI model is a second encoding of Appendix D, written by the same generator as the TLA+ model. Three checks keep
it from becoming a second semantics. They are part of ADR 0115 D5's gate:

1. **The graphs are one.** With every distribution erased, the reachable state graph of the JANI model, as Storm builds
   it and exports it, equals the state graph TLC writes of the TLA+ model for the same configuration (`-dump`, as ADR
   0115 D9 reads it). They are compared state by state over the same variable names, not up to isomorphism. A model
   whose graph differs is refused, with the first state or edge that differs.
2. **The runs are accepted by both.** Every trace that ADR 0115 D8 accepts against the TLA+ model is replayed over
   the explicit JANI state space, step by step, by `scripts/formal/models.py`.
3. **A part taken out changes a verdict.** Each distribution and each property has a configuration that must change
   its verdict: a loss probability moved, a deadline shortened, a retransmission taken out. The rules of ADR 0115 D5
   and D11 apply as written: a configuration not declared, a property no configuration can fail, and a
   properties document edited in the same commit as the code it judges are refused.

### D5. A refused construct has no model

A construct that the JANI generator does not translate is refused by name, and the document has no probabilistic model.
It never has a partial one. ADR 0115 D7's rule applies to this output too. A document refused by the TLA+ generator has
no JANI model either, since D4 needs both.

### D6. The code is sampled, as an independent check

On the Rust lane, `backends/rust/world` runs the generated machines under virtual time over a network it owns (ADR
0115 D4). It samples the network's choices from the distributions the properties document assumes, with a seed that is
recorded. For each property the gate declares to be sampled, a sequential test decides, at an error probability stated
in the manifest, whether the code's frequency is consistent with the model's exact value. A rejection is a defect of
the code, the generator or the configuration, and which one is settled before any is edited, as in ADR 0115 D8.

This is evidence and not a proof. It is the check that does not share the generator: it sees the code, which the two
models of D2 do not.

## Consequences

- A design claim of the form "the probability that a request is answered within 50 ms at 1% loss is at least p" can be
  stated, checked exactly on the model, and sampled on the code. The verdict is of the model under the assumed
  distributions, and says so: whether a deployment's network meets them is measured outside this record.
- The generator writes a third output (code, TLA+, JANI). D4 is what keeps it right, and its cost is paid with ADR
  0115 D7's.
- Storm is a new dependency, added when D2 is built and the owner approves it, as ADR 0115 decided for its tools.

## Alternatives rejected

- **A hand-written PRISM model of each protocol.** It drifts from the document, which is the fault ADR 0115 is about.
- **The PRISM language as the output.** JANI is read by Storm, Modest and the tools of the JANI interface. PRISM's
  language is read by fewer, and Storm reads JANI as well as PRISM.
- **Distributions on the machine's choices.** The machine has none. Giving one would change the semantics of SCXML.
- **Plain value iteration in the gate.** Its result can be wrong by more than the gate's tolerance. A verdict is exact
  or bounded.
- **Statistical model checking alone.** It never shows a probability. It estimates one, and only on the runs it
  samples.
- **Continuous time from the start.** No property asked for yet needs it, and discrete time keeps D4's comparison with
  TLC's graph exact.

## Order of work

After ADR 0115 D7 and D8 are built, and after its amendment that makes the environment's choices parameters. Each step
lands with a test that fails without it.

1. D2 for one machine of ADR 0115 D7's first, with D4.1 and D4.2. Written on 2026-11-27, before this record is
   accepted; Storm is a dependency the owner has not yet approved (Consequences). ADR 0115's amendment of 2026-11-23
   (the faults of a channel and the events from outside as parameters of D7) is the one this step waited for.
   As written:
   - **The machine.** `W3CTest403c`, whose delayed send coming due is the environment's choice, interleaved with
     the interpreter's steps. `docs/formal/W3CTest403c.jani`, declared in `docs/formal/quantities.json`.
   - **D2.** `sce-codegen model --jani` (`sce-build/src/formal/document_jani.rs`) reads the tables and operators the
     TLA+ module is printed from (`ModelTables` of `document_model.rs`, one translation of each expression).
     JANI has no sets, sequences or recursion, so Appendix D cannot be given tables in it as `AppendixD.tla` is: the
     generator evaluates the control of a run (configuration, history, queues, current event, phase, what a step
     told, the events that came from outside, the outcome) by AppendixD.tla's operators written again over the tables,
     and leaves the datamodel to the model, each step an edge whose guard is the values of the conditions that select
     it and whose assignments are what its blocks write. It is partial evaluation of the semantics with the data held
     symbolic, and D4 checks its output. No distribution is given yet: every edge has probability one, and the
     environment's edges (`due`, `arrive`) are the ones a distribution will weigh.
   - **D4.1 and D4.2.** `sce-codegen formal quantities` (`quantities.rs`; ADR 0146 moved the judges out of
     `scripts/formal`): the model must be what the generator writes of its document now; Storm's explicit space of it,
     exported with every state's valuation and every choice's action, read as states of the TLA+ module, must be the
     graph TLC writes under the configuration's constants, with the steps and the initial states; every recorded run of
     `traces.json` of the module is replayed over that space, and each altered run must be refused at the altered
     step. The gate is `formal-models`'s first part.
   - **Storm.** It publishes no static build. Its releases carry source and, from 1.13.0, Debian packages for Debian 12
     and 13 on x64 and arm64; Docker images (`movesrwth/storm`) and Homebrew are its other channels. The pin is the
     2.0.0 package for Debian 12 on x64, by its digest: it holds the program and the libraries built with it and asks
     the system for GMP, CLN, GiNaC, GLPK, Z3 and libarchive, which the host image (ADR 0145) and `bx.toml` name; the
     Debian 13 package asks the C library for symbols the Ubuntu 24.04 of the image has not. The package is unpacked,
     never installed. A build from source, pinned by its commit in a stage of the image, is the alternative if a
     package's libraries stop matching the image's.
2. D3 with one property per kind (a probability, a deadline, an expected reward), and D4.3's configurations that must
   fail.
3. D6 for that machine.
4. Every document that has a TLA+ model.

## Sources

Baier and Katoen 2008, chapter 10 (Markov decision processes; PCTL); Hansson and Jonsson 1994 (PCTL); Budde,
Dehnert, Hahn, Hartmanns, Junges and Turrini 2017 (JANI); Haddad and Monmege 2018 (interval iteration; value
iteration's stopping criterion); Younes and Simmons 2002 (statistical probabilistic model checking); Wald 1945 (the
sequential probability ratio test). Not yet checked against the texts. The passages D2 to D4 rest on are to be checked
before this record is accepted.
