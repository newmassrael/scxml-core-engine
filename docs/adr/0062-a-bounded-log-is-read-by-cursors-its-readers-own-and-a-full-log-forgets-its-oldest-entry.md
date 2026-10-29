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
| Seqlock (the Linux kernel's `seqlock.rst`; Boehm 2012; WG21 P1478) | The read protocol of §4 and its correctness under a language memory model |
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
a document that writes `sce:on-full` on a queue writes an attribute nothing reads. **Confirmed on 2026-10-29** against
the built `sce-codegen check`: on the root of a `queue` document it is refused as `validation/unknown-sce-attribute`
(a name no document of the tree uses), and on `<sce:bounded>` the schema refuses it (`The attribute ... is not
allowed`); the same document without it passes. The first is held by
`an_overflow_policy_written_on_a_queue_or_a_broadcast_log_is_refused` for both kinds. The refusal carries no hint
naming `broadcast-log`, and none is added: the check that refuses it is generic (its reads are the list of what a
parser takes, and its header disclaims a table of which attribute belongs to which kind), and a name-to-kind table in
it would be that list kept a second time. The kind's name is found where the kinds are listed (`SCE_FORGE.md`, the
acceptance document and RFC §5.S).

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
| `broadcast-log/runtime-missing` | the language has no template over its runtime row yet (Swift alone, since step 7) |

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

This is the sequence lock (the Linux kernel's, `Documentation/locking/seqlock.rst`; see References) in the form Boehm
showed correct under the C++11 model (Boehm, "Can seqlocks get along with programming language memory models?", MSPC
2012): the payload words are *atomic* loads and stores with relaxed order, so there is no data race,
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
  the stamp width and of one word. The Rust 64-bit row adds one, an 8-bit swap that takes the publisher's place, and
  needs 64-bit atomics; the form for a target with only loads and stores keeps a narrow stamp and takes the once-only
  handle by ownership (see "What the narrow and load/store-only rows landed"). Wait-freedom of read holds because a failed validation is not retried:
  a mismatch *is* the answer, `Lagged`. A single producer is what makes this true; with several, a claimed but
  unpublished sequence number is a hole that readers can neither return nor skip (§8).
- **Why `R + 1` slots.** A reader that has seen `h - r <= R` and found the slot reused did so because the producer began
  writing sequence `r + S`, which it does only after `head` reached `r + R + 1`. The reload `h'` therefore satisfies
  `h' - R > r`, so `skipped >= 1` and the answer agrees with the specification. With `R` slots there would be a state in
  which the entry the specification retains is the one being overwritten.
- **Stamp width.** The stamp is 64-bit where the target has 64-bit atomic loads and stores, and the sequence stays below
  `2^63` (292 years at a billion publishes per second). A target without them uses a 32-bit stamp, which wraps and
  would let a reader that stopped reading for about `2^31` publishes mistake a later lap for its own (the exact
  bound is `2^(w-1)` less the slots, and is proved in "What the narrow and load/store-only rows landed"). That
  hazard is not decidable from the document, so it is handled as `queue` handles 32-bit SCQ entries: the deploy states
  the longest pause it relies on (`queues.<q>.min_wrap_ops` generalised to name a log) and a missing or insufficient
  figure is a refusal, not a silent weakening. The slot index `n mod S` must stay continuous across the stamp's wrap,
  so `S` must divide `2^w`; the narrow-stamp form refuses a retention for which `R + 1` is not a power of two.
- **Per backend.** Rust: `AtomicU64` words (a 32-bit word in a pointer-width atomic under a narrow stamp) and `core::sync::atomic::fence`; `Copy`
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
- **Caller inputs are part of the element (decided 2026-10-29).** A codec may ask its caller for what its wire form does
  not carry: one or more single-bit **flag inputs** (`<sce:flag-input>`, which gate fields through `sce:present-if`), and,
  for a variant that has no tag field of its own, the **tag** that selects the arm on decode. The first revisions of this
  ADR refused such an element, on the ground that a log's `publish` and `read` call `encode` and `decode` with a sink and
  a cursor alone. That was the cheap answer and not the right one. An entry of a log must mean the same to every reader
  at every time (§2: all readers read the one sequence; Kreps's replay: the same log gives the same result), so what its
  bytes are decoded under cannot be supplied by whoever reads, and cannot be recovered from the publisher's context after
  the fact. It is a part of the entry, and the abstract element type of §2 is
  `E = Inputs x Value`, where `Inputs` is the product of the codec's caller inputs (the unit type for a codec that has
  none). Nothing else in §2 changes: it is stated over an arbitrary `E`, the protocol of this section moves opaque
  words, and the TLA+ model, the memory-model checkers and the history checker are untouched.
  - **The slot.** `[length][context words][payload words]`. The context words are the flags, one bit each in declaration
    order, bit `i` of the flags in bit `i mod w` of word `i div w`, `ceil(n / w)` words for `n` flags of `w`-bit words,
    followed, when the variant takes its tag from the caller, by one word that holds the tag. Every number is a
    build-time constant of the element (a flag input is one bit, which the parser fixes; a tag is a `u8`), so the entry
    is still a fixed number of words and `broadcast-log/element-size-unbounded` stays sound. An element with no caller
    input has no context word, and its slot, its constants and its API are exactly what they were.
  - **The typed API.** `publish` takes the flags (as booleans, never the codec's own integer, so that the value the
    codec encodes under and the value stored are one value) and the tag where the element has them; `read` returns them
    with the entry, so a reader sees under what an entry was published. Both are in the item the way the element is.
    An element with no caller input has the signatures it had.
  - **What the log does not do** for a tag: it does not derive the tag from the value. The codec's `encode` takes no tag
    (the active arm is the value's own discriminant), and no generated codec exposes the tag of its active arm; the
    publisher supplies the tag its value corresponds to, as every caller of such a codec does, and a reader receives
    exactly the pair that was published. Deriving it would remove the redundancy and is the better end state; it needs
    each codec backend to expose the arm's tag, and is a change to the codecs and not to the log.
  - **What stays refused.** A CBOR map entry of text or bytes in Rust, because the decoded form borrows the slot's copy
    (below, "Element forms the Rust template refuses"): the remedy is an owned form bounded by `sce:max-size`, a change to
    that codec's generator.

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
  second definition: it is a theorem with a proof ("What the specialised checker landed"), it is admitted only while a
  differential test holds it equal to the general search on random, corrupted and adversarially shrunk histories,
  `check` asks it first, and the general search remains the definition and decides any history the specialisation
  declines.
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
the same command. The measurement was made twice. The first time was on the algorithm as `broadcast_log.rs` held it; the
algorithm then moved, once, into `broadcast_log_core.rs` and became generic over the width of the stamp (the narrow row),
so the table below is the second measurement, on the core, with the same edits at their new lines. The symptom of one
weakening changed with the move (the release fence, below) and the verdict did not.

| weakened | Miri | what it reported |
|---|---|---|
| busy stamp stored relaxed | caught | a lag that skipped nothing |
| head stored relaxed | caught | a lag that skipped nothing |
| first stamp loaded relaxed | caught | a lag that skipped nothing |
| head loaded relaxed in `read` | caught | a lag that skipped nothing |
| second stamp loaded relaxed | caught | a lag that skipped nothing |
| release fence removed | caught | a lag that skipped nothing (a torn copy that passed validation, before the move to the core) |
| acquire fence removed | caught | a torn copy that passed validation |
| any written stamp accepted | caught | a torn copy: an item that is not one whole entry |
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

### What the specialised checker landed (2026-10-29)

- **The theorem.** `sce-build/src/broadcast_history/specialised.rs` decides a history of the one-producer log without a
  search. Write `x < y` for `x.returned <= y.invoked`, the order the general search keeps (it places an operation next
  only while no unplaced operation has returned at or before the instant it was invoked), `R` for the retention and `P`
  for the number of publishes. Every operation but a publish is an *observer*: a reader's open, read or head, and the
  publisher's head. For an observer `x`, `A(x)` is the set of `h` in `0..=P` at which the sequential log of section 2
  gives `x` the result it was observed to give (it is always an interval, and the module gives it in closed form for
  each call and outcome); `c_x` is the number of publishes that precede `x`, `d_x` the number that `x` does not precede,
  and `L_x = max(lo_x, c_x)`, `U_x = min(hi_x, d_x)` with `A(x) = [lo_x, hi_x]`. **A well-formed history is linearizable
  if and only if (I) `L_x <= U_x` for every observer `x`, and (II) `L_x <= U_y` for every pair of observers `x < y`.**
- **The proof**, in the module's documentation, is short. Necessity: take a linearization and let `g(x)` be the number of
  publishes before `x`; they are a prefix of the publishes, which contains the `c_x` that precede `x` and is contained in
  the `d_x` that `x` does not precede, so `g(x)` is in `[c_x, d_x]` and in `A(x)`, which is (I), and `x < y` gives
  `g(x) <= g(y)`, which is (II). Sufficiency: give `y` the greatest of `L_y` and of every `L_x` with `x < y`; (I) and (II)
  bound it by `U_y`, `<` is transitive so it never decreases along `<`, and ordering the observers by that number, each
  group by `returned`, with publish `j` after group `j`, is a total order that extends `<` and gives every operation the
  `h` it needs. The two properties the proof leans on are that `<` is transitive and that the publishes are chained by
  program order, and the second is exactly "one producer".
- **The procedure.** (I) is a test of each observer alone, with `c_x` and `d_x` found by binary search on the publish
  intervals, which increase. (II) is one sweep over the observers in order of invocation with a pointer into the same
  observers in order of return: the greatest `L_x` among those already returned at the instant `y` was invoked is what `y`
  must not be forced above. The cost is two sorts, `O(n log n)` for `n` operations, and no state is explored.
- **What it declines, and why that is all.** Two things, typed (`Declined`): a history that is not well formed (the
  hypothesis of the theorem; the general search then answers `Malformed` with the same message, since both run the one
  check), and an operation whose call and outcome have no row in the table, which no well-formed history has and which is
  there so that a call added to the history type is declined until the table is extended and is never guessed at. The
  cases a reader would expect to be declined are not: several readers are more observers; a reader's cursor is always
  determined, because the general search itself takes it from the readings the reader recorded and a read before an open is
  malformed; a `head` is an observer whose `A(x)` is one point; a second publisher is not a history of this type and a
  publish in a reader's list is malformed.
- **What it reports.** `check_specialised` returns `Linearizable`, `NotLinearizable(Unplaceable)` or `Declined`.
  `Unplaceable` names the first observer, in order of invocation, that fails: because no `h` gives it its result, because
  the result and the clock do not meet (`ClockWindow`), or because an observer that returned before it was invoked needs
  more entries than it can have seen (`Overtaken`); it also carries, per participant, the position of the first operation
  that cannot be placed, which is what `deepest` means in the general search's verdict. `check` keeps its signature and
  its class on every history: it asks the specialised checker and falls back to `check_general` on a decline, and a
  `NotLinearizable` that the specialised checker gave has `explored: 0` (it explored no state). `judge(history, Ask)` says
  which method decided, and `sce-codegen check-broadcast-history --method auto|general|specialised` chooses: the default
  is `auto`, `general` runs the definition alone, `specialised` refuses a history it declines and names the reason
  (a malformed history is reported as malformed under all three). A refusal by the specialised checker reads "reader 0's
  operation 1 cannot be placed: ...", where the general search's reads "the search explored N states".
- **The differential test**, `sce-build/tests/broadcast_history_differential.rs`, with one seeded SplitMix64 and no
  dependency: 3000 histories (4 retentions by 3 reader counts by 250) produced by running the sequential log in a random
  order with random interval widths, both checkers answering `Linearizable`; 2750 histories (11 corrupting changes by 250)
  each with one change (a wrong item value, an item at the wrong entry, a cursor, a lag count off by one, a return at or
  before the invocation, an empty where there was something, an item the window had dropped, a publish duplicated, a wrong
  head, an operation missing, an interval moved to the instant another operation returned or was invoked), both checkers
  giving the same class; 150 histories the general search refuses, shrunk by delta debugging with the two checkers
  compared on every history met on the way and on the 1-minimal result; a checker made blind to item values, to show that
  a disagreement is found and shrunk (to at least a publish, an open and an item); the shrinker's own test on a structural
  predicate; three histories of 2,000, 5,000 and 10,000 operations that only the specialised checker can decide, one wrong
  value in each found and named; and every fixture, through both checkers and through the command under each `--method`.
  A unit test in the module holds each closed form of `A(x)` equal to the sequential log's own `admits` at every `h`, for
  small numbers and at the edge of `u64` (17,820 comparisons).
- **What the test shows, and what it does not.** It shows that the two checkers give the same class (linearizable, not
  linearizable, malformed) on histories of at most fifteen operations, three readers, a retention of 1 to 4, and intervals
  that overlap their neighbours by up to six steps of a clock that moves four per operation, after one corrupting change.
  It does not show what the generator does not write: more readers, a retention above 4, two changes at once, histories
  longer than the general search can finish; the proof is what says the equality holds there. It compares classes, not
  explanations: `explored` and `deepest` differ between the methods by construction. And it cannot show that the definition
  is right: the proof is of the specialised checker against the code of the general search, so a defect that the general
  search and the closed forms share (a misreading of section 2) is carried to both. That is the work of the fixtures, which
  hold the two answers the TLA+ model found, and of the model itself.
- **Not run.** The change was written with nothing built: the module, its unit tests, the differential test and the command
  have not been compiled or run, and neither the running time of the differential file (the aim is about ten seconds in a
  debug build) nor the counts it asserts have been observed.
- **Mutation cases.** `sce-build/tests/mutations/a_specialised_broadcast_history_check_holds_both_conditions.cases` holds
  40 cases on `specialised.rs` and on the fall-back of `check` to the general search, and names nine mutants that are not
  cases, each with its reason (an equivalent, a tie that no test reads, a loop that never ends). The cases mutate the
  decisions the proof leans on: which end of a publish bounds each side of a window, each strict or non-strict
  comparison of (I) and (II), the order of the sweep, each closed form of `A(x)` with its window arithmetic and its
  edge of `u64`, and the declines. Each case says which test should turn red; most are held by a history built by
  hand, and eight are reached only by the seeded generator of the differential test and say so. The casefile was
  written with nothing built, and its first round is for CI to judge.

### What step 7 landed for Python (2026-10-29)

- **The kind is generated for Python.** `sce-codegen` writes `<name>.py` for a `broadcast-log` document from
  `tools/codegen/templates/forge/python/broadcast_log.py.jinja2`, through `render_broadcast_log_python`, and
  `BROADCAST_LOG_BACKENDS` lists Python, so the matrix answers `Emit` for it and `broadcast-log/runtime-missing` is no
  longer Python's answer. The module states `RETENTION`, `SLOTS = RETENTION + 1`, `MAX_ENCODED_BYTES`,
  `WORDS = 1 + (MAX_ENCODED_BYTES + 7) // 8` (one length word, then the bytes, eight to a word, little-endian: the entry
  every backend's log holds, so a Python log and a Rust log of one document store one thing) and `DECLARED_PROGRESS`;
  names the log as the runtime's `BroadcastLog` with a `new_<snake>()` that builds it from `WORDS` and `SLOTS`, as the
  queue's module names its queue; and gives typed `publish(publisher, element)` and `read(log, reader)`.
- **One resolution for every language.** What the Rust renderer checked before it read its template (the progress
  ceiling, a missing `BroadcastLogResolution`, an element the language cannot hold, a deploy-key retention nothing
  resolved, a retention of 0) is now `resolve_broadcast_log_render_inputs`, which both renderers call, so two languages
  cannot answer one document differently: what differs is `BroadcastLogElementFacts::refusal_for`, which each language
  states in an arm of its own, and the name the refusal gives the template ("the Rust template", "the Python template").
  Rust's refusals and their wording are unchanged. The renderer reads `options.broadcast_log_resolutions[log name]` as
  Rust's does, so a single-document compile of a log is refused for Python as it is for Rust.
- **Bytes in, bytes out.** `publish` encodes the element through the codec's own `encode` into a `MemoryviewSink` over a
  buffer of `MAX_ENCODED_BYTES`, so an element that encodes to more raises `BufferOverflow` and nothing is published: an
  entry is never a truncated element. `read` calls the runtime's `read` and decodes only on an `Item`; an `Empty` or a
  `Lagged` is the runtime's own value, returned as it is. A stored length above `MAX_ENCODED_BYTES` raises
  `LengthOutOfRange`, a new exception of the runtime's `codec` module that mirrors Rust's `CodecError::LengthOutOfRange`,
  and no bytes are cut by it. Python's `decode` answers `None` for bytes that are not an element, so a stored entry
  that does not decode raises `CodecError`, since the log wrote what the codec's `encode` produced and a refusal to
  decode it means the slot is not one this log wrote. A typed read returns a `<Pascal>Item` (sequence number and decoded
  element), where Rust returns an enum.
- **A CBOR text or byte-string entry is held.** `BroadcastLogElementFacts::refusal_for` answers `None` for Python where it
  answers a refusal for Rust. Python's decoded values own what they hold: the log hands `decode` a `bytes` and not a
  `memoryview`, the generated CBOR codec reads an entry through `sce_forge_runtime.cbor`, whose `read_text` decodes to a
  `str` and whose `read_bytes` copies, and a plain codec's byte field is `bytes(raw)`. There is no view of the log's buffer
  to refuse, and no owned mirror to build. The two inputs a caller must give (flag inputs, a variant's tag) are refused
  for Python as for every language.
- **Progress.** Python gives `blocking`, so a document that declares `lock-free` or `wait-free` is refused by
  `broadcast-log/progress-unreachable-on-backend` before anything else, for a single document and for a set. The
  conformance documents the Rust crate compiles declare `wait-free`; the Python test derives its logs from them by
  replacing that word, as the queue's Python test does, so one set of log documents is in the tree, and hands the generator
  the unchanged ones for the refusal.
- **Tests.** `sce-build/tests/broadcast_log_kind.rs` holds the renderer (the constants with their computed values, the typed
  functions, the order of the length check and the cut, an element with a payload, the refusal of a progress Python cannot
  give, a CBOR text entry that is held, what a caller must give, the resolution's numbers and the deploy's limit, and one
  text of the shared refusals for Rust and Python). `backends/python/forge-runtime/tests/test_broadcast_log_generated.py`
  generates the two logs as a document set through `sce-codegen orchestrate`, with a deploy for the second one's
  retention, imports the modules over the runtime, and on one thread reads the entries back whole and in order, tells a
  lapped reader how many it missed, keeps two readers' cursors apart, checks the words an entry packs, refuses an element
  past its bound without publishing, and refuses a length word above the bound and bytes that do not decode.
  `scripts/gates/forge-python.sh` runs it with the runtime's own tests.
- **Not built.** The generated module has no mutation case of its own. Under several threads the Python log is the runtime
  row's own test and history judgement. The change was written with nothing built: neither the renderer, the template,
  the new tests nor the Python test have been compiled or run.

### What step 7 landed for Kotlin (2026-10-29)

- **The kind is generated for Kotlin.** `sce-codegen` writes `<Name>.kt` for a `broadcast-log` document from
  `tools/codegen/templates/forge/kotlin/broadcast_log.kt.jinja2`, through `render_broadcast_log_kotlin`, and
  `BROADCAST_LOG_BACKENDS` lists Kotlin, so the matrix answers `Emit` for it. The file, in package
  `com.sce.generated.<log>` as the queue's is, states `RETENTION`, `SLOTS = RETENTION + 1`, `MAX_ENCODED_BYTES`,
  `WORDS = 1 + (MAX_ENCODED_BYTES + 7) / 8` (one length word, then the bytes, eight to a word, little-endian) and
  `DECLARED_PROGRESS` as `Int` and `String` constants; names the log as an alias of the runtime's `BroadcastLog` and its
  publisher as an alias of `BroadcastLog.Publisher`; and gives `new<Log>()` and typed `publish(publisher, element)` and
  `read(log, reader)` over the element's generated class.
- **One resolution, asked first.** `render_broadcast_log_kotlin` calls `resolve_broadcast_log_render_inputs`, as the Rust and
  Python renderers do, and repeats none of what it refuses. What is Kotlin's own is the width of its integers: the runtime
  takes its slot and word counts as `Int`, so a retention above 2147483646 (its `SLOTS` would not fit) and an element that
  encodes to more than 2147483640 bytes (`MAX_ENCODED_BYTES + 7` would wrap) are refused by name as `generate/invalid-config`
  on the row that wrote them, after the shared checks.
- **Kotlin's decoded CBOR owns its bytes, so the element Rust refuses is held.** `Cbor.readText` builds a new `String` with
  `decodeToString`, and `Cbor.readBytes` and `Cbor.readBytesExact` copy out of the cursor through `SceCursor.peekSlice`
  (`copyOfRange`); a non-CBOR codec's `bytes` payload is the same copy. A decoded value outlives the buffer a read decodes it
  from, so a log hands it back with no owned mirror to build. This is Kotlin's arm of `BroadcastLogElementFacts::refusal_for`,
  which returns `None` for the CBOR fact; the refusals that hold for every language stay (flag inputs and a variant's tag
  taken from the caller).
- **Bytes in, bytes out.** `publish` encodes through a `ByteArraySink` over a buffer of `MAX_ENCODED_BYTES` bytes, so an
  element that encodes to more is refused by the sink (`CodecError.BufferOverflow`) and nothing is published, and the
  sequence has no gap where it was; it returns `Published(seq)` or `Refused(error)`, never a thrown exception. `read` calls
  the runtime's `read` and decodes only on an item, which the runtime returns after the copy was validated against the
  slot, so a torn copy is `Lagged` and never decoded. A stored length above `MAX_ENCODED_BYTES`, or a word with its top bit
  set (the length is unsigned in the contract and a `Long` here), or bytes that do not decode, is `Unreadable(seq, why)` and
  never an index out of range; the reader has moved past that entry either way. A Kotlin `read` allocates the runtime's
  result, the entry buffer, the decoded element and the typed result; the horizon is the signed `Long`'s.
- **Tests.** `sce-build/tests/broadcast_log_kind.rs` holds the renderer (the constants with their computed values, the
  packing and unpacking, the stored-length refusal placed before any buffer is sized by it, the CBOR element Rust refuses, the
  caller-input refusal, the two integer bounds on both sides of their edge, the deploy key read from the machine's limits, and
  the shared refusals read as one text for Rust, Kotlin and Python); the tests that took C++ as the language with no template
  now read the list. `BroadcastLogGeneratedTest` in the Kotlin forge runtime's `jvmTest` compiles what the generator writes for
  a log with a literal retention over a three-byte element and a log over a record with a payload of at most eight bytes, both
  generated as one document set by the new `generateBroadcastLogFixtures` task of `build.gradle.kts` (`sce-codegen
  orchestrate`, since a lone `generate` has no element document to read the size from), and on one thread reads the entries
  back whole and in order, tells a lapped reader how many it missed, keeps two readers' cursors apart, refuses a record past
  the bound without publishing, and refuses a stored length above the bound and bytes that do not decode.
  `tests/forge/resources/broadcast_log_conformance_recent_records.scxml` is the one document added; it moves the `source-hash`
  line of the Go round-trip tree.
- **Not run.** The change was written with nothing built: neither the renderer, the template, the new tests nor the Kotlin
  conformance test have been compiled or run, and no mutation case covers the generated file.

### What step 7 landed for Go (2026-10-29)

- **The kind is generated for Go.** `sce-codegen` writes `<name>.go`, a package named like the log, from
  `tools/codegen/templates/forge/go/broadcast_log.go.jinja2` through `render_broadcast_log_go`, and `BROADCAST_LOG_BACKENDS`
  lists Go, so the matrix answers `Emit` for it and the accepted gap of `a_backend_holds_what_the_other_backends_hold` is
  gone. The package states `Retention`, `Slots = Retention + 1`, `MaxEncodedBytes`, `Words = 1 + (MaxEncodedBytes+7)/8` (one
  length word, then the bytes, eight to a word, little-endian: Rust's layout) and `DeclaredProgress`; names the log as an
  alias of `broadcastlog.Log` with a constructor `New<Log>()`; and gives `Publish(publisher, *Element) (uint64, error)` and
  `Read(log, *Reader) (broadcastlog.Read, *Element, error)` over the element package's struct. The runtime's own `Read`
  struct is the one statement of the three answers, so the typed read returns it with the decoded element, which is set only
  for an item, and defines no second result type. Everything refused before a template is read (the declared progress
  against the ceiling, a missing resolution, the language's verdict on the element, an unresolved deploy key, a retention of
  0) is the one shared function `resolve_broadcast_log_render_inputs`; what is Go's own is the verdict on the element and
  the module prefix the element's package is imported by (`broadcast-log '<name>'` ... `go_module_prefix`, as the queue's
  element import is refused). The element's package is imported under `go_package_alias`, as every stateless import is,
  because the typed functions reach it in their bodies.
- **Bytes in, bytes out.** `Publish` encodes with the element's own `Encode` into a `codec.BoundedSink` over an array of
  `MaxEncodedBytes`, so an element that encodes to more is refused by the codec (`codec.ErrBufferOverflow`) and nothing is
  published. `Read` calls the runtime's `Read` and decodes only for `ReadItem`, which the runtime returns after the copy
  was validated; a stored length above `MaxEncodedBytes` is `codec.ErrLengthOutOfRange` (a new sentinel beside Rust's
  `CodecError::LengthOutOfRange`) and is checked before any slice is cut by it.
- **Does a decoded Go element own its bytes.** For a CBOR text or byte-string entry it does: the runtime's `CborReadText`
  and `CborReadBytes` copy out of the cursor's buffer, so `BroadcastLogElementFacts::refusal_for` answers `None` for Go and a
  CBOR map with such an entry is held, where Rust refuses it. A positional `bytes` field of another codec does alias the
  cursor's buffer (`raw[off:]`); that is not the fact's shape, and the typed read decodes every entry from a buffer it makes
  for that entry alone, which the alias keeps alive and nothing else writes, so it stays valid for as long as the element is
  held. A test holds that the buffer is made once, inside the read.
- **Tests.** `sce-build/tests/broadcast_log_kind.rs` holds the package's text (the constants, the typed functions, the order
  of the length check and the slice it guards, the encode refusal before the publish), the buffer of its own, the refusal of
  an element that asks its caller for input in the shared words with `Go` as the template, the CBOR string held, the missing
  prefix and the resolved numbers. `backends/go/forge-runtime/conformance/broadcast_log_generated_test.go` compiles what the
  generator writes for three logs and, on one goroutine, reads the entries back whole and in order, tells a lapped reader how
  many it missed, keeps two readers' cursors apart, refuses a payload past the bound and the nearest above it without
  publishing, checks the packed words, and refuses a stored length above the bound and one too short for the element.
  `conformance/generate.sh` generates the set with `orchestrate` into a staging directory and moves each file into the
  package named like it, because a log is generated from its document set and a Go package is a directory.
- **One document added.** `broadcast_log_conformance_bounded.scxml` is a log with a literal retention over the record: the
  Rust conformance resolves the record's log from a deploy key, and carrying a deploy through `orchestrate` would also write
  the deploy's own per-machine artifacts, so the Go package takes the document that needs none. The deploy-key resolution
  is language-independent and is tested through Rust and `sce-build`. Adding a document under `tests/forge/resources` moves
  the `source-hash` line of the committed Go round-trip tree.
- **Not run.** The change was written with nothing built: neither the renderer, the template, the Go package it writes, the
  new tests nor `generate.sh` have been compiled or run. Go's atomics are sequentially consistent and the stamp is 64 bits,
  so `judge_broadcast_log_stamp` is not asked anything for Go, as for Rust. The generated package has no mutation case of
  its own.

### What step 7 landed for C11 (2026-10-29)

- **The kind is generated for C11.** `sce-codegen` writes `<name>.h` for a `broadcast-log` document from
  `tools/codegen/templates/forge/c/broadcast_log.h.jinja2`, through `render_broadcast_log_c`, and the codegen matrix answers
  `Emit` for C11 (`C11` joined `BROADCAST_LOG_BACKENDS`). The renderer asks the same shared judgement as Rust's first
  (`resolve_broadcast_log_render_inputs`: progress ceiling, missing resolution, the element's shape per language, an
  unresolved deploy key, a retention of 0), so the two cannot refuse one document in two ways. The header states
  `RETENTION`, `SLOTS = RETENTION + 1`, `MAX_ENCODED_BYTES`, `WORDS = 1 + ceil(MAX_ENCODED_BYTES / 8)` and
  `DECLARED_PROGRESS` as macros, defines the log type as the runtime's `sce_blog_t` with its storage laid out statically
  beside it (nothing allocates), and gives typed `publish` and `read` over the element's generated struct. `publish`
  encodes with the codec's own `<element>_encode_to_buf` into a buffer of `MAX_ENCODED_BYTES`, packs the length and the
  bytes little-endian into the entry's words, and calls the runtime's publish; a codec status other than OK is a failed
  publish and nothing reaches the runtime, and a log past its horizon answers `PUBLISH_HORIZON`. `read` calls the
  runtime's read and decodes only on an item, which the runtime returns after the copy was validated; a stored length above
  `MAX_ENCODED_BYTES` is `READ_BAD_LENGTH` and is never copied. Both results are small structs, as the runtime's own, and a
  failed encode or decode carries the codec's status.
- **The element is the codec's, and C11's struct owns its bytes.** The C codec decodes a CBOR text entry into a `char[N]`
  and a byte-string entry into a `uint8_t[N]`, each with its length beside it (the runtime's `sce_forge_cbor_read_text`
  and `_read_bytes` copy), so a decoded element outlives the buffer it was read from and a CBOR map with a text or byte
  string entry, which the Rust template refuses, is held. The arm of `BroadcastLogElementFacts::refusal_for` says so. The
  flag inputs and variant tag a caller must give are refused for C11 as for every language.
- **The runtime counts a log's slots and an entry's words in 32 bits**, so a retention above `4294967294` and an element
  that needs more than `4294967295` words are refused where the numbers are (`generate/invalid-config`, on the retention
  and element-type rows), and the header repeats the arithmetic as a static assertion against the target's address space.
  The header does not read `platform.atomic_rmw_width`: a target without the 64-bit `sce_atomic_*` symbols the runtime row
  needs finds out at link time, where a queue is refused at generation.
- **Tests.** `sce-build/tests/broadcast_log_kind.rs` holds the renderer (the macros and the typed functions as text, the order
  of the refusals before the copies, no allocation or abort, the empty element, the 32-bit limits at their edges, the shared
  refusal, a deploy-key retention). `backends/c/forge-runtime/tests/conformance/broadcast_log_generated_test.c`, built by
  that directory's `CMakeLists.txt` and registered as `broadcast_log_generated_c`, compiles what `sce-codegen orchestrate`
  writes for three logs (a literal retention over a three-byte element, a deploy-limit retention over a record with a payload
  whose bytes cross a word, and a log over a CBOR map with a text and byte strings) and runs, on one thread, the entries in
  order, a lapped reader told how many it missed, two readers with cursors of their own, the publisher's single place, the
  horizon, a stored length above the bound, a stored frame the codec refuses, and a CBOR element the codec refuses and the
  log does not store. The positional codec writes nothing of a payload whose length member is above its bound instead of
  refusing it, so the refusal of an oversize element is shown through the CBOR codec, which refuses.
- **Not run.** The change was written with nothing built: neither the renderer, the template, the new tests nor the C
  conformance target have been compiled or run, and the empty-element branch of the template has no generated-code test.

### What step 7 landed for C++ (2026-10-29)

- **The kind is generated for C++.** `sce-codegen` writes `<name>.h` for a `broadcast-log` document from
  `tools/codegen/templates/forge/cpp/broadcast_log.h.jinja2`, through `render_broadcast_log_cpp`, and `BROADCAST_LOG_BACKENDS`
  lists `Language::Cpp`, so the matrix answers `Emit` for it and `broadcast-log/runtime-missing` is no longer its refusal. The
  renderer asks the one function every language asks first (`resolve_broadcast_log_render_inputs`: the progress ceiling, a
  missing resolution, the language's verdict on the element, an unresolved deploy key, a retention of 0), so its refusals are
  Rust's word for word, and only the template's name differs. The header states `RETENTION`, `SLOTS = RETENTION + 1`,
  `MAX_ENCODED_BYTES`, `WORDS = 1 + (MAX_ENCODED_BYTES + 7) / 8` (one length word, then the bytes, eight to a word,
  little-endian) and `DECLARED_PROGRESS`; defines the log as `SCE::Forge::Broadcast::BroadcastLog<WORDS, SLOTS>` with aliases
  for its publisher and reader; and gives typed `publish(publisher, element)` and `read(log, reader)` in the namespace named
  after the log. A `static_assert` ties the runtime's retention to the header's, so the one number said twice cannot differ.
- **Where C++ differs from Rust, on purpose.** The answers are sum types, because C++17 has no `Result`. `publish` returns a
  `Published` holding the codec's refusal, if any, and the sequence number, which is the runtime's `kNoSequence` whenever
  nothing was published (a refused element, or the horizon). `read` returns a `std::variant` of `Item`, `Empty`, `Lagged` and
  `Malformed`: C++'s `CodecError` has no `LengthOutOfRange` and its decode convention is `std::nullopt`, so an entry whose stored
  length is above `MAX_ENCODED_BYTES`, or whose bytes the element does not decode, is a `Malformed{seq}` that names the entry and
  moves the reader past it, and is never a copy out of bounds. `publish` encodes through the runtime's bounded `SpanSink`
  into a staging array of whole words (never empty, even for an element that encodes to nothing) with the sink bounded at
  `MAX_ENCODED_BYTES`, so a longer element is refused with `BufferOverflow` and nothing is published.
- **C++ holds a CBOR map with a text or byte-string entry.** The decoded value owns its bytes: the C++ CBOR reader returns a
  `std::string` and a `std::vector<std::uint8_t>`, and every other codec encoding holds a string and a byte field as those
  types too. The arm of `BroadcastLogElementFacts::refusal_for` for C++ therefore refuses only what a caller must give (flag
  inputs, a variant tag), and no mirror of the element is needed.
- **Tests.** `sce-build/tests/broadcast_log_kind.rs` holds the header's constants with their computed values, its typed
  functions, the order of its two refusals, each name it calls against the runtime header that declares it, the set's deploy
  resolution, and the C++ verdict on a CBOR string entry; the shared refusals' test now runs for Rust, C++ and Python. The
  tests that used C++ as the language with no row use Swift. `backends/cpp/forge-runtime/tests/conformance/` compiles what the
  generator writes for the four Rust conformance documents, generated as one set with a deploy (`sce-codegen orchestrate`; no
  document was added) into `broadcast_log_generated_test`, run as ctest `broadcast_log_generated`: on one thread the entries
  come back whole and in order, a lapped reader is told it missed 3 and resumes at the oldest, two readers keep their cursors, a
  payload past the bound is refused without publishing, the packed words are the documented ones, and a length above the
  bound and undecodable bytes are `Malformed`.
- **Not run.** The change was written with nothing built: neither the renderer, the template, the new tests nor the C++
  conformance target have been compiled or run, and the generated header has not been through the interface-standard gate at
  C++17. The generated header has no mutation case of its own.

### What the narrow and load/store-only rows landed (2026-10-29)

Two Rust rows that no code had, and the model that justifies them. Which target gets which row:

| Target | Atomics the target has | Row (module of `sce-forge-runtime`) | Stamp | One publisher by |
|---|---|---|---|---|
| x86-64, AArch64, RISC-V 64 with `A` | 64-bit compare-and-swap (`target_has_atomic = "64"`) | `broadcast_log` | 64 bits | the 8-bit swap (`publisher()`), or `split` |
| Cortex-M3 and up, RISC-V 32 with `A`, ESP32 | 32-bit compare-and-swap, not 64 | `broadcast_log_narrow` | 32 bits | the 8-bit swap, or `split` |
| Cortex-M0, RISC-V 32 without `A` | loads and stores only | `broadcast_log_narrow` | 32 bits | `split`, or the `unsafe` `publisher_unchecked` for a `static` |
| AVR, MSP430 | a 16-bit pointer | none | | |

The generator reads the target as the Rust queue does: `devices.<d>.target` through `target_atomics::rmw_width`, or
`platform.atomic_rmw_width`; a width of 64, or a deploy that states neither, keeps the 64-bit row and its generated
text is byte-identical to what it was; 32 and 0 give the narrow row. The runtime module is present where its atomics
are (`cfg`), and the generated module carries a `compile_error!` under the negation of the same `cfg`, which a test
reads out of `forge-runtime/src/lib.rs`, as for the 64-bit row.

- **One implementation, generic over the width.** `forge-runtime/src/broadcast_log_core.rs` holds the algorithm once,
  over a sealed trait `Lane`: the atomic cell the stamp, the head and the words are kept in, the payload word, and the
  width `w` of the stamp. `Wide` is the 64-bit row (`AtomicU64`, `u64` words) and `Narrow<B>` a stamp of `B` bits in a
  pointer-width `AtomicUsize` over `u32` words. `broadcast_log` is an alias of the log over `Wide`, so its public names,
  the loom models of the 64-bit row and its unit tests are unchanged; `broadcast_log_narrow` is the alias over
  `Narrow<32>`. `B` is a parameter so that a model can use a stamp of two or three bits and reach the wrap in a few
  publishes, which a 32-bit stamp cannot do in a run of any length a model has. The cell is `AtomicUsize` and not
  `AtomicU32` because `cfg(target_has_atomic = "32")` means compare-and-swap, and the atomics of a load-and-store target
  are not nameable by a stable `cfg`; `AtomicUsize` is what the Lamport ring has used on every target. The price is the
  16-bit pointer, which has no row.
- **A sequence number is a count modulo `2^w`.** Nothing in shared memory is wider than the stamp, so the log cannot tell
  a reader the count itself. `head()`, `Reader::cursor()`, `Read::Item { seq }` and what `publish` returns are residues
  modulo `2^w`, the same for every handle; for the 64-bit row the count never reaches `2^64` and they are the counts.
  The first form of this argument had each handle keep its own 64-bit count; the residue is that count's low bits, and
  every handle reports the residue so that the handles agree. Three consequences are decided and tested. A narrow
  publisher has no horizon to stop at.
  `reader_at(seq)` reads `seq` modulo `2^w` as an entry the caller got from this log: fewer than `K = 2^(w-1)` entries
  behind the head it is taken as it is, and anything else (which is what a position past the head looks like) is the
  head. And `reader_oldest()` has to tell a log that published fewer than `RETENTION` entries (the oldest retained is
  entry 0) from one that published more and wrapped (it is `RETENTION` behind the head), which a residue cannot say:
  it asks the last slot. That slot is first written by the publish of entry `RETENTION`, so a stamp of `UNWRITTEN`
  there means at most `RETENTION` entries were published and the cursor is 0, and any other stamp means at least that
  many, and the cursor is `(head - RETENTION) mod 2^w`. `UNWRITTEN` is 1 in a narrow lane, an odd stamp that no write
  into the last slot produces (the last slot holds the entries `n = S - 1 (mod S)`, stamped `2S - 1` and `0` modulo
  `2S`, and `2S` divides `2^w`); 0 in the 64-bit lane, where no stamp is 0.
- **The theorem.** Let `M = 2^w`, `K = M / 2` and `S = R + 1` with `S` dividing `K` (a narrow log keeps a power of two of
  slots, at most `K`, and refuses another at its `new`). Let `H(t)` be the publishes completed at time `t` and `B(t)` the
  publishes begun (the busy stamp of entry `B - 1` stored). A read of a reader whose cursor stands for the count `c`,
  with its last shared access at `t_e`, is *within the bound* when `c <= H` and `B(t_e) - c <= K`. A read within the
  bound answers as the specification of section 2 does at its linearization point: `Empty` only at the head, `Item` only
  with entry `c` whole and in the window, and `Lagged` with the exact count and the cursor at the oldest retained.
  *Proof.* (a) The head extension is exact: `H - c <= B - c <= K < M`, so `(head - cursor) mod M = H - c`, which gives
  `Empty` (distance 0) and the range `Lagged` (distance at least `S`) their exact value, and the reload after a
  mismatch likewise. (b) For two entries `n, n'` of one slot, `done(n) = done(n')` exactly when `n = n' (mod K)`
  (`2n + 2 = 2n' + 2 mod 2K`; `n - n'` is a multiple of `S`, which divides `K`). A stamp equal to `done(c)` is the store
  of `done(c)` and no other: another store of that value is `done(c + jS)` with `jS` a positive multiple of `K`, and it
  follows the busy store of entry `c + jS >= c + K`, so `B >= c + K + 1`, against `B(t_e) - c <= K`. So the comparison of
  stamps means what it meant with no wrap, and the 64-bit proof of the seqlock read, which uses only "equal means the
  same store", carries over. (c) A stamp that is not `done(c)` is a later store than it (the load of `head` that put
  `c` in the window synchronizes with the release store of `done(c)`'s publish), so the busy stamp of entry `c + S` has
  been stored; that publish began after `head` reached `c + S`, the stamp load that saw it is an acquire, and the reload
  sees `H >= c + S`: the lag is `H - c - R >= 1`, exact by (a). (d) The first load of `head` is the last instant at which
  `c` was certainly in the window, and `R + 1` slots make that agree with the validation, as section 5 argues for the
  64-bit row; nothing there depends on the width. *QED.*
- **What a deploy states.** The hypothesis is about distance, and a deploy reasons about time. A read leaves the cursor
  within `R` of the head it loaded (`Item`: `c + 1 > H - R`; `Lagged`: exactly `H - R`; `Empty`: `H`), and one publish may
  be in flight, so if the publisher begins at most `P` publishes between a reader's load of the head in one read and its
  last access of the next, then `B(t_e) - c <= P + S`. The hypothesis therefore holds when `P <= K - S`: that `P` is
  `machines.<m>.queues.<log>.min_wrap_ops`, and `broadcast_log_longest_pause_ops` is the bound it is judged against. This
  corrects the bound the step that wrote `judge_broadcast_log_stamp` took, `K` itself (the stamp's capacity), which is the
  bound of the distance and not of the pause: a pause of exactly `K` leaves the reader `K + S` behind. The three
  refusals are unchanged; the `bound` they carry is `K - S`, and their message still says the stamp "tells `bound`
  publishes apart", which is now the usable part of what it tells apart (the wording is for a follow-up; a message change
  moves the diagnostic's pinned id). A deploy states nothing of a 64-bit stamp, whose count never reaches `2^63`.
- **The bound and the divisor are needed, not merely sufficient.** Without the bound: with `w = 3` and 2 slots, a reader at
  entry 0 that loaded the head with entry 0 published and was then held while the publisher began entries 1 to 4 finds
  `done(4) = 10 mod 8 = 2 = done(0)` in the slot of entry 0, copies entry 4, and the stamps agree: the entry of a lap on,
  handed as entry 0. Without the divisor: with 3 slots over 8 stamp values, a reader opened after the head wrapped
  holds a cursor of `n mod 8` and reads slot `(n mod 8) mod 3`, which is not the publisher's `n mod 3` (for `n = 8`:
  slot 0, against slot 2). The model finds both, and the loom model of the first fails when the hypothesis is broken.
- **Outside the bound the answer is defined and wrong in a bounded way, not undefined.** The reader cannot know it has
  broken the hypothesis, so what it does is the arithmetic of its cursor modulo `2^w`: its cursor is always right modulo
  `2^w` (`Empty` leaves it at the head's residue, `Lagged` at the oldest retained entry's); a reader exactly `2^w`
  behind is told `Empty`, and one more behind is told a `skipped` short by a multiple of `2^w`; a mismatch with a head
  that looks nearer than the window (a head that wrapped since the reader's cursor was valid) is read as one lap
  further behind, so a lag is never reported as skipping nothing; and a copy that validated while the reader was `K` or
  more behind is the entry a later lap wrote, handed as the one wanted. Four unit tests of `broadcast_log_core.rs` assert
  each outcome, from `a_reader_one_stamp_space_behind_is_told_empty_and_keeps_its_place` to
  `a_head_that_wrapped_during_a_mismatch_still_gives_a_lag_of_at_least_one`, with the arithmetic in their comments.
- **The model.** `docs/formal/BroadcastLogWrapped.tla` is the protocol over a stamp of `StampBits` bits (3 here, so
  `M = 8` and `K = 4`), with the head, the stamps and the cursors reduced modulo `M`, the reader's distance taken modulo
  `M`, and two history variables the specification is stated over (`n`, the publisher's completed count, and `ab`, the
  count the reader's cursor stands for). The wrap bound is `EnforceBound`: while it is on, a reader starts a read only
  when the newest publish begun is at most `K` past its cursor, and the publisher does not begin a publish that would put
  it further past the cursor of a reader inside a read: the theorem's hypothesis, at every instant. A reader opens at
  the head or, with `OpenMode = "oldest"`, by the two reads of the last slot and the head. The invariants are the
  existing model's (`AnswerRefines` with the exact gap added, `CursorNotPastHead`, `LostIsNonNegative`) and two more:
  `SlotIndexContinuous` and `OpenRefines`. `scripts/formal/check.sh` runs five configurations. Expected outcomes
  (not observed by the author, who could not run TLC): `BroadcastLogWrapped.cfg` (2 slots), `_r3.cfg` (4 slots, as many
  as `K`) and `_oldest.cfg` hold every invariant, with an *estimate* of 10^5 to 10^6 distinct states each and a run of
  seconds to a few minutes; `BroadcastLogWrapped_without_bound.cfg` (the bound switched off) violates `AnswerRefines`,
  with the entry of a lap on handed as the one wanted; `BroadcastLogWrapped_non_divisor.cfg` (3 slots over 8) violates
  `SlotIndexContinuous` when a reader opens at a head whose count has passed 8. A model that cannot say no proves
  nothing, so the last two are the checks that it can.
- **The loom models.** `tests/loom_broadcast_log_narrow.rs`, registered in `Cargo.toml` and run by the forge-rust gate
  beside the others, under the same three preemptions and the same shim, over a log of 2 slots and a **3-bit stamp**
  (`Narrow<3>`): five models, each starting from a log that already published six or seven entries on one thread, so
  that three more cross the head's wrap at 8 and the stamps' repeat at 4. Each reader keeps its own full count beside
  the log's cursor and every answer is held against the exact specification, through a count of what the publisher has
  completed that the reader reads before and after each call; the premise of the theorem holds in them by
  construction (at most three entries are published after a cursor is set). The fifth breaks it and must fail
  (`#[should_panic]`). The memory orders are one text for both rows, so the nine cases of the 64-bit casefile mutate both
  and remain the 64-bit models' to kill; the mutation
  casefile `a_narrow_broadcast_log_reads_its_head_modulo_the_stamp_space.cases` adds eight cases for what a 64-bit count
  cannot show (the head read modulo the stamp space, the cursor and the stamp truncated, the test for a young log),
  and says in its header which four it does not claim and why (the slot-count check is a compile-time refusal whose
  oracle is the `non_divisor` configuration, and so is the slot index; the lap added to a lag from a wrapped head and
  `reader_at` are held by unit tests, which the loom command does not run). `a_broadcast_log_orders_every_slot_hand_over.cases`
  was re-aimed at `broadcast_log_core.rs`, where the protocol now is.
- **What is generated.** The Rust renderer chooses the row from the target, after the refusals every language shares
  (`resolve_broadcast_log_render_inputs`, which also hands it the three new fields of the resolution, so no other renderer
  reads them), judges the log with
  `judge_broadcast_log_stamp` at the width the row keeps (`BROADCAST_LOG_NARROW_STAMP_BITS`, held equal to the runtime's
  `STAMP_BITS` by a test) and resolves `machines.<m>.queues.<log>.min_wrap_ops` for it, through
  `BroadcastLogResolution` (new fields: `atomic_rmw_width`, `target`, `min_wrap_ops`, filled from the deploy as the
  queue's resolution is). The narrow module states the pause the stamp covers, packs the entry in 32-bit words (four
  bytes to a word), and says in its header which of `publisher()` and `split()` the target has. A target the table does
  not know with no width beside it, and a width that contradicts the triple, are refused as `generate/invalid-config`,
  where the queue has typed codes naming a queue: no diagnostic code was added. `sce-build/tests/broadcast_log_kind.rs`
  holds the row choice, the three refusals through the renderer and through the set compile with a deploy, and the
  `compile_error!`'s gate; `backends/rust/forge-conformance` generates a narrow log and an ownership log from this
  crate's own documents and reads them back on the host, through `publisher()`, `split()` and a `static`.
- **The Miri gate and the thread tests.** `broadcast_log_threads.rs` and the unit tests of `broadcast_log::` are written against
  the names of the 64-bit row, which are unchanged (the alias, `publisher()`, `reader_oldest()`, `head()`, `RETENTION`).
  `reader_oldest()` of the narrow row reads the last slot's stamp before the head, because a count modulo `2^w` cannot tell
  a young log from one that wrapped. The 64-bit row does not: it takes the head less the retention, no lower than 0, with
  one load. The first version of the core asked the last slot in both rows, and the wide loom model
  `two_readers_each_hold_their_own_cursor` went from 0.6 s to 187 s, because the load added schedules to a three-thread
  model that opens two readers; no outcome of the 64-bit row depended on it. It is asked only where a count can wrap now,
  and the model is back to 0.6 s. The first thread test tolerates the narrow behaviour by construction: it takes `start`
  from the cursor it got, and a reader opened on a stale stamp is simply lapped by the first read. The gate's Miri filter is
  `--lib broadcast_log`, so the generic and narrow unit tests run under Miri with the 64-bit row's.
- **Run on the merged tree (2026-10-29).** Clippy over the workspace; the runtime's unit tests (28); the generation tests
  (101); the conformance crate (13: the 64-bit, narrow and ownership logs); loom (the 3 wide models and the 5 narrow ones,
  the `#[should_panic]` model panicking as it must); and TLC. The three configurations that hold finish with no violation
  (301,380, 28,722 and 331,516 distinct states); the two refused ones finish with their counterexample, a later lap read as
  the reader's own when the bound is dropped, and slot continuity broken when the slots do not divide `2^w`.
  **Not run:** the mutation round of the narrow casefile (CI's), and the builds of the runtime for `thumbv6m-none-eabi`
  and `thumbv7em-none-eabihf` (`scripts/gate nostd-mcu`). The generic code is type-checked there only as far as that gate
  builds it; nothing instantiates it for a load-and-store target.
- **A fact outside this change.** `scripts/formal/check.sh` pins the TLC release by SHA-256, and the build machines' copy
  of the same release, fetched from the same URL, has a different one (`fc6878c6...` against the pinned `15d103b7...`), so
  the script refuses it there. The run above used a copy that matches the pin. A lane that fetches it afresh would refuse
  it the same way.

### What the caller inputs landed for Kotlin (2026-10-29)

- **The template stores the context.** `broadcast_log.kt.jinja2` reads the `context` value that
  `broadcast_log_context_value` computes for every language (the renderer passes it over 64-bit words), and `Language::Kotlin` is
  in `BROADCAST_LOG_CONTEXT_BACKENDS`, so `refusal_for` no longer refuses a Kotlin log for flag inputs or a variant's tag. The
  slot is `[length][context words][payload words]`: flag `i` is bit `i % 64` of context word `i / 64` in the `LongArray` the
  runtime holds (the 64th is the sign bit, written with `shl 63` and read with `ushr`), the tag is one more word after the flag
  words, and the bytes start at word `1 + CONTEXT_WORDS`. `CONTEXT_WORDS` is emitted only when the element has a caller
  input, and then `WORDS = 1 + CONTEXT_WORDS + (MAX_ENCODED_BYTES + 7) / 8`; an element with none renders the file it always
  did, line for line.
- **The typed API.** `data class <Log>Flags(val <name>: Boolean, ...)` holds the flags, named as the codec names its parameters
  (camelCase). `publish(publisher, element, flags, tag)` takes only what the element has, the tag as the `UByte` the codec's
  `decode` takes, and `Item` carries `flags` and `tag` beside `element`; the sealed answer type and `Refused`/`Unreadable` are
  unchanged. A flag crosses to the codec through one private `flagInput(Boolean): UByte` (1 or 0), so the value the codec
  encodes under and the bit stored are the one Boolean, and `read` passes the stored bits, as the same conversion, to `decode`
  ahead of the tag. The log does not derive the tag from the value: the publisher gives it.
- **Tests.** In `sce-build/tests/broadcast_log_kind.rs` the Kotlin test that asserted the refusal of a flag input now asserts the
  file the set compile writes for a flag-input codec, and four are added beside it: the flags and the tag in context words of
  their own, a tag alone, 65 flags carried past one word (the sign-bit flag and the first flag of the second word), and an
  element with no caller input keeping the entry and the signatures it had. The two registry-driven tests
  (`what_a_caller_must_give_is_stored_where_the_template_stores_it_and_refused_by_name_elsewhere` and the shared refusals test)
  read the list and so follow the registration. `BroadcastLogContextGeneratedTest` in the Kotlin forge runtime's `jvmTest`
  compiles what the generator writes for the two logs over the six documents in
  `tests/forge/conformance/broadcast_log_context/` (generated as one set by `generateBroadcastLogContextFixtures`): the
  constants, the words of an entry as the runtime holds them (the context word between the length and the bytes), a reader
  getting back the flag it was published under, two readers decoding every entry under the same flag, and a reader getting
  back the tag.
- **A difference from Rust that the conformance does not hide.** The Rust codec refuses a flag that disagrees with the element
  (`PresentIfMismatch`, from the source form of ADR 0058), so a Rust log never holds an entry whose bytes and flag disagree. The
  Kotlin codec has no such check for a field gated on a flag input: its encode writes a gated field whenever the field holds a
  value and skips it when it does not, whatever the flag says (the chain-membership gates are the only ones it checks). A
  Kotlin publisher that gives a flag the element contradicts is therefore not refused, and the entry reads back decoded under
  the stored flag. No Kotlin test asserts the refusal the Rust test does, because the code does not do it; closing it is a
  change to the Kotlin codec generator, and moves its goldens and every generated Kotlin tree that has a flag-gated field.
- **Run (2026-10-29, build machines).** `cargo test -p sce-build --features cli --test broadcast_log_kind` passed 114
  tests with Kotlin, Python and Rust registered, the workspace clippy with `-D warnings` was clean, and
  `scripts/gates/forge-kotlin.sh` ended `BUILD SUCCESSFUL` after running `generateBroadcastLogContextFixtures`, compiling
  `BroadcastLogContextGeneratedTest` and running `jvmTest`. The Kotlin lane does not print a per-test count, so the six
  tests of that file are known to have compiled and the task to have passed, and no more than that.

### What the caller inputs landed for Python (2026-10-29)

- **The slot.** An element whose codec asks its caller for flag inputs or a variant tag has the entry
  `[length][context words][payload words]` in the Python module as in the Rust one (section 4, "Caller inputs are part of
  the element"). `broadcast_log_context_value` hands the template the layout over 64-bit words, and `render_broadcast_log_python`
  passes it as `context`. The module then states `CONTEXT_WORDS` and `WORDS = 1 + CONTEXT_WORDS + (MAX_ENCODED_BYTES + 7) // 8`;
  flag `i` is bit `i % 64` of context word `i // 64`, and a caller-tag variant adds one word after the flag words that holds
  the tag. An element with no caller input has no `CONTEXT_WORDS`, and its module, constants and signatures are exactly what
  they were (a test pins the lines). Python is in `BROADCAST_LOG_CONTEXT_BACKENDS`, so `refusal_for` no longer refuses it
  for a flag input or a tag.
- **The typed API.** A frozen dataclass `<Pascal>Flags` holds the flags as `bool` fields in the codec's order. `publish`
  takes `flags` and `tag` where the element has them (`publish(publisher, element, flags, tag)`), passes `int(flag)` to
  `encode` and stores the same value, and `read` returns them in the item (`<Pascal>Item(seq, element, flags, tag)`), after
  turning the stored bits back into `bool` and passing them, then the tag, to `decode`. A tag outside 0 to 255 is a
  `ValueError` before the codec is asked anything, because the entry stores the tag as one byte and Python has no `u8` to
  refuse it; nothing is published.
- **A difference from Rust that the log does not hide.** The Rust codec refuses a flag that disagrees with the field it gates
  (`PresentIfMismatch`, from its source-based `encode`). The Python codec's `encode` does not: it writes a gated field when
  the value is not `None` and never reads the flag, and the log, which does not judge the publisher's flags either, stores
  what it was given. A value published under a flag that says it is absent is stored with its bytes and read back without
  the value; a value published as absent under a flag that says it is there is stored and raises `CodecError` at `read`.
  `test_the_flag_that_was_stored_decides_how_an_entry_is_read_and_the_log_does_not_judge_it` pins this and turns red on the
  first `publish` the day the Python codec refuses the disagreement, which is the better end state and a change to the
  codec generator and not to the log.
- **Tests.** `sce-build/tests/broadcast_log_kind.rs` holds the Python module for flags alone, for flags and a tag, for a tag
  alone, for 65 flags (flag 63 closes the first context word, flag 64 opens the second), and for an element with no caller
  input; the refusal test for Python is now one of acceptance, and the registry-driven tests follow the list.
  `backends/python/forge-runtime/tests/test_broadcast_log_generated.py` generates the logs over the shared documents in
  `tests/forge/conformance/broadcast_log_context/` (a flag-input codec and a caller-tag variant with its two arms) through
  `sce-codegen orchestrate`, and checks the constants, the words of an entry, that a reader gets back the flag and the
  tag, that every reader decodes under the same flag, that a lapped reader resumes at an entry with its own flag, the
  behaviour above, and the refusal of a tag that is not one byte.
- **Run (2026-10-29, build machines).** The sce-build tests passed as listed in the Kotlin section, and
  `scripts/gates/forge-python.sh` ended with every `unittest` arm `OK`, the broadcast-log arm (45 tests, which includes
  `test_broadcast_log_generated`) among them. No generated tree moves, because no `.scxml` of the fixtures or the standard
  library changed.

### What the caller inputs landed for Go (2026-10-29)

- **The slot.** An element whose codec asks its caller for flag inputs or a variant tag has the entry
  `[length][context words][payload words]` in the Go package as in the Rust one (section 4, "Caller inputs are part of the
  element"). `broadcast_log_context_value` hands the template the layout over 64-bit words and `render_broadcast_log_go`
  passes it as `context`. The package then states `ContextWords` and `Words = 1 + ContextWords + (MaxEncodedBytes+7)/8`; flag
  `i` is bit `i % 64` of context word `i / 64` in the `[Words]uint64` the runtime copies, the tag is one more word after the
  flag words, and the bytes start at word `1 + ContextWords`. An element with no caller input has no `ContextWords`, and its
  package, constants and signatures are what they were (a test pins the lines). `Language::Go` is in
  `BROADCAST_LOG_CONTEXT_BACKENDS`, so `refusal_for` no longer refuses a Go log for a flag input or a tag.
- **The typed API.** `<Log>Flags` is a struct of `bool`s named as the codec names its flags (PascalCase, exported). `Publish`
  takes `flags` and `tag` where the element has them (`Publish(publisher, &element, flags, tag)`), the tag as the `byte` the
  codec's `Decode` takes, so Go has no out-of-range tag to refuse. A flag crosses to the codec through one unexported
  `flagInput(bool) byte` (1 or 0), so the value the codec encodes under and the bit stored are the one `bool`, and `Read` passes
  the stored bits, through the same conversion, to `Decode` ahead of the tag. Go's `Read` already returned the element
  beside the runtime's answer, which carries the sequence number, so an element with a caller input has `Read` return a
  `*<Log>Item{Element, Flags, Tag}` in the element's place (`Flags` and `Tag` only where the element has them), and an element
  with none keeps `(broadcastlog.Read, *Element, error)`. The log does not derive the tag from the value: the publisher
  gives it.
- **What the Go codec does with a flag the element contradicts.** Nothing: the generated `Encode` of a field gated on a flag
  input tests the optional field (`IsPresent()`) and never reads the flag parameter (only the chain-membership gates are
  checked, `ErrPresentIfMismatch`), and `Decode` reads the field under `(flag & mask) != 0`. So a Go publisher that gives a
  flag the element contradicts is not refused, by the codec or by the log (which does not judge flags either): a value
  published under a flag that says it is absent is stored with its bytes and reads back without the value, and a value
  published as absent under a flag that says it is there is stored and `Read` returns the codec's error for the short
  frame. The Rust codec refuses the disagreement (`PresentIfMismatch`, from its source form); Kotlin and Python do not,
  as above. `TestBroadcastLogContextTheFlagThatWasStoredDecidesHowAnEntryIsReadAndTheLogDoesNotJudgeIt` pins the Go
  behaviour and turns red on the first `Publish` the day the Go codec refuses the disagreement, which is the better end
  state and a change to the codec generator and not to the log (it moves the goldens and every generated Go tree with a
  flag-gated field). A tag that selects no arm is the codec's `codec.ErrNeedMoreBytes` at `Read` (the variant has no
  catch-all arm), which a test also pins.
- **Tests.** `sce-build/tests/broadcast_log_kind.rs` holds the Go package for flags alone, for flags and a tag, for a tag
  alone, for 65 flags (flag 63 closes the first context word, flag 64 opens the second) and for an element with no caller
  input; the two Go refusal tests (a flag input, a tag) are replaced by those, and the registry-driven tests follow the
  list. `backends/go/forge-runtime/conformance/generate.sh` generates the six documents of
  `tests/forge/conformance/broadcast_log_context/` as one set through `sce-codegen orchestrate`, and
  `broadcast_log_context_generated_test.go` compiles the two logs over them: the constants, the words of an entry as the
  runtime holds them, a reader getting back the flag and the tag, two readers decoding every entry under the same flag, a
  lapped reader resuming at an entry with its own flag, and the two behaviours above.
- **Run (2026-10-29, build machines).** `cargo test -p sce-build --features cli --test broadcast_log_kind` passed 122
  tests with the six languages registered so far, the workspace clippy with `-D warnings` was clean, and
  `scripts/gates/forge-go.sh` ended with the `conformance`, `round_trip` and `queue` packages `ok` and no `FAIL` line. The
  lane prints no per-test count, so that the nine `TestBroadcastLogContext` tests ran is known from the package passing and
  not from a count. No generated tree moves, because no `.scxml` of the fixtures or the standard library changed; the shared
  documents are new inputs of the Go lane only through `tests/forge/**`, which the lane's filter already covers.

### What the caller inputs landed for C11 (2026-10-29)

- **The slot.** `tools/codegen/templates/forge/c/broadcast_log.h.jinja2` reads the `context` value that `broadcast_log_context_value`
  computes for every language, and `render_broadcast_log_c` passes it over 64-bit words. An element whose codec asks its caller
  for flag inputs or a variant tag has the entry `[length][context words][payload words]`: the header then states
  `<LOG>_CONTEXT_WORDS` and `<LOG>_WORDS = 1 + CONTEXT_WORDS + ceil(MAX_ENCODED_BYTES / 8)`, flag `i` is bit `i % 64` of context
  word `i / 64` (written with `UINT64_C(1) << n`, so the 64th bit is a plain bit of a `uint64_t`), a caller-tag variant adds one
  word after the flag words that holds the tag, and the bytes start at word `1 + CONTEXT_WORDS`. An element with no caller input
  has no `CONTEXT_WORDS`, and its header, constants and signatures are the ones it always had (a test pins the lines).
  `render_broadcast_log_c` counts the context words in the entry's words it refuses past 32 bits, which the runtime counts in.
  C11 is in `BROADCAST_LOG_CONTEXT_BACKENDS`, so `refusal_for` no longer refuses it for a flag input or a tag.
- **The typed API.** A struct `<log>_flags_t` holds the flags as `bool` members in the codec's order. `<log>_publish(publisher,
  element, flags, tag)` takes `const <log>_flags_t *flags` and `uint8_t tag` where the element has them, passes each flag to
  `<element>_encode_to_buf` as the `uint8_t` 1 or 0 (through one `<log>_flag_input(bool)`, so the value the codec encodes under
  and the bit stored are one value) and stores the same bits. `<log>_read` fills `result.flags` and `result.tag`, turns the
  stored bits back into the `uint8_t`s it passes, flags then tag, to `<element>_decode`, and sets them before the decode, so a
  `READ_DECODE_FAILED` result also says what the codec refused under. The status enums, the length check and the bounds are
  unchanged.
- **A difference from Rust that the log does not hide.** The Rust codec refuses a flag that disagrees with the field it gates
  (`PresentIfMismatch`, from its source-based `encode`). The C codec does not for a field gated on a flag input: its generated
  `encode` writes a gated field whenever the field's optional value is present, and never reads the flag (read for a gated fixed
  field, which is what the shared document has; the other gated shapes were not each read; the check that returns
  `SCE_FORGE_CODEC_PRESENT_IF_MISMATCH`, `present_if_chain_encode_prologue`, is written only for chain-membership predicates).
  The log judges nothing either and stores what it was given: a present value published under a clear flag is stored with its
  bytes and reads back without the value, and an absent value published under a set flag is stored and refused at `read` by the
  codec. `the_flag_that_was_stored_decides_how_an_entry_is_read_and_the_log_does_not_judge_it` pins this and turns red at the
  first `publish` the day the C codec refuses the disagreement, which is the better end state and a change to the codec
  generator, not to the log. Nothing here was changed in the codec.
- **Tests.** `sce-build/tests/broadcast_log_kind.rs`: the C11 refusals of a flag input and of a tag are now acceptance tests, and
  five shapes are pinned (a flag input from the real codec, flags with a tag, a tag alone, 65 flags where flag 63 closes the
  first context word and flag 64 opens the second, and an element with no caller input keeping the lines it had); one more holds
  that the context words count in the 32-bit limit on an entry's words. The registry-driven tests follow the list.
  `backends/c/forge-runtime/tests/conformance/broadcast_log_context_generated_test.c` (ctest `broadcast_log_context_generated_c`)
  compiles, under `-Wall -Wextra -Wpedantic -Werror`, the headers `sce-codegen orchestrate` writes for the six documents of
  `tests/forge/conformance/broadcast_log_context/` in one set, and checks the constants, the words of an entry read through the
  runtime (the context word between the length and the bytes), a reader getting back the flag and the tag, two readers decoding
  under the same flag, a lapped reader resuming at an entry with its own flag, and the behaviour above. The variant's elements
  are made by the codec's own `decode` under the arm's tag, and read back by encoding them again, so the test names nothing of
  the variant struct's internals.
- **Run (2026-10-29, build machines).** The sce-build tests passed as listed in the Go section, and
  `scripts/gates/forge-c11.sh` ended with every one of its 28 ctest tests passed, `broadcast_log_context_generated_c`
  (built with `-Werror`) and the older `broadcast_log_generated_c`, which exercises the no-context path, among them.
  `clang-format` was run on the C test and not on the generated header. The short-entry case asserts only that the codec's
  status is not `SCE_FORGE_CODEC_OK`, so which status it answers is not pinned. No generated tree moves: no `.scxml` of the
  fixtures or the standard library changed.

### What the caller inputs landed for C++ (2026-10-29)

- **The slot.** `tools/codegen/templates/forge/cpp/broadcast_log.h.jinja2` reads the `context` value that `broadcast_log_context_value`
  computes for every language, and `render_broadcast_log_cpp` passes it over 64-bit words. An element whose codec asks its caller
  for flag inputs or a variant tag has the entry `[length][context words][payload words]`: the header then states
  `CONTEXT_WORDS` and `WORDS = 1 + CONTEXT_WORDS + ceil(MAX_ENCODED_BYTES / 8)` as `std::size_t` constants, flag `i` is bit
  `i % 64` of context word `i / 64` (written with `std::uint64_t{1} << n`, so the 64th bit is a plain bit of a `std::uint64_t`),
  a caller-tag variant adds one word after the flag words that holds the tag, and the bytes start at word `1 + CONTEXT_WORDS`.
  An element with no caller input has no `CONTEXT_WORDS`, and its header, constants and signatures are the ones it always had
  (a test pins the lines). The C++ words are a `std::size_t` count, so the renderer has no 32-bit refusal to count the context
  words into, unlike the C11 one. C++ is in `BROADCAST_LOG_CONTEXT_BACKENDS`, so `refusal_for` no longer refuses it for a flag
  input or a tag; with it, only Swift still refuses an element of this kind (`broadcast-log/runtime-missing`), and Rust a CBOR
  text or byte-string entry.
- **The typed API.** A struct `<Log>Flags` holds the flags as `bool` members in the codec's order. `publish(publisher, element,
  flags, tag)` takes `const <Log>Flags &flags` and `std::uint8_t tag` where the element has them, passes each flag to
  `element.encode(sink, ...)` as the `std::uint8_t` 1 or 0 (through one `flag_input(bool)` of the log's namespace, so the value
  the codec encodes under and the bit stored are one value) and stores the same bits. `read` takes the bits back out of the
  entry, passes them, flags then tag, to `<Element>::decode(cursor, ...)`, and returns them in `Item::flags` and `Item::tag`,
  after the element. The result alternatives `Empty`, `Lagged` and `Malformed` are unchanged: a malformed entry carries no flags
  or tag here, where the C11 result reports what its codec refused under. The length check and the bounds are unchanged.
- **A difference from Rust that the log does not hide.** The Rust codec refuses a flag that disagrees with the field it gates
  (`PresentIfMismatch`, from its source-based `encode`). The C++ codec does not for a field gated on a flag input, as the C
  one does not: its generated `encode` of a gated fixed field writes the field whenever the field's `std::optional` holds a
  value and takes the flag parameter only to silence an unused-parameter warning (read for a gated fixed field, which is what the
  shared document has; the other gated shapes were not each read; the check that returns `CodecError::PresentIfMismatch`,
  `present_if_chain_encode_prologue`, is written only for chain-membership predicates). The log judges nothing either and stores
  what it was given. `the_flag_that_was_stored_decides_how_an_entry_is_read_and_the_log_does_not_judge_it` pins this in the
  lane test and turns red at the first `publish` the day the C++ codec refuses the disagreement, which is the better end state
  and a change to the codec generator, not to the log. Nothing here was changed in the codec.
- **Tests.** `sce-build/tests/broadcast_log_kind.rs`: the C++ refusal of a flag input is now an acceptance test with five shapes
  pinned (a flag input from the real codec, flags with a tag, a tag alone, 65 flags where flag 63 closes the first context word
  and flag 64 opens the second, and an element with no caller input keeping the lines it had), and the C++ test of the CBOR
  element no longer expects a tag to be refused. There is no test of a 32-bit limit, because C++ has none. The registry-driven
  tests follow the list. `backends/cpp/forge-runtime/tests/conformance/broadcast_log_context_generated_test.cpp` (ctest
  `broadcast_log_context_generated`) compiles, under `-Wall -Wextra -Wpedantic -Werror` at C++20, the headers
  `sce-codegen orchestrate` writes for the six documents of `tests/forge/conformance/broadcast_log_context/` in one set, and
  checks the constants (as `static_assert`s), the words of an entry read through the runtime (the context word between the
  length and the bytes), a reader getting back the flag and the tag, two readers decoding under the same flag, a lapped reader
  resuming at an entry with its own flag, and the behaviour above. The variant's elements are made by the codec's own `decode`
  under the arm's tag, and read back by encoding them again, so the test names nothing of the variant's internals. For a value
  present under a clear flag the test reads the entry's words through the runtime and does not decode it, because whether the
  C++ `decode` leaves trailing bytes unread was not read.
- **Run (2026-10-29, build machines).** `cargo test -p sce-build --features cli --test broadcast_log_kind` passed 126
  tests with all six languages registered, the workspace clippy with `-D warnings` was clean, and
  `scripts/gates/forge-cpp.sh` ended with all 15 of its ctest tests passed, `broadcast_log_context_generated` (built with
  `-Werror`) and the older `broadcast_log_generated`, which exercises the no-context path, among them. `clang-format` was
  run on the C++ test and not on the generated header. No generated tree moves: no `.scxml` of the fixtures or the standard
  library changed.

### What is still undone

What was undone and is now done, as of 2026-10-29: the RFC section (§5.S of `docs/spec/synth/rfc-sce-protocol-synthesis.md`,
with its ledger section `§synth-5-S` and the bindings of every file of the kind, added through the Mnemosyne CLI); the
References, each checked against its venue (see References, which also withdraws one citation that was wrong); the
check that `sce:on-full` on a queue is refused (§1); the specialised checker's first run (on the merged tree, its 9 unit
tests, the 10 of the differential file and the 11 of the format file pass); and the narrow rows' loom, TLC, unit,
generation and conformance runs and their ARMv6-M and ARMv7-M builds (see their section).

What is left:

- **Swift** is refused by name (`broadcast-log/runtime-missing`) until it has a forge runtime. The narrow rows of the
  languages other than Rust, which each template will need on a target without 64-bit atomics, are not built: Rust's are.
- **Generated mutation coverage** of the Rust template and its renderer, in the existing `.cases` form.
- **Element forms the Rust template refuses**: a CBOR map with a string or byte-string entry (a codec that takes flag inputs or
  a variant tag from its caller is stored with the entry by every template that ships, section 4). A typed diagnostic for an
  unresolved deploy limit.
- **The stress runs of the six rows** still write short histories, a limit the general search imposed and the specialised
  checker does not; lengthening them is a change to each recorder, with a round of CI to read after it.
- **Rounds only CI can judge**: the casefile of the narrow row's mutation round, and the nine Rust cases after the
  strengthened window assertion, neither read yet; the first round of the specialised checker's casefile; and the Miri measurement, which CI cannot repeat because the
  mutation selector passes its arguments to `cargo test` and a Miri runner for `scripts/mutate` is not built.
- **The differential run against the downstream consumer's own ring** (section 6, layer 5), which needs that ring's cases.

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

Checked on 2026-10-29 against a web search (author, title, venue, volume and pages as listed; the abstract or the
bibliographic record was read, not the full text):

- M. P. Herlihy and J. M. Wing, "Linearizability: A correctness condition for concurrent objects", ACM TOPLAS 12(3),
  July 1990, pp. 463-492.
- J. M. Wing and C. Gong, "Testing and verifying concurrent objects", Journal of Parallel and Distributed Computing
  17(1-2), 1993, pp. 164-182. The exhaustive search over the sequential histories consistent with a concurrent one is the
  algorithm `check_general` implements; the specialised checker replaces it for this kind alone.
- M. Batty, M. Dodds and A. Gotsman, "Library abstraction for C/C++ concurrency", POPL 2013, pp. 235-248. Restricted to
  the sequentially consistent fragment its criterion implies linearizability, and not the reverse: the grounds on which
  section 5 states the criterion for the sequentially consistent model and leaves the weak-memory statement to the
  layers that run under it.
- M. Abadi and L. Lamport, "The existence of refinement mappings", Theoretical Computer Science 82(2), May 1991,
  pp. 253-284.
- L. Lamport, *Specifying Systems: The TLA+ Language and Tools for Hardware and Software Engineers*, Addison-Wesley,
  2002 (first printing July 2002).
- M. Kokologiannakis, O. Lahav, K. Sagonas and V. Vafeiadis, "Effective stateless model checking for C/C++
  concurrency", PACMPL 2 (POPL), article 17, January 2018; M. Kokologiannakis and V. Vafeiadis, "GenMC: A model checker
  for weak memory models", CAV 2021 (the project page lists RC11, IMM and LKMM as its models).
- P. A. Abdulla and B. Jonsson, "Verifying programs with unreliable channels", Information and Computation 127(2),
  1996, pp. 91-101; G. Cece, A. Finkel and S. P. Iyer, "Unreliable channels are easier to verify than perfect
  channels", Information and Computation 124(1), 1996, pp. 20-31. Both treat unbounded lossy FIFO channels, which is the
  sense in which a log is a lossy channel whose loss is reported to the reader; the log's capacity is bounded, which
  neither paper assumes.
- J. Kreps, "The Log: What every software engineer should know about real-time data's unifying abstraction", LinkedIn
  Engineering, 16 December 2013. The post defines the log as an append-only sequence of entries each numbered in
  sequence, read left to right.
- M. Kleppmann, *Designing Data-Intensive Applications*, O'Reilly, March 2017, chapter 11 "Stream Processing". The
  chapter's log-based brokers let several consumers read independently, and retention is decoupled from consumption.
- The Linux kernel, `Documentation/locking/seqlock.rst` ("Sequence counters and sequential locks"). It states that the
  mechanism cannot protect data that contains pointers, because the writer can invalidate one the reader is following;
  the slot of a `broadcast-log` holds words and no pointer, which is what §4 relies on.

**A citation that was wrong and is withdrawn.** Earlier revisions of this ADR cited "Lameter 2005" as a source of the
seqlock. The Gelato 2005 paper of that name ("Effective synchronization on Linux/NUMA systems") is about hierarchical
backoff locks, not sequence locks, and the table of prior art no longer cites it. The seqlock is credited in the
literature to Hemminger, after earlier work by Arcangeli, and its documentation is the kernel's file listed above.

**P1478 (byte-wise atomic memcpy)**, status as found on 2026-10-29: the WG21 record has it voted into the Concurrency
TS version 2 working paper at the Kona meeting in 2022 (revision R8, mailing of December 2022). Whether the C++
standard itself has adopted it, and under which function names, was not found; this ADR relies on the Rust and C++
runtimes' own word-by-word relaxed atomic copies and does not depend on the answer.

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
