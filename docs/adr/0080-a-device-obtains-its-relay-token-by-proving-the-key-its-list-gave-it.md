# ADR 0080 — A device obtains its relay token by proving the key its list gave it, and the relay issues it

- Status: Accepted (2026-11-02), as proposed and with the proposed answers to its open questions (see the end).
  Being built in its order of work: steps 1 to 3 landed.
- Date: 2026-11-02
- Related:
  - ADR 0034 (a shared list is held by its members, and the hub only relays), sections 8 and 9 and its threat
    model: the relay is not trusted with what a list holds or with what members accept;
  - ADR 0079 (a device holds a shared list through one peer core with one C ABI), sections 3 to 5: the core is
    sans-I/O, the transport carries the token;
  - `docs/SCE_SHARED_LIST_RELAY.md` section 2 (tokens), `docs/SCE_RELAY_DEPLOY.md` sections 6 and 12 (who issues a
    token, and the decision this ADR answers), `docs/SCE_SHARED_LIST_PEER_ABI.md` sections 3, 7 and 11.
- Scope: how a device that is a member of a list, the first device of a list, and a device being invited obtain what
  the relay needs to let them in, with no operator and no sign-in service; what a public relay bounds; what stays
  with `sce-relay token issue`. Not in scope: a person's identity or account (there is none here), recovery of a lost
  device (ADR 0034 section 9), the relay's storage.

## Context

A device presents a token naming one list and its node (`token.rs`): HMAC-SHA256 under the relay's authority key, an
audience, an expiry. Whoever holds that key issues tokens: today the relay and an application's sign-in service, which
holds a copy. An application with no sign-in service of its own (a list shared among a household's phones, built by a
developer who runs no server but the relay) has no one to issue its devices' tokens but an operator, which does not
scale and puts a person in every device's loop. `docs/SCE_RELAY_DEPLOY.md` section 12 left this open.

What the relay already knows makes most of the answer: it holds each list's membership log, and the log names each
member device's node and the public key it signs with. A member can therefore prove to the relay who it is by
signing, and the relay can check it, without any account. Three cases are not covered by that:

1. **A member** (its node's grant is in the log the relay holds): proof of possession of its key.
2. **The first device of a list** (the relay holds no log of it yet): nothing to check a key against.
3. **A device being invited** (not in the log yet): it must leave a key package for an administrator to add it with
   (`Peer::add_member`, ADR 0034 section 6), and today that needs a token for the list.

## Decision

**The relay issues a member's token itself, against a signature by the key the list's log gave the member's node,
over a fresh challenge from the relay, the relay's name and an expiry. A device that is not yet a member speaks to
the relay on an anonymous connection that can only log in, create a list under the deployment's creation policy, and
publish a key package under an administrator's signed invitation. The token stays what it is; only who issues it
changes.**

### 1. The anonymous connection

An upgrade to `/sce-relay/1` with no `Authorization` header is an anonymous connection (one with a header the relay
did not issue is still refused, 401, so a device knows its token went stale). It may make only the requests of
sections 2 to 4 and `append` of entries to a list the relay holds (so that a device can bring the grant that admits
it). It is held to its own bounds: a few requests, a short idle time, and a per-source allowance (section 5). Other
transports (the deployment transport's adapter, ADR 0079 section 5) carry the same requests on whatever they have for a
peer that presents no token.

### 2. A member logs in by proof of possession

1. `{"v":1,"op":"challenge","list":L,"node":"N"}` is answered
   `{"ok":"challenge","audience":A,"nonce":B64,"expires_at_ms":"T","held":bool,"work":bits?}`. The nonce is 16
   random bytes and a 16-byte tag, HMAC-SHA256 under a key the relay process draws at start, over `L`, `N`, `T` and the
   random bytes: the relay keeps no state per challenge, and a nonce made for one node or list is no use for another.
   `T` is the relay's clock plus 60 s; the device never reads its own clock for it.
2. The device signs, with its node's key, the bytes `part("sce-relay/login/1") part(A') part(L) part(N) part(nonce)
   part(T)` in the trust layer's framing (each part its length as eight bytes big-endian, then its bytes), where `A'`
   is the audience **the device was configured with** (shipped with the relay's address, or carried in the invitation),
   never the one the challenge said; a device whose configured audience differs from the challenge's stops there.
3. `{"op":"login","list":L,"node":"N","nonce":..,"expires_at_ms":"T","signature":B64,"entries":[..]?}`. The relay
   checks the tag, that `T` has not passed, that the nonce is not in its replay cache (used nonces are kept until their
   `T`, in memory: a restart draws a new key and voids every outstanding nonce), that the latest grant of `N` in the
   log at its head gives a role of at least 1 and a key `K`, and the signature under `K` against its own audience. It
   answers `{"ok":"token","token":..,"expires_at_ms":".."}`: an ordinary token of the relay's authority for `(L, N)`,
   good for `[login] token_seconds` (default 3600). `entries` are appended first, as `append` would, so that a device
   whose grant the relay has not yet been given brings it.

Replay safety follows from the parts: the nonce is single use and expires; the audience binds the signature to one
relay (a relay that forwards another relay's challenge to a device gets a signature for its own name, which the other
refuses); the domain `sce-relay/login/1` is distinct from every domain the trust layer signs under
(`sce-trust/event/2`, `operation/1`, `membership/1`, `checkpoint/1`, `link/1`), so no login signature is an operation,
an entry, a checkpoint or a link, and none of those is a login. A refusal says which check failed with the existing
codes: `unauthorized` (signature, nonce, expiry), `forbidden` (no role), `behind` (the relay lacks the grant).

### 3. The first device of a list: creation by policy

A list exists at a relay from its first entry. Who may give a relay a first entry is a policy of the deployment,
`[creation] policy`:

- **`closed`** (the default): only a token from the authority creates a list, as today. A token from the authority
  for a name the relay does not hold *is* the operator-signed creation ticket; nothing new is needed for it.
- **`open`**: an anonymous connection may `create`. The challenge for a list the relay does not hold says `held:false`
  and a proof-of-work difficulty; `{"op":"create","list":L,"node":"N","nonce","expires_at_ms","work":"<counter>",
  "entries":[genesis, grant, ..],"signature"}` carries the entries `Peer::open_list` wrote, a counter such that
  SHA-256 of `part("sce-relay/work/1") part(nonce) part(hash of the genesis) part(counter)` has at least `bits`
  leading zero bits, and the login signature of section 2 by the key the grant gives `N`. The relay checks the work,
  the entries as `append` does, the name (below), its bounds (section 5), then logs the device in.

Under `open`, a list's name is **self-certifying**: `n1.<salt>.<digest>`, the digest the first 16 bytes of
SHA-256 over `part("sce-relay/list-name/1") part(salt) part(the genesis's first administrator key)`, so that no one
without that key can take a name another device made, before or after it lapses. And a list created openly
**lapses**: one no login has touched for `[creation] lapse_days` (default 90) is removed from the store, and a device
that still holds it creates it again on its next login, with the same genesis and name, and gives it back what it
holds (`docs/SCE_SHARED_LIST_RELAY.md` section 6: devices hold everything a relay holds). Open creation is not
enabled before lapse is built: without it `max_lists` is a bound a stranger can use up for good.

**Recommendation:** `closed` for any deployment that has a service of any kind (it issues creation tokens for its
users) and as the default; `open`, with proof of work, the per-source bounds and lapse, for an application with no
service at all, accepting the residual of section 6. Rejected as the answer for that application: an anonymous
credential (Privacy Pass) or a platform attestation (Play Integrity, App Attest) as the gate. Both need an issuer or
a verifier online, which is the service the application does not have; either can be added later as a third policy
by a deployment that has one.

### 4. A device being invited publishes its key package under a signed invitation

An administrator's device makes an invitation (`Peer::invite`): it draws a 32-byte secret `s`, derives an Ed25519 key
pair from it (`HKDF-SHA256(s, info "sce-relay/invitation-key/1")`), and signs with an administrator key of the log
`part("sce-relay/invitation/1") part(A) part(L) part(genesis hash) part(N) part(expiry) part(P_s)`, for a node `N` the
log has never granted. The QR code carries the relay's address and audience, the signed invitation and `s`. The core
keeps `(N, P_s)` in its sealed state.

The joining device (`sce_peer_invited` with the genesis hash from the invitation) asks a challenge for `(L, N)` and
sends `{"op":"join","invitation":{..},"nonce","expires_at_ms","key":B64,"key_package":B64,"proof":B64}`: its identity
key, its key package, and a signature under the key derived from `s` over `part("sce-relay/join/1") part(A') part(L)
part(N) part(nonce) part(T) part(key) part(SHA-256 of the key package)`. The relay checks the administrator's
signature against the log it holds, the expiry, that `N` was never granted, that the invitation was not used (a key
package for `N` kept under an invitation is the record of its use), and the proof under `P_s`. It keeps the key
package with the invitation, the identity key and the proof (the key package record gains these fields), and answers
`accepted`. No token is issued: the device has nothing more to do until it is a member, and then it logs in by
section 2 to read its Welcome.

Why no token scoped to "may publish a key package": a scope would change the token's form and every check of it for
one request, which the join request authenticates by itself. Why `P_s` and not `s` at the relay: the administrator's
device, adding the device (`Peer::add_member`), takes only a key package whose proof verifies under the `P_s` it made
for that node, and grants exactly the identity key the proof names. A relay that knew `s` could make that proof for a
key package of its own and have itself added to the group; with `P_s` it cannot. A person who photographed the QR code
holds `s` and can race the invited device: the invitation is single use, short-lived (default one day), and the
administrator's device shows the key's fingerprint for the two people to compare before it adds, as group
messengers do.

Who may invite: a key the log names as an administrator, since adding is an administrator's act.

### 5. What a public relay bounds

- **Per source** (the address, IPv4 as /32 and IPv6 as /64; behind a proxy, `X-Forwarded-For` from the addresses in
  `[listen] trusted_proxies` and from no other): anonymous connections open, challenges, logins, joins and creations
  per minute, each a token bucket as `Limits` already keeps per device.
- **Per list and node**: logins per node per hour; joins outstanding per list; key packages per node
  (`max_key_packages_per_node`, as now); records per list (`max_records_per_list`) and a new byte quota per list.
- **Per relay**: creations per hour, `max_lists`, and lapse (section 3).
- Tables are in memory, keyed by prefix, and forget an entry when its window passes.

### 6. What the relay learns, revocation, and ADR 0034's threat model

**Learns.** That node `N` of list `L` logged in at a time from an address, which it already learned from every
connection the token opened; under `open`, when and from which prefix a list was created; for a join, that node `N`
was invited and its identity key before the grant names it. It learns no person and no account. It logs counts, as
now, never a key, a token or a nonce.

**Revocation and expiry.** A token is short-lived, and a member taken off (role 0 at the relay's head) is refused a
new one; one it holds still connects until it expires, and every request is judged by the log, as now. Changing the
authority key voids every token, and each device logs in again with no one to ask. An invitation expires and is used
once. Challenges die with the relay process.

**The relay that issues its own tokens gains nothing it lacks.** Under ADR 0034 the relay is trusted for availability
only: it may withhold, and nothing it says is believed by a host without the members' signatures. A token
authenticates a device to the relay, for the relay's own refusals (spam, the removed, strangers reading); the relay can
already let anyone in or refuse anyone, and already holds every record it would serve, sealed. A token it mints for a
member's node lets it submit nothing a host would take, because an operation still needs that member's signature,
which the login signature is not (separate domain), and lets it read nothing it does not hold. Login transcripts it
collects are bound to its own audience and to single-use nonces, so it cannot log in as the device anywhere else. It
cannot forge a join proof (section 4) nor an administrator's invitation or entry. A relay that creates lists under
`open` creates them on itself. And under the present symmetric key the relay already can issue tokens
(`docs/SCE_RELAY_DEPLOY.md` section 6): this ADR removes the need for anyone else to, and adds no power to it.

### 7. The core and the ABI

The peer core makes these exchanges as it makes a round: request texts out, response texts in, no clock read but the
ones the host gives (ADR 0079 section 3). Additions to `sce-rust-peer-capi`, with its ABI version raised:

| Function | What |
|---|---|
| `sce_peer_login(peer)` | begins the exchange that obtains a token for the device's node: challenge, then `login`, or `create` when the relay holds no such list and the core opened it; the outcome gains `"token":{"token":..,"expires_at_ms":".."}` |
| `sce_peer_invite(peer, node, now_ms, valid_ms, out)` | an administrator's invitation for `node`, as one JSON text for a QR code, `s` included |
| `sce_peer_join(peer, invitation, len)` | begins the exchange that leaves the device's key package under the invitation |
| `sce_peer_link_set_token(link, token)` | the token a link presents from its next dial; the connection open now is given up when it was dialled with another |

`sce_peer_link_wss(base, token)` and `_believing` take a null or empty token for a link that dials anonymous
connections. A host's flow is: an anonymous link, `sce_peer_login`, `sce_peer_run`, the token from the outcome to
`sce_peer_link_set_token`, then rounds as now; on `unauthorized`, log in again. `sce_peer_add_member` refuses a key
package whose join proof does not verify under the invitation the core made for its node.

### 8. What stays with `sce-relay token issue`

The authority key and `Authority::issue` are unchanged, and so is every token they issue: an operator's tokens (the
first administrator's device before an application exists, a list created under `closed`), probes
(`sce-relay probe`), and an application that has its own sign-in service and prefers its own rules. Such a deployment
may turn proof-of-possession login off (`[login] enabled = false`), and leaves creation `closed`.

## Alternatives considered

- **A sign-in service for every application** (the present state). Correct, and what an application with accounts
  should keep; rejected as the only way, because it makes a server and an account system the price of a shared list.
- **No token: sign the upgrade itself** (a device signs a timestamp on every connection). Saves a round trip; needs the
  device's clock to agree with the relay's and a replay window, and drops the token every transport already carries
  (ADR 0079 section 5). Rejected.
- **TLS client certificates per device.** Bound to whatever terminates TLS (a proxy, option 3 of the runbook's section
  4), awkward on mobile platforms, and a second identity beside the log's key. Rejected.
- **A token scoped to publishing a key package** (section 4). Rejected for the join request that authenticates itself.
- **Open creation by name, first come** (no self-certifying name). Rejected: anyone takes any name, including one a
  lapse freed.
- **Anonymous credentials or platform attestation for creation** (section 3). Deferred: they need the service this
  ADR is for applications without.

## Consequences

- The relay gains an anonymous connection state, a per-process challenge key, a replay cache and per-source tables,
  all in memory; and a new surface, bounded by section 5.
- Protocol version 1 gains `challenge`, `login`, `create` and `join`, and the key package record gains the join's
  fields; a relay of today answers them `invalid`, so a device can tell a relay that does not offer login.
- The store gains removal of a lapsed list, the first thing a relay ever forgets.
- The peer core, its ABI (version raised), the recorded scenario and every lane's replay gain the login and the join.
- The runbook's sections 6 and 12 describe the policies; `sce-relay probe` may gain a login mode.

## Order of work

1. The anonymous connection and proof-of-possession login in the relay (`challenge`, `login`), its replay cache and
   per-source bounds, `[login]` in the configuration; tests of replay, another relay's audience, another node's nonce,
   an expired challenge, a removed member, a grant brought with the login.
2. The core's login exchange and the ABI (`sce_peer_login`, `sce_peer_link_set_token`, anonymous links); the recorded
   scenario logs in instead of being handed tokens.
3. Invitations and the join (`Peer::invite`, `join`, the extended key package record, `add_member`'s check); the
   scenario invites its second device by QR text.
4. Lapse in the store, then `[creation] policy = "open"` with self-certifying names and proof of work.
5. The runbook, and a probe that logs in.

## Status of the plan

**Steps 1 and 2 landed (2026-11-02).** What was built, and where the build chose what the decision
left open:

- **The relay** (`sce_rust_relay::login`, the `Doorway`; `Relay::with_login`, `Shards::with_login`).
  A relay offers login once it is given a doorway; the library's relay offers none until then, and
  `sce-relay` gives it one unless `[login] enabled = false`. The challenge's 16 bytes are HMAC-SHA256
  under the process's key of a counter, so a relay draws once at start and a test or a recording
  fixes the key (`Doorway::with_key`); the tag is HMAC-SHA256 under the same key over the list, the
  node, `T` and those bytes, under domains of their own. Used nonces are kept until `T` to a bound
  (100 000), past which a login waits for the oldest to expire. Entries brought with a login that the
  relay holds a position for are passed over, the rest appended as `append` appends them, and none
  makes a list the relay holds no log of (`behind`). A source is an IPv4 address or an IPv6 /64, and
  one source for a transport that names no address; its defaults are 30 anonymous connections, 30
  challenges and 30 logins a minute; a node's are 60 logins an hour, charged once the signature held,
  so that a stranger spends no member's allowance. An anonymous connection makes at most 8 requests
  and waits at most 30 s. `create` and `join` are answered `invalid`, with a reason that names them,
  until steps 3 and 4.
- **The signature's bytes** (`sce_rust_trust::login`): each part in the trust layer's framing, the
  node and `T` in decimal and the nonce as its text in the URL alphabet, as the trust layer writes
  every byte string it frames. The domain `sce-relay/login/1` is the trust crate's, beside every
  other domain it signs under.
- **The connection.** An upgrade with no `Authorization`, or with a token the relay issued that
  expired, is anonymous; a token the relay did not issue is refused (401), and a source past its
  allowance too (429, with `Retry-After`). A request to log in is answered on any connection
  (`Hub::answer`), so a device whose token expired on a connection that stays open logs in on it.
  `X-Forwarded-For` names the source only on a connection from an address of `[listen]
  trusted_proxies`, read from the right past the trusted proxies.
- **The core and the ABI.** `Peer::login(audience)` is an exchange as a round is (a `LoginMachine`
  of the relay's client), and brings the entry of its node's latest grant. The audience is an argument
  of `sce_peer_login(peer, audience)`, where section 7's table wrote `sce_peer_login(peer)`: the core
  is a list's, not a relay's, and the name a device was configured with is its host's to give. A login
  never creates a list yet (step 4). The core holds no token and does not log in again by itself: a
  round the relay refuses `unauthorized` ends as before, and the host gives the link no token, logs
  in over it and gives it the new token (`sce_peer_link_set_token`), since the token is the
  transport's (ADR 0079 section 5) and a token the relay did not issue (its key changed) is refused at
  the upgrade still. The ABI is version 5.
- **The recorded scenario.** Bob's device logs in and is never handed a token, and once taken off is
  refused one (`forbidden`); alice's device, which gives the relay the list's first entries, presents
  a token of the authority, as a relay whose creation is `closed` takes it.
- **Not built in these steps:** a probe that logs in (step 5), invitations and the join (step 3),
  lapse and open creation (step 4).

**Step 3 landed (2026-11-03).** What was built, and where the build chose what the decision left
open:

- **The bytes** (`sce_rust_trust::invitation`). The invitation's parts are signed as section 4 lists
  them, the genesis's hash and `P_s` in the URL alphabet and the node and the expiry in decimal, in
  the trust layer's framing; the key pair is Ed25519 from the seed HKDF-SHA256 of `s` with no salt and
  the info of section 4. The proof's parts are section 4's, the key package's hash in the URL alphabet.
- **The text for the QR code** is `SCEI1.` and, in the URL alphabet, each part its length as two bytes
  big-endian and then its bytes: the relay's address, the audience, the list, the genesis, the node,
  the expiry, the administrator's key, the signature and `s`. `P_s` is not carried, being made from
  `s`. A binary form under a versioned prefix, rather than the JSON text section 7's table named,
  because it is half the size in a QR code and no lane reads it: `sce_peer_invitation_read` gives
  what it says as JSON, and `sce_peer_invited_by` makes the invited device's core from it.
- **The fingerprint** is twenty decimal digits in five groups, from SHA-256 of `sce-relay/fingerprint/1`,
  the list, the node, `P_s` and the identity key (`docs/SCE_SHARED_LIST_PEER_ABI.md` section 12, with a
  known answer): about 66 bits, so that a relay and a person who photographed the code cannot find a
  key of their own that shows the same digits.
- **The relay** answers `join` on an anonymous connection: the doorway checks the challenge, spends
  its nonce and holds a source to `joins_per_minute` (30); the list's side (`Relay::take_join`) checks
  the audience, the genesis, an administrator the log names and the signature, the expiry, a node
  never granted, no key package kept for the node under an invitation, and the proof, and refuses a
  join past `joins_outstanding_per_list` (32) unexpired joins of nodes not granted yet. The record
  of use is per node: a node whose invitation was used is invited again under another node. The key
  package record gains `"j"`: the invitation and the proof's key, nonce, expiry and signature, the
  nonce and the expiry because the administrator's device checks the proof again and cannot without
  them. A record read back from the store is checked again, but for its expiry and audience. Both are
  `[login]` keys of `sce-relay`.
- **A list no group keys** is admitted by a grant, and its device has no key package to leave: its
  join carries none (the proof signs the hash of no bytes), and `Peer::grant` of a node the core
  invited takes only the key a join proved, as `Peer::add_member` takes only the key package of the
  join that proved the key given. This is what lets the recorded scenario, whose list no group keys
  since MLS draws randomness, invite its second device by the text and replay it in every lane.
- **The core and the ABI.** `sce_peer_invite(peer, relay, audience, node, now_ms, valid_ms, out)` takes
  the relay's address and name, which section 7's table left out (a core is a list's, not a relay's,
  as for the login), refuses a lifetime past a day, and keeps the node, `P_s` and the expiry in the
  sealed state, now the core's version 5; `sce_peer_invite_under` takes a secret the host gives, for a
  recording. `sce_peer_join(peer, invitation, len, fingerprint)` gives the fingerprint the invited
  device shows, where the table gave none; `sce_peer_joined(peer, node, out)` gives each device that
  joined with its fingerprint, all of them, so that two joins of one invitation (a relay that took two,
  a person who raced the device) are shown and the people's comparison chooses. `add_member` and
  `grant` of an invited node take the key the host gives only when a join proved it
  (`SCE_PEER_ERR_NOT_JOINED`), and the core forgets the node's invitations once it is admitted, and
  those that expired a day before a later invitation with no join read. Functions added alone, with
  statuses appended (16 to 18), and the ABI stays 5.
- **The recorded scenario** invites bob's device by the text, makes it from the text, joins, shows one
  fingerprint on both devices and grants the key the proof names.
- **Not built in this step:** lapse and open creation (step 4), a probe that logs in (step 5).

## The owner's answers (2026-11-02)

The owner accepted the proposals: a relay that names no policy is `closed`; administrators invite; tokens live an
hour and invitations a day; openly created lists lapse after 90 idle days and lists created with an authority token
do not; no platform attestation, the proof of work at the difficulty section 3 proposes; an invitation is not
withdrawn before it expires (its day is the bound, and withdrawing it would make it the log's business); the relay
trusts `X-Forwarded-For` only from proxies its configuration names. The questions as they were asked:

## Open questions for the owner (answered above)

1. The default creation policy of a relay that names none: `closed` (proposed) or `open`.
2. The proof-of-work difficulty, and whether a platform-attestation policy is wanted for open creation.
3. Who may invite: administrators only (proposed), or members of role 3 as well, an administrator adding later.
4. The lapse period (90 days proposed), and whether lists created with an authority token lapse too (proposed: no).
5. The token lifetime (one hour) and the invitation lifetime (one day), and whether an outstanding invitation needs a
   way to be withdrawn before it expires (a log entry, which would make it the log's business).
6. Whether the relay trusts `X-Forwarded-For` from configured proxies (proposed) or a deployment that wants
   per-source bounds terminates TLS at the relay.
