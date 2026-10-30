# ADR 0069 — The delayed-send scheduler holds event numbers

- Status: Accepted (2026-10-30), on the owner's instruction to optimise the footprint structurally
  until no optimisation point is left.
- Amends: ADR 0064 (which kept `PullScheduler<E, S>` generic) and ADR 0068 (whose consequences
  left the scheduler per machine).
- Evidence: `backends/rust/probes/nostd-footprint`, which now drives both of its machines with
  `tick`.

## Context

After ADR 0068 the engine takes events as numbers on its queues. The delayed-send scheduler did
not: `PullScheduler<E, S>` held the machine's event type `E`, and the std `ScheduledAct<E>` did
too. Every machine that is driven with `tick` therefore compiled its own copy of the scheduler's
scans (`next_ready_at_with_event`, `pop_ready_event_at`) and of everything that schedules.

The footprint probe could not show this. Its driver never called `tick`, so no image it weighed
held the scheduler's code at all.

## Decision

### 1. An entry names its event by number

`PullScheduler<S>` and `ScheduledAct` hold the event as the `EventId` the machine's tables give
it. `S`, the storage of the cancel key, stays a parameter. It is what lets a cancel-free machine
keep no send id at all (`ElidedSendId`), and a document's longest id sizes the others
(`SendIdString<N>`).

The engine's typed entry points (`schedule_event`, `schedule_event_as`, `schedule_routed_event`,
the `send_*` doors) keep their signatures and convert with `StatePolicy::event_id`. The
delivery in `tick` converts back with `P::EVENTS`. A saved state names a pending send by the
event's name, as before.

### 2. The drain loop stays in each machine's `tick`

Moving the loop of `tick` into code shared behind a `dyn` hook (the scheduler, the clock, the
delivery and the macrostep as the machine's methods) was built and measured. On the probe's
two plain machines it was 40 bytes larger than leaving the loop in each `tick`: the hooks and
their vtable cost more than the loop they replaced, once the scheduler's own scans were shared.
The loop stays where it is.

### 3. The probe weighs `tick` on every machine

Both of the probe's machines are driven with `tick`, as a host with a clock drives each of its
machines. The core's figure counts `engine::PullScheduler`. It is compiled once for each kind of
send id the program's machines keep. A machine whose kind of send id no other machine shares
inlines its copy into its own `tick`.

## Measured (2026-10-30)

The probe's two machines without a `<history>` or a `<parallel>`, both `ElidedSendId`, both
driven with `tick`:

| | image |
|---|---|
| scheduler generic over the machine's event | 192,730 |
| this ADR | 192,526 |
| this ADR with the drain loop shared behind hooks (not taken) | 192,566 |

The scheduler's scans are one copy of 326 bytes, counted in the core. One machine alone is 20
bytes larger, for the conversions at the scheduler's edge.

The configuration with features does not share: its machines keep `SendIdString<9>` and
`ElidedSendId`. The watching-zenoh documents use no `<send delay>` (their hosts own the
timeouts), so their images do not change.

## Consequences

- A program whose machines keep the same kind of send id carries one scheduler. A program whose
  machines all have no `<cancel>` keeps `ElidedSendId` in every machine, so all of them share it.
- Machines that keep `SendIdString<N>` share only with the same `N`. Making the scheduler's code
  independent of `N` would store the send ids out of line, at a cost in RAM per entry. That is
  not taken.
- `PullScheduler` and `ScheduledAct` lost a type parameter, which breaks a caller that names
  them. Nothing generated names them. The queue-size probe does, and it is updated.
