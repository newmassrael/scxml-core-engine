# ADR 0113 — A machine of a deployment may run as an open set of instances, and a server binds each as it arrives

- Status: Accepted (2026-11-22, by the owner). Its model (D8) was written and checked the same day, before any code, and
  amended D4 and D5. What TLC found and what was seen while writing the model are told apart in D8. Amended 2026-11-22
  by the owner: what is signed (D4), the judgement as a condition of the write (D5), and an offer that waits for a proof
  (D4, ADR 0116). Amended 2026-11-27: a `sce-static` document's route over the instances of an open machine (D2).
  Steps 3 to 6 of the order of work landed (2026-11-22 to 2026-11-28); one refusal of step 3 stays open, stated there.
  An instance's restore from a checkpoint of its own, left open by step 5, was decided on 2026-11-28 and is told there.
- Amended by: ADR 0116 (D1, D6 and D7): who an incarnation acts for, the "for whom" D1 left to a record of its own, is
  a delegation the membership holds from the confirmation that bound it to the incarnation's end, ended by D5's
  condition of the write and the fence as every incarnation is; a fact a session says and an audit entry a run appends
  record the instance and that person (`Fact::actor`, `EngineState::register_invoker_for`).
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

**Amended (2026-11-27), before step 5: a document with no script engine, and a written origin.** A `sce-static`
document holds a computed target to the routes it declares (`sce:targets`, ADR 0005, decision 3), and the instances of
an open machine are a set no document can list. They are one route, `mesh://<machine>/{instance}`: an RFC 6570 URI
template whose one variable expands to an instance id, which holds a value exactly when it is the origin of an
instance of `<machine>`. It is not a wildcard over names (rejected in "Alternatives" for deploy.yaml keys): it is the
shape of the one address an instance has, written where a document declares the addresses it sends to, and an
instance's own origin may also be an entry. `_event.origin` is a string such a machine holds while it processes the
event, so `<send targetexpr="_event.origin" sce:targets="mesh://phone/{instance}">` answers the instance that sent the
request. A written `target="mesh://<machine>/<instance>"` is a Mesh target as `#peer` is, and so is the same `src` of
`<invoke type="sce:mesh-rpc">`; each reaches the open machine's one binding. Only the Rust host core routes an
instance's origin (D7): every other backend refuses, by its name, a document that writes one, declares the route, or
reads `_event.origin` under `sce-static`, and its runtime answers a computed one as a target it cannot address.

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
  it"). An offer that waits for a person's proof (ADR 0116 D2) lives `login_ms` and is confirmed only by the exchange
  that carries the proof. A name that arrives once and goes away costs that, which the rate limits of §22.4 bound.
- **An incarnation the server does not hold is fenced.** The exchange is answered **`412`** (RFC 9110 §15.5.13: the
  condition of `If-Match` is false) and takes nothing in. The client's core ends each request still waiting as *outcome
  unknown* (a row of §16.7 of its own), reports every envelope it had not seen acknowledged, and joins anew; the
  document decides whether to ask again (the end-to-end argument). `410` says a resource is gone for good and is
  cacheable by default; the resource here is named again by the next join, and `412` is what a false precondition
  answers.
- **Only what is signed changes what a party holds.** Under the `key` authenticator (SCE_MESH.md §22.9) every field the
  binding defines and a party acts on is a row of one table: its name, its direction, when it is carried. The signer and
  the verifier build what they cover from that table, and a field is read through it, so one that is not a row cannot be
  read. A request thereby signs `if-match` and the low-water mark of D5, and a response `etag`. RFC 9421 section 7.2.1
  says why: the verifier should trust only what was signed. The rule is of every answer and not of `200` alone: an
  answer that changes what the client holds (`200`, the fence's `412`) is signed over its `@status` and the request it
  answers, and one that is not signed, or is of any other status, is to a client that verifies no more than an exchange
  that failed at the transport. A binding of an open machine under `key` names both `sign:` and `verify:`, and is
  refused where the deployment is built without either. The incarnation is a value the server drew, so a request signed
  ahead of the offer is of no use under it. (Amended by the owner, 2026-11-22, before step 4 is built, which lands
  §22.4's and §22.9's change.)
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
  **again as it runs**, since the mark may have moved while the request waited in its inbox. The last judgement of the
  incarnation is the store's: the write that keeps a run names each incarnation the run was made under (`Write::under`),
  and is refused whole, writing nothing, when the store no longer holds one (`Conflict::Incarnation`). The session is
  then restored from what the store keeps and goes on with what is not fenced. So nothing is kept of a run after its
  incarnation ended, whichever process ended it and whenever the run began.
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
| the signature over a request's incarnation | a request that takes effect twice: one of an ended incarnation, given the next one's |
| the signature over a request's mark | a request that never ends: the mark is raised past it |
| the signature over the incarnation an offer names | an undisturbed instance that loses an outcome |
| the signature over the answer that fences | an undisturbed instance that loses an outcome |
| the refusal of a member past the bound | more members than the bound |

**What TLC found** that the second draft did not have: an empty poll under an offer was fenced, and the request after it
ended unknown. TLC also answered a question the draft did not ask: the handshake is not needed for at-most-once (the
protocol without it passes `AtMostOnce` and `Answered`), only for not disturbing a live member, which is why
`Undisturbed` is a property of its own.

**What was seen while writing the model**, and shown necessary by taking it out: a repeated join must be offered the
same incarnation, the second judgement in the session, the durable mark, and matching an offer to its join. That one
mark per member cannot serve several sessions was read from RIFL, not run: no configuration models the single mark.

The model is bounded: it does not cover the outbox direction (ADR 0109 D10), more than two requests in flight, or the
cryptography of an authenticator. Its messages are the parties' own: on a link an intermediary reads, that stands on
what §22.9 signs (D4), and each of the four parts signed is taken out in a run of its own. A crash of an instance is one
with or without a checkpoint of its own: `CCrash` is also the restore step 5 decided, and covers it unchanged.

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
3. (2026-11-22 to 2026-11-27) Parser, topology, refusals and the envelope key (D1, D2, D7). One refusal of D4 is
   not built: an open machine's binding under `authenticator: key` that names no `sign:` on the server's side or no
   `verify:` on the instance's is refused only where a document reads the person an instance acts for
   (`mesh/codegen-principal-unproved`, ADR 0116), and taken otherwise. `docs/SCE_SERVER_DEPLOY.md` section 0 lists it
   under what is not built.
4. (2026-11-27) Router, endpoint, checkpoint (D3, D5, D6) and the store's membership (D4).
5. (2026-11-28) Pool and outboxes (D4, D6); the `https` server and client (D4, D7). Test in the server suite: 200
   instances never listed, lost responses, a killed server, an expired lease and the client's unknown outcome
   (`tests/server/tests/open_instances_reach_a_pool_over_https.rs`). The killed server is one whose runtime is dropped
   with every task on it and started again on its store, in the test's process; the suite's campaigns that kill a
   server process with `SIGKILL` serve listed clients.
6. (2026-11-28) The documents named in "Amends".

What step 5 decided while it landed, each by the record's sources:

- **The mark a client carries** is the lowest number of what it carried that no `200` it took covered, or of what
  waits to be carried, and one past the highest number carried when nothing is (`Mark(i)` of the model). A `200`
  covers only what the server's durable checkpoint holds, whose answer the completion record and the instance's
  outbox keep (RIFL's completion record), so the mark passes nothing whose answer is lost. A join repeated carries the
  nonce of the first until an offer is taken (`SameOffer`).
- **A fence ends what left, and nothing else.** The core tells its host every envelope its server had not
  acknowledged (`LinkEvent::Fenced`); the router forgets each request still waiting that left and raises the row of
  its own (`outcomeUnknown`, row 20 of §16.7) with its `invoke_id`, and each other envelope it held with its
  `envelope_id`, and lets go of what durable receipt held of them, so none is carried under the next incarnation,
  where it would be a request of its own (`CFenced`: what waited becomes unknown, and a new request is new). A request
  that has not left (a checkpoint not yet durable holds it, or it waits for readiness) waits on.
- **An instance names itself.** Its router writes the instance into every envelope it makes
  (`Router::speak_as_instance`), since the server holds the pair to the link (invariant 11).
- **An outbox per instance ends with its membership** as `retain_ms` ends an entry: what it owes is told to its
  senders as row 15 and the outbox is closed (D4, "what is forgotten together is dropped together").
- **A restore is a recovery, and a recovery ends the incarnation it holds** (decided 2026-11-28, where this list had
  left it open). An instance's host that restores its router from a checkpoint of its own never speaks again under the
  incarnation the checkpoint's contents were said under: an incarnation is drawn once and never given again across a
  start (the model's own assumption; the fencing token of Gray and Cheriton 1989 and Kleppmann 2016), and the
  instance's client, which keeps nothing across its process, joins anew. Before it does, the restore ends what the
  checkpoint holds exactly as a fence ends it (`Router::restore`, beside `Router::peer_fenced`). A request leaves only
  once a durable checkpoint holds it (the write-ahead rule), so every envelope the checkpoint holds for the server,
  unreleased, waiting for the link, held for an acknowledgement or waiting out its backoff, may have left, under that
  incarnation or one the lost process took after the seal. The checkpoint names each request waiting on the server by
  its wire `invoke_id` (a peer's `waiting`, written only by an instance), and each ends as an unknown outcome, row 20
  with that `invoke_id`; each other envelope is row 20 with its `envelope_id`; durable receipt holds none of them, and
  none is carried under the next incarnation, where the server would take it as a request of its own. A request the
  checkpoint does not hold never left: the restored machine makes it, if at all, after the restore, numbered past the
  numbers the checkpoint gave (its `nextseq`) and carried under the next incarnation, whose completion records are
  its own. A host restores from its latest durable checkpoint only, the precondition `Endpoint::restore` states:
  stable storage, whose retrieve returns the last store (Cachin, Guerraoui and Rodrigues, *Introduction to Reliable
  and Secure Distributed Programming*, 2nd ed. 2011, section 2.2.4, crash-recovery). Exactly once across the restore
  is not promised: an at-most-once call reports an outcome it cannot know after a crash (Birrell and Nelson 1984), and
  the document decides whether to ask again (the end-to-end argument, D4).

  The checkpoint does not name the incarnation's number, and need not. The router never holds it (the client does,
  and tells its host only a fence), and a restore cannot be judged by it: a checkpoint sealed while the instance still
  joins names none, yet what it holds may leave once it is durable, under the incarnation taken after the seal. What
  the restore needs of the incarnation is that it ended, which is true of every one the lost process held, and of its
  numbers which requests they named that still wait, which the checkpoint names. So the rule is the router's: one that
  speaks as an instance (`Router::speak_as_instance`) binds only servers of its membership, and ends all it holds for
  them; one that does not refuses a checkpoint that names waiting requests.

  The model covers the restore as it stands (D8): `CCrash` forgets the incarnation, keeps `cnext` and `slot`, and
  turns every waiting request into unknown, which is the restore in the run where every request the checkpoint holds
  was sent before the crash. A request the restore ends as unknown that never left (the checkpoint was sealed while the
  instance had no incarnation) is a weaker report and never an effect, so `AtMostOnce`, `Answered` and `Ends` are
  what TLC checked. The action is unchanged, only its comment, and no new run is needed.

## Open questions

Decided with the acceptance, each with its reason.

1. The status of a fenced exchange: `412` with `If-Match` (D4).
2. A host of one endpoint, not a pool, keeps the membership store too: the same contract, so one rule.

3. Which fields a signature covers: every one the binding acts on, by one table (D4), and not a list that grows as a
   field is found missing. Decided by the owner, 2026-11-22.
