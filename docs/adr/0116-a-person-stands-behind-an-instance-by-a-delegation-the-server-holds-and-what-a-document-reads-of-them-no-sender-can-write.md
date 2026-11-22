# ADR 0116 — A person stands behind an instance by a delegation the server holds, and what a document reads of them no sender can write

- Status: Proposed (2026-11-22). The owner accepted the scope (an authenticated person as trusted context) and ordered
  this draft; nothing is built until the owner accepts the record.
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

The key of a link speaks for the instance it is pinned to (§22.9). The instance acts for at most one person at a time,
by a **delegation** the server holds. Every reader is given the compound: the instance and the person. An instance with
no delegation acts for nobody, which is a value (`null`) and never a default person.

**Invariant 12 of §10.9**: what a machine reads of a person comes from the delegation its own server holds for the link
the event came on, and from no field of an envelope.

### D2. A delegation is made once, by a proof bound to the instance's key

- The instance asks for a challenge. The server draws it, single use, for one incarnation (ADR 0113 D4) and one key, and
  forgets it after `challenge_ms`.
- A proof carries the challenge: an OpenID Connect ID Token as its `nonce`, a WebAuthn assertion as its `challenge`. The
  request that brings it is signed by the instance's key (§22.9). So a proof stolen without the key, replayed, or made
  for another instance is refused.
- The host's port judges it: `IdentityProof::verify(proof, challenge) -> Person`, the issuer and the subject, when and
  how they were proved, and what the document declares (D3). With OpenID Connect the server's program is the
  confidential client and exchanges the code: a browser holds no token of the identity provider, the pattern the OAuth
  working group's draft for browser applications ranks first.
- The server writes the **delegation** in one write: the machine, the instance, its incarnation, the issuer and subject,
  the moment and method of the proof, its expiry. It ends with the incarnation, with its expiry, with a revocation (D6),
  or when the instance gives it up. No later exchange carries a proof or a token: the key's signature is enough.

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

- A delegation lives `person_ttl_ms`, which has no default, and is renewed by a proof or by the host's answer that the
  identity provider's session stands.
- A revocation names a person, an instance or one delegation, and is a write of the store. The edge judges the
  delegation as it admits an event and the session judges it **again as it runs**, as ADR 0113 D5 does with the
  incarnation, since it may have ended while the event waited.
- **The promise**: a run that begins after a revocation is committed is under no delegation it ended; and what an
  identity provider ended and did not tell ends within `person_ttl_ms`. A provider that tells (back-channel logout) is
  an adapter that writes the revocation.

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

`PrincipalService.tla`, the promise: a run is under a person only if a proof for that person was judged good, for the
challenge of that incarnation and key, and no revocation of it was committed before the run began; a quoted person is
never told as the one acting; what a run recorded is who it ran under. `Principals.tla` refines it, over the network and
the crashes of `OpenInstances`, with a thief who sees every message and holds no key. The parts, each with the run that
fails without it: the challenge's single use; its binding to the incarnation; its binding to the key; the end of a
delegation with its incarnation; the second judgement as the session runs; the durability of a revocation; the
difference of a quote.

### D10. Backends

Rust, as a server and as the dialing side that asks for a challenge. An `authenticator` other than `key` proves no key
and is refused with `sce:principal` where the deployment is built, and so is a machine that is not `instances: open` or
whose transport does not name its sender. Every other backend refuses the document by name.

## Not in scope

Sessions of an identity provider, single sign-on across applications, anonymous credentials, a person's consent, a
policy language, erasure, and several people behind one instance at a time.

## Consequences

- A document decides on a person it can trust, and the decision stays the document's.
- A leaked token is of no use: there is none after the proof.
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

## Order of work

Each step lands with a test that fails without it.

1. This record and its model (D9), accepted.
2. The delegation's store, its conformance kit, sealing and re-sealing.
3. The challenge, the port and the edge; the second judgement in the session.
4. `sce:principal`, `_x.principal`, `_x.quoted` and `sce:on-behalf`, with each refusal's appendix row.
5. What the journal and the audit log record (D7), and D5 over ADR 0114's followers.

Step 3 needs ADR 0113 step 4; step 5 needs ADR 0114 step 4.

## Open questions

1. Whether a passkey (WebAuthn) verifier is SCE's reference adapter or the application's. Recommended: the
   application's, with SCE's kit, since a relying party's policy is its own.
2. Whether `_x.quoted` is in the first landing. Recommended: yes, as a refusal of every quote until a deployment lists
   one; the syntax is then fixed once.
3. Whether a delegation may be renewed by the host's word alone. Recommended: yes, and the method it records says so, so
   a document can ask for a person proved again.

## Sources

Checked in the text on 2026-11-22: Lampson, Abadi, Burrows and Wobber 1992, the DEC report (principals, `speaks for`,
quoting, limited lifetimes and the two sources); RFC 8693 (section 1.1); RFC 9449 (sections 2, 8, 11.2 and 11.4); RFC
9700 (section 2.2); `draft-ietf-oauth-browser-based-apps-25` (sections 5, 6.1 and its abstracts); Pang et al. 2019 (the
two examples of the new enemy); Saltzer and Schroeder 1975 (the principles named); OpenID Connect Core 1.0 (`sub`, a
subject "locally unique and never reassigned" within its issuer; `aud`, `nonce`, `auth_time`); OpenID Connect
Back-Channel Logout 1.0 (what it is for). WebAuthn Level 3 (the relying party verifies that the client data's
`challenge` is the one it gave, and its `origin`). None was read through. **Not reached:** Hardy 1988, for the confused
deputy, cited for what is generally reported of it.
