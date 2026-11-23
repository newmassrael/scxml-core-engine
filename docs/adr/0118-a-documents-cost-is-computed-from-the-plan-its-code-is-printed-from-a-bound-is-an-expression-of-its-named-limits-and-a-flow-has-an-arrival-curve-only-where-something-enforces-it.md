# ADR 0118 — A document's cost is computed from the plan its code is printed from, a bound is an expression of its named limits, and a flow has an arrival curve only where something enforces it

- Status: Proposed (2026-11-23). The owner ordered it written on 2026-11-23, relayed by a session of a downstream
  consumer, together with four amendments to ADR 0115 that the session owning that record writes. It
  is written under the owner's standing rule: the most long-term correct structure, textbook in its foundations, cost
  ignored.
- Date: 2026-11-23
- Amends, once landed (each amended document gains an "Amended by: ADR 0118" line in the landing that moves it):
  - ADR 0062 section 4: `largest_encoded_bytes` becomes the value of D2's expression at the build's limits;
  - `SCE_FORGE.md` section 4.6: what a codec's size is, and where it is read;
  - `docs/SCE_ACCEPTED_SUBSET.md`, "A bound a deployment chooses": a bound that names a limit is read as an
    expression of it;
  - `SCE_WIRE_CONTRACTS.md`: the row of D5's surface.
- Scope: `sce-build`. It reads the documents and the deployment, and runs nothing it generates. A generated machine's
  output does not change.
- Related:
  - ADR 0061 (a codec's length is a synthesized attribute of its one encode);
  - ADR 0062 section 4 (the largest encoded size of an element);
  - ADR 0065 and ADR 0077 (where an encode refuses; a bound is part of the type);
  - ADR 0081 (the encode plan, decided once and printed by every backend);
  - ADR 0101 (one deterministic CBOR form per type);
  - ADR 0109 D1 and D2 (what a client may cost; GCRA at the edge);
  - ADR 0110 (the C11 image weighed by a gate);
  - ADR 0115 D6 to D8 and D15 (assumptions are named; a document's model; a run is a behaviour of it; the
    properties and exploration documents).

## Context

A downstream project means to prove, at the level of design, that one protocol fits a closed system (in-vehicle
networks first) better than another. Its claim names a design space, a workload, constraints, an objective and a bound.
The protocols are SCE documents: the candidate's own, and the opponent's as a consumer models it. ADR 0115 gives the
proof its behaviour: a model generated from the document, and runs of the code accepted by it. What the proof also
needs is cost, and SCE cannot say it today:

1. **The bytes of a value.** `codec_size::largest_encoded_bytes` gives an upper bound for each codec, sound because a
   codec it cannot bound is refused with the reason (`Unbounded`). The exact size of one value is known only to the
   generated encode of each language (`measure`, ADR 0061), which runs in a program and not in the build.
2. **A bound as a function of its parameters.** The bound is one number, at the limits of one build. A build moves a
   named bound with `--limit name=N`, but nothing says how the size grows with `N`. A design claim is about N peers
   and M keys, not about one build.
3. **What a machine holds.** Nothing states the most a machine can hold. ADR 0110 weighs the image of a C11 machine,
   which is code and static storage in one language, not the bound of its state.
4. **A flow's arrival curve.** A network-calculus tool needs, for each flow, a function that bounds what the flow may
   send in any window. Nothing exports one. A period read from the document's delays is not one, since another
   transition may send on the same flow.

How many messages and round trips a scenario takes is not here. The statechart's graph cannot say it, because guards
decide which transitions run. One run's count is read from its trace (ADR 0089), and a bound over every run is a
property of the model of ADR 0115 D7. The total for a workload is not here either: a workload (a schedule of messages)
is the analysis's input, not the design's. It is computed by the consumer's tool from what D1 to D4 export, for the
same reason ADR 0115 D7 keeps a model's domains outside the document.

## The textbook structure

- **The size of a production is a synthesized attribute** of the grammar that writes it (Knuth 1968), and a fold over
  the one grammar is another interpretation of it, not a second grammar (ADR 0061: tagless final). The encode plan is
  that grammar (ADR 0081).
- **A generator is checked by checking its outputs** (Pnueli, Siegel and Singerman 1998). A second reading of the plan
  is held equal to what every backend writes, case by case, as ADR 0115 D8 holds the model to the runs.
- **A bound over parameters is a term**, not a table of numbers. Sizes compose by sum (a sequence), product (a list of
  `n` entries) and maximum (a choice of arms). Terms over the natural numbers built from these operators are closed
  under the same operators. Each such term is monotone in every variable, so its value at larger limits is never
  smaller.
- **An arrival curve is a constraint the flow is guaranteed to meet** (Cruz 1991; Le Boudec and Thiran 2001,
  chapter 1). A flow meets one because a regulator enforces it or a proof shows it, never because a document intends
  it. GCRA (ITU-T I.371), which SCE already generates (`sce:std/rate/gcra_admit`), is a token bucket. A flow through a
  GCRA of interval `T` and burst `B`, whose packets are at most `L` bytes, has the arrival curve
  `alpha(t) = L * (B + t / T)`.
- **What is assumed is named** (ADR 0115 D6). A number the document declares but nothing enforces is exported as a
  named assumption, never as a bound.

## Decision

### D1. The bytes of one value are a fold over the encode plan, held to every backend's bytes

`sce-build` evaluates the size attribute of the encode plan (`forge::encode_plan`) on a value. The value is given in
the language-neutral form that the conformance cases already use (`tests/forge/conformance`). The result is the exact
number of bytes the encode writes, or the refusal the encode makes (ADR 0065), named as the encode names it.

- **It is the plan's**, and decides nothing the plan does not decide. A rule of the wire that the plan does not hold
  (one a printer decides alone) is a defect of the plan, found by the next bullet, and is moved into the plan.
- **It is held to the bytes.** For every case that every backend's codec conformance runs, and every CBOR pin
  (`tests/forge/cbor_pin`), the fold's size equals the length of the case's bytes, or its refusal is the case's. A
  fold that differs on one case fails the test. A mutation that changes one rule of the fold (a length prefix's width,
  a VLE's bytes, an absent field's cost) must be caught.

### D2. A codec's largest size is a term of its named limits

`codec_size` computes a term rather than a number. Its constants are natural numbers and its variables are the
`sce:limit` names the codec's bounds carry. Its operators are `+`, `*` and `max`. A bound written as a plain number
is a constant. A bound that names a limit is a variable whose default is the number the document wrote.

- The number the build uses today is the term's value at the build's limits. `largest_encoded_bytes` becomes that
  evaluation and keeps its `Unbounded` refusal and reasons unchanged. A test holds them equal for every codec of the
  tree, at the document's limits and at a second setting that moves each limit.
- **Sound, and tight where a witness exists.** For every codec of the tree whose largest value can be built from its
  bounds, the test builds that value, encodes it with D1, and requires the term's value. A codec whose largest value
  cannot be built (a refinement that excludes the largest of each field) has the term marked `upper`, not `exact`, and
  the refinement that makes it so is named.

### D3. A machine's state bound is a term of its named limits, or the reason it has none

For a `sce-static` machine, `sce-build` computes the most the machine can hold in one measure that is the same on
every backend: the size of ADR 0101's deterministic CBOR form of each value. It is computed per part, and the total is
their sum:

- each top-level variable, at the largest value of its type (ADR 0077, ADR 0086, ADR 0108; a bounded map by its
  capacity);
- each queue whose depth is bounded, at its bound times the largest event it can hold;
- the pending delayed sends, where their number is bounded.

A part with no bound (an internal queue that a loop of raises can grow, delayed sends a loop can schedule) makes the
total `Unbounded` with the part named, as `largest_encoded_bytes` names its reason. It is never left out of the sum.

The bytes a language spends on that state (its layout, alignment, allocator) are measured, not derived: ADR 0110's gate
weighs them for C11. D3 is the bound those measurements are compared against.

### D4. A flow's facts: its largest packet, an arrival curve only where something enforces it, and its declared periods as assumptions

For a deployment, every flow (a sending machine, the event, the binding it leaves on) is exported with:

- **`L`**, the largest bytes of the event's codec, D2's term;
- **an arrival curve**, only where one is guaranteed:
  1. a GCRA that SCE generates on that flow, of interval `T` and burst `B`, gives `L * (B + t / T)`. Today these are
     the edge's limiters of ADR 0109 D2;
  2. a guarantee of the deployment's properties document (ADR 0115 D15) that the flow sends at most `k` events in
     any window `W`, proved by its model (ADR 0115 D7), gives the staircase `L * k * ceil(t / W)`. The export cites
     the guarantee by name and lists the assumptions of the environment it was proved under, since the curve holds only
     where they do;
  3. otherwise the flow is exported with no arrival curve, and the reason "no regulator and no proved bound".
- **the periods the document declares** for sends on that flow (a constant `delay`, a timer), each marked as an
  assumption with its name and where it is written. A consumer may take it as a hypothesis of its proof and must say
  so. SCE does not turn it into a bound.

How a flow's packets are framed below the event's codec (a transport's bytes) is not here, as it is not in ADR 0115.
Where a link's framing is itself an SCE codec (ADR 0111), its overhead is the codec's D2 term and is added to `L`.

### D5. One surface

`sce-codegen cost` writes D1 to D4 as JSON in a schema of its own, `schemas/sce-cost.v1.schema.json`. The schema is
registered in `SCE_WIRE_CONTRACTS.md` as `pre-release` and follows the registry's procedure from then on:

- `cost value <codec document> <value>`: D1;
- `cost bounds <document>`: D2 for each codec the document reaches, D3 for a machine;
- `cost flows <deploy.yaml>`: D4.

A term is written as a tree of `sum`, `product`, `max`, `const` and `limit` nodes, so a consumer evaluates it at
its own parameters without parsing an expression language. Each term carries its value at the build's limits.

## Consequences

- A design claim can say how a message and a machine grow with N peers and M keys, and cite the build that computed
  it.
- The plan gains a second reader, kept honest by D1's cases. A wire rule a printer decides alone becomes visible as a
  failing case. That is a gain: ADR 0081 already asks that no printer decide one.
- D4 says less than a reader of the document may expect: a flow whose only bound is an intention has no arrival curve.
  This is deliberate. A curve nothing enforces is the error a design proof must not contain.
- A send-side regulator for a flow (a GCRA a deployment binds to an outgoing flow) would give D4's first case to every
  flow. It is a decision of its own, not made here.

## Alternatives rejected

- **Running the generated encode to measure a value.** It needs a compiled program at analysis time and a choice of
  language. The plan is what every language prints, and D1 holds the fold to all of them.
- **A size per language.** It describes the layout of a language, not the design. D3 gives the neutral bound, and
  ADR 0110's gate measures one language against it.
- **A symbolic algebra library, or a general expression language, for the terms.** Sizes need `+`, `*` and `max` over
  the natural numbers and nothing else. A tree of five node kinds is evaluated by any consumer.
- **An arrival curve from the document's periods.** It holds only if no other transition sends on the flow, which is a
  property of every run, and that is ADR 0115's to prove (D4, item 2).
- **Message counts from the graph.** Guards make it undecidable from the graph alone. It belongs to ADR 0115's model and
  traces.
- **The workload total and the network-calculus computation inside SCE.** They are an analysis of a design, made with
  inputs the design does not hold. SCE exports the facts and the consumer's tool computes.

## Order of work

Each step lands with a test that fails without it.

1. D2: the term in `codec_size`, with `largest_encoded_bytes` as its evaluation, held equal at two limit settings.
2. D1: the size fold over the plan, held to every codec conformance case and CBOR pin; then D2's tightness test, which
   uses it.
3. D5's surface for D1 and D2, with its schema registered `pre-release`.
4. D3.
5. D4: its first case (the edge's GCRA) and its third, then its second once ADR 0115 D7 generates a model with
   properties.

## Sources

Knuth 1968 (synthesized and inherited attributes); Pnueli, Siegel and Singerman 1998 (translation validation); Cruz
1991 (a calculus for network delay); Le Boudec and Thiran 2001, chapter 1 (arrival curves; the leaky bucket); ITU-T
I.371 (GCRA). These are cited from what other records of this repository have checked (ADR 0061, ADR 0115, ADR 0109)
or from the texts as known. The passages D4 rests on are to be checked against the texts before this record is
accepted.
