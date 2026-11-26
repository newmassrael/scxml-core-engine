# ADR 0146 — A bounded FIFO declares what it does when full: it refuses, drops the newest or evicts the oldest, and an element it does not deliver goes back to the push that lost it

- Status: Proposed (2026-11-26). Item 4b of the order agreed with a downstream consumer on 2026-11-19, as ADR 0111
  "Open questions" placed it ("U17 (a full queue's policy) reopens ADR 0062 in a record of its own"). Written under the
  owner's standing rule: the most long-term correct structure, textbook in its foundations, cost ignored; decided,
  not asked. It goes to the consumer for review before it is put to the owner for acceptance.
- Date: 2026-11-26
- Reopens: ADR 0062 §1, the cell *consumed once x forget the oldest*, and the third bullet of ADR 0062 §8
  ("Overwrite of a consumed queue ... not revisited"), on a reason §1 did not weigh (§"Why ADR 0062's refusal is
  reopened").
- Amends, once landed (each amended document gains an "Amended by: ADR 0146" line in the landing that moves it):
  - RFC §synth-5-P (`queue`): a declared full policy, the outcomes of a push, the selection table's new row (D2, D3);
  - `sce-build/src/queue_history.rs`: the sequential specification it judges histories against (D1, D5);
  - `docs/SCE_ACCEPTED_SUBSET.md`: `<sce:append>` gains `on-full` and `outcome` (D4);
  - ADR 0086's rule that an append to a full list raises `error.execution`, which stays the default (D4).
- Scope: the `queue` kind on every backend that generates one (Rust, C11, C++, Go, Kotlin, Python, Swift); a
  statechart's `<sce:append>` on every lane that lowers it.
- Related: ADR 0062 (`broadcast-log`), ADR 0084 (one inbox, one door), ADR 0088 (an intrusive queue's element stays
  a value), ADR 0108 (a statement writes its outcome into a standard enum), ADR 0111 D4 (a queue's capacity `c` is
  given at construction under its bound) and D5 (the owner is whoever holds the drive), ADR 0145 (the door proved as a
  concurrent object), `sce-build/src/queue_history.rs`, `scripts/gates/genmc-queue.sh`.

## Context

A consumer is moving a protocol stack's whole logic into SCE documents. Its census
(`docs/zenoh-logic-as-sce-kinds.md` in the consumer's repository, gap U17) finds three places where a bounded FIFO
that one side removes from does something other than refuse when it is full:

| Census row | What it does when full | Who pushes, who removes |
|---|---|---|
| S7 #61, the channels of the consumer's first C API (its `handlers.rs`) | the ring evicts its oldest element and releases it through the caller's `free_f`; the fifo drops the new one (single-threaded arm) or blocks the producer (multi-threaded arm) | the session's callbacks push; the application's `recv` moves an element out (`move_f`) |
| S8 #38, the RING handler of its second C API | drops its oldest value to make room | the session's drive thread pushes; the application's `try_recv` removes |
| S11 #31, the sends its session holds while it answers a replay | the event-send queue of `max(1, depth / 2)` evicts its oldest, and counts it | inside one session's state, one thread |

The census reads each as blocked: a `queue` refuses a full push and "only ever gives back the oldest element", and
`broadcast-log` forgets its oldest but is read without removal, by readers that never take an element.

ADR 0062 §1 drew the design space of a bounded channel on two axes, delivery (consumed once, or retained for every
reader) and what happens at the bound (refuse, wait, forget the oldest), and refused the cell *consumed once x forget
the oldest* for two reasons: the producer would advance the consumer's end, so the head becomes a location two
contexts write, which ends the Lamport row's wait-freedom on loads and stores alone and the invariants SCQ's entries
rely on; and "everything this cell is used for ... is a `broadcast-log` with one reader, which also reports what it
lost". §8 closed it as "not revisited".

### Why ADR 0062's refusal is reopened

The first reason is true of two rows and not of the cell. A head that the producer may also advance is a head written
by a compare-and-swap, and a ring whose head is claimed that way serves the cell with a producer that is still
wait-free (D3). The refusal took the two algorithms that existed for the abstract data type that did not.

The second reason fails where an element owns something. Each row above holds elements that are resources: an owned
sample the queue must release exactly once, by the caller's function, when it is evicted, and hand over exactly once
when it is removed. A log retains an entry for every reader and never knows that a reader has taken it, so no party
can release an overwritten entry that a reader may have taken, and two readers can each copy it. The log is the right
structure for values that are bytes and nobody's property; it cannot carry the one property these rows need:

> **Conservation.** Every element pushed ends with exactly one owner: the consumer that removed it, the push that
> was refused or dropped it, the later push that evicted it, or the queue that still holds it when it is torn down.

So the cell is a different abstract data type from the log, as ADR 0062 §1 argued for the log against the queue, and
it is reopened here with this consumer's reason.

### What is not part of U17

- **Several producers.** Every input that reaches a machine from another context is posted through ADR 0084's one
  door and applied by whoever holds the drive (ADR 0111 D5). The channels above are fed by a machine's outputs, so at
  any instant one context pushes, and the hand-over of the drive orders one holder's pushes before the next one's
  (release on give-back, acquire on take). A queue fed that way has one producer in the sense the selection table
  means. Producers that do not pass through a door are the case D3 refuses by name with the condition for revisiting
  it.
- **A producer that waits.** The multi-threaded fifo's blocking push is a refused push plus a wait the host owns
  (ADR 0062 §1: waiting is a scheduler's property, not a data structure's). The wait is ADR 0111's door that can be
  waited on, and is not a policy here.
- **The capacity given at run time** (gap U16) is ADR 0111 D4: a queue is constructed with `1 <= c <= CAPACITY`.
  This record uses that `c` everywhere it says capacity. The consumer's "a capacity of 0 is treated as 1" is its
  C API's own adapter and stays there.

### Prior art

| Source | What it gives this design |
|---|---|
| crossbeam-queue `ArrayQueue::force_push` (checked 2026-11-26 in crossbeam-queue 0.3.14, `src/array_queue.rs:293`) | The cell in a widely used library: a push into a full bounded MPMC queue moves the head by a compare-and-swap and *returns the element it replaced* (`Option<T>`), so the caller, not the queue, disposes of it. That is D1's hand-back. Its ring is Vyukov's bounded MPMC queue, in which a producer stalled between its claim and its stamp store stalls the consumers at that slot, so its progress is `blocking` (the reason D3 does not take it for several producers) |
| zenoh-pico `_z_ring_push_force_drop` and `_z_fifo_push_drop`, and the consumer's port of them (`handlers.rs`) | The three policies as one implementation with a discriminant, and the rule that the evicted element is released *after* the lock is dropped, because the release is caller code that may re-enter the library. D1 makes that rule structural: the queue never runs element code |
| tokio `sync::broadcast` (ADR 0062's prior art) | The retained-for-every-reader contract this record is *not* |
| Linearizability (Herlihy and Wing, TOPLAS 12(3), 1990); quiescent consistency (Aspnes, Herlihy and Shavit, "Counting networks", JACM 41(5), 1994) | The criterion for the outcomes, and the weaker, sufficient one for the loss count (D1) |

## Decision

### D1. One sequential specification: a bounded FIFO of capacity `c` with a full policy

The state is a sequence `s` with `|s| <= c`, `c >= 1`, and a count `lost`. The policy `P` is one of
`refuse`, `drop-newest`, `evict-oldest`, fixed for the object.

| Operation | When | Effect | Outcome |
|---|---|---|---|
| `push(v)` | `|s| < c` | `s := s . v` | `Pushed` |
| `push(v)` | `|s| = c`, `P = refuse` | none | `Full(v)`: `v` handed back |
| `push(v)` | `|s| = c`, `P = drop-newest` | `lost := lost + 1` | `Dropped(v)`: `v` handed back |
| `push(v)` | `|s| = c`, `P = evict-oldest` | `s := tail(s) . v`, `lost := lost + 1` | `Evicted(head(s))`: the oldest handed back |
| `pop()` | `|s| > 0` | `s := tail(s)` | `Popped(head(s))` |
| `pop()` | `|s| = 0` | none | `Empty` |

- **Hand-back, never disposal.** Whatever the queue does not deliver it returns to the push that lost it, as a value
  of the element type. The queue never runs element code (a destructor, a release function) under its own
  synchronisation; Conservation is then a theorem of the table, and each backend's push returns the value as that
  language moves values (Rust by move, C11 into a caller's out-parameter, the managed languages as the reference).
- **The outcome type is the policy's.** A push's outcome has exactly the arms its policy can produce: `Pushed` or
  `Full` under `refuse` (today's push, unchanged in every backend), `Pushed` or `Dropped` under
  `drop-newest`, `Pushed` or `Evicted` under `evict-oldest`. A caller cannot be handed an arm its queue
  cannot produce, and cannot mistake an eviction for a refusal.
- **`lost` is a statistic, quiescently consistent.** It is monotone, saturates at its width, and is exact whenever
  no push is in flight. It is not ordered against the outcomes: the per-push outcome is the exact, linearizable
  record of every loss, and `lost` is what a party that saw none of those outcomes (the consumer, a telemetry read)
  learns. Making it linearizable would put it in the word the eviction's compare-and-swap writes and give the
  Lamport row an atomic read-modify-write for `drop-newest`, for an exactness no row's consumer uses. It is kept
  per producer and summed on read, so no row gains a read-modify-write for it. A queue under `refuse` has none: a
  refused push loses nothing, its caller still holds the value.
- **One specification, two homes.** The same table is the sequential object a statechart's list is (D4) and the
  specification a concurrent `queue` is linearizable to (D2, D3), as ADR 0062 §7 shares one structure between two
  kinds.

### D2. The `queue` kind declares its full policy

```xml
<sce:on-full>evict-oldest</sce:on-full>   <!-- refuse (default) | drop-newest | evict-oldest -->
```

- A child element, as `<sce:producers>` and `<sce:progress>` are; absent means `refuse`, so every queue that
  exists keeps its behaviour, its generated code and its API byte for byte. The attribute spellings ADR 0062 tested
  and refused (`sce:on-full` on the root, `on-full` on `<sce:bounded>`) stay refused as they are.
- `drop-newest` is served by every `bounded` and `segmented` row: it is the row's refusal, reported as
  `Dropped` and counted in the pushing participant's own count. A row whose refusal is excused while other
  participants hold slots (`Refusal::WhileSlotsAreHeld`) is excused the same way for `Dropped`.
- `evict-oldest` is served by the row D3 adds, with `<sce:bounded>` storage only.
- Refused by name, each a new `DiagnosticCode` with its row in the acceptance document's appendix:
  - `queue/full-policy-not-for-storage`: any policy but `refuse` on `<sce:intrusive>` (the caller owns every node,
    so a push cannot meet a full queue, ADR 0088), and `evict-oldest` on `<sce:segmented>` (D6);
  - `queue/evict-oldest-many-producers`: `evict-oldest` with `<sce:producers>many</sce:producers>` (D3).

### D3. The row: a ring whose head is claimed

For `evict-oldest` with one producer and one or many consumers, the selection table gains a row: `B` slots
(`B = CAPACITY`), a 64-bit `tail` that only the producer stores, and a 64-bit `head` that every remover advances
by compare-and-swap.

- **Removal claims, then owns.** A consumer loads `head = h`, loads `tail` with acquire and answers `Empty` when
  `h = tail`; otherwise it copies slot `h mod B` with per-word atomic loads (the read of ADR 0062 §4, which a
  concurrent store cannot make a data race) and then attempts `CAS(head, h, h + 1)`. On success the copy is the
  element and the consumer owns it; on failure the copy is discarded unused (no destructor runs on it) and the
  consumer retries.
- **The producer evicts by the same claim.** When `tail - head = c` the producer copies slot `h mod B` and attempts
  the same compare-and-swap. On success it owns the oldest and returns it as `Evicted`; on failure a consumer took
  it, the ring is no longer full, and the push proceeds as an ordinary one. It then stores the new element into slot
  `tail mod B` with per-word atomic stores and publishes it by storing `tail + 1` with release.
- **Why a copy whose claim succeeds is exact.** `head` only grows. The producer writes slot `h mod B` only for a
  `tail` congruent to `h`, which with `tail - head <= c <= B` happens only once `head > h`. A successful
  `CAS(head, h, h + 1)` shows `head` was `h` from the copier's load to its claim, so nothing wrote that slot in
  between; a failed one is the only way a torn copy can arise, and it is discarded.
- **Ordering.** Every claim is a compare-and-swap with acquire-release, and the producer reads `head` with acquire
  before it decides the ring is full or writes a slot, so a remover's copy that its claim made the element happens
  before any later store into that slot. A copy whose claim fails is ordered against nothing, which is why it is made
  of per-word atomic loads and never used.
- **Progress, declared and checked.** The producer is **wait-free**: its claim fails at most once per push, and after
  a failure the ring holds fewer than `c` elements. Removers are **lock-free**: a claim fails only because another
  party claimed that element. A document that requires more of a side is refused by the existing
  `queue/progress-unreachable`.
- **Width.** `head` and `tail` are lap counters; the existing wrap analysis (ADR 0062 §7, the `min_wrap_ops`
  check) applies to them unchanged, and a target without 64-bit compare-and-swap takes the narrow rows' treatment
  ADR 0062 records for the log.
- **Several producers are refused.** A tail shared by producers is claimed and published in two steps; a remover that
  meets a claimed, unpublished slot either waits for a producer that may be stopped (the progress of crossbeam's
  `force_push`, `blocking`) or skips it and breaks FIFO order. The row is added when a consumer needs producers
  that do not pass through a door, and states the progress it accepts for removers; at `blocking`, crossbeam's
  algorithm is then the textbook row.

### D4. A statechart's append declares its full policy and may write its outcome

```xml
<sce:append target="held" expr="frame" on-full="evict-oldest" outcome="appendResult"/>
```

- `on-full` is `refuse` (the default), `drop-newest` or `evict-oldest`, per statement: a statechart's list is a
  sequential value the machine alone writes, so the policy is a property of the write, not of an object that a
  selection table must pick an algorithm for. `target` is a place (ADR 0108 D12), so a list held at a map's key is
  appended to under a policy too (`map<K, list<E>[N]>`, the ring of census rows S8 #59 and S11 #29).
- `outcome`, when written, names a variable of the new standard enum `sce:std/collection/append_outcome.scxml`
  (`appended`, `full`, `dropped`, `evicted`), imported as any enum is, exactly as `<sce:put>`'s outcome is
  (ADR 0108). The statement writes what it did there and raises nothing.
- Without `outcome`: under `refuse` a full list appends nothing and raises `error.execution`, ending the block
  (§scxml-4.9), as today; under `drop-newest` and `evict-oldest` nothing is raised, since the document asked for
  the loss. A machine that counts its losses (S11 #31's "counted") reads the outcome and counts in its own variable;
  a statechart's list keeps no `lost` of its own.
- The evicted element is not handed back: a statechart's values own nothing, and a document that wants it reads
  `held[0]` before an append it knows will evict.
- `on-full` other than the three, `outcome` naming no variable of that enum, and either attribute on anything but
  `<sce:append>` are `scxml/static-datamodel-rule`, each naming what is accepted.
- Every lane that lowers `<sce:append>` lowers both attributes. The saved form of a list does not change.

### D5. The verification, by layer (the structure ADR 0062 §6 gives the log)

1. **The specification and its checker.** `queue_history.rs` gains the policy and the outcomes `Dropped(v)` and
   `Evicted(v)`; its search (Wing and Gong with Lowe's memo) judges a history against D1's table, and value
   identity makes Conservation part of what it checks: a value popped twice, popped and handed back, or handed back by
   an eviction that the sequence could not have made, is a violation. Counterexample histories for each are kept as
   tests that must fail.
2. **A TLA+ model with a refinement mapping** (`docs/formal/ClaimedHeadQueue.tla`): the ring of D3, each atomic
   access a step, refines the sequence of D1 under `s = slots[head .. tail)`, for one producer and two consumers, and
   for `c < B` and `c = B`. It must say no when the claim is made after the copy is used, and when the producer
   writes the slot before its claim.
3. **The memory-model checkers**: `loom` for Rust and GenMC for C and C++ on the `genmc-queue` harness, with an
   eviction racing a removal at `c = B`; Lincheck for Kotlin; the race detector for Go.
4. **The history checker**, fed by every backend's stress run in the one JSON form.
5. **Mutation cases** in `sce-build/tests/mutations/`: the claim replaced by a store; the copy taken after the claim;
   the tail published before the slot; the eviction's write moved before its claim; `Evicted` returning the new
   element; `lost` counted before a failed claim.
6. **A reference that is not ours**: the consumer's mutex-based channel (`handlers.rs`) as a differential oracle
   for the outcomes of the same push and pop sequences, in the manner of `tools/reference-oracles/`.

### D6. What stays refused, and when each is revisited

- `evict-oldest` with several producers that do not pass through a door: D3.
- `evict-oldest` on `<sce:segmented>`: an LSCQ's oldest element sits in the oldest ring of a list whose rings are
  retired by their consumers; claiming it from the producer side is a second algorithm. Revisited when a consumer needs
  an evicting queue larger than one ring.
- A waiting producer: ADR 0111's door, as above.
- A consumed queue that also reports to each consumer how much it missed (`Lagged`): that is the log's contract.

## Consequences

- The three census rows of U17 have a home: S7 #61 and S8 #38 a `queue` with `evict-oldest` (or `drop-newest`)
  fed by a machine's outputs, S11 #31 an `<sce:append on-full="evict-oldest">` with its outcome counted. Together with
  ADR 0111 D4's run-time capacity, S7 #61 is no longer blocked by SCE.
- Every existing `queue` document is unchanged: same rows, same API, same histories.
- The history checker's specification gains two outcomes; every existing recorded history stays valid, since a
  `refuse` queue produces neither.
- Seven backend rows and their gates grow by one algorithm; the acceptance document gains two codes and one standard
  enum.

## Alternatives rejected

1. **Keep the refusal; the consumer uses a `broadcast-log` with one reader.** It cannot keep Conservation for
   elements that own something (§"Why ADR 0062's refusal is reopened").
2. **A consuming reader on `broadcast-log`.** One kind would name two abstract data types, the category error ADR
   0062 §1 refused for the queue.
3. **Evicting as a pop followed by a push on the existing rows.** Not one operation: under contention a push can
   evict twice, and on the Lamport row the producer cannot pop.
4. **Releasing the evicted element inside the queue** (zenoh-pico's `free_f`). It runs caller code under the queue's
   synchronisation (the re-entrant self-deadlock the consumer's port documents), and C11 has no destructor to run.
5. **A linearizable `lost`.** D1: an atomic read-modify-write on the Lamport row for an exactness no consumer reads,
   while the outcomes already record every loss exactly.
6. **A ring type for statecharts (`ring<E>[N]`).** A second sequential type repeating everything a list is (index,
   `len`, `<foreach>`, its saved form) for one difference in one statement.
7. **Crossbeam's `force_push` for several producers now.** `blocking` progress for a case no consumer has; D3
   names when it is added.

## Order of work

1. This record: the consumer's review, then the owner's acceptance.
2. D1 in `queue_history.rs`: the policy, the two outcomes, Conservation, the counterexample histories.
3. D4: the schema, the checker, `append_outcome.scxml`, the lowering on every lane, scenarios shaped like S11 #31
   (a held send queue that evicts and counts) and like S8 #59 (a ring held at a map's key).
4. D2: the grammar, the selection, the two codes with their acceptance rows, `drop-newest` on the existing rows.
5. D3: the TLA+ model first, then the Rust and C11 rows (the consumer's lanes) with `loom`, GenMC, the stress
   histories and the mutation cases; then C++, Go, Kotlin, Python and Swift.
6. D5 item 6: the differential run against the consumer's channel.

## Open questions

None for the owner. The consumer is asked whether any row of its census needs `Lagged`-style reporting to the
consumer of a consumed queue (D6, last item), which this record leaves to the log.
