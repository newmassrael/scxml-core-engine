# ADR 0062 — A bounded log is read by cursors its readers own, and a full log forgets its oldest entry

- Status: Accepted (2026-10-28, by the owner, with the order to proceed), after the owner answered the three questions it was proposed with the same day (end of this document); being implemented in the order below. Requested by a
  downstream consumer that wants a fixed-capacity event ring with one producer and several readers, which overwrites
  its oldest record when full; designed under the owner's rule: the most long-term correct structure, textbook in its
  foundations, cost ignored
- Date: 2026-10-28
- Scope: a new forge kind `broadcast-log` and its schema; a sequential specification and a history checker for it
  (`sce-build`); a runtime row per backend (`backends/*/forge-runtime`) and a template per backend
  (`tools/codegen/templates/forge/`); new `DiagnosticCode` variants and their acceptance-doc rows;
  `schemas/sce-forge-ext.xsd`, `schemas/sce-kind-catalog.v1.schema.json`; a TLA+ model under `docs/formal/`; the
  extraction of what `queue` and `broadcast-log` share. **Nothing in `queue` changes behaviour.**
- Related: SCE Protocol-Synthesis RFC §synth-5-P (`queue`), SCE_FORGE.md "Application: 2026-09-30 `queue`" (the
  admission test), ADR 0002 (no liveness diagnostics), ADR 0049 and 0058 (the codec's window), ADR 0053 (features only
  add), `sce-build/src/queue_history.rs` (the checker this one extends in kind), `scripts/gates/genmc-queue.sh`

## Context

The request, as made: give the `queue` kind a policy for a full queue, `sce:on-full="reject" | "overwrite-oldest"`,
default the present behaviour, and a way to read how many entries were overwritten.

The `queue` kind has no such policy and, by its own contract, no place for one. A push into a full queue returns
`Full` (`Outcome::Full`, `sce-build/src/queue_history.rs`); a pop *consumes*; `consumers=many` is competing
consumers, in which each element goes to exactly one of them. The consumer's ring is something else: every reader
reads every record at its own pace, and a record that falls out of the window is lost to the reader that was too slow
to take it. That is not the queue with a flag set. It is a different abstract data type, and the textbook has a name
for it: a **log**, an append-only sequence whose entries are addressed by their position and read by position, in
which reading removes nothing (Kreps, "The Log: What every software engineer should know about real-time data's
unifying abstraction", LinkedIn Engineering, 2013; Kleppmann, *Designing Data-Intensive Applications*, O'Reilly 2017,
ch. 11; as a bounded in-memory structure, Thompson, Farley, Barker, Gee and Stewart, "Disruptor: High performance
alternative to bounded queues for exchanging data between concurrent threads", LMAX technical paper, 2011;
tokio's `sync::broadcast` is the same contract with a `Lagged(n)` error).

So the design question is not "which overflow policy" but "which abstract data type does a kind name". SCE's rule is
that a kind's name fixes its sequential specification and its progress claims are judged against it. The admission
note of 2026-09-30 rejected extending `bounded-collection` to serve a FIFO on exactly this ground: serving two
specifications under one name needs the bolt-on validator that marks a category error. The same ground applies here.

### Prior art

| Source | What it gives this design |
|---|---|
| tokio `sync::broadcast` | The same contract in a widely used library: a send into a full channel overwrites the oldest value, a receiver that missed one gets `RecvError::Lagged(n)` with the count skipped, and its position moves to the oldest value still held (checked 2026-10-28 in the module documentation: with capacity 2, sending 10, 20, 30 makes the first `recv` return `Lagged(1)`, then 20 and 30). That is `Lagged{skipped}` and the resume rule of §2. Whether its implementation is lock-free was not checked |
| LMAX Disruptor (2011) | The ring of pre-allocated entries addressed by a sequence number, and the account of why it beats a bounded queue. Its multi-producer sequencer and gating sequences are the designs §8 declines for now |
| Seqlock (Lameter 2005; Boehm 2012; WG21 P1478) | The read protocol of §4 and its correctness under a language memory model |
| Logs as an abstraction (Kreps 2013; Kleppmann 2017) | Position-addressed, read-does-not-remove, per-reader offsets |
| Operating-system trace buffers with an overwrite mode (the Linux tracing ring buffer; the `perf` mmap buffer) | Established practice outside the consumer's domain. Named from memory and not checked in this revision |

## Decision

### 1. The design space, and which cell each kind occupies

A bounded channel between execution contexts differs on two independent axes: **delivery** (an entry is consumed by
one reader, or retained for all of them) and **what happens at the bound** (the writer is refused, the writer waits,
the oldest is forgotten).

| | refuse the writer | the writer waits | forget the oldest |
|---|---|---|---|
| consumed once | `queue` (exists) | `queue` plus a wake-up the host owns | not offered (below) |
| retained for every reader | not offered (§8) | not offered (§8) | **`broadcast-log` (this ADR)** |

- *Waiting* is not a property of a data structure: it is a scheduler's, and the existing kinds specify none.
- *Consumed once + forget the oldest* is refused. The producer would advance the consumer's end, so the head becomes a
  location two contexts write. That destroys the one property that makes the Lamport row wait-free on loads and stores
  alone, and it invalidates the invariants SCQ's entries rely on. Everything this cell is used for (a telemetry stream
  that keeps the latest N) is a `broadcast-log` with one reader, which also reports what it lost, and the queue cannot.
- *Retained for every reader + refuse or wait* is a log whose writer is gated by the slowest reader (the Disruptor's
  gating sequences). It needs a registry of readers and makes the producer's progress depend on the readers'. §8 gives
  the gating condition under which it is revisited.

The consequence is a fixed boundary: `queue` is the cell `consumed once x refuse`, completely, and gains no attribute;
a document that writes `sce:on-full` on a queue writes an attribute no reader reads, which is the case
`validation/sce-attribute-unread` exists for (the step that adds the kind confirms that check covers kind documents,
and adds a hint naming `broadcast-log` if it can carry one).

### 2. The abstract data type

A **bounded log** of retention `R >= 1` over an element type `E` has the state
`(h, entries)`: `h` is the number of entries ever published, `entries : [0, h) -> E` the entries by sequence number.
Only the last `R` are retained: the **window** is `W(h) = [max(0, h - R), h)`.

Operations. A **reader** is a handle that owns a cursor `r` (a sequence number, `r <= h` always) and a running total
`lost` of the entries it has been told it missed. The handle is part of the object, and its state is not shared.

```
publish(e)     : h' = h + 1 ; entries[h] = e ; returns h            -- never refuses, never waits
read(reader r) :
    r  in W(h)  ->  Item(r, entries[r]),   cursor' = r + 1
    r  = h      ->  Empty,                 cursor' = r
    r  < h - R  ->  Lagged{skipped = (h - R) - r},  cursor' = h - R, lost' = lost + skipped
                                                      -- the reader resumes at the oldest retained
head()         : h
reader_oldest() : cursor = max(0, h - R)       reader_head() : cursor = h       reader_at(n) : cursor = min(n, h)
```

A batch read into a caller's buffer is `n` reads in sequence and promises no more: the entries it returns are not a
snapshot of one instant, and a `Lagged` ends the batch at the point it was found.

- **Loss is a value, not a counter.** What the consumer asked for as "a way to read the overwrite count" is
  `Lagged.skipped` per read, the reader's own `lost` total, and `head()`. The log keeps no count of overwrites, because
  an overwrite that no reader suffered is not a fact about any reader, and the number any one reader lost is only that
  reader's. A global count would also be a location every reader wrote, which would put the readers back into the
  producer's progress.
- **In the theory of channels this is a lossy channel** (Abdulla and Jonsson, "Verifying Programs with Unreliable
  Channels", Information and Computation 127(2), 1996; Cécé, Finkel and Iyer, "Unreliable Channels Are Easier to Verify
  Than Perfect Channels", Information and Computation 124(1), 1996) whose loss is not arbitrary but fixed by the bound,
  and which tells the receiver what was lost. That second property is what makes it implementable without
  coordination, and what the lossy FIFO of §1 lacks.
- **Readers are unbounded and need no registration.** A cursor is the reader's own value, so the producer never learns
  how many readers exist. This is what lets publish be wait-free, and it is the Disruptor's difference from a queue per
  reader.
- **Retention is `R`, the slot count is `R + 1`.** The slot being overwritten is in flux and cannot be promised to a
  reader (§4); promising `R` entries and spending `R + 1` slots is the price. The Lamport ring pays its one slot as lap
  indices over `2N` (`backends/rust/forge-runtime/src/queue/spsc.rs`); that trick is not available here, because a
  reader may be arbitrarily far behind, so the sequence is a counter that never wraps in the abstract and its width is a
  concrete obligation (§4).

### 3. The document

```xml
<scxml xmlns="http://www.w3.org/2005/07/scxml" xmlns:sce="http://sce.dev/ext"
       sce:kind="broadcast-log" name="event_ring" version="1.0">
  <sce:element-type>event_record</sce:element-type>
  <sce:producers>one</sce:producers>
  <sce:progress>wait-free</sce:progress>
  <sce:retention entries="1023"/>          <!-- or source="deploy" key="..." -->
</scxml>
```

Every element but `<sce:retention>`'s `source` form is required and none has a default, as for `queue`: a reader of the
document learns the contract from the document. `<sce:retention>` is a count written the two ways a capacity is
(`CapacitySource`). `<sce:producers>many</sce:producers>` parses and is refused (§8). Refusals, each a
`DiagnosticCode` with its acceptance-doc row:

| Code | Fires when |
|---|---|
| `broadcast-log/element-type-not-a-kind` | the element names no codec document in the build (as `queue/element-type-not-a-kind`) |
| `broadcast-log/element-size-unbounded` | the element's encoded size is not a build-time constant (§4 stores bytes) |
| `broadcast-log/producers-many-unsupported` | `producers` is `many` (§8) |
| `broadcast-log/progress-unreachable` | the declared progress exceeds what the selection table gives the row |
| `broadcast-log/progress-unreachable-on-backend` | the backend's ceiling is lower (Python is `blocking`) |
| `broadcast-log/wrap-bound-unstated`, `.../wrap-bound-below-deploy-minimum` | the stamp is narrower than 64 bits and the deploy states no sufficient wrap bound (§4) |
| `broadcast-log/slots-not-a-divisor-of-the-stamp-space` | the stamp is narrower than 64 bits and `retention + 1` is not a power of two (§4) |
| `broadcast-log/runtime-missing` | the language has no template over its runtime row yet (every language but Rust, since step 6b) |

### 4. The algorithm: one producer, a stamped ring, and a seqlock read

A **slot** holds a `stamp` and the element's encoded bytes as `W` words. The log stores *bytes of the element's wire
form* (ADR 0049/0058), never a value with ownership: an entry has no destructor to run when overwritten, and a copy
that was torn is harmless until it is validated and thrown away. The producer keeps `n`, the number published, in its
own handle (no read of shared state), and publishes `head = n` after the slot.

```
publish(e):            // n is the producer's own count; i = n mod S, S = R + 1
  stamp[i].store(busy(n), release)         // release: see "Orders the memory model adds" below
  fence(release)
  for k: words[i][k].store(e[k], relaxed)
  stamp[i].store(done(n), release)
  head.store(n + 1, release)               // the linearization point of publish
  n += 1

read(r):               // returns Item / Empty / Lagged, a bounded number of steps
  h = head.load(acquire)
  if r == h:            return Empty
  if h - r >= S:        return Lagged{skipped = (h - R) - r}        // r is outside the window
  s1 = stamp[r mod S].load(acquire)
  if s1 == done(r):
      copy words[r mod S] with relaxed loads
      fence(acquire)
      if stamp[r mod S].load(acquire) == s1:  return Item(r, copy)
  h' = head.load(acquire)                    // the slot was reused while we looked: the producer is past us
  return Lagged{skipped = (h' - R) - r}
```

This is the sequence lock (Lameter, "Effective synchronization on Linux/NUMA systems", Gelato Federation Meeting 2005)
in the form Boehm showed correct under the C++11 model (Boehm, "Can seqlocks get along with programming language memory
models?", MSPC 2012): the payload words are *atomic* loads and stores with relaxed order, so there is no data race,
and the two fences carry the ordering. The standard's own answer to the same problem is WG21 P1478 (Boehm, "Byte-wise
atomic memcpy", revised from R0, 2019, to R6, 2020; renamed `atomic_load_per_byte_memcpy`), whose stated first use is the
seqlock reader and whose semantics is a per-byte relaxed atomic load loop followed by a fence: the protocol above is that
facility written out by hand at word granularity, which the paper notes is acceptable on the hardware that matters. It is
a proposal whose adoption was not checked (see References); the design does not wait for it, and a backend whose
language gains the facility may use it without changing the document or the contract. The stamp carries the **sequence number**, not a bit that is flipped, so a
reader that compares against `done(r)` cannot mistake a later lap for the one it wants (the ABA hazard a version stamp
exists to defeat).

**Orders the memory model adds to the protocol.** The first version of this section wrote the busy stamp relaxed and
read the second stamp relaxed, as the textbook seqlock does, and the loom model of §6 refused it twice, each time with
a reader told it had lagged by nothing. The textbook seqlock needs a reader only to *detect* that a slot changed; this
one also needs it to *know how far the publisher has come* when it does, by reloading `head`. That reload is ordered
after the publish that began the lap only through a synchronizes-with edge, and each of the two places a reader can
learn of a later lap has to supply one: the busy stamp is a release store, so that a reader seeing it sees the `head`
store that ended the publish before, and the second stamp load is an acquire load, because an acquire fence orders a
load only against the reads before it. Neither is a fault of the sequentially consistent model; both are outside what
it can express, which is why §6 keeps the two layers.

Properties, each of which is checked (§6):

- **Wait-free** publish and read, in `O(W)` steps. The algorithm needs no read-modify-write atomic: loads and stores of
  the stamp width and of one word. The Rust row below adds one, an 8-bit swap that takes the publisher's place, and
  needs 64-bit atomics; the form for a target with only loads and stores, with a narrow stamp and the once-only handle
  by ownership, is not built (see "What step 4 landed"). Wait-freedom of read holds because a failed validation is not retried:
  a mismatch *is* the answer, `Lagged`. A single producer is what makes this true; with several, a claimed but
  unpublished sequence number is a hole that readers can neither return nor skip (§8).
- **Why `R + 1` slots.** A reader that has seen `h - r <= R` and found the slot reused did so because the producer began
  writing sequence `r + S`, which it does only after `head` reached `r + R + 1`. The reload `h'` therefore satisfies
  `h' - R > r`, so `skipped >= 1` and the answer agrees with the specification. With `R` slots there would be a state in
  which the entry the specification retains is the one being overwritten.
- **Stamp width.** The stamp is 64-bit where the target has 64-bit atomic loads and stores, and the sequence stays below
  `2^63` (292 years at a billion publishes per second). A target without them uses a 32-bit stamp, which wraps after
  `2^31` publishes and would let a reader that stopped reading for that long mistake a later lap for its own. That
  hazard is not decidable from the document, so it is handled as `queue` handles 32-bit SCQ entries: the deploy states
  the wrap bound it guarantees (`queues.<q>.min_wrap_ops` generalised to name a log) and a missing or insufficient
  figure is a refusal, not a silent weakening. The slot index `n mod S` must stay continuous across the stamp's wrap,
  so `S` must divide `2^w`; the narrow-stamp form refuses a retention for which `R + 1` is not a power of two.
- **Per backend.** Rust: `AtomicU64` words (`AtomicU32` under a narrow stamp) and `core::sync::atomic::fence`; `Copy`
  element bytes; the producer handle is unique by ownership (`split`), the reader handle is `Clone`. C11 and C++:
  `stdatomic.h` and `<atomic>`, the same fences. Go: `sync/atomic`, sequentially consistent, hence at least as strong.
  Kotlin: the atomic array and fences the queue rows use (confirmed against `commonMain` when the row is written; a
  target without separate fences takes sequentially consistent accesses, which are stronger). Python: one
  ring under one lock, so `blocking` (the ceiling of `queue_progress_ceiling`). Swift: refused until its runtime has
  atomics, exactly as for `queue`.
- **ISR placement.** Both operations are wait-free, so both sides satisfy `queue/progress-insufficient-for-isr`'s bar;
  the deploy check is carried over with the log's own code.
- **Layout is a runtime concern and not part of the contract.** A slot keeps its stamp and its words together, so a
  read touches one run of memory, and `head` sits alone on its own line, as the queue rows pad their indices
  (`Padded` in `backends/rust/forge-runtime/src/queue/`), because the producer writes it on every publish and every
  reader loads it. Neither choice is visible to a document or to the history checker.
- **Cardinality is enforced by construction where the language can, and by the same once-only handle as the other
  rows where it cannot.** Two publishers would break the stamp protocol silently, so a second `producer()` is an error.

### 5. The correctness criterion

The specification of §2 is held against concurrent histories by **linearizability** (Herlihy and Wing,
"Linearizability: A Correctness Condition for Concurrent Objects", TOPLAS 12(3), 1990), with the real-time order of
operations read, on the weakly ordered backends, as the happens-before the memory model provides (Batty, Dodds and
Gotsman, "Library abstraction for C/C++ concurrency", POPL 2013). The linearization point of `publish` is the store of
`head`; that of a `read` is always a load of `head`: the first one for `Empty`, for `Item` and for the range `Lagged`,
and the reload for the late `Lagged`. The point of an `Item` is the first load and not the stamp validation, and the
difference matters: by the time the validation passes the producer may have published until `head = r + R + 1`
without having begun to write the slot, at which instant the specification no longer retains `r`. The first load is
the last instant at which `r` was certainly in the window, the validation shows that the bytes copied are the ones that
were there, and the `R + 1` slots of §4 are what make the two agree.

The checker is `sce-build/src/broadcast_history.rs`, beside `queue_history.rs`, with its own JSON history format and
the command `sce-codegen check-broadcast-history`, so that every backend's job already runs one binary.

- **The general search** is Wing and Gong's (Wing and Gong, "Testing and Verifying Concurrent Objects", JPDC 17(1-2),
  1993) with Lowe's memo, as the queue's is, over the sequential log of §2 and with each reader's cursor as part of the
  object state. This is the *definition* of the checker.
- **The specialisation.** With a single producer the publishes are totally ordered by program order, so `h` is a step
  function of time whose steps lie inside the publish intervals, and each read constrains only the value of `h` at one
  instant of its own interval (`h = r` for `Empty`, `h > r + R` for the late `Lagged`, and so on). That is a system of
  precedence constraints between instants and is solved without a search of permutations. It is an optimisation, not a
  second definition: it is admitted only while a differential test holds it equal to the general search on random and
  on adversarially shrunk histories, and the general search remains for any history the specialisation declines.
- **The value check.** `Item(r, v)` must carry exactly the `r`-th published value. The conformance recorders publish
  values whose words are all a function of the sequence number, so a torn copy that passed validation shows as words
  that do not agree (the test that fails when the second stamp load is removed, §6).

### 6. The verification, by layer

Each layer answers a different question and none stands in for another (the structure `queue` already has).

1. **A TLA+ model with a refinement mapping** (`docs/formal/BroadcastLog.tla`, one `.cfg` per bound): the concrete
   ring, with each atomic access a step, refines the abstract log of §2 under the mapping `h = head`,
   `entries[n] = words[n mod S]` for `n` in the window (Abadi and Lamport, "The Existence of Refinement Mappings",
   TCS 82(2), 1991; Lamport, *Specifying Systems*, 2002). TLC checks it for small `R`, two readers and a handful of
   publishes. It is sequentially consistent by construction, so it proves the protocol and not the ordering; no
   liveness is claimed (ADR 0002). **Written and checked on 2026-10-28** (TLC 1.8.0, `scripts/formal/check.sh`):
   with retention 2 and with retention 1, six publishes and two readers, every reachable state (3,573,068 and
   15,412,000 distinct states) satisfies `AnswerRefines`, `CursorNotPastHead` and `LostIsNonNegative`. The model is
   able to say no: with `R` slots instead of `R + 1` it finds a `Lagged` that skipped nothing, and without the second
   stamp load it finds an `Item` whose words are of two different entries (3 and 0). Each of the two is the failure
   §4 and §5 predicted for that decision. The model contains the protocol of §4 and does not contain the
   `lost` total's sum, the batch read, or the width of the stamp.
2. **The memory-model checkers, which prove the ordering.** `loom` for Rust and GenMC for C and C++ (Kokologiannakis,
   Lahav, Sagonas and Vafeiadis, "Effective stateless model checking for C/C++ concurrency", POPL 2018; GenMC, CAV 2021)
   with scenarios of one lap, of a reader across a reuse, and of two readers; Lincheck for Kotlin; the race detector
   for Go. The scenarios reuse the `queue` gates' harness (`scripts/gates/genmc-queue.sh`, `scripts/lib/queue_sanitizer.sh`).
3. **The history checker of §5**, fed by each backend's stress run (every backend's recorder writes the one JSON form).
4. **Mutation cases** for each protocol decision, in the existing `.cases` form under `sce-build/tests/mutations/`:
   the first stamp store moved after the payload; the release fence removed; the acquire fence removed; the second
   stamp load removed; the stamp compared as a lap-blind index; `head` stored before the `done` stamp; the lag test
   with `>` for `>=`; retention counted as `R` slots instead of `R + 1`. A mutant must be killed by the layer that
   owns the decision, and a mutant a layer cannot kill is recorded against that layer rather than hidden.
5. **A reference that is not ours.** The downstream consumer builds its own ring first, as Rust types under a model
   test. Its model's cases (publish counts, reader positions, expected `Item`/`Lagged`) are exported as histories and
   judged by the checker, and the generated Rust row is run against the consumer's implementation as a differential
   oracle, in the manner of `tools/reference-oracles/`. Where the two disagree, the case is a `deviation:` that says
   which side the specification sides with. This is the admission's first consumer, and the reason the order of
   landing starts from the specification.

### 7. One structure under two kinds

`queue` and `broadcast-log` share, and must not copy: the resolution and size of the element document, the capacity
source and its deploy key, the width-and-wrap analysis (`QueueAlgorithm` rows' atomic requirements, `queue_progress_ceiling`,
`rust_runtime_row_atomics`, the `min_wrap_ops` check), the progress lattice (`QueueProgress`: `blocking < lock-free <
wait-free`), the history wire format's participant and clock model, and the gate harness. The first step of the order
below extracts these into a module both read; the extraction is behaviour-preserving and lands with the `queue`
tests unchanged and unedited. The names of the shared types stop saying `Queue`; the kind-specific codes keep their
prefixes, because a code names the rule of a kind.

### 8. What is refused, and the condition for revisiting it

- **`producers=many`.** A producer claims a sequence number with `fetch_add` and publishes later. Between the two,
  readers see the sequence either as absent (and cannot move past it, so reads are `blocking` on a stalled producer)
  or return `Empty` while later entries are visible, which no linearization of the log of §2 allows. Both are real
  designs (the Disruptor's multi-producer sequencer and its availability buffer), at a weaker progress claim and with
  an RMW atomic. They are revisited when a second independent consumer needs several producers and states the
  progress it can accept for readers.
- **Backpressure (a gated writer).** Revisited when a consumer states that a record may not be lost and the slowest
  reader may slow the producer. That is a different contract (no loss, so a reader registry and a producer that waits
  or is refused) and is then a different kind, not a flag on this one.
- **Overwrite of a consumed queue**, for the reason in §1: not revisited. The log with one reader is its replacement.

### 9. Admission (SCE_FORGE.md, "Admission test")

| Axis | Pass/Fail | Reason |
|---|---|---|
| Domain alignment | PASS | A concurrency primitive of the same family as `queue`; its observable contract (sequence, window, `Lagged`) is byte-identical across backends and its progress guarantee is declared per backend and checked, not assumed |
| Futamura projection compatibility | PASS | Element type, producer cardinality, retention, stamp width and required progress are build-time; only `head` and the slot contents are runtime |
| Cross-domain reuse | PASS | Three or more of the axis's reference domains, each with a use of this contract and none with an attribute of its own: **embedded telemetry and sensor pipelines** (an interrupt handler publishes samples and several tasks, a logger and a monitor each read every sample at their own pace, a slow one told how many it missed); **network protocols and observability** (a session or an observation plane publishes events that several diagnostic readers follow, an overflow reported and not stalling the session); **game networking** (a tick's recent events kept for a late joiner or a replay reader, who resumes from the oldest retained); **robotics control** (a sensor stream read by a controller, a logger and a safety monitor, none able to slow the sensor) |

The axis counts **domains the contract serves, drawn from the plausible consumer set**, and not consumers that have asked
(SCE_FORGE.md section 10, Axis 3). The contract here carries nothing of the first consumer: its document names an
element, a producer cardinality, a progress guarantee and a retention, and the loss report is the reader's own. That one
consumer has asked is the reason the kind is built now; the axis is passed on the contract. This reading is the owner's
decision of 2026-10-28 (below), and the sentence that states it in Axis 3 of SCE_FORGE.md was added with this ADR.

## Order of landing

Each step is green on its own, and the last two are the ones that need the consumer.

1. **The specification and its checker, with no kind.** `broadcast_history.rs`, the JSON schema of a history, the
   `check-broadcast-history` command, the general search and the differential test against the specialisation, the TLA+
   model and its configs. Nothing is generated yet; the consumer can already judge its own ring's histories.
2. **The extraction of §7**, behaviour-preserving, `queue` tests unchanged.
3. **The kind**: model, parser, the XSD, the kind catalog, the diagnostics and their acceptance-doc rows, the AST
   export, the new RFC section beside §synth-5-P (added through the Mnemosyne MCP, with its bindings), the SCE_FORGE.md
   admission note.
4. **Rust**: runtime row, template, loom scenarios, the conformance crate, the mutation cases.
5. **C11 and C++**: runtime, templates, GenMC scenarios. Then **Go**, **Kotlin**, **Python**; Swift stays refused.
6. **The differential run against the consumer's ring** (§6.5), then the consumer's switch to the generated row.

The templates of steps 4 and 5 (the code a document generates, as against the runtime row it calls) land one language at a
time after the element's bound, which they are sized from: step 6a gave the bound, and step 6b the Rust template, which
ends `runtime-missing` for Rust. C11, then C++, Go, Kotlin and Python follow, each ending it for its language; Swift stays
refused.

**How a language lands.** `codegen_matrix::BROADCAST_LOG_BACKENDS` is the one list of the languages whose template has
landed. `template_ships` and `awaits_runtime` are derived from it, so they cannot disagree, and every test that holds the
kind to the backends that ship it (the matrix, the catalog's `backends`, the refusal of the others) reads it. A language's
change therefore adds that one element and, with it, its template, its dispatch arm, its generated-code conformance, and
the removal of its row from the accepted gaps of `a_backend_holds_what_the_other_backends_hold`. The list is not
`Language::ALL` for the reason `STATIC_DATAMODEL_BACKENDS` is not: a backend joins a capability when it has it.

### What step 1 landed (2026-10-28)

- **The checker and its form.** `sce-build/src/broadcast_history.rs`: the general search of §5, the JSON form
  (`tests/forge/conformance/broadcast_history.schema.json`, held as strict as the reader by a test), and
  `sce-codegen check-broadcast-history`. 17 fixture histories are judged as their names say, among them the two answers
  the TLA+ model found for the rejected designs (a `lagged` that skipped nothing, an item of two entries), and the
  linearization-point case of §5 (a read that takes effect before an overlapping publish moves the window on). 11 tests
  pass on the build machine.
- **The search state is the frontier alone.** `h` is the number of publishes among the publisher's placed operations,
  and a reader's cursor is a function of the readings it recorded, so neither needs to be part of the state.
- **Still to do in step 1:** the specialisation of §5 and its differential test against the general search. The general
  search is the definition and is exponential in the number of readers' operations in the worst case, so until the
  specialisation exists the stress runs of the backends keep their histories short.
- **Step 2, the part that landed:** `sce-build/src/history_common.rs` holds the checker's answer (`Verdict`), the error of
  a file that is not a history, the clock model and the real-time order. `queue_history.rs` re-exports and uses them;
  its tests are unedited and pass. The element-document resolution and the width-and-wrap analysis wait for step 3,
  where the kind first needs them: nothing reads them before, and extracting what nothing reads would be a guess about
  their shape.

### What step 4 landed for Rust (2026-10-28)

- **The runtime row.** `backends/rust/forge-runtime/src/broadcast_log.rs`: `BroadcastLog<W, SLOTS>` with `W` 64-bit words
  an entry and `SLOTS = retention + 1`, a `Publisher` handle taken once (an 8-bit swap), copyable `Reader` cursors that
  carry their `lost` total, and `Read::{Item, Empty, Lagged}`. It shares the crate's atomics shim and `Padded` with the
  queue rows, so `--cfg loom` reaches it. Present only where 64-bit atomics are, as SCQ is. Seven unit tests and clippy
  with `-D warnings` pass on the build machine.
- **The memory-model check found two faults in this ADR's first protocol** (see "Orders the memory model adds"), both
  of which the sequentially consistent TLA+ model had passed. With both fixed the three loom models pass under three
  preemptions. That bound is the other loom models' and what is established is "no violation within it".
- **Nine mutation cases** (`sce-build/tests/mutations/a_broadcast_log_orders_every_slot_hand_over.cases`): one per
  order and fence the protocol needs, a lap-blind stamp, and a window one entry too wide. The first round of CI (2026-10-28)
  ran eleven and caught eight. Three survived, and none was a fault of the code. A window one entry too wide survived
  because the loom models checked that an item was the whole entry at the cursor and never that the window held it, which a
  unit test did check; the models now assert it against the head loaded before the read. The other two, the done stamp
  stored relaxed and the reload of `head` after a mismatch loaded relaxed, are not observable: the words of an entry reach a
  reader through `head`, and the value a reload needs is ordered by the stamp the reader just saw, so the casefile no longer
  claims them and says why where they stood. The code keeps both orders.
- **Real threads, judged.** `backends/rust/forge-conformance/tests/forge_broadcast_log.rs` runs one publisher and one or
  two readers on threads over logs of two to four slots, readers opened at the oldest, at the head and at the first
  entry, eight recordings a shape, and judges each recording in process and again, as files, by `sce-codegen
  check-broadcast-history` (five files, 61 to 98 operations each, all linearizable). `scripts/gates/forge-rust.sh` runs
  both, and the loom models. Whether any recorded run actually contained a `Lagged` answer is not asserted; the loom
  models are what reach that branch for certain.
- **Not built for Rust:** the generated template (it waits for the kind of step 3), Miri and ThreadSanitizer runs, the
  narrow-stamp form with its wrap bound, and the form for a target with only loads and stores. The conformance crate's
  `clippy -D warnings` over all targets was already red before this change, on an `into_*` method of generated codec
  code under the toolchain the build machine has; the new test target is clean.

### What step 3 landed (2026-10-28)

- **The kind, parsed and checked on every backend.** `ForgeKind::BroadcastLog` and `ForgeDocument::BroadcastLog` hold a
  `BroadcastLogModel` (`producers` and `progress` are the queue's `QueueCardinality` and `QueueProgress`, `retention` is a
  `CapacitySource`, so a count is written the two ways a capacity is). The parser shares the cardinality and progress reads
  with the queue's, requires all four elements with no default, and refuses `producers` `many` after it parses
  (`broadcast-log/producers-many-unsupported`, placed on the producers row). The grammar (`<sce:retention>`), the kind
  catalog and its example (`sce-build/kind-examples/broadcast-log.scxml`), the pseudocode head line, the manifest and
  catalog schemas, and the SCE_FORGE.md admission note are in.
- **Nine codes, each with its acceptance-doc row.** `element-type-not-a-kind` is raised by the cross-document pass beside
  the queue's and carries a `Fix::ReplaceOneOf`. `progress-unreachable-on-backend` (Python gives `blocking`) and
  `runtime-missing` are raised by the generator: the codegen matrix has a new outcome, `EmitOutcome::RuntimeMissing`
  (`awaits_runtime`), so that a valid document for any language, Swift included, is refused by the kind's own code with the
  document and the language named, and not as `codegen/generic-kind-backend-emit-missing`. The backend ceiling is checked
  first, because waiting for the row would not repair it.
- **Left undone, and why.**
  - `element-size-unbounded` was declared and not raised at this step: the codec model had no encoded size that is a
    constant of the build, so there was nothing to compare. Step 6a gives it one and raises the refusal.
  - `wrap-bound-unstated`, `wrap-bound-below-deploy-minimum` and `slots-not-a-divisor-of-the-stamp-space` are implemented as
    one function (`judge_broadcast_log_stamp`) over the stamp width, and no row calls it: the width is a property of a
    row, and no row exists. The deploy figure is the one the queue's SCQ row reads, `machines.<m>.queues.<name>.min_wrap_ops`,
    here keyed by the log's name; nothing resolves it for a log yet, and a retention written as `source="deploy"` has no
    resolution either (the queue's `resolve_queue_deploy_limits` has no counterpart), so a log with a deploy-key retention
    parses and is refused as `runtime-missing` before a number is needed.
  - `progress-unreachable` is wired to the selection table and cannot fire: the one row is wait-free, which no declaration
    exceeds. It fires when a row that gives less (a multi-producer one, §8) is added.
  - The new RFC section beside §synth-5-P is not added (it goes through the Mnemosyne MCP with its bindings), and the
    `validation/sce-attribute-unread` hint naming `broadcast-log` for an `sce:on-full` written on a queue is not confirmed
    or added (§1).
  - `apis/forge-ast.v1.schema.json` is regenerated from the types, not edited by hand.

### What step 5 landed: the other five runtime rows (2026-10-28)

Each row is the Rust protocol ported access for access, written by an agent that could not build and then built and run
once on the build machine. Every recorded run was judged by `sce-codegen check-broadcast-history`.

| Row | Where | Built and run | Histories judged |
|---|---|---|---|
| C11 | `backends/c/forge-runtime/include/sce/forge/broadcast_log.h`, four new `sce_atomic_*` externs | ctest 22 of 22 (4 new) | 100, linearizable |
| C++ | `backends/cpp/forge-runtime/include/sce/forge/broadcast_log.h` | ctest 13 of 13 (4 new) | 100, linearizable |
| Go | `backends/go/forge-runtime/broadcastlog/` | `gofmt`, `go vet`, 18 tests under `-race` | 100, linearizable |
| Kotlin | `backends/kotlin/forge-runtime/.../broadcastlog/` | Gradle build, contract, stress and Lincheck tests | 100, linearizable |
| Python | `backends/python/forge-runtime/sce_forge_runtime/broadcast_log.py` | 27 tests | 50, linearizable |

- **Go and Kotlin are sequentially consistent, and say so.** Their atomics are at least as strong as every Rust order, so
  the two fences have no counterpart there and the faults of "Orders the memory model adds" cannot occur. A weakened
  order is therefore not a mutation those rows can show, and none was written for them: the mutation cases are the Rust
  row's. Python is one ring under one lock and gives `blocking`, the ceiling the codegen matrix already gives it.
- **Where a row differs from Rust, on purpose.** C11 returns `SCE_BLOG_NO_SEQUENCE` from `publish` past the horizon
  instead of panicking, as a no-allocation, no-abort header must. Kotlin's horizon is the signed `Long`'s, half of the
  other rows', and a Kotlin `read` allocates one small result object. Neither changes what a reader observes below the
  horizon.
- **Not run.** The GenMC scenarios of C11 and C++ (four harnesses, two new fence mutants over every harness) are a gate
  whose cost the owner has sent to CI; the lists of mutants a scenario requires to be killed were chosen by reasoning and
  may need trimming when the first run is read. No ThreadSanitizer or Miri run.
- **Not asserted by any row's stress.** That a recorded run contained a `Lagged` answer. The deterministic tests reach
  it on one thread and the loom models reach the late branch for certain.

### What step 6a landed: the element's largest encoded size (2026-10-29)

- **`broadcast-log/element-size-unbounded` is raised.** `sce-build/src/forge/codec_size.rs` gives a codec the number the
  slot of section 4 needs: `largest_encoded_bytes` returns the most bytes any value of the codec encodes to, or an
  `Unbounded` that says why there is none, in the clause the diagnostic carries after "is not a build-time constant". The
  cross-document pass beside `broadcast-log/element-type-not-a-kind` asks it of every log's element and places the refusal
  on the element-type row. The pass reads the documents and no language, so the check holds for every language and for
  `check`. A procedure has no wire form and is refused with that reason. Nothing reads the number yet: the slot width, and
  the template that stores bytes, are the next piece.
- **The bound is read from the encoders, and is refused where it cannot be proved.** The positional path is the fixed
  fields by ascending byte offset (sub-byte fields that share an offset are one byte) and a trailing payload at its
  `sce:max-size`, after the zero padding a `sce:dma-burst-align` codec writes up to the payload's offset. The streaming path
  is each field in order: a fixed field at its width rounded up to bytes, a VLE integer at its cap, a payload at its
  `sce:max-size`, a `<sce:repeat>` at `max-count` times its body, a `<sce:tlv-chain>` at `max-depth` times its entry, an
  embedded body and the largest arm of a variant at that codec's own bound; an optional field counts present. A CBOR map
  and a content-line component are bounded by the arithmetic their C11 renderers already carry for `MAX_ENCODED_BYTES`: the
  two functions are the single implementation, and the renderers' results are unchanged. An enum's texts do not depend on
  the language, so the content-line bound takes the longest text of each imported enum from the enum document, and needs
  no language. The refusals: a `tail` or `length-ref` payload with no `sce:max-size`, a CBOR string or byte string with
  neither `sce:length` nor `sce:max-size`, a `<sce:repeat>` with no `max-count`, an import that does not resolve (a missing
  file, another kind, or a cycle of imports, which is how a recursive codec or variant is caught), and a codec held by
  another whose own bound is missing. Each names the field or entry and the declaration. The default caps that
  `CodecModel::max_frame_bytes` falls back to size a buffer hint, and are not a declaration; a bound over them could be
  smaller than a frame.
- **Not run.** The change was written with nothing built: neither the new unit tests nor the broadcast-log integration
  tests have been run, and the committed C11 constants of the CBOR and content-line codecs have not been regenerated and
  compared, though no arithmetic was edited.

### What step 6b landed: the Rust template (2026-10-29)

- **The kind is generated for Rust.** `sce-codegen` writes `<name>.rs` for a `broadcast-log` document from
  `tools/codegen/templates/forge/rust/broadcast_log.rs.jinja2`, through `render_broadcast_log_rust`, and the codegen matrix
  answers `Emit` for Rust alone: `awaits_runtime` is now per language, so every other language still answers
  `RuntimeMissing` and is refused by `refuse_broadcast_log` with `broadcast-log/runtime-missing`, after the progress ceiling
  it judges first (`judge_broadcast_log_progress_on_backend`, shared with Rust's arm; wait-free is Rust's ceiling, so it
  asks Rust nothing today). The module refuses a target without 64-bit and 8-bit atomics with a `compile_error!` under the
  cfg that gates the runtime module (a test reads the cfg out of `forge-runtime/src/lib.rs` and holds the two equal); states
  `RETENTION`, `SLOTS = RETENTION + 1`, `MAX_ENCODED_BYTES`, `WORDS = 1 + ceil(MAX_ENCODED_BYTES / 8)` (one length word,
  then the bytes, eight to a word, little-endian) and `DECLARED_PROGRESS`; defines the log type over
  `BroadcastLog<WORDS, SLOTS>`, an alias for its publisher and a three-way read result; and gives typed
  `publish(&mut Publisher, &Element)` and `read(&Log, &mut Reader)`. No `unsafe`, no allocation, no `std`.
- **Bytes in, bytes out.** `publish` encodes through the runtime's bounded slice sink into a stack buffer of
  `MAX_ENCODED_BYTES`, so an element that encodes to more is refused by the codec (`BufferOverflow`) and nothing is
  published: an entry is never a truncated element. `read` calls the runtime's `read` and decodes only on `Item`, which the
  runtime returns after the copy was validated against the slot, so a torn copy is `Lagged` and never decoded; a stored
  length above `MAX_ENCODED_BYTES` is `CodecError::LengthOutOfRange` and never a slice out of bounds. `publish` takes the
  element codec's own type, where `encode` is defined; `read` returns the stored form `rust_stored_element` gives (the
  helper the queue uses): that type when it holds no borrow, and its owned mirror at the non-allocating storage profile
  when it does, built by `try_into_owned_in::<Inline>` from the decode that borrowed the stack buffer.
- **How the three numbers travel.** The renderer needs the retention, the element's largest encoded size and whether the
  template can hold the element, and the document gives none of them. `ForgeCompileOptions::broadcast_log_resolutions` carries
  a `BroadcastLogResolution` per log, and only the document-set compile fills it, where the element document is in hand:
  `validate_broadcast_log_cross_refs` already computes the size for `element-size-unbounded` and now returns it, so the
  refusal and the render read one number. The retention is the literal, or, when the set is given a deploy and
  `target_machine`, the limit `machines.<m>.limits.<limit>` that `resolve_broadcast_log_retention` (the counterpart of
  `resolve_queue_deploy_limits`, reading the same `limits:`) finds; a key for another machine, or none given, leaves it
  `None`, as for the queue. A limit that is not a number is refused where the deploy file is read, for both. A single-document
  compile has none of this and is refused, naming the log and what is missing, never given a guessed number (so the
  single-document `check` sweep reports Rust as rejecting a valid log, and `check --forge <element> --forge <log>`
  generates it).
- **What is refused, and why.** A key naming this machine and an undeclared limit, a retention that resolves to 0 (one slot,
  which the protocol cannot represent), and an element the template cannot hold, each as `generate/invalid-config` placed
  on the row that wrote it. No new diagnostic code was added: the queue's `queue/deploy-limit-unresolved` carries a
  candidates fix, and a typed code for the log is the follow-up if a consumer wants the repair machine-readable. An element
  is refused when its `encode` or `decode` takes an input from the caller that a log has none to give (the flag inputs of
  a codec, the tag of a caller-tag variant), or when it is a CBOR map with a string or byte-string entry, whose decoded
  form borrows its buffer and has no owned mirror to copy it into.
- **`judge_broadcast_log_stamp` is not called.** The Rust row's stamp is 64 bits, which the function answers `Ok` for
  without reading the retention or the deploy: calling it would assert nothing. The row that keeps a narrower stamp calls it.
- **Tests.** `sce-build/tests/broadcast_log_kind.rs` holds the renderer (the constants with their computed values, the
  compile error and its cfg, the typed functions, the refusal of a stored length above the bound, the owned form of a
  borrowing element, each refusal above, the set's deploy resolution), and the matrix's unit tests hold the per-language
  answer. `backends/rust/forge-conformance/tests/forge_broadcast_log_generated.rs` compiles what the generator writes for a
  log with a literal retention over a three-byte element and a log with a deploy-key retention over a record with a payload
  of at most eight bytes, and on one thread reads the entries back whole and in order, tells a lapped reader how many it
  missed, keeps two readers' cursors apart, refuses a payload past the bound without publishing, and refuses a length
  word above the bound.
- **Not run.** The change was written with nothing built: neither the renderer, the template, the new tests nor the
  conformance crate have been compiled or run. The generated module has no mutation case of its own, and no loom or
  ThreadSanitizer model reaches it: the nine Rust cases and the loom models are of the runtime row it calls.

### What the thread tests under Miri establish (2026-10-29)

`backends/rust/forge-runtime/tests/broadcast_log_threads.rs` runs one publisher against two readers on real threads, a
stopped reader, and two threads asking for the publisher, asserting the contract of section 2 on every read (whole
entry, entry at the cursor, entry the window holds, cursor moves as the answer says, a lag skips at least one). The gate
(`scripts/gates/forge-rust.sh`) runs it, and the runtime's unit tests, under Miri with the pinned nightly and eight seeds.
It does not run it under ThreadSanitizer, on purpose: every access to a slot is atomic, so the sanitizer has no plain
access to report a race on, and a run would read as a watch that is not one.

What Miri sees was measured and not assumed. Each order of the protocol was weakened by hand, one at a time, in the Rust
source (the `.cases` file's own edits), and the test run under `cargo +nightly-2026-10-08 miri test -p sce-forge-runtime
--test broadcast_log_threads` with `MIRIFLAGS=-Zmiri-many-seeds=0..8` on one build machine; the unmodified source passed
the same command twice.

| weakened | Miri | what it reported |
|---|---|---|
| busy stamp stored relaxed | caught | a lag that skipped nothing |
| head stored relaxed | caught | a lag that skipped nothing |
| first stamp loaded relaxed | caught | a lag that skipped nothing |
| head loaded relaxed in `read` | caught | a lag that skipped nothing |
| second stamp loaded relaxed | caught | a lag that skipped nothing |
| release fence removed | caught | a torn copy that passed validation |
| acquire fence removed | caught | a torn copy that passed validation |
| any written stamp accepted | caught | an item that is not the entry at the cursor |
| window one entry too wide | caught | an item outside the window |
| done stamp stored relaxed | not caught | (not observable, see the casefile) |
| `head` reload after a mismatch loaded relaxed | not caught | (not observable, see the casefile) |

The two that are not caught are the two the casefile does not claim, and a second tool failing to see them is evidence
for the argument that they are not observable and not a proof of it. What the table does not say: it is one machine, one
nightly, eight seeds and one run each, and Miri's emulation of weak memory does not produce every outcome the memory model
allows (it does not reorder a load before an earlier store, for one). A caught mutant shows Miri can see that fault on
those schedules; an order that were not caught would not be shown safe. The mutation round of CI cannot repeat this
measurement: the selector hands its arguments to `cargo test`, and Miri is `cargo miri test`. Giving the selector a Miri
runner would let CI judge it, and is not built.

### What is still undone

- **The generated code for the other languages.** Rust's template landed (step 6b). The C11, C++, Go, Kotlin and Python
  templates, and generated tests that regenerate and build them, are next, one language at a time; each is refused by name
  (`broadcast-log/runtime-missing`) until its change flips the matrix, and each reads the same `BroadcastLogResolution`.
  Swift stays refused until it has a forge runtime. The rows with a narrower stamp, and the form for a target with only loads
  and stores, are not built, so `judge_broadcast_log_stamp` and the deploy's wrap bound still have no caller.
- **Generated mutation coverage** of the Rust template and its renderer, in the existing `.cases` form.
- **Element forms the Rust template refuses**: a codec that takes flag inputs or a variant tag from its caller, and a CBOR map
  with a string or byte-string entry. A typed diagnostic for an unresolved deploy limit.
- **The specialised checker** of section 5 and its differential test.
- **The narrow-stamp row and the load/store-only row** of section 4.
- **The differential run against the downstream consumer's own ring** (section 6, layer 5), which needs that ring's cases.
- **The new RFC section** beside §synth-5-P, and a round of CI that judges the nine Rust cases after the strengthened
  window assertion (the next round has not been read).

## Consequences

- `queue` keeps its definition. A reader of it is not asked to understand a policy it does not have.
- The consumer's request is met by a new kind that does more than the request: loss is reported per reader, the
  producer is wait-free, and the row exists on a target with no read-modify-write atomic.
- The cost is a kind, a specification, a checker, a TLA+ model and six runtime rows. The checker and the model are
  written once, before any generated code, so the specification is not shaped by an implementation.
- A reader that stops reading is not an error, and the log cannot make it one: how a context learns that there is
  something to read is the host's (a poll of `head()`, a flag it owns). The kind specifies no wake-up, as the others
  specify none.

## References

Checked on 2026-10-28 against a web search (title, venue and year as listed; the full text was not read unless stated):

- H.-J. Boehm, "Can seqlocks get along with programming language memory models?", MSPC 2012, pp. 12-20 (technical
  report HPL-2012-68). Its abstract analyses several implementations under the C++11 model and briefly the Java case.
- H.-J. Boehm, "Byte-wise atomic memcpy", WG21 P1478, R0 (January 2019) to R6 (December 2020). **Whether it was adopted
  into the standard or a Technical Specification was not found.**
- tokio, `sync::broadcast` module documentation (the `Lagged` contract above).
- M. Thompson, D. Farley, M. Barker, P. Gee and A. Stewart, "Disruptor: High performance alternative to bounded queues
  for exchanging data between concurrent threads", LMAX technical paper, 2011. Confirmed through secondary sources; the
  paper was not opened.

Cited from memory and **not yet checked**; each is verified against its venue before this ADR is accepted:
Herlihy and Wing (TOPLAS 12(3), 1990); Wing and Gong (JPDC 17(1-2), 1993); Batty, Dodds and Gotsman (POPL 2013);
Abadi and Lamport (TCS 82(2), 1991); Lamport, *Specifying Systems* (2002); Kokologiannakis, Lahav, Sagonas and
Vafeiadis (POPL 2018) and the GenMC tool paper (CAV 2021); Lameter (Gelato 2005); Kreps (LinkedIn Engineering, 2013);
Kleppmann (O'Reilly, 2017); Abdulla and Jonsson (Information and Computation 127(2), 1996); Cécé, Finkel and Iyer
(Information and Computation 124(1), 1996).

No IETF RFC defines this structure; none was found.

## Questions left open for the owner

None of the design. The three questions this ADR was proposed with were answered by the owner on 2026-10-28:

1. **Admission.** A kind with one consumer is admitted when its contract is domain-neutral. The third axis is judged on
   the domains the contract serves, not on how many consumers have asked; §9 states them, and SCE_FORGE.md section 10
   says so in Axis 3.
2. **The kind's name** is `broadcast-log`, as recommended: it names the type and the delivery, where `ring` names the
   storage and not the contract, and `journal` and `stream` collide with existing meanings.
3. **Retention is written as `entries="R"`**, as recommended: `R` is what a reader may rely on; the slot count `R + 1`
   is derived, and a document that stated slots would have to say "minus one" in every reading.

The owner accepted the ADR and told it to proceed on the same day.
