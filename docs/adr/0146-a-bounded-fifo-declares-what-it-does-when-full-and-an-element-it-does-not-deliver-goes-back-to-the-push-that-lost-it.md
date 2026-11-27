# ADR 0146 — A bounded FIFO declares what it does when full: it refuses, drops the newest or evicts the oldest, and an element it does not deliver goes back to the push that lost it

- Status: Proposed (2026-11-26). Revised 2026-11-28 on the consumer's review, whose twelve items are each taken
  (section "Review", which names where). Item 4b of the order agreed with a downstream consumer on 2026-11-19, as
  ADR 0111 "Open questions" placed it ("U17 (a full queue's policy) reopens ADR 0062 in a record of its own"). Written
  under the owner's standing rule: the most long-term correct structure, textbook in its foundations, cost ignored;
  decided, not asked. It is put to the owner for acceptance once the consumer has read this revision.
- Date: 2026-11-26
- Reopens: ADR 0062 §1, the cell *consumed once x forget the oldest*, and the third bullet of ADR 0062 §8
  ("Overwrite of a consumed queue ... not revisited"), on a reason §1 did not weigh (§"Why ADR 0062's refusal is
  reopened").
- Amends, once landed (each amended document gains an "Amended by: ADR 0146" line in the landing that moves it):
  - RFC §synth-5-P (`queue`): a declared full policy and fullness, a close, a drain, a declared sharing, storage the
    caller lends, the outcomes of a push and a pop, the selection table's three new rows, the waiting protocol, and its
    bullets "Overwriting the oldest element is excluded" and "Elements still queued when the queue is destroyed are
    destroyed with it" (D1 to D3);
  - `sce-build/src/queue_history.rs`: the sequential specification it judges histories against (D1, D5);
  - `docs/SCE_ACCEPTED_SUBSET.md`: `<sce:append>` gains `on-full`, `limit` and `outcome` (D4), and the `queue` kind's
    paragraphs gain what D2 adds;
  - ADR 0086's rule that an append to a full list raises `error.execution`, which stays the default (D4).
- Scope: the `queue` kind on every backend that generates one (Rust, C11, C++, Go, Kotlin, Python, Swift); a
  statechart's `<sce:append>` on every lane that lowers it.
- Related: ADR 0062 (`broadcast-log`), ADR 0084 (one inbox, one door), ADR 0088 (an intrusive queue's element stays
  a value), ADR 0108 D4 (a statement writes its outcome into a standard enum; a run-time limit is a value of the
  machine), ADR 0111 D4 (a queue's capacity `c` is given at construction under its bound; `Pool::over`, storage the
  caller holds; U16's `c <= B`) and D5 (the owner is whoever holds the drive; the door that can be waited on), ADR 0141
  (a fixed list's raw parts), ADR 0145 (the door proved as a concurrent object), ADR 0154 (in draft: a map over storage
  its caller lends), `sce-build/src/queue_history.rs`, `scripts/gates/genmc-queue.sh`.

## Context

A consumer is moving a protocol stack's whole logic into SCE documents. Its census (gap U17) finds three places where
a bounded FIFO that one side removes from does something other than refuse when it is full:

| Census row | What it does when full | Who pushes, who removes |
|---|---|---|
| S7 #61, the channels of the consumer's C ABI in its no-heap MCU edition | the ring evicts its oldest element and releases it through the caller's release function after its lock is dropped; the fifo drops the new one (single-threaded arm) or blocks the producer (multi-threaded arm) | what peers send is pushed by whoever reads the network (a read task, where the edition runs one), and the thread that called into the session pushes, inside that call, the session's delivery to its own local subscriber or queryable; the application's receive moves an element out |
| S8 #38, the RING handler of the consumer's C ABI in its heap edition | drops its oldest value to make room, released inside its lock | the drive thread pushes what peers send, and the thread that called a put pushes, inside that call, the session's delivery to its own subscriber; the application's try-receive removes |
| S11 #31, the sends its session holds while it answers a replay | the event-send queue of `max(1, depth / 2)` evicts its oldest, and counts it | inside one session's state, one thread |

The census reads each as blocked: a `queue` refuses a full push and "only ever gives back the oldest element", and
`broadcast-log` forgets its oldest but is read without removal, by readers that never take an element. No row reports
to the removing side how many elements it missed: the consumer confirmed that D1's `lost` is all it reads.

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
> was refused, dropped or closed out with it, the later push that evicted it, or the caller of the drain that tore the
> queue down while it still held it.

So the cell is a different abstract data type from the log, as ADR 0062 §1 argued for the log against the queue, and
it is reopened here with this consumer's reason.

### How many producers, and what stays outside U17

- **Several producers are part of U17.** The first draft read every row as fed by one context, on the ground that an
  input reaching a machine from another context is posted through ADR 0084's door and applied by whoever holds the
  drive (ADR 0111 D5). That holds only where the consumer's answer to U18 is ADR 0111 D5 *and* every push into the
  channel is made by the drive's holder while it holds the drive. The consumer's C ABI does otherwise today, in both
  editions: a delivery the session makes to itself (its own subscriber, its own queryable) runs its callback on the
  calling thread, inside that call, which is the order ADR 0111 D5 keeps as R3069, while the drive thread pushes
  into the same ring what came from peers. Two producers then push into one ring at once. A queue fed only by the
  drive's holder has one producer in the sense the selection table means, since the hand-over of the drive orders one
  holder's pushes before the next one's (release on give-back, acquire on take); a queue fed otherwise has many, and
  D3 gives it a row: a ring under one lock, whose progress is `blocking`. That costs the consumer nothing it has now,
  since its channels are a ring under a mutex.
- **A producer that waits.** The multi-threaded fifo's blocking push is a refused push plus a wait the host owns
  (ADR 0062 §1: waiting is a scheduler's property, not a data structure's), and no queue operation waits. The order
  by which a party that waits is paired with a queue that never does, so that no wake-up is lost, is part of this
  record (D3, "Waiting").
- **The capacity given at run time** (gap U16) is ADR 0111 D4: a queue is constructed with `1 <= c <= CAPACITY`.
  This record uses that `c` everywhere it says capacity, and lets the caller lend the slots (D3, "Storage the caller
  lends"). The consumer's "a capacity of 0 is treated as 1" is its C ABI's own adapter and stays there. A statechart's
  list takes its run-time capacity per statement (D4).
- **Reporting to the removing side how much it missed** (`Lagged`) is the log's contract, and no census row needs it.

### Prior art

| Source | What it gives this design |
|---|---|
| crossbeam-queue `ArrayQueue::force_push` (checked 2026-11-26 in crossbeam-queue 0.3.14, `src/array_queue.rs:293`) | The cell in a widely used library: a push into a full bounded MPMC queue moves the head by a compare-and-swap and *returns the element it replaced* (`Option<T>`), so the caller, not the queue, disposes of it. That is D1's hand-back. Its ring is Vyukov's bounded MPMC queue, in which a producer stalled between its claim and its stamp store stalls the consumers at that slot, so its progress is `blocking` (Alternatives rejected, 7) |
| crossbeam-channel's bounded flavour (checked 2026-11-28 in crossbeam-channel 0.5.17, `src/flavors/array.rs`) | A close that is part of the queue's state: `disconnect` sets a mark bit in `tail` by `fetch_or` (:498-499), so a send that claims `tail` meets the close in the same word and hands its message back (`TrySendError::Disconnected(msg)`). And the order a waiting receiver keeps: it registers (:442), then checks the channel again (:445), and a write notifies the receivers (:245) |
| zenoh-pico `_z_ring_push_force_drop` and `_z_fifo_push_drop`, and the consumer's port of them | The three policies as one implementation with a discriminant, and the rule that the evicted element is released *after* the lock is dropped, because the release is caller code that may re-enter the library. D1 makes that rule structural: the queue never runs element code |
| Nikolaev, "A scalable, portable, and memory-efficient lock-free FIFO queue", DISC 2019 | LSCQ closes a ring by a *finalize* bit in its `Tail`, which an enqueuer's fetch-and-add reads in the same step: the close of the SCQ rows (D2) |
| Hoare, "Monitors: an operating system structuring concept", CACM 17(10), 1974 | The bounded buffer under one lock with two conditions: the row for several producers (D3) and its waiting |
| Reed and Kanodia, "Synchronization with eventcounts and sequencers", CACM 22(2), 1979 | The eventcount: a waiter reads a count, checks its condition, and waits only while the count is unchanged, so a notification between its check and its wait is not lost (D3, "Waiting") |
| Herlihy, "Wait-free synchronization", TOPLAS 13(1), 1991 | Loads and stores cannot solve consensus between two parties wait-free; deciding whether the producer or a consumer owns the oldest element is such a consensus, which is why a core without compare-and-swap takes a critical section (D3) |
| tokio `sync::broadcast` (ADR 0062's prior art) | The retained-for-every-reader contract this record is *not* |
| Linearizability (Herlihy and Wing, TOPLAS 12(3), 1990); quiescent consistency (Aspnes, Herlihy and Shavit, "Counting networks", JACM 41(5), 1994) | The criterion for the outcomes, and the weaker, sufficient one for the loss count (D1) |

## Decision

### D1. One sequential specification: a bounded FIFO of capacity `c` with a full policy, a close and a drain

The state is a sequence `s` with `|s| <= c`, `c >= 1`, a count `lost`, and a flag `closed`, false at construction. The
policy `P` is one of `refuse`, `drop-newest`, `evict-oldest`, fixed for the object.

| Operation | When | Effect | Outcome |
|---|---|---|---|
| `push(v)` | `closed` | none | `Closed(v)`: `v` handed back |
| `push(v)` | not `closed`, `|s| < c` | `s := s . v` | `Pushed` |
| `push(v)` | not `closed`, `|s| = c`, `P = refuse` | none | `Full(v)`: `v` handed back |
| `push(v)` | not `closed`, `|s| = c`, `P = drop-newest` | `lost := lost + 1` | `Dropped(v)`: `v` handed back |
| `push(v)` | not `closed`, `|s| = c`, `P = evict-oldest` | `s := tail(s) . v`, `lost := lost + 1` | `Evicted(head(s))`: the oldest handed back |
| `pop()` | `|s| > 0` | `s := tail(s)` | `Popped(head(s))` |
| `pop()` | `|s| = 0`, not `closed` | none | `Empty` |
| `pop()` | `|s| = 0`, `closed` | none | `Closed` |
| `close()` | any | `closed := true` | `Done` (a second close changes nothing) |
| `drain()` | the last operation on the object | `s := <>` | `Drained(s)`: every element still held, oldest first |

- **Hand-back, never disposal.** Whatever the queue does not deliver it returns to the push that lost it, as a value
  of the element type. The queue never runs element code (a destructor, a release function) under its own
  synchronisation; Conservation is then a theorem of the table, and each backend's push returns the value as that
  language moves values (Rust by move, C11 into a caller's out-parameter, the managed languages as the reference).
- **A close is part of the queue's state.** A closed queue refuses every later push with `Closed(v)`, and still
  delivers what it held: a pop answers `Closed`, not `Empty`, only once it holds nothing, and from then on it answers
  nothing else, since no push can add to it. The flag lives in the word whose update is a push's linearization point
  (D3 names it for each row: a mark bit in the tail, a field under the lock), never in a flag beside the queue. A push
  that read a separate flag and then published could succeed after a close had returned, and a pop could answer
  `Closed` before an element it later delivers; no order of the operations explains either. Close is offered by a
  queue that declares it (D2), so a queue that does not keeps the rows, the API and the histories it has.
- **A drain is the teardown.** `drain` hands every element the queue still holds to its caller, in order, and ends
  the object. It is the last operation by definition: Rust takes the queue by value, C11 and the other languages take
  the caller's word that every handle is released, as they do for freeing it. So it needs no synchronisation, every
  queue offers it, closable or not, and Conservation is closed on a language that runs no element code: in C11 an
  element still held at the end goes to the drain's caller, which releases it. The RFC's "elements still queued when
  the queue is destroyed are destroyed with it" stays true for Rust's `Drop` and C++'s destructor, which run with no
  other party left and so under no synchronisation; C11 has no destructor to run, and its teardown is `drain`.
- **The outcome type is the policy's.** A push's outcome has exactly the arms its policy can produce: `Pushed` or
  `Full` under `refuse` (today's push, unchanged in every backend), `Pushed` or `Dropped` under
  `drop-newest`, `Pushed` or `Evicted` under `evict-oldest`, and `Closed` besides on a closable queue; a pop's has
  `Closed` only on a closable queue. A caller cannot be handed an arm its queue cannot produce, and cannot mistake an
  eviction for a refusal.
- **`lost` is a statistic, quiescently consistent.** It is monotone, saturates at its width, and is exact whenever
  no push is in flight. It is not ordered against the outcomes: the per-push outcome is the exact, linearizable
  record of every loss, and `lost` is what a party that saw none of those outcomes (the consumer, a telemetry read)
  learns. Making it linearizable would put it in the word the eviction's compare-and-swap writes and give the
  Lamport row an atomic read-modify-write for `drop-newest`, for an exactness no row's consumer uses (the consumer
  confirmed that `lost` is all it reads). It is kept per producer and summed on read, so no row gains a
  read-modify-write for it. A queue under `refuse` has none: a refused push loses nothing, its caller still holds the
  value, and a push closed out loses nothing either.
- **One specification, two homes.** The same table, without `close` and `drain`, is the sequential object a
  statechart's list is (D4), and the whole table is the specification a concurrent `queue` is linearizable to (D2,
  D3), as ADR 0062 §7 shares one structure between two kinds. A list is a value the machine alone writes, so it has
  nothing to close and nothing to tear down.

### D2. The `queue` kind declares its full policy, its fullness, its close and its sharing

```xml
<sce:on-full fullness="exact">evict-oldest</sce:on-full>   <!-- refuse (default) | drop-newest | evict-oldest -->
<sce:closable/>                                            <!-- absent: no close, as today -->
<sce:sharing>owner</sce:sharing>                           <!-- shared (default) | owner -->
```

- `<sce:on-full>` is a child element, as `<sce:producers>` and `<sce:progress>` are; absent means `refuse`, so every
  queue that exists keeps its behaviour, its generated code and its API byte for byte. The attribute spellings ADR
  0062 tested and refused (`sce:on-full` on the root, `on-full` on `<sce:bounded>`) stay refused as they are.
- **`fullness`** says what a full queue is for the outcomes `Full` and `Dropped`. `at-rest`, the default, is the
  RFC's "capacity is exact at rest": a row may also refuse while operations of other participants hold slots
  (`Refusal::WhileSlotsAreHeld`, the SCQ and LSCQ rows), so under it a `Dropped` can be answered while `|s| < c`.
  `exact` is D1's table as written: `Full` and `Dropped` only at `|s| = c`, judged under `Refusal::AtCapacity`, which
  is what the consumer's no-heap fifo does (it drops only when its length equals its capacity). A document that
  writes `exact` is never given a row whose refusal is excused by held slots; the table below says which rows those
  are, and where only such a row reaches the declared progress the document is refused by the existing
  `queue/progress-unreachable`.
- `drop-newest` is the row's refusal, reported as `Dropped` and counted in the pushing participant's own count, on
  every row that serves `refuse`.
- `evict-oldest` is served by the rows D3 adds, with `<sce:bounded>` storage only.
- **`<sce:closable/>`** gives the queue D1's `close` and the `Closed` outcomes. Absent, the queue has neither, and
  the rows that would publish by a read-modify-write to see a close (the Lamport ring) keep their loads and stores.
- **`<sce:sharing>`** is ADR 0111 D4's spelling for a pool. `shared`, the default, is every queue today: its
  producers and consumers may run in different contexts. `owner` says that every producer and every consumer is one
  context (one thread, or whoever holds the drive under ADR 0111 D5), and selects the sequential ring (D3), which
  uses no atomic access.

The selection table of RFC §synth-5-P, with this record's rows in place. The generator still takes the first row that
matches and whose guarantee is at least the declared progress; a row added after an existing one never takes a
document the existing one serves.

| Storage | Sharing | Producers | Consumers | On full | Row | Producer | Consumer | Fullness | Close |
|---|---|---|---|---|---|---|---|---|---|
| `bounded` | `owner` | one | one | any | sequential ring (D3, new) | wait-free | wait-free | exact | a field |
| `bounded` | `shared` | one | one | `refuse`, `drop-newest` | Lamport ring | wait-free | wait-free | exact | a mark in `tail`, the publish a compare-and-swap |
| `bounded` | `shared` | one | any | `evict-oldest` | claimed-head ring (D3, new) | wait-free | lock-free | exact | a mark in `tail` |
| `bounded` | `shared` | any other | | `refuse`, `drop-newest` | SCQ | lock-free | lock-free | at rest | the finalize bit in `Tail` |
| `bounded` | `shared` | any | any | any | locked ring (D3, new) | blocking | blocking | exact | a field under the lock |
| `segmented` | `shared` | one | one | `refuse`, `drop-newest` | linked Lamport rings | wait-free, bounded by the allocator | wait-free | exact | a mark in the tail of the ring the producer writes |
| `segmented` | `shared` | any other | | `refuse`, `drop-newest` | LSCQ | lock-free, bounded by the allocator | lock-free | at rest | the finalize bit |
| `intrusive` | `shared` | any | any | `refuse` only | the Vyukov list | as today | as today | (a push cannot fail) | refused |

- Refused by name, each a new `DiagnosticCode` with its row in the acceptance document's appendix:
  - `queue/full-policy-not-for-storage`: any policy but `refuse`, or a `fullness`, on `<sce:intrusive>` (the caller
    owns every node, so a push cannot meet a full queue, ADR 0088), and `evict-oldest` on `<sce:segmented>` (D6);
  - `queue/close-not-for-storage`: `<sce:closable/>` on `<sce:intrusive>`. The Vyukov push is an exchange that cannot
    fail, so it cannot meet a close in the word it writes (D6);
  - `queue/owner-queue-in-two-contexts`: an `owner` queue whose deploy placement names more than one context, an
    interrupt handler and a thread among them (D3).
- Several producers under `evict-oldest` are no longer refused: they take the locked ring, and a document that
  declares `lock-free` or `wait-free` for them is refused by `queue/progress-unreachable`, whose fix names
  `blocking`.

### D3. The rows: a ring whose head is claimed, a ring under one lock, a ring in one context

#### The claimed-head ring: one producer, one or many consumers, `evict-oldest`

`B` slots with `c < B` (`B = CAPACITY + 1` when the queue holds its own storage, so every `c <= CAPACITY` fits; the
length of the span when the caller lends it), a 64-bit `tail` that the producer advances and a closer marks, and a
64-bit `head` that every remover, the producer's eviction among them, advances by compare-and-swap.

- **Removal claims, then owns.** A consumer loads `head = h`, loads `tail` with acquire and, when `h` equals the
  tail's count, answers `Closed` if the tail is marked and `Empty` otherwise; else it copies slot `h mod B` with
  per-word atomic loads (the read of ADR 0062 §4, which a concurrent store cannot make a data race) and then attempts
  `CAS(head, h, h + 1)`. On success the copy is the element and the consumer owns it; on failure the copy is discarded
  unused (no destructor runs on it) and the consumer retries.
- **The producer publishes first and evicts after.** It loads `tail = t` (if marked, it answers `Closed(v)`) and
  `head = h` with acquire. It stores the new element into slot `t mod B` with per-word atomic stores and publishes it
  by `CAS(tail, t, t + 1)` with release, which fails only when a closer has marked the tail since: the push then
  answers `Closed(v)`, and the slot it wrote is never published. If `t - h` was `c`, the ring now shows `c + 1`
  elements, and the producer copies slot `h mod B` and attempts `CAS(head, h, h + 1)`. On success it owns the oldest
  and returns it as `Evicted`, counting it in `lost`; on failure a consumer took that element, the ring holds at most
  `c`, and the push answers `Pushed`. A queue that is not closable publishes by the same compare-and-swap, which
  then cannot fail.
- **Why the new element is published before the oldest is claimed.** The first draft claimed the oldest and then
  wrote the new element into the slot it left, at `c = B`. Between the two steps the ring shows `c - 1` elements while
  the abstract queue holds `c` before the push and `c` after it, and a pop in that window can answer `Empty`: at
  `c = 1`, a push into a ring that holds `a` claims `a`, a pop finds `head = tail` and answers `Empty`, the push
  publishes `v` and returns `Evicted(a)`. Ordered before that pop the push leaves `[v]` and the pop must take `v`;
  ordered after, the pop must take `a`; neither answers `Empty`, so that history is not linearizable, and a row that
  excused it would be the Vyukov list's `EmptyPops::WhileAPushIsInFlight`, which this row has no reason to need.
  Publishing first leaves the window with `c + 1` elements shown, all of them abstractly present: a pop in it takes
  the oldest, which is the element the push would evict, so the push's claim then fails and the history is the pop
  followed by an ordinary push. The price is one slot: the new element's slot must be free while the oldest is still
  held, which is `c < B`, the ring with one spare slot of the textbook.
- **Why a copy whose claim succeeds is exact.** `head` only grows. The producer writes slot `t mod B` only when
  `t - h' <= c < B` for the `h'` it read, so it never writes a slot of an element whose index is `head` or above: an
  element at index `x >= head >= h'` has `x < t < h' + B` and so another slot. A successful `CAS(head, h, h + 1)`
  shows `head` was `h` from the copier's load to its claim, so nothing wrote slot `h mod B` in between; a failed one
  is the only way a torn copy can arise, and it is discarded. The argument uses `c < B` and nothing else about `B`.
- **Ordering.** Every claim is a compare-and-swap with acquire-release, and the producer reads `head` with acquire
  before it decides the ring is full or writes a slot, so a remover's copy that its claim made the element happens
  before any later store into that slot. The publish is a release, so a consumer that acquires the tail sees the
  slot's words. A copy whose claim fails is ordered against nothing, which is why it is made of per-word atomic loads
  and never used.
- **Close.** `close` sets the tail's top bit by `fetch_or` from any context, as crossbeam-channel's `disconnect`
  does. Its linearization point is that update. The producer's publish meets it in the same word: a push either
  published before the mark, and is ordered before the close, or fails its publish and answers `Closed(v)` having
  evicted nothing, since the publish comes before the claim. A pop that answered `Closed` saw the mark and nothing
  between `head` and the tail, and no later publish can succeed. The mark costs the counters one bit: `2^63` laps.
- **Progress, declared and checked.** The producer is **wait-free**: its publish fails at most once (a close is the
  only other write to `tail`, and it is sticky), it attempts one claim, and after a failed claim the ring holds at
  most `c`. Removers are **lock-free**: a claim fails only because another party claimed that element. A document
  that requires more of a side is refused by the existing `queue/progress-unreachable`.
- **Width.** `head` and `tail` are lap counters; the existing wrap analysis (ADR 0062 §7, the `min_wrap_ops`
  check) applies to them unchanged, and a target without 64-bit compare-and-swap takes the narrow rows' treatment
  ADR 0062 records for the log.

#### The locked ring: any producers and consumers, any policy

For several producers under `evict-oldest`, and for a document whose declared fullness only a lock can give it, the
row is D1's table run as written under one lock over a ring of `B >= c` slots: a start index, a length, `lost` and
`closed` as plain fields, an eviction a move of the start (D4 gives the same lowering to a list).

- **The lock.** Rust: `std::sync::Mutex` under `std`, the `critical-section` crate without it; C11: the deployment's
  `_sce_critical_enter` and `_sce_critical_exit` seams, declared exactly when such a queue is generated, as ADR 0111
  D4 declares them for a `shared` pool (a mutex on a hosted deployment, interrupts masked on a single core); C++,
  Go, Kotlin and Swift: the language's mutex; Python: the lock every Python row already takes.
- **Nothing but the table runs under it.** Every outcome hands its value back (D1), so the critical section is a
  copy of one element and a few words, and no caller code runs inside it: the release function that re-enters the
  library, the self-deadlock the consumer's port documents, cannot occur. Close and drain are atomic by construction.
- **Progress** is `blocking` on both sides: a holder preempted inside the section stops the others. It is the
  bounded buffer of Hoare's monitor, and the consumer's channels are this row today.
- **Why a lock and not a lock-free algorithm for several producers.** With several producers a tail is claimed and
  published in two steps, and a remover that meets a claimed, unpublished slot either waits for a producer that may
  be stopped or skips it and breaks FIFO order. The textbook algorithm that does not skip, Vyukov's bounded MPMC
  queue as crossbeam's `force_push` runs it, therefore has `blocking` progress too (Alternatives rejected, 7).

#### The sequential ring: `<sce:sharing>owner</sce:sharing>`

When every producer and every consumer is one context, the row is D1's table with no atomic access and no fence: a
start index, a length, `lost` and `closed` as plain fields over `B >= c` slots. It serves every policy, close and
drain, at exact fullness, and both sides are trivially wait-free. This is the consumer's no-heap single-threaded
edition, whose ring takes no lock.

- Rust: the queue is `!Sync` (its fields are `Cell`s, as ADR 0111 D4's `owner` book), so the compiler keeps every
  handle on one thread.
- C11 and the languages that cannot say so in a type: the deploy placement of the queue names one context, and an
  `owner` queue placed in two is refused (`queue/owner-queue-in-two-contexts`). An interrupt handler is a context of
  its own, so a queue an interrupt handler pushes into and a thread removes from is `shared`.

#### A core without compare-and-swap (ARMv6-M)

Which party owns the oldest element, the producer evicting it or a consumer removing it, is consensus between two
parties, and loads and stores cannot solve it wait-free (Herlihy 1991); ARMv6-M has no compare-and-swap and no
load-linked/store-conditional to solve it with. On a single core the textbook mutual exclusion is a masked
interrupt. So an `evict-oldest` queue on such a core is either the sequential ring, where its producer and consumer
are one context and nothing needs excluding, or the locked ring with interrupts masked as its lock, which is the
RFC's rule for every row on such a target (`sce_irq_save` and `sce_irq_restore`, `platform.core_count` 1, and
`queue/no-atomics-across-cores` for more cores). The claimed-head ring is not emulated there: under the masked section
its claims would all succeed, and the locked ring is that ring with nothing left to claim.

#### Interrupt handlers

The RFC's placement rule decides, unchanged: an operation in an interrupt handler must be `lock-free` or better, or
run inside the masked section of a single-core target without atomics (`queue/progress-insufficient-for-isr`).

- The claimed-head ring: its producer is wait-free and its removers lock-free, so either side may be an interrupt
  handler; so may a closer, whose close is one `fetch_or`.
- The locked ring: refused in an interrupt handler where its lock is a mutex, since the handler would wait for the
  thread it preempted; accepted where its lock is the masked section of a single core.
- The sequential ring: an interrupt handler and anything else are two contexts, so the queue is not `owner`.
- The waiting below: an interrupt handler notifies and never waits, as ADR 0111 D5 lets it post and never ask.

#### Storage the caller lends

A queue may hold its own `B` slots, as today, or be made over slots its caller lends, sized at run time. The
contract is the one ADR 0111 D4 gave a pool (`Pool::over`) and the one ADR 0154 (in draft) gives a map over lent
storage; it is stated here in full, under the name **Lent**, so that it stands whatever that draft becomes:

- **The count is a run-time value under the bound.** The queue is constructed with `1 <= c <= CAPACITY` (ADR 0111 D4).
- **The caller holds the storage, and its length is checked once.** `Queue::over(slots, c)` (C11:
  `<queue>_init_over(q, slots, n, c)`) takes a span of the row's slot type; the row exports
  `slots_for(c)` (C11: `<queue>_slots_for(c)`), which is `c` for the sequential, locked and Lamport rings, `c + 1`
  for the claimed-head ring and SCQ's power of two for SCQ. A span shorter than that, or misaligned for the slot type,
  is refused at construction (`Err`; `false`), and the queue never grows into more.
- **An empty queue is all zero bytes.** Its indices, its mark, `lost` and its start are zero, and its slots are never
  read before they are written, so they need no initialisation: the constructor is a `const fn` (C11: a static
  initializer) and a `static` queue lies in `.bss`, as ADR 0108 made an empty map.
- **The span outlives the queue.** Rust holds it as a borrow for the queue's lifetime; C11 takes the caller's word
  that the span stays until the drain.

What the rows prove needs nothing more of `B` than `c < B` (the claimed-head ring, above) or `c <= B` (the others),
for `B` the span's slot count; `B = CAPACITY` appears in no argument. So a C ABI that gives each channel the capacity
its caller asked for lends exactly `slots_for(c)` slots, as the consumer's no-heap edition allocates exactly `c`
today, and no channel reserves `CAPACITY` slots it was not asked for.

#### Waiting: a queue that never waits, paired with a party that does

No queue operation waits (the RFC: "There is no blocking pop"). A consumer that blocks until an element arrives, and
a producer that blocks until there is room (the multi-threaded fifo's push: a refused push plus a wait), pair the
queue with a waiting object by one order, the eventcount's (Reed and Kanodia 1979), which is the order
crossbeam-channel's bounded flavour keeps:

1. The waiter announces itself: it reads the waiting object's count and registers as a waiter, then issues a full
   fence.
2. It tries the queue again. If the pop answers `Popped` or `Closed` (or the push anything but `Full`), it withdraws
   and returns that answer.
3. Otherwise it waits while the count is still the one it read, with a deadline, answering `timed-out` as ADR 0111
   D5's `Completion::wait(deadline)` does, and on waking goes back to step 2.

The other side, after an operation that can end a wait, issues a full fence and then notifies when a waiter is
registered: one waiter after a push that answered `Pushed` or `Evicted`, one waiting producer after a pop that
answered `Popped`, and every waiter after a `close`.

- **No wake-up is lost.** The waiter writes its registration and then reads the queue; the notifier writes the
  queue and then reads the registrations; with a full fence between each write and read, at least one of them sees
  the other's write (the store-buffering outcome is forbidden), so either the waiter's re-check finds the element or
  the notifier finds the waiter. The count closes the remaining gap, a notification between step 2 and step 3: it
  changes the count, so the wait in step 3 returns at once. Removing step 2 is a mutation case (D5).
- **What it costs a push.** A fence and one load when nobody waits; a notification only when somebody does. The
  consumer's channels notify on every push today, which this order keeps.
- **Where it lives.** Under `std` the runtime gives an `EventCount` beside the queue, and a blocking receive is the
  three steps above around `try_pop`. Without `std`, and in C11, the notification calls the waker the host installs,
  as a post wakes the owner through the inbox's waker (ADR 0084 D2), and the host's wait follows the same three
  steps. This is the order ADR 0111's door is waited on by, so a host that waits on a door and on a queue waits one
  way. The locked ring may use its monitor's condition variables under its own lock instead, which keeps the same
  order by holding the lock across steps 1 to 3.

### D4. A statechart's append declares its full policy, may take a run-time limit, and may write its outcome

```xml
<sce:append target="held" expr="frame" on-full="evict-oldest" limit="heldMax" outcome="appendResult"/>
```

- `on-full` is `refuse` (the default), `drop-newest` or `evict-oldest`, per statement: a statechart's list is a
  sequential value the machine alone writes, so the policy is a property of the write, not of an object that a
  selection table must pick an algorithm for. `target` is a place (ADR 0108 D12), so a list held at a map's key is
  appended to under a policy too (`map<K, list<E>[N]>`, the ring of census rows S8 #59 and S11 #29).
- **`limit`, when written, is the run-time capacity of this append.** It is an integer expression, evaluated once
  before the append, and the list is full for this statement when `len(target) >= limit`; without it the list is full
  at its bound `N`, as today. A limit outside `1 <= limit <= N` appends nothing and raises `error.execution`, ending
  the block, as an index outside a list does, whatever `outcome` says. Where the list holds more than the limit (the
  limit fell since earlier appends), `refuse` and `drop-newest` treat it as full, and `evict-oldest` removes the
  `len(target) - limit + 1` oldest before it appends, so the list then holds exactly the limit; its outcome is
  `evicted`, and a document that counts each loss reads `len` first. This is ADR 0108 D4's form, "build bound N,
  run-time limit L <= N", with L a value of the machine: S11 #31 holds its `max(1, depth / 2)` in a variable it
  computes when the depth is delivered, and names that variable as the limit. It does not repeat what ADR 0108's
  Alternatives rejected, 1 refused, a limit held on the map that hides the decision: here the limit stands on the
  statement beside the policy that says what reaching it does, so the decision is in the document's text, and the
  bound stays the type's.
- `outcome`, when written, names a variable of the new standard enum `sce:std/collection/append_outcome.scxml`
  (`appended`, `full`, `dropped`, `evicted`), imported as any enum is, exactly as `<sce:put>`'s outcome is
  (ADR 0108). The statement writes what it did there and raises nothing.
- Without `outcome`: under `refuse` a full list appends nothing and raises `error.execution`, ending the block
  (§scxml-4.9), as today; under `drop-newest` and `evict-oldest` nothing is raised, since the document asked for
  the loss. A machine that counts its losses (S11 #31's "counted") reads the outcome and counts in its own variable;
  a statechart's list keeps no `lost` of its own.
- The evicted element is not handed back: a statechart's values own nothing, and a document that wants it reads
  `held[0]` before an append it knows will evict.
- **The lowering rotates; it never shifts.** A list that some `<sce:append on-full="evict-oldest">` writes (a
  variable, or the list type at the place a `target` names, a map's value type included) is held as a ring: its `N`
  slots, a start index and its length. An eviction moves the start by one, modulo `N`, and the new element is written
  once, into the slot the evicted one left; no other element moves, so an append costs one element's copy whatever
  the list holds, where a shift would copy `c - 1` elements of a frame's size on every eviction. `held[i]` reads slot
  `(start + i) mod N`, and `<foreach>`, the snapshot a host reads and the saved form read the elements in order, in at
  most two runs. The saved form, and every value a document or a host reads, are unchanged. A list no append evicts
  from keeps its present storage (ADR 0141's raw parts), so nothing else pays for the start index. The Interpreter,
  which writes a list again on every append, is the reference for the outcomes and not for the cost.
- **One write per element, the copy the append already makes.** The acceptance document has a list hold a copy of a
  record appended by name, as it stands then (its "A list of records"), so a `bytes[N]` field is carried into the
  list's `SceBytes<N>`: the constraint the consumer's item 4a(i) names. That copy is the append's one write, into
  the slot the rotation chose; the eviction adds none, since it moves the start and not an element. So the two
  compose: an append that evicts copies one record, as an append that does not.
- `on-full` other than the three, `outcome` naming no variable of that enum, `limit` of a type that is not an
  integer, and any of the three on anything but `<sce:append>` are `scxml/static-datamodel-rule`, each naming what is
  accepted.
- Every lane that lowers `<sce:append>` lowers the three attributes. The saved form of a list does not change.

### D5. The verification, by layer (the structure ADR 0062 §6 gives the log)

1. **The specification and its checker.** `queue_history.rs` gains the policy, the operations `close` and `drain`,
   and the outcomes `Dropped(v)`, `Evicted(v)`, `Closed(v)`, `Closed`, `Done` and `Drained(s)`; a declared fullness
   maps to its `Refusal` (`exact` to `AtCapacity`). Its search (Wing and Gong with Lowe's memo) judges a history
   against D1's table, and value identity makes Conservation part of what it checks: a value popped twice, popped and
   handed back, drained and popped, or handed back by an eviction that the sequence could not have made, is a
   violation. Counterexample histories for each are kept as tests that must fail, with three more: a pop that
   answered `Closed` followed by a `Popped`; a push answered `Pushed` that began after a close returned; and the
   first draft's window, a pop answering `Empty` while an evicting push at `c = 1` is in flight, under
   `EmptyPops::Exact`.
2. **A TLA+ model with a refinement mapping** (`docs/formal/ClaimedHeadQueue.tla`): the ring of D3, each atomic
   access a step, refines the sequence of D1, close included, under `s = slots[head .. tail)` and `closed = the
   tail's mark`, for one producer, two consumers and a closer, at `c = B - 1` and `c < B - 1`. It must say no when
   the claim is made after the copy is used, when the oldest is claimed before the new element is published (the
   first draft's order, at `c = B`), and when the close is a flag beside the tail. The locked and sequential rings
   are D1's table itself and need no refinement of their own; the history checker judges them.
3. **The memory-model checkers**: `loom` for Rust and GenMC for C and C++ on the `genmc-queue` harness, with an
   eviction racing a removal at `c = B - 1` and a close racing a publish; `loom` also on the waiting order of D3,
   where a lost wake-up is a waiter that never returns; Lincheck for Kotlin; the race detector for Go.
4. **The history checker**, fed by every backend's stress run in the one JSON form.
5. **Mutation cases** in `sce-build/tests/mutations/`: the claim replaced by a store; the copy taken after the claim;
   the tail published before the slot; the oldest claimed before the publish; `Evicted` returning the new element;
   `lost` counted before a failed claim; the publish made a plain store on a closable queue; the waiter's re-check
   (step 2) removed; a ring list's eviction made a shift rather than a move of the start (caught by a test that counts
   element writes, since the outcomes do not change).
6. **A reference that is not ours**: the channel handler of the consumer's C ABI in its no-heap MCU edition, as a
   differential oracle for the outcomes of the same push, pop and close sequences, in the manner of
   `tools/reference-oracles/`. It releases an evicted element after its lock is dropped, so its eviction is D1's
   hand-back one for one: the element leaves the queue as a value and its caller releases it. The heap edition's RING
   handler releases inside its lock; its outcomes agree for a release function that neither re-enters the channel nor
   waits, but there the release is part of the critical section, a concurrent receive waits for it, and a release
   that re-enters deadlocks, so it is not the oracle.

### D6. What stays refused, and when each is revisited

- `evict-oldest` with several producers at `lock-free` or better: no textbook algorithm reaches it without breaking
  FIFO order (D3). Revisited when one does.
- `evict-oldest` on `<sce:segmented>`: an LSCQ's oldest element sits in the oldest ring of a list whose rings are
  retired by their consumers; claiming it from the producer side is a second algorithm. Revisited when a consumer needs
  an evicting queue larger than one ring.
- `<sce:closable/>` on `<sce:intrusive>`: revisited when a consumer needs a closable queue whose nodes it owns.
- A waiting producer as a queue operation: the waiting is the host's, by D3's order.
- A consumed queue that also reports to each consumer how much it missed (`Lagged`): that is the log's contract, and
  the consumer confirmed it needs none.

## Consequences

- The three census rows of U17 have a home: S7 #61 and S8 #38 a `queue` with `evict-oldest` (or `drop-newest` at
  `exact` fullness), closable, drained at teardown, over slots the C ABI lends for the capacity its caller asked for;
  the claimed-head ring where every push is made by the drive's holder, the locked ring where the caller's own thread
  pushes beside the drive thread, the sequential ring in the single-threaded edition. S11 #31 is an
  `<sce:append on-full="evict-oldest" limit="...">` with its outcome counted. Together with ADR 0111 D4's run-time
  capacity, S7 #61 is no longer blocked by SCE, whichever answer U18 takes.
- Nothing changes for the consumer's census beyond these rows: no row needs a count of what the removing side
  missed, and `lost` is what it reads.
- Every existing `queue` document is unchanged: same rows, same API, same histories. `fullness`, `<sce:closable/>`
  and `<sce:sharing>` are absent from every document that exists, and their defaults are today's behaviour.
- The history checker's specification gains four outcomes, two operations and a fullness; every existing recorded
  history stays valid, since a `refuse` queue that is not closable produces none of them.
- Seven backend rows and their gates grow by three algorithms (the claimed-head, locked and sequential rings), a
  close on the rows that serve it, a drain, a lent construction and an eventcount; the acceptance document gains three
  codes and one standard enum.

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
   `len`, `<foreach>`, its saved form) for one difference in one statement. The ring is the lowering's choice of
   storage for a list that some append evicts from (D4), not a type a document writes.
7. **Crossbeam's `force_push` for several producers.** Its progress is `blocking`, the locked ring's, and it needs a
   stamp per slot and a second mark for a close, while the locked ring is D1's table as its code, makes close and
   drain atomic by construction, and waits by a monitor. At equal progress the simpler structure is the textbook one.
8. **Claiming the oldest before writing the new element, at `c = B`** (the first draft's D3). Not linearizable: a pop
   in the window answers `Empty` (D3). The spare slot is the price of the order that is.
9. **A closed flag beside the queue.** A push that reads it and then publishes can succeed after a close returned, and
   a pop can answer `Closed` and then deliver (D1).
10. **Shifting a list on eviction.** `c - 1` element copies per append on a full list, of a frame's size where the
    element is a frame, for a result a moved start gives in one step (D4).
11. **One queue row of `B = CAPACITY` slots for every channel.** Every channel would reserve the bound whatever
    capacity its caller asked for; no argument of D3 needs it (D3, "Storage the caller lends").

## Order of work

1. This record: the consumer's review (taken, section "Review"), then the owner's acceptance.
2. D1 in `queue_history.rs`: the policy, the fullness, close and drain, the outcomes, Conservation, the
   counterexample histories.
3. D4: the schema, the checker, `append_outcome.scxml`, `limit`, the ring lowering on every lane, scenarios shaped
   like S11 #31 (a held send queue that evicts at a run-time limit and counts) and like S8 #59 (a ring held at a map's
   key).
4. D2: the grammar, the selection, the three codes with their acceptance rows, `drop-newest` and close on the
   existing rows, drain everywhere, lent construction.
5. D3: the TLA+ model first, then the Rust and C11 rows (the consumer's lanes) with `loom`, GenMC, the stress
   histories and the mutation cases, in the order the consumer's editions need them: the locked ring, the sequential
   ring, the claimed-head ring, then the waiting order; then C++, Go, Kotlin, Python and Swift.
6. D5 item 6: the differential run against the no-heap edition's channel.

## Open questions

None for the owner. Two needs of the consumer lie on the same axis and outside U17, each left to a record of its own
when the consumer asks for it:

- a keep-latest slot on its offload path (S8 #65), which may prove to be a queue of capacity one under
  `evict-oldest`; that is read when it is asked for;
- a `drop-newest` queue that also feeds an observer which does not consume (S3 #42), which is a log's reader beside
  a queue's consumer and so two kinds, not one.

## Review

The consumer's review of 2026-11-28, item by item, and where each is taken:

| Item | Request | Where |
|---|---|---|
| 1 | Several producers: the C ABI's own deliveries run on the calling thread beside the drive thread; state the dependence on ADR 0111 D5; give several producers under `evict-oldest` a `blocking` row; withdraw "no consumer has" | Context, the census table and "How many producers"; D2's table; D3 "The locked ring"; D6; Alternatives rejected, 7 |
| 2 | A close in D1: `Closed(v)` on push, `Empty` told from `Closed` on pop, its atomicity inside the queue's state, in the model and the checker | D1 (table, "A close is part of the queue's state"); D2 `<sce:closable/>`; D3 "Close"; D5 items 1 to 3 and 5; Alternatives rejected, 9 |
| 3 | A drain that hands back what is left at teardown, so Conservation closes in C11 | D1 (table, "A drain is the teardown"), Conservation in Context |
| 4 | Storage the caller gives, sized by its `c`, under 0154's Lent contract | D3 "Storage the caller lends", with the contract stated; the claimed-head ring needs `c + 1` slots (the note below the table) |
| 5 | A sequential row for one context, and the row of a core without compare-and-swap | D2 `<sce:sharing>`; D3 "The sequential ring" and "A core without compare-and-swap" |
| 6 | Exact fullness for `drop-newest` | D2 `fullness` and the table's Fullness column |
| 7 | `on-full` at a run-time capacity the document holds | D4 `limit` |
| 8 | Eviction as an O(1) rotation, one write per element, and its relation to 4a(i) | D4 "The lowering rotates" and "One write per element"; Alternatives rejected, 10 |
| 9 | A waking order between a blocking wait and a lock-free queue | D3 "Waiting" |
| 10 | The no-heap edition's handler as the oracle, and how the heap edition differs | D5 item 6 |
| 11 | Whether D3's producer is safe in an interrupt handler | D3 "Interrupt handlers" |
| 12 | No row reports a miss count to the consumer; `lost` suffices | Context; D1 `lost`; D6; Consequences |

Reading item 4 against D3 found that the first draft's order of the claimed-head ring was not linearizable at
`c = B` (D3, "Why the new element is published before the oldest is claimed"). The corrected order needs `c < B`, so
the ring the consumer lends to that row is `c + 1` slots, and the sequential and locked rings, which its editions use
today, take exactly `c`.
