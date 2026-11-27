# ADR 0116 — A person stands behind an instance by a delegation the server holds, and what a document reads of them no sender can write

- Status: Accepted (2026-11-22, by the owner), as amended before it was accepted: a person is bound where an instance
  joins (D2), a revocation is held by the write (D6), and an event of an ended delegation is refused (D6). Its model
  (D9) was written and checked before any code.
- Date: 2026-11-22
- Written under the owner's standing rule (the most long-term correct structure, textbook in its foundations, cost
  ignored) and the owner's scope rule of 2026-11-22: built on the backend a consumer needs, Rust first; the contract is
  general; what is special to one application stays in the application.
- Amends, once accepted (each amended document gains an "Amended by: ADR 0116" line in the landing that moves it):
  `SCE_MESH.md` §10.9 (a twelfth invariant), §16.7, §22.5 and §22.9; ADR 0113 (D1 left "who it acts for" to a record of
  its own: this one); ADR 0114 (D1: what an entry records; D6: facts of authority); ADR 0109 (D14: what an audit row
  records); `docs/SCE_ACCEPTED_SUBSET.md` §2.15 and the appendix; `docs/SCE_SERVER_DEPLOY.md`.
- Scope: the Rust host core, `https` under the `key` authenticator, `sce-static` documents. A machine that declares no
  principal is as it is.
- Related: ADR 0113, ADR 0114, ADR 0115, ADR 0117.

## Context

A server machine knows which *instance* speaks (ADR 0113): what the link's authenticator proved. It does not know for
whom. The four authenticators prove a client's name and a person is no notion of any of them (SCE_MESH.md §22.5), so an
application that needs the person puts them in the payload, which the sender writes, or in a token each exchange
carries, which an intermediary that reads the exchange may keep. A document then decides on a value it cannot trust, or
a host reads the token beside the machine and the decision is no longer the document's.

## The textbook structure

- **A statement is made on a channel, and a channel speaks for a principal** (Lampson, Abadi, Burrows and Wobber 1992).
  A key speaks for an instance; an instance acts for a person. The second is a *delegation*, a compound principal
  (`instance for person`), and not the same as being the person.
- **Delegation is not impersonation** (RFC 8693 section 1.1): the one who acts stays visible.
- **Quoting is not saying** (Lampson et al.: `A | B` says what A reports, "A could be mistaken or lying").
- **Every access is checked, and the default is lack of access** (Saltzer and Schroeder 1975: complete mediation,
  fail-safe defaults); a principal has no more than it needs (least privilege).
- **Authentication and authorization are two things.** Who it is comes from a proof; what they may do comes from facts
  that change, and the facts are ordered (Pang et al., Zanzibar, 2019: an old list must not be applied to new content).
- **What is cached expires, and a source that is secure and one that is available must both agree** (Lampson et al.): if
  the available one is compromised, the worst is a late revocation.
- **A proof is bound to a key**, so that one that leaks is of no use elsewhere (RFC 9449; RFC 9700 section 2.2).

## Decision

### D1. Three principals, and their relation is told

The key of a link speaks for the instance it is pinned to (§22.9). An incarnation of the instance (ADR 0113 D4) acts for
at most one person, from its confirmation to its end: that is the **delegation**, a part of the membership the server
holds and not a record with a life of its own. Every reader is given the compound: the instance and the person. An
incarnation with none acts for nobody, which is a value (`null`) and never a default person.

Logging in, logging out and a change of person are each a new incarnation. What was sent to the last one, its answers
and its outbox, ends with it and is never given to the next: two people at one device would otherwise read each other's
mail. (OWASP's guidance, that a session's id is renewed when its privilege changes, is the same rule.)

**Invariant 12 of §10.9**: what a machine reads of a person comes from the delegation its own server holds for the link
the event came on, and from no field of an envelope.

### D2. A person is bound where the instance joins, by a proof bound to its key

- A join says that a proof will follow. The server offers an incarnation as for any join (ADR 0113 D4), and the offer
  waits `login_ms` for it, no default. The incarnation the instance has meanwhile goes on.
- **The challenge is not drawn; it is computed**: a hash, under a name of its own, of the server machine, the instance,
  the thumbprint of its key and the offered incarnation. The server keeps nothing for it, and what a challenge must be
  (chosen by the server, for one instance and one key, of use once) is what an offered incarnation already is: an offer
  is confirmed once. OpenID Connect Core 15.5.2 names the form, a `nonce` that is a hash of a value of the session.
- A proof carries the challenge: an OpenID Connect ID Token as its `nonce`, a WebAuthn assertion as its `challenge`. It
  travels in the body of the exchange that confirms the offer, which the instance's key signs with `if-match` (§22.9).
  So a proof stolen without the key, replayed, or made for another instance or another incarnation is refused, and the
  offer stands until `login_ms` for a good one.
- The host's port judges it: `IdentityProof::verify(proof, challenge) -> Person`, the issuer and the subject, when and
  how they were proved, and what the document declares (D3). With OpenID Connect the server's program is the
  confidential client and exchanges the code: a browser holds no token of the identity provider, the pattern the OAuth
  working group's draft for browser applications ranks first.
- The confirmation is one write, ADR 0113's: the member, its person (issuer, subject, the moment and method of the
  proof), the moment it ends, and the end of the incarnation before it. No later exchange carries a proof or a token:
  the key's signature is enough.
- A client that changes its person first waits, for `drain_ms` and no longer, for the answers it has not, and gives up
  as *outcome unknown* what is left (ADR 0113 D4).

### D3. A document declares what it reads, in a place no sender writes

A statechart declares `sce:principal="Person"`, an imported record schema, and reads `_x.principal`, an
`optional<record:Person>` of the event being processed: the platform's variable, as W3C SCXML 5.10 has it, beside
`_x.limits`. It is present on an event that came on a link whose instance acts for a person, and `null` on any other: an
internal event, a timer, an entry of a journal, a completion.

The record is a schema of the document's own, whose fields are taken by name from a list this record fixes: `issuer`,
`subject` (a subject is unique only within its issuer, and the two are one key), `proved_ms`, `method`, and the
instance. A field not declared is not given. No display name, address or group is among them: they are data of the
application, and facts (D5).

A machine that must have a person says it in its guard, as any condition; `sce:principal` with `required="true"` on a
transition is the same guard, and an event without one is not selected.

### D4. Nothing passes on by itself

An event a machine sends carries the machine, not the person it was acting for. A machine that asks another on a
person's behalf says so: `<send … sce:on-behalf="_x.principal">`, and the receiver is told a **quoted** person,
`_x.quoted`, with the machine that quotes: never `_x.principal`. A deployment lists which machines a machine takes a
quote from; from any other it is refused at the edge. This is the confused deputy's remedy: the authority used is named
by the one who has it.

### D5. What a person may do is facts in order, not claims in a proof

Groups, roles and relations change while a delegation stands, and a claim in a proof is what they were when it was made.
They are facts of an **observed stream** (ADR 0114 D6), which an adapter of the application writes from the directory
that decides them, each with a position. A machine that decides keeps the facts it needs by following the stream, and a
decision records the position it read. A request may carry the position an earlier answer told, and the machine waits
until it has followed that far: so a content made after a removal is not judged by the list before it (Zanzibar's "new
enemy"). SCE gives the stream, its order and its horizon; what a group means is the document's.

### D6. A revocation is a write, and its bound is promised

- An incarnation with a person ends at `until`, `person_ttl_ms` after its proof or its last renewal; `person_ttl_ms` has
  no default. A renewal is a proof of the same person, or the host's answer that the identity provider's session stands;
  it moves `until` alone. `proved_ms` and `method` stay those of the last proof, so a document that asks for a person
  proved lately is not answered by a renewal. A proof of another person renews nothing and is refused: it is a join.
- A revocation names a person, an instance or one incarnation, and **ends the incarnations it names**, in one write of
  the store. So does `until`, and so does the instance that gives it up. There is one way to end, and it is ADR 0113's:
  the fence, with its reason.
- **What is not yet done is not done.** An event that came under an incarnation that ended while it waited is fenced and
  not run. It is never run as nobody's: an event that was said for a person and is run for none is another event.
- **What is in hand is not kept.** The write that keeps a run names the incarnations it was made under and is refused
  when the store no longer holds one, its `until` read against the write's own moment (ADR 0113 D5, `Write::under`).
- **The promise**: a run whose write comes after the write that ended its incarnation keeps nothing and sends nothing
  (what an earlier write kept stands, and what it sent to the instance ends with the instance's outbox); what an
  identity provider ended and did not tell ends within `person_ttl_ms`. A call a host had already begun for the run is
  not taken back, as no answer of the network is: it is what a host-run invocation's operation identity is for (ADR 0107
  D4). A provider that tells (back-channel logout) is an adapter that writes the revocation.

### D7. Who did it is recorded by the platform

A journal entry (ADR 0114) and an audit row (ADR 0109 D14) made in a run under a delegation record the instance, the
person and, for a quote, the machine that quoted. The pool writes them; a document neither writes nor leaves them out.
They are sealed at rest as the rest of a row is (ADR 0109 D8), a person by a keyed digest where a store finds a row by
one. Erasing a person is a record of its own (ADR 0114 D8).

### D8. What is the application's

The identity providers and their adapters, the pages a person sees, linking accounts, what a group means, who may do
what. SCE has the port, the record, the delegation and its store, the conformance kit a proof adapter runs (a proof of
another challenge, key, audience, issuer, or time, each refused), and a proof for tests that no deployment may name.

### D9. The protocol is a model before it is code

`PrincipalService.tla`, the promise: a run is kept for a person only if a proof of that person was judged good for that
incarnation, and not after the incarnation ended; an incarnation acts for one person; what was made for one person is
read by no other; a quoted person is never told as the one acting. `Principals.tla` refines it, over a network that
repeats and delays and a server that crashes, with a thief who sees every message and every proof and holds no key but
their own. Checked by TLC on 2026-11-23, before any code: two devices (one the thief's) and two people, 8,721,642
distinct states; that whoever stays at a device is let in (`Enters`), one device, 18,622; no violation.

| Taken out | What TLC finds |
|---|---|
| the challenge's binding to the instance and its key | a person told who never proved it there: a proof made for one device, given by another |
| its binding to the incarnation | the same: a proof of who was there before |
| one person per incarnation | one person reading what was made for another |
| the condition of the write | a run kept after its incarnation ended |
| the durability of an end | the same, which a crash took back |
| the difference of a quote | a person told who was only quoted |
| the fence told to a device that waits for its confirmation | a person who is never let in |

**What was seen while writing the model**: the last row. TLC's first run of `Enters` failed, since an incarnation that
ended before its device learned that it was confirmed was told to nobody; ADR 0113's fence answers that exchange too.
The model takes the handshake as OpenInstances has it, is bounded to two requests, and says nothing of groups (D5) or of
time but that an end can come at any moment.

### D10. Backends

Rust, as a server and as the dialing side that asks for a challenge. An `authenticator` other than `key` proves no key
and is refused with `sce:principal` where the deployment is built, and so is a machine that is not `instances: open` or
whose transport does not name its sender, or whose binding names no `sign:` and `verify:` (ADR 0113 D4). Every other
backend refuses the document by name.

## Not in scope

Sessions of an identity provider, single sign-on across applications, anonymous credentials, a person's consent, a
policy language, erasure, and several people behind one instance at a time.

## Consequences

- A document decides on a person it can trust, and the decision stays the document's.
- A leaked token is of no use: there is none after the proof.
- A login costs an incarnation: what was waiting is drained or of unknown outcome, and an outbox is not carried over.
- A machine between two others loses the power to act as whoever asked it last.
- Groups cost a stream to follow. In return a removal has an order, and a decision can say what it knew.

## Alternatives rejected

- **The person in the payload.** The sender writes it.
- **A bearer token on each exchange.** It is the proof itself, carried past everyone who reads the exchange, and a host
  decides beside the machine.
- **Group claims in the proof.** They are what was, and have no order.
- **The person in place of the instance** (`source` names a person). Two devices of a person are one sender, and their
  windows and outboxes are one (ADR 0113, "Context").
- **A delegated person passed on with every send.** The confused deputy.
- **A policy language of SCE's.** The document is one.
- **A delegation as a record of its own, with a challenge of its own** (the first draft). Two lives to keep in step, two
  ways to end, and a person could change under an incarnation whose mail was another's.
- **Judging as the run begins** (the first draft). A revocation between the beginning and the write is then not kept.

## Order of work

Each step lands with a test that fails without it.

1. (2026-11-23) This record, accepted, and its model (D9), checked, with its failing configurations.
2. (2026-11-28) The person in the membership's store, its conformance kit, sealing and re-sealing.
3. (2026-11-28) The offer that waits, the port and the edge; the condition of the write (ADR 0113 step 4 builds it).
4. (2026-11-28) `sce:principal`, `_x.principal`, `_x.quoted` and `sce:on-behalf`, with each refusal's
   appendix row (`validation/principal-record`, `validation/quote-refused`,
   `mesh/codegen-principal-unproved`; every backend but Rust by `generate/unsupported-feature`).
5. What the journal and the audit log record (D7), and D5 over ADR 0114's followers.

Step 3 needs ADR 0113 step 4; step 5 needs ADR 0114 step 4.

## Decided while landing step 4

Each decided by the record's own text and the source it cites, and written here so the next step reads it.

1. **The spelling of the required guard.** `sce:principal="required"` on a `<transition>`: the attribute D3 names,
   whose one value says what D3's `required="true"` says, so no second attribute is read. It is the guard
   `_x.principal !== null` joined before the transition's own condition, the same guard as D3 says, so the condition
   reads the person where it is proven present (docs/adr/0055, step 9).
2. **A text of a person is held to the bound its record states.** Every text a document declares states its bound,
   and `issuer`, `subject`, `method` and `instance` are the identity provider's and the server's texts, of no length
   the document fixes. A person whose text is longer than its field holds refuses the delivery whole, by the verdict a
   payload that does not fit its type takes (ADR 0124, D2: `error.execution` in its place): a text cut short could
   name another person (OpenID Connect Core 1.0, section 2: `sub` is unique within its issuer as written), and an
   event said for a person is never run as nobody's (D6). A subject, at most 255 ASCII characters (the same section),
   fits a record that states that bound.
3. **A quote is refused where it is written.** Open question 2 decided that `_x.quoted` lands as a refusal of every
   quote until a deployment lists one. No deployment can list one yet, so the build refuses `sce:on-behalf` where it
   is written (`validation/quote-refused`) rather than send a quote to be dropped at the receiver's edge (Saltzer and
   Schroeder 1975, fail-safe defaults), and `_x.quoted` is absent on every event. The syntax is fixed: the landing
   that lists quotes changes the refusal and the read, and no document.
4. **A machine that reads a person receives durably.** An event is told the person of the incarnation it was taken in
   under (`Endpoint::receive_member_for`), and only an instance's completion records judge an envelope by its
   incarnation (ADR 0113, D5); a binding that does not receive durably knows no incarnation of what it takes in, and
   could tell an event the person of a later one. So D10's bindings are held to `receipt: durable` beside `key`,
   `sign:` and `verify:`.
5. **A saved state and a checkpoint hold no person.** A saved state is one text on every backend, and a person is the
   Rust host core's alone; an event of a person in a queue or released and not applied is never run as nobody's
   (D6), so the save and the seal that would hold one refuse, and a host saves after the run that takes the event, as
   the write that keeps a run is made after the run (ADR 0113, D5).

## Decided while landing step 5

D7 landed on 2026-11-28; D5 over ADR 0114's followers is the part of step 5 still to land, so the step is not dated.
Each decision is the record's own text and the source it cites.

1. **Whose run a fact is.** A macrostep is made for the person the external event that began it was said for
   (`EngineState::acting`), so a fact said on an internal event the macrostep raised, which carries no `_x.principal`
   (D3), is still recorded as that person's, and one begun by an event said for nobody (a timer, a completion, an entry
   of a journal) records nobody. A statement is made on the channel its event came on (Lampson et al. 1992).
2. **What is recorded.** The instance and the whole person as the delegation names them: issuer, subject,
   `proved_ms` and `method` (`Fact::actor`, `JournalEntry::actor`; `SqliteStore` layout 13; inside the record
   under `Sealed`, docs/SCE_SEALED_RECORD.md section 3). An observed fact is another system's and names no one; a store
   refuses one that does (`ObservedActor`).
3. **A quote is not recorded yet.** Decision 3 of step 4 refuses every `sce:on-behalf`, so no run is made under a
   quote and the record keeps no quoting machine; the landing that lists quotes adds it beside `_x.quoted`.
4. **A page names no person.** A page of `sce-host-journal-read` carries no actor: a machine reads a person only from
   the delegation its server holds for the link its event came on (invariant 12), and the person of a past entry is
   a row of a store, not that. Who did what is the host's and an auditor's to read (`JournalStore`).
5. **The audit row.** The host writes the member `sce:principal` into the entry before its canonical form is taken,
   so the chain covers it and the bytes of a row, its chain and its checkpoints (docs/SCE_AUDIT_LOG.md) are those of
   ADR 0109; the subject is written as a personal value is, since it is who the person is, and an entry that names
   the member itself is refused (`invalid_argument`).
6. **A running invocation started for a person holds the save.** A restart from a saved state, which holds no
   person (decision 5 of step 4), would run it for nobody (D6) and record its work as nobody's (D7), so the save
   refuses while one runs, as it does for a queued event of a person.

For the rest of step 5, D5 over the followers, decided by the record's text and Zanzibar's consistency protocol
(Pang et al. 2019, the zookie): a fact a follower session says records, beside its actor, the position of each
journal it follows that its checkpoint holds (the decision's read set); a request carries a position in a field
the deployment names, as `sessions.key_from` names the key, and the pool holds it in the session's inbox until its
reader's mark (`JournalStore::keep_read`) has passed that position, refusing a position above the journal's horizon,
which no answer told.

## Open questions

Decided with the acceptance, each as recommended and with its reason.


1. A passkey (WebAuthn) verifier is the application's, with SCE's kit, since a relying party's policy is its own.
2. `_x.quoted` is in the first landing, as a refusal of every quote until a deployment lists one; the syntax is then
   fixed once.
3. The host's word alone renews, and moves `until` and nothing else (D6).

## Sources

Checked in the text on 2026-11-22: Lampson, Abadi, Burrows and Wobber 1992, the DEC report (principals, `speaks for`,
quoting, limited lifetimes and the two sources); RFC 8693 (section 1.1); RFC 9449 (sections 2, 8, 11.2 and 11.4); RFC
9700 (section 2.2); `draft-ietf-oauth-browser-based-apps-25` (sections 5, 6.1 and its abstracts); Pang et al. 2019 (the
two examples of the new enemy); Saltzer and Schroeder 1975 (the principles named); OpenID Connect Core 1.0 (`sub`, a
subject "locally unique and never reassigned" within its issuer; `aud`, `nonce`, `auth_time`; 15.5.2, a hash of a
session's value as the `nonce`); OWASP's Session Management Cheat Sheet (the session id renewed after a change of
privilege); OpenID Connect Back-Channel Logout 1.0 (what it is for). WebAuthn Level 3 (the relying party verifies that
the client data's `challenge` is the one it gave, and its `origin`). None was read through. **Not reached:** Hardy 1988,
for the confused deputy, cited for what is generally reported of it.
