# ADR 0113 — A machine of a deployment may run as an open set of instances, and a server binds each as it arrives

- Status: Accepted (2026-11-22, by the owner). Its model (D8) was written and checked the same day, before any code, and
  amended D4 and D5. What TLC found and what was seen while writing the model are told apart in D8.
- Date: 2026-11-22
- Written under the owner's standing rule (the most long-term correct structure, textbook in its foundations, cost
  ignored) and the owner's scope rule of 2026-11-22: a new facility is built only on the backends a consumer needs,
  Rust by default; its contract is general; what is special to one application stays in the application.
- Amends, once accepted (each amended document gains an "Amended by: ADR 0113" line in the landing that moves it):
  `SCE_MESH.md` §10.5, §10.7, §10.9 (a new invariant 11), §10.11, §14, §14.4, §19, §22.1, §22.3 to §22.5; ADR 0090 (one
  more shape of origin); `docs/SCE_ACCEPTED_SUBSET.md` §2.8 and the appendix; `docs/SCE_SERVER_DEPLOY.md` sections 0
  to 2; `docs/SCE_CODEC_CONSUMER_MIGRATION.md` (the envelope's new key).
- Scope: the contract names no language. Built first: `sce-build` and the Rust host core `sce-rust-mesh`, with wasm32 as
  the Rust lane. Every other backend refuses by name (D7).
- Related: ADR 0090, ADR 0107 (D2, D3), ADR 0109 (D1, D3, D9, D10), ADR 0112 (a separate landing).

## Context

A keyed pool's keys are an open set (ADR 0107 D3) and so is the `https` edge (`is_bound` is a closure; `install`,
`bearer` and `key` list no names). The core between them is closed:

- `Machine.peers` is a `&'static [Peer]` from deploy.yaml, and each session's router binds exactly those
  (`backends/rust/mesh/src/pool.rs`, `endpoint_for`). Any other name is `RouterError::UnknownPeer`. Widening
  `is_bound` in a program lets an exchange in that the core then refuses.
- `key_from: client` opens session keys, not peers. A durable outbox and its window are bound per declared client. A
  checkpoint holding state for a peer the router does not bind is refused (`CheckpointError::Unbound`).
- `Pool::peer_ready` tells every session in memory of each edge of each client.

Two foundations decide the structure:

1. **§10.9 says what a name is.** `source` is the machine's document name, the deploy.yaml key, the axis an author
   reasons about. Invariant 5 keys sender state on it and records the gate: a second process hosting the same machine
   name needs a second keying axis, "recorded as a new invariant under that gate". An open set of instances is that
   gate. The first draft wrote the instance into `source` as `phone~id`; it changed the meaning of the axis, made
   authors build strings, and gave one name two readings. It is withdrawn. `routing_id` cannot be the axis either: it is
   drawn at each construction and changes at every restart, while an instance outlives its process.
2. **§10.5's window is bounded by count** (256 ids by default; "a request repeated after its id left the window is taken
   as a new one"). So keeping an entry for a long time does not make effects happen at most once, and the first draft's
   `forget_ms` was neither safe nor bounded: it also let anyone fill a session's entries for the whole period.

## The textbook structure

- **A name and what it is bound to are separate** (Saltzer, RFC 1498), and the binding is resolved at one place.
- **At most once needs an identity for the operation, not a window of time**: the operation is (client, sequence), the
  receiver keeps the result until the client acknowledges it, and the client's state may be dropped only when the
  client can no longer ask (Birrell and Nelson 1984; Lee et al., RIFL, SOSP 2015: a completion record beside the
  object).
- **State kept for another is leased** (Gray and Cheriton 1989); RIFL drops its records by a client's lease and
  acknowledgements. An expired lease is told, never silently renewed as a new client. (Liskov, Shrira and Wroclawski
  1990, MIT/LCS/TR-476, refuse a duplicate though they keep nothing of its sender: what is dropped from the table raises
  one bound, and a message not later than the bound is refused. The durable mark of this record is that bound, over
  counted numbers. Their numbers are clock readings, whose skew costs performance and never correctness; it is not the
  reason they are not used. A client whose clock runs behind is refused until it is set right, and a reply is dropped by
  its age, so a client that asks again after that is ignored and never learns the outcome. Here it is told.)
- **A kind and its instances are addressed separately** (Bernstein et al., Orleans, 2014: type and key).
- **The end decides** (Saltzer, Reed and Clark 1984): the application, not the transport, decides what to do with an
  operation whose outcome is unknown.

## Decision

### D1. An instance is the second axis of a sender's identity

A machine entry may carry `instances: open`, written once, on the machine that is many, so the two sides of a binding
cannot disagree. Its meaning: any number of copies run, each an instance. The identity of a sender is the pair
**(machine, instance)**. `source` stays the machine's name, the axis authors reason about; the instance is an optional
key of the envelope of its own (the next free key, taken in step 2), 1 to 64 characters of `A-Z a-z 0-9 . _ -`, absent
for a machine that is not open.

**Invariant 11 of §10.9**: on a binding to an open machine every container of sender state keys on the pair, and the
pair is held to the link as `source` is by invariant 10: what the link's authenticator proved, not what the envelope
says. The instance id is opaque. It is not a key and not a person: a credential rotates under it, and who it acts for is
another notion, left to its own record.

### D2. A document addresses an instance by its origin

`_event.origin` of an instance's event is `mesh://<machine>/<instance>`, each segment percent-encoded: the two-segment
shape, beside `mesh://<machine>` and the session endpoint's three segments (§9.6.2). A document replies to it with
`<send targetexpr="_event.origin">`, as W3C SCXML 5.10.1 and ADR 0090 have it, so no author concatenates a name. It is
text the document may keep and send to later. A literal `target="#phone"` to an open machine names no one and is refused
when the machine is generated. The name the binding's path carries is `<base>/sce-mesh/1/<machine>/<instance>`.

### D3. One lookup binds

The router resolves (machine, instance) at one place: a declared peer; else an instance of an open machine under that
machine's one `PeerConfig`; else `UnknownPeer`. Receive, send, readiness and restore use it, so no second rule exists.
Its responder is the instance itself, a case of `PeerConfig` and not a static list. `reply_from:` beyond the target
stays refused.

### D4. Membership is offered, confirmed, leased and fenced

- **A join offers.** An exchange that names no incarnation carries no envelope. The server answers it with an
  incarnation it draws (`ETag`) and takes nothing else in. The join carries a nonce (`Sce-Mesh-Nonce`), and a join
  repeated while its offer stands is offered the same incarnation again, so an answer lost and asked for again changes
  nothing.
- **An exchange confirms.** The first exchange that names the offer in `If-Match`, an empty poll included, makes the
  instance a member under it, in the store, in one write, and ends the incarnation it had before. A join that arrives
  late only offers, so it puts no live member out (the three-way handshake, RFC 9293 §3.5: an old duplicate opening must
  not be taken for a new one).
- **A member is leased.** Each authenticated exchange renews its lease (`lease_ms`); an offer and a member whose
  messages no session kept live only `provisional_ms` (ADR 0109 D1, "a session is kept only once its document keeps
  it"). A name that arrives once and goes away costs that, which the rate limits of §22.4 bound.
- **An incarnation the server does not hold is fenced.** The exchange is answered **`412`** (RFC 9110 §15.5.13: the
  condition of `If-Match` is false) and takes nothing in. The client's core ends each request still waiting as *outcome
  unknown* (a row of §16.7 of its own), reports every envelope it had not seen acknowledged, and joins anew; the
  document decides whether to ask again (the end-to-end argument). `410` says a resource is gone for good and is
  cacheable by default; the resource here is named again by the next join, and `412` is what a false precondition
  answers.
- **What is forgotten together is dropped together**: a member's records, window and outbox end with its lease, an
  outbox that still owes what `retain_ms` keeps ending first (ADR 0109 D10).

`lease_ms`, `provisional_ms` and `max_instances` have **no default** and are written by the deployment, as
`lease_ttl_ms` and `renew_every_ms` have none: no number in this record is guessed.

### D5. At most once, by completion records and a low-water mark

One instance's requests go to several sessions (`key_from: data.<field>`), so no session sees its numbers one after
another and a single mark cannot tell a repeat (the second draft had one). The form is RIFL's:

- The instance numbers its requests within its incarnation and sends with each exchange its **low-water mark**, the
  lowest number whose answer it has not.
- The session that runs a request writes a **completion record** (incarnation, number, the answer) in the write that
  keeps its effect; a repeat is answered from the record.
- The edge admits a request only under the current incarnation and at or above the mark. The session judges both
  **again as it runs**, since the mark may have moved while the request waited in its inbox.
- A record is dropped below the mark **the store holds**, not below the one the process has seen: a crash takes the
  latter back, and a repeat admitted under the earlier mark would run again.
- What a session holds for a member is therefore at most the requests the member has in flight (its `window`, §10.11).

This is a `durable` receipt's guarantee. A `transport` receipt keeps §10.5's window as today, which a repeat older than
the window defeats, and the document says which it has by its choice of receipt.

### D6. Readiness and capacity cost what they concern

The pool keeps the index of members each session knows, and an edge reaches those sessions and no other. A session's
router holds at most `max_instances` members; at the bound the members whose lease ended are dropped first, and
otherwise a new one's envelope is shed with `503` (ADR 0109 D1), never by evicting a member whose state is owed.

### D7. Transports and backends

A transport serves an open machine when its link names its sender (invariant 10) and it keeps a mailbox per sender: a
row of `TransportDescriptor`, so the next transport is a row and an adapter and not a rule. `https` is built, in Rust,
as server and as dialing side (its host gives the instance id, and keeps it). Everything else refuses by name, with this
record's code: the other backends; a transport without the row; `instances:` beside `sessions:`, `server:` or
`invoked_by:`, on a partition, and on a causal binding (whose identity is the replica's, invariant 9). A pool of many
open machines is several masters of one key; it is refused because the lease and fence of §14.4 already choose one.

`Listening` answers `admits(machine, instance)` from the peer table, so a program writes no predicate to widen.

### D8. The protocol is a model before it is code

`docs/formal/OpenInstances.tla`, checked whole by `scripts/formal/check.sh`: a network that loses, repeats, delays and
reorders, a crash of either side, an expiry of a member or an offer. It checks that a request takes effect at most once
(`AtMostOnce`), that an answer is the answer to what was asked and follows an effect (`Answered`), that an instance that
neither crashed nor was expired never loses an outcome whatever the network repeats (`Undisturbed`), that members stay
at their bound (`Bounded`), and that every request ends (`Ends`). TLC, on 2026-11-22 (one instance, two sessions, two
requests): 1,423,556 distinct states, no violation; two instances with room for one: 35,999; liveness, a message
arriving once: 2,162. Both outcomes (answered, unknown) are reached.

Each part is shown to be needed: with one of them taken out, TLC must find a violation, and does.

| Taken out | What TLC finds |
|---|---|
| the handshake (a join makes a member at once) | a repeated join puts a live member out |
| the same offer for a repeated join | a member put out before its first request |
| matching an offer to the join waited on | an answer to another request |
| the fence (`412`) | a request that takes effect twice |
| completion records (a window instead) | a request that takes effect twice |
| the second judgement in the session | a request that takes effect twice |
| the durable mark for dropping records | a request that takes effect twice |
| keeping a live member's records (eviction) | a request that takes effect twice |

**What TLC found** that the second draft did not have: an empty poll under an offer was fenced, and the request after it
ended unknown. TLC also answered a question the draft did not ask: the handshake is not needed for at-most-once (the
protocol without it passes `AtMostOnce` and `Answered`), only for not disturbing a live member, which is why
`Undisturbed` is a property of its own.

**What was seen while writing the model**, and shown necessary by taking it out: a repeated join must be offered the
same incarnation, the second judgement in the session, the durable mark, and matching an offer to its join. That one
mark per member cannot serve several sessions was read from RIFL, not run: no configuration models the single mark.

The model is bounded: it does not cover the outbox direction (ADR 0109 D10), more than two
requests in flight, or the cryptography of an authenticator.

### Not in scope

Subscriptions to an open set (a record of its own, over the durable mailbox), a person or group behind an instance,
discovery, an instance that moves between machines, and every limit of one application.

## Consequences

- A public server needs no list of clients and no predicate in its program.
- A closed deployment's table, wire, checkpoints and cost do not move; the envelope's key is optional.
- The client-visible outcome of a long absence is explicit (`412` and an unknown outcome), never a silent repeat.
- A machine's document can keep an origin and address it later, across a restart of either side, within the lease.

## Alternatives rejected

- **The instance in `source`** (`phone~id`, the first draft). See "Context".
- **`routing_id` as the axis.** It changes at every restart.
- **A time-based `forget_ms`, or an unbounded table.** Neither bounds the state nor keeps effects at most once.
- **`add_peer` called by the pool for any authenticated name.** A policy nothing declares, and an unrestorable session.
- **A wildcard key (`"#phone*"`).** The sides can disagree, and a topology key then reads as a pattern.
- **The credential's thumbprint as the id** (self-certifying names). It ties the identity to a key that must rotate.
- **Evicting a live member at the bound.** It drops what is owed.

## Order of work

Each step lands with a test that fails without it.

1. (2026-11-22) This record, accepted.
2. (2026-11-22) The model of D8, checked, with its failing configurations.
3. Parser, topology, refusals and the envelope key (D1, D2, D7).
4. Router, endpoint, checkpoint (D3, D5, D6) and the store's membership (D4).
5. Pool and outboxes (D4, D6); the `https` server and client (D4, D7). Test in the server suite: 200 instances never
   listed, lost responses, a killed server, an expired lease and the client's unknown outcome.
6. The documents named in "Amends".

## Open questions

Decided with the acceptance, each with its reason.

1. The status of a fenced exchange: `412` with `If-Match` (D4).
2. A host of one endpoint, not a pool, keeps the membership store too: the same contract, so one rule.
