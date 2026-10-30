# ADR 0075 — An engine keeps a delayed-send queue and a donedata slot only for a document that fills them

- Status: Accepted (2026-10-31), on the owner's instruction to optimise the footprint structurally
  until no optimisation point is left, and a consumer's request (watching-zenoh).
- Amends: ADR 0069 (the delayed-send queue holds event numbers), ADR 0070 (an engine's empty state
  is a constant).
- Evidence: watching-zenoh's T2G client image (thumbv7em, `opt-level = "s"`, `-Zprint-type-sizes`):
  `Engine<SessionFsmUnicastPolicy>` 1,472 bytes and `Engine<PeerConnectionPolicy>` 968, of which the
  delayed-send queue is 528 and the donedata slot 260 in each; neither document has a delayed send,
  a `<cancel>` or a `<donedata>`. And `backends/rust/probes/nostd-footprint`.

## Context

Every `Engine<P>` held a `PullScheduler<P::ScheduledSendId>` (§scxml-6.2), which under `no_std` is
a ring of `MAX_SCHEDULED_EVENTS` = 32 entries, and a `SceString` for the donedata a top-level
`<final>` hands its parent (§scxml-5.5, 6.3.1), a `heapless::String` of 256 bytes. A document that
has no delayed send and no `<cancel>` never puts anything in the first, and one with no `<donedata>`
never writes the second, so on a target without an allocator each machine of such a document
carried 788 bytes of capacity it could not use, and the code that drains the queue in `tick`.

Which of the two a document needs is known when it is generated: the analyzer already decides
`needs_event_scheduler` from `delay`, `delayexpr`, `<cancel>` and a host invoke's deadline, and a
top-level `<final>` either carries `<donedata>` or does not.

## Decision

1. `StatePolicy` names both: `type Scheduler: SchedulerStorage` and `type DoneData:
   DoneDataStorage`. `ScheduledSendId` stays, and names the send-id storage of the queue a policy
   keeps.
2. `SchedulerStorage` is implemented by `PullScheduler<S>`, unchanged, and by the zero-size
   `NoDelayedSends`. Its operations are a sealed trait (the queue's no_std and std drains differ,
   ADR 0053), so no implementation outside the runtime depends on them.
   - `NoDelayedSends` answers every query with "nothing pending", so `tick`'s drain ends at once.
   - Arming a delayed send on it stops the program, as a full queue does: the generator never
     emits an arming call for a document that has none, and dropping one would lose a
     transition silently.
   - A restore that lists a waiting send or a deadline for such a machine is refused, not run.
   - `PullScheduler`'s pop and ready query are out of line, so the machines that keep a queue of
     one send-id type share one copy; reached through the trait, they were otherwise inlined into
     each machine's `tick`.
3. `DoneDataStorage` is implemented by `SceString` and by the zero-size `NoDoneData`, which reads
   as the empty text and stops the program if a stash reaches it.
4. The generator writes `PullScheduler<Self::ScheduledSendId>` when `needs_event_scheduler` holds
   and `NoDelayedSends` otherwise, and `SceString` when a top-level `<final>` carries `<donedata>`
   and `NoDoneData` otherwise, on both profiles. Both start from their type's `EMPTY`, so
   `Engine::new_const` stays a constant expression (ADR 0070).

## Consequences

- A machine pays for a delayed-send queue and a donedata slot only when its document can fill them:
  788 bytes of RAM per engine on a target without an allocator, by the consumer's type sizes.
- A host that arms a delayed send by hand (`Engine::schedule_event`) on a generated machine whose
  document has none now stops; such a host names `PullScheduler` in its own policy.
- Every hand-written `StatePolicy` names the two types (Rust has no associated-type defaults).

## Measured (2026-10-31)

`thumbv7em-none-eabihf`, `opt-level = "s"`, the probe's text figures
(`backends/rust/probes/nostd-footprint/footprint.budget`). The probe's first machine has no
delayed send; its second, and the plain configuration's first, keep a queue.

| | one more machine | shared core | two-machine image | worst-case stack |
|---|---|---|---|---|
| diagnostics on | 1,516 -> 962 B | 7,524 -> 7,870 B | 195,355 -> 194,825 B | 952 -> 952 B |
| diagnostics off | 1,468 -> 954 B | 7,416 -> 7,762 B | 195,127 -> 194,639 B | 952 -> 952 B |
| two machines without `<history>` or `<parallel>` | 1,290 -> 1,008 B | 5,496 -> 5,496 B | 192,006 -> 191,780 B | 760 -> 760 B |

The core grew because the queue's pop and query are now counted there as one shared copy.
