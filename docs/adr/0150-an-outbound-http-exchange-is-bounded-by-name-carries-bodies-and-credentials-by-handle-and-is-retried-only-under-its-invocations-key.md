# ADR 0150 — An outbound HTTP exchange is bounded by name, carries large bodies and credentials by handle, and is retried only under its invocation's key

- Status: Accepted (2026-11-28) under the owner's standing rule (the most long-term correct, textbook structure, cost
  ignored), by the main session the owner delegated the decision to, after an independent review; amended on
  acceptance as listed in "Amended on acceptance".
- Date: 2026-11-27
- Written under the owner's scope rule of 2026-11-22: built on the backend a consumer needs, Rust first; the contract is
  general; what is special to one application stays in the application.
- Amends (each amended document gains an "Amended by: ADR 0150" line in the landing that moves it):
  - `docs/SCE_HOST_FEATURES.md` section 1: the identity of an invocation gains the session's incarnation (D6); a host
    repeats an operation only as its request states, and once after a credential refresh (D5, D6); the failure
    `outcome_unknown` is the one code that does not say nothing changed (D6).
  - Section 3 (the put-if's first result), section 13 (key generation) and section 14 (the token's tag, now the
    deterministic CBOR array `[session id, incarnation, invoke id, start, name]`): each keeps the identity of section 1,
    so each takes the incarnation (D6).
  - Section 5: the named bounds and their relations (D1); `sce-host-http-fetch` started again after a restore (D6);
    the new operation (D3 to D6).
  - Sections 7 to 9: the lanes, the scenarios, what is not provided.
  - Two new sections, blobs (D2) and outbound credentials (D5), numbered with the next free numbers of the document
    when this record lands. Section 16 is the event journal (ADR 0114).
  - `docs/SCE_ACCEPTED_SUBSET.md` §2.15: the saved state holds the session's incarnation (D6).
  - `stdlib/host/http_fetch_request.scxml` and `http_fetch_result.scxml` (D1).
  - ADR 0109 Open question 7: a store of blobs now exists (ADR 0153); signing one stays a record of its own.
  - ADR 0153 amends this record's D2, and D2 below is written in the form ADR 0153's "What ADR 0150 must change"
    gives it.
- Scope: the contract names no language. D1, and the incarnation and the restart rule of `sce-host-http-fetch` (D6),
  change what every lane's runtime and reference host serves, so they land on all seven. D2 to D6's new operations are
  built in the Rust host core (`sce-rust-host-features`, `sce-rust-host-http`, `sce-rust-session-store`) and
  `sce-build`; every other lane refuses them by name (D7).
- Related: ADR 0050 (a bound read by name), ADR 0077 (a bound is part of a type), ADR 0107 D4 (the identity of an
  invocation, a repeat answered with the first answer) and D6 (the conditional write), ADR 0109 (D0 the lanes, D3
  credentials, D4 the destination policy, D7 the key ring, D8 sealing at rest, D14 the audit log and
  `sce:sensitivity`, D17 named bounds of host records and their relations, Open question 7), ADR 0113 (at most once by
  the operation's identity), ADR 0114 (section 16, the event journal), ADR 0152 and ADR 0153 (the session store's
  layouts; the blob store), ADR 0158 (the message catalog, which numbers a new section of the same document).

## Amended on acceptance

Each line is one amendment of the independent review of 2026-11-28, decided by the main session.

1. D2 is written in ADR 0153's form: an address by SHA-256 in the store, a keyed handle per holder, a grant that ends
   with its holder, `sce:max-size` on every `record:BlobRef`, `body_blob` a `record:BlobRef`, chunks of 2^20 bytes
   sealed by chunks; "a blob shared between sessions or handed to a child" leaves "Not in scope" (review item 1).
2. The identity hashed into the key gains the session's incarnation, 16 random bytes drawn when the session is
   created and kept in its saved state; section 14's token tags take it too (review item 2).
3. The host keeps the request it first sent under the identity, and every repeat sends those same bytes (review item 3).
4. A POST or PATCH without a key, started again after a restore, completes with the new outcome `outcome_unknown` and
   is never sent again; `sce-host-http-fetch` ignoring `restarted` today is a defect the order of work repays first
   (review item 4).
5. `idempotency_window_ms`, written by the deployment with no default: a repeat past it completes `outcome_unknown`
   (review item 5).
6. The repeated outcomes are the transient ones; `409` is repeated for a keyed request; `400` and `422` are answers;
   the Idempotency-Key draft is cited as work in progress with its revision (review item 6).
7. A `Retry-After` longer than `retry_max_ms`, or a wait past the deadline, completes with the last attempt's outcome;
   the resend after a `401` refresh is not counted against `retry_max` and is recorded in `attempts` (review item 7).
8. A refresh token is redeemed by a conditional write on its generation; the window a crash leaves is stated; the
   `oauth2-refresh` kind carries client authentication (review item 8).
9. D1 no longer claims that no byte moves for the transport: the transport's bounds are their own named bounds, held
   to the records' by relations `codegen/standard-host-bound-relation` checks (review item 9).
10. The new sections take the next free numbers when this lands; the session store's layouts are assigned in landing
    order across ADR 0150, ADR 0152 and ADR 0153 (review item 10 and its cross-record finding).
11. Sources added: Dennis and Van Horn for the capability discipline, Hohpe and Woolf's Claim Check, Brooker's venue,
    RFC 9110 section 9.2.2's sentence quoted (review item 11).
12. D7 cites the owner's Rust-first scope rule of 2026-11-22, under which C++ is deferred against ADR 0109 D0 (review,
    "Conflicts with other records").

## Context

An adapter that speaks to a service over HTTPS needs four things the standard HTTP operation does not give.

**The bounds are literals, written eight times.** `sce-host-http-fetch` (SCE_HOST_FEATURES.md section 5) takes
`stdlib/host/http_fetch_request.scxml` (`url` `string[2048]`, `headers` `string[2048]`, `body` `string[16384]`) and
answers `http_fetch_result.scxml` (`headers` `string[4096]`, `body` `string[16384]`). Each lane's reference host
repeats the numbers: Rust's `HEADERS_BOUND` and `HTTP_BODY_MAX_BYTES` (`backends/rust/host-features/src/http.rs`),
read again by the network transport (`backends/rust/host-http/src/lib.rs`, `max_response_bytes`); C++'s response
check (`sce/include/host/StandardHost.h`, `joined.size() > 4096 || payload.size() > 16384`) and
`CurlTransport.h`'s `maxResponseBytes = 16384`; and the Go, Kotlin, Python, Swift and C hosts (`http.go`, `Http.kt`,
`http.py`, `Http.swift`, `standard_host.c`). The network transport bounds the raw head of a response by a literal of
its own, `max_header_bytes` 65536, which is not the record's 4096. The records of signing and sealing had the same
shape and were given named bounds by ADR 0109 D17 (`bytes[cose_payload=4096]`, read by the hosts through
`NamedBounds`, `backends/rust/host-features/src/bounds.rs`); the HTTP records were not.

**A body is a text of at most 16 KiB.** A response that is not UTF-8 fails `invalid_body`, a larger one fails
`response_too_large`, and a request's body is a text of the record. There is no way to send a file or to receive
one, and nothing to build `multipart/form-data` from. ADR 0109 Open question 7 records the same gap for signing ("a
detached payload over a blob the host stores").

**A credential can only be machine data.** The host "keeps no cookies and answers no authentication challenge"
(section 5), so a document that authenticates writes `Authorization: Bearer ...` into `headers`. The token is then
in a variable, in the session's checkpoint, in its outbox if it is sent anywhere, and in every log line that writes
the variable; `sce:sensitivity="secret"` (ADR 0109 D14, `sce-build/src/forge/sensitivity.rs`) replaces it in a log
line and nowhere else. Nothing refreshes an expiring token but the document itself, by another exchange whose answer
holds the token in its text. The network transport refuses the fields that frame a message (`TRANSPORT_FIELDS`) and
none that carry a credential.

**A request is never repeated, and so a lost answer is a lost outcome.** Section 1: "A host does not repeat an
operation by itself"; section 5: the host "does not retry", the exchange "is not idempotent by itself", and a
statechart that cancels it "does not learn whether the server saw the request". A document that retries does it with
a new start of the `<invoke>`, which is a new identity (section 1), so nothing ties the second request to the first.
The identity exists (ADR 0107 D4: the session, the `<invoke>`'s id, its start number, kept across a restore), and
three operations already answer a repeat under it with their first result (`sce-host-kv-put-if`, key generation,
credential issue); HTTP is not one of them.

**A restore sends a request again that may already have been applied.** A restore starts a running invocation again
with `restarted` set (SCE Accepted Subset §2.15). No source file of `backends/rust/host-features/src` reads
`restarted`, so `sce-host-http-fetch` sends a POST again after a restore as if it were new. That is at least once
where the operation states nothing, and it is a defect of the current contract, not only of a new operation.

**The identity is not unique over time.** A keyed session that reaches a final state is removed, and the next request
for its key starts a new one (SCE_MESH.md section 14); a host may give the root session its id (SCE Accepted Subset
§2.15). So a new session can carry the id of an old one, and its starts count from 1 again: the triple
`[session id, invoke id, start number]` names two different operations. Every tag made from it (a key, a token, a kept
first result) then answers the new operation with the old one's.

SCE's earlier answer to this request, which this record keeps: the idempotency key is ADR 0107 D4's identity, and the
named bounds may land first.

## The textbook structure

- **A bound a deployment chooses is named once** and read by every party from one number (ADR 0050), so a build that
  moves it moves the buffer, the check and the host together. Two bounds that must agree are held to each other by a
  relation the build checks (ADR 0109 D17).
- **Data larger than a message is passed by reference.** A large value lives in a store, and what moves between
  parties is a reference to it, with its size and digest (the Claim Check pattern of Hohpe and Woolf, 2003); the bytes
  flow from store to network in bounded chunks with back-pressure, never whole in memory. Bytes that never change are
  named in the store by their digest (Quinlan and Dorward, Venti, FAST 2002), as ADR 0032 names a picture.
- **A name of content is not a right to read it, and a secret is held by the party that uses it.** Others hold a
  reference that the holder checks at each use (the capability discipline of Dennis and Van Horn, 1966). A store
  that tells a putter whether a content was already there tells one person what another stored (Harnik, Pinkas and
  Shulman-Peleg, 2010). An access token is short-lived and refreshed by its holder (OAuth 2.0, RFC 6749 sections 4.4
  and 6; RFC 9700, the current security practice, section 4.14 on rotating refresh tokens). A credential is bound to
  the resource it is for, so it is not presented elsewhere (RFC 8707).
- **A request may be repeated when repeating it is the same as doing it once.** RFC 9110 section 9.2.2 names the
  idempotent methods (GET, HEAD, OPTIONS, PUT, DELETE, TRACE), and says a client "SHOULD NOT automatically retry a
  request with a non-idempotent method unless it has some means to know that the request semantics are actually
  idempotent". A POST or a PATCH is made idempotent by an identity of the operation the server keeps (Birrell and
  Nelson, 1984; RIFL, Lee et al., SOSP 2015, which ADR 0113 follows), on HTTP the `Idempotency-Key` header field (an
  Internet-Draft of the IETF HTTPAPI working group, cited as work in progress). The same key must be sent for every
  repetition of one operation, with the same request (RIFL: a retried request carries the same arguments), and a
  different key for a different operation, ever.
- **At most once is decided by the caller when it cannot know.** An RPC whose answer is lost and which cannot be sent
  again reports that the call may or may not have run (Birrell and Nelson, 1984). A server keeps a key for a time it
  publishes, and a repeat past that time is a new request to it.
- **Which failures may be repeated is the protocol's: a transient one, not a permanent one.** No response leaves the
  outcome unknown; `408`, `429` (RFC 6585 section 4), `502`, `503` and `504` (RFC 9110 sections 15.6.3 to 15.6.5)
  report a condition expected to pass, and `Retry-After` (RFC 9110 section 10.2.3) is the server's advice of when; a
  `401` says the request "has not been applied" (RFC 9110 section 15.5.2). Whether a failed request was applied does
  not separate them: a gateway's `502` may follow an upstream that applied it, and a repeat is safe only because the
  request is idempotent or keyed. Waits grow exponentially and are jittered (M. Brooker, "Exponential Backoff and
  Jitter", AWS Architecture Blog, 2015), as the Mesh's retries already are.
- **The end decides** (Saltzer, Reed and Clark, 1984): whether the server honours a key is known to the author of the
  document, not to the transport, so the document states it.
- **A rotating secret is redeemed once by a conditional write.** Two holders that may redeem one refresh token agree
  through the store that keeps it, by a compare-and-set on its generation (ADR 0107 D6's conditional write), not by a
  lock of one process.
- **A multipart form is RFC 7578's**: one part per field, `Content-Disposition: form-data; name=...`, a file's part
  with its file name and media type, parts separated by a boundary that occurs in no part (RFC 2046 section 5.1.1).

## Decision

### D1. The bounds of the HTTP records are named, every host reads them, and the transport's bounds are held to them

The records' bounds become named bounds, with today's numbers as the library's:

| Record | Field | Type |
|---|---|---|
| `http_fetch_request` | `url` | `string[http_url=2048]` |
| `http_fetch_request` | `headers` | `string[http_request_headers=2048]` |
| `http_fetch_request` | `body` | `string[http_request_body=16384]` |
| `http_fetch_result` | `headers` | `string[http_response_headers=4096]` |
| `http_fetch_result` | `body` | `string[http_response_body=16384]` |

`method` (8) and `error` (24) keep their numbers: each is the length of the longest member of a closed set. Every
lane's reference host judges a request and an answer by the numbers the build resolved, in place of its literals:
Rust through `NamedBounds` and `ReferenceHost::set_named_bounds`; C++ through its bounds struct as `CoseHost.h` holds
its own; Go, Kotlin, Python, Swift and C each by the form its lane gives a machine's bounds
(`StaticTarget::limits_def`, `sce-build/src/forge/static_lowering.rs`). This is ADR 0109 D17 for HTTP.

**The transport's bounds are not the records'.** The network transport reads a response's raw head, its status line
and every field with its framing, which is longer than the record's `headers` that are made from it; and a body it
streams into a blob (D3) is not bounded by a record at all. So the transport keeps bounds of its own, named:

| Named bound | Library number | Read as | Relation the build checks |
|---|---|---|---|
| `http_response_head` | 65536 | the transport's `max_header_bytes` | `http_response_headers` at most `http_response_head` |
| `http_response_body` | 16384 | the transport's `max_response_bytes` for a body read into a record | the record's own bound |
| `http_stream_body` | 67108864 | the transport's `max_response_bytes` for a body streamed into a blob (D3) | `http_response_body` at most `http_stream_body`, at most 2^30 (ADR 0153 D7) |

Each relation is a row of the standard-feature table, checked as ADR 0109 D17's are, and a build that breaks one is
refused with `codegen/standard-host-bound-relation`, naming the two bounds, their numbers and the `--limit` that
repairs it. With the library's numbers no response that passes today fails, and none that fails today passes; a build
that moves a record's bound and not the transport's is refused, not served with a transport that disagrees.

### D2. A body larger than a record is a blob of ADR 0153's store, which a party names by its own handle

- **A blob** is ADR 0153's: immutable bytes, at most 2^30 of them, whose address in the store is their SHA-256 (ADR
  0153 D1), kept as chunks of 2^20 bytes (ADR 0153 D6) and sealed at rest by chunks under `Sealed` (ADR 0153 D8, ADR
  0109 D8). The bytes never enter the machine.
- **A statechart holds a reference**, `record:BlobRef` (`sce:std/host/blob_ref.scxml`, new): `handle` (64), `size`
  uint64, `sha256` (64, lowercase hexadecimal), `media_type` (255), `name` (255, the file name a form part gives it,
  empty for none). The platform writes `size` and `sha256` from the store, a document writes `media_type` and
  `name`, and `handle` is read by the platform alone. Every field or variable of type `record:BlobRef` carries
  `sce:max-size="<name>=<bytes>"`, the most bytes its blob may hold, with no default (ADR 0153 D7).
- **A handle is a grant to one holder, not a name of content.** It is `blob:` and the lowercase hexadecimal of the
  first 16 bytes of the HMAC-SHA-256, under the store's handle key (a `mac` key of the key ring, section 13), of the
  holder and the address (ADR 0153 D3). The store resolves a handle only under the holder that asks: another holder's
  handle, or one of no grant, is `unknown_blob`, whether or not the store holds the bytes. A handle's text carried to
  another session is `unknown_blob` there; a `record:BlobRef` carried by an event, a journal entry, a parameter or a
  result is translated into the receiver's own grant and handle (ADR 0153 D3). An address anyone can compute grants
  nothing.
- **Three operations** (Rust): `sce-host-blob-put` (`bytes` up to the named bound `blob_put`, `media_type`, `name`;
  answers the `BlobRef`), `sce-host-blob-read` (`handle`, `offset`, `length` up to `blob_read`; answers the bytes of
  that range, each chunk it covers checked, ADR 0153 D5) and `sce-host-blob-release` (`handle`; ends the session's
  grant, answers `released`). Failure codes: `unknown_blob`, `quota_exceeded`, `corrupt` (a chunk did not check, ADR
  0153 D5), `unavailable`.
- **A grant ends with its holder, and the bytes end when no grant names them** (ADR 0153 D4). The removal of a session
  ends its grants (`Write::sessions`); a release ends one sooner. The bytes are kept by ADR 0153's blob store, in the
  session store's file.
- **What a session may hold.** A deployment states `blob_bytes` (no default): the sum of the sizes of the distinct
  addresses the session's grants name. A put or a streamed body past it fails `quota_exceeded` and keeps nothing. A
  blob the store already held is charged as a new one (ADR 0153 D2, D7), so the putter learns nothing of another
  holder.
- **A blob made under an identity is made once.** The same bytes make no second blob and the same holder gets the same
  handle (ADR 0153 D2, D3), so a put or a streamed response repeated under one identity answers the first one's
  reference with nothing kept to make it so. The invocation's grant holds the blob until its result is taken in.

### D3. `sce-host-http-exchange`: one exchange that may stream

A new operation beside `sce-host-http-fetch`, which keeps its contract on every lane:

| Request `http_exchange_request` | Result `http_exchange_result` |
|---|---|
| `method` (8), `url` (`http_url`), `headers` (`http_request_headers`) | `error` (24), `status` uint16 |
| at most one of `body` (`http_request_body`), `body_blob` (`record:BlobRef`, its handle empty when unused), `form` (D4) | `headers` (`http_response_headers`) |
| `response_to_blob` bool | `body` (`http_response_body`) or `body_blob` (`record:BlobRef` with `sce:max-size` `http_stream_body`, its handle empty when unused) |
| `credential` (64, D5) | `attempts` uint8 (D6) |
| `retry_max` uint8, `retry_initial_ms` uint32, `retry_max_ms` uint32, `retry_multiplier_milli` uint16, `idempotent` bool (D6) | |

- **A request body from a blob is streamed** from the store, one chunk at a time, with `content-length` its size; each
  chunk is checked before it is sent, and one that does not check ends the exchange `corrupt` before the body is
  complete (ADR 0153 D5).
- **With `response_to_blob`, the response body is streamed into a new blob** through ADR 0153's upload, one chunk at a
  time, up to `http_stream_body` (D1); a longer one fails `response_too_large` and keeps nothing (ADR 0153 D2). The
  body may be any bytes; `invalid_body` is for a text body only. The host holds at most one chunk of a body at a time,
  whatever its size (ADR 0153 D6).
- **The rest is section 5's**: the methods, the refusals, the destination policy (ADR 0109 D4), no redirect followed,
  no cookie kept, `_sce_deadline_ms` bounding the whole operation, retries included.
- **New failure codes** of the operation: `unknown_blob`, `quota_exceeded`, `corrupt`, `credential_refused`,
  `credential_unavailable` (D5), `outcome_unknown` (D6).

### D4. A multipart form is a record whose fields are its parts

- `<invoke type="sce-host-http-exchange" sce:form="UploadForm">` names an imported event schema, and
  `<param name="form" expr="upload"/>` hands it the machine's value of that record. A request is text (SCE Accepted
  Subset §2.12), so the build writes the record's JSON into the request's `form` field (bounded by the named bound
  `http_form`) with the writer the composed lowering already has for a record, and the schema's id into a field the
  build writes and the document does not, as it writes `answer_schema` (`written_by_build`,
  `sce-build/src/forge/standard_host.rs`).
- **Each field is one part, in declaration order** (RFC 7578): a text or a number is a part of its decimal or UTF-8
  text, named by the field's id; a `record:BlobRef` is a file part with the blob's name and media type, its bytes
  streamed from the store; a `list<record:BlobRef>[N]` is several parts of one name (RFC 7578 section 4.3). A field of
  any other type is refused at the `<invoke>` (`validation/typed-invoke-request`, naming it).
- **The boundary** is the base64url of 32 bytes the host draws (43 characters of RFC 2046's alphabet). The host checks
  that no text part holds its delimiter; a streamed blob is not scanned, and the delimiter occurs in it by chance with
  probability below 2^-250 per position, which is how RFC 2046's duty is kept for a body the host does not read whole.

### D5. A credential is the host's, named by a handle, and never machine data

- **A credential is configured by the deployment** and named: the request's `credential` holds its name, never a
  secret. Its kinds are a closed set, each a row: `bearer` (a static token), `basic` (RFC 7617), `header` (a named
  header field and its value, for a key a service issues), `oauth2-client-credentials` (RFC 6749 section 4.4: a token
  endpoint, a client id and secret, a scope, a resource of RFC 8707) and `oauth2-refresh` (RFC 6749 section 6: a
  token endpoint, a refresh token the operator provisioned, and the client's authentication the token endpoint
  requires of a confidential client, RFC 6749 section 2.3).
- **A credential is sent only to its origins.** Each names the origins (scheme, host, port) it may be presented to;
  an exchange whose URL's origin is not one of them fails `credential_refused` before anything is sent, so a URL that
  came from a client cannot carry a server's credential away.
- **A document cannot write one.** On this operation, a header line named `authorization`, `proxy-authorization` or
  `cookie`, or the field a `header` credential writes, is refused (`invalid_argument`). The result leaves out
  `set-cookie`, `authentication-info` and `proxy-authentication-info`. `sce-host-http-fetch` is unchanged: its
  surface is every lane's, and section 5 says that a credential a document writes there is in its state.
- **The host refreshes.** It keeps an access token with its expiry and refreshes it once less than the credential's
  `refresh_before_s` remains (a deployment value with no default). A `401` whose `WWW-Authenticate` names the
  credential's scheme with `error="invalid_token"` (RFC 6750 section 3.1) makes it refresh once and send the request
  again: RFC 9110 section 15.5.2 says the first was not applied. A second `401` is the answer. The resend is not
  counted against `retry_max`, and `attempts` counts it (D6).
- **A rotating refresh token is redeemed once, across processes.** The store keeps each refresh token with a
  generation. A refresh reads the token and its generation, calls the token endpoint, and writes the new refresh token
  and access token by a conditional write that succeeds only while the generation is the one it read (ADR 0107 D6),
  in one transaction, before the access token is used. A process whose write is refused does not call the endpoint
  again: it reads the token the winner wrote. Within one process one refresh per credential is in flight, and every
  exchange waiting on it uses its result. So two processes sharing the store do not both redeem one token, which RFC
  9700 section 4.14.2's reuse detection would answer by revoking the grant.
- **The window that cannot be closed.** A process that stops after the token endpoint answered and before its write
  committed has redeemed the token and kept nothing; the store still holds the spent token. The next refresh is
  refused `invalid_grant`, and the credential fails `credential_unavailable` until the operator provisions it again.
  The token endpoint's answer cannot be made part of the store's transaction; a metric and the runbook say what the
  operator does.
- A token endpoint that answers `invalid_grant`, or no answer, fails `credential_unavailable`.
- **Where a secret is.** In the host's outbound credential store (a contract of `sce-rust-session-store`,
  `SqliteStore` in a layout assigned as D8 says, sealed at rest under `Sealed`) and in the host's memory while it is
  used, in a type whose `Debug` writes `sce-secret` and which is wiped when dropped, as the key ring keeps its material
  (section 13). Never in a result record, a checkpoint, an outbox, a kept request (D6), a log line, the audit log, or a
  metric's label or value.

Section 14's tokens are another matter, and this record does not move them out of machine data: a token a statechart
issues to a client passes through the statechart to be sent, and so through its checkpoint and outbox, sealed by ADR
0109 D8. Their tags take the incarnation (D6).

### D6. A request is repeated only as it states, with the bytes it first sent, under a key no other operation has

- **The identity is unique over time.** Section 1's identity gains a fourth member, the session's **incarnation**:
  16 bytes drawn from the random source (section 11) when a root session is created, kept in its saved state (SCE
  Accepted Subset §2.15), and handed to the host with the other three. A restore keeps it; a new session, under any
  id, draws another. A child session takes its root's: a child's id is derived from the tree, and so is unique within
  one incarnation of its root. A target with no source of randomness takes it from the host, as it takes the root's id.
  Every operation that keeps or tags the identity takes it: a key's tag below, a put-if's first result (section 3), key
  generation (section 13) and a token's tag, the deterministic CBOR array `[session id, incarnation, invoke id, start,
  name]` (section 14). A saved state of the format before has no incarnation; its sessions keep the identity they were
  started under, the array without the member, so a repeat after the change answers what the first answered.
- **The document states the retry**: `retry_max` (0, none, to 10), `retry_initial_ms`, `retry_max_ms` and
  `retry_multiplier_milli` (at least 1000). The host computes each wait with the standard library's
  `retry_next_backoff` and `retry_jittered` (`stdlib/mesh/`), the functions the Mesh's sends use, drawing from its
  random source. A `Retry-After` longer than the computed wait and at most `retry_max_ms` is honoured.
- **When a wait is not made.** A `Retry-After` longer than `retry_max_ms`, or a wait that would end past
  `_sce_deadline_ms`, is not waited: the operation completes with the last attempt's outcome, its `status`,
  `headers`, `body` and `attempts`, as it would have after its last allowed attempt. `Retry-After` is advice (RFC 9110
  section 10.2.3), and the document reads the status and decides.
- **What is repeated: a transient failure.** An attempt that got no response (`network`) and the statuses `408`,
  `429`, `502`, `503` and `504`, each a condition expected to pass. For a request with `idempotent` true, also `409`,
  by which the Idempotency-Key draft says that a request with this key is still being processed. Nothing else: a
  `500` reports a condition the server did not expect, which a wait does not change; `400` and `422`, by which the
  draft answers a key missing or a key reused with another request, are answers; any other status is the answer. The
  last attempt's outcome is the operation's.
- **What `attempts` counts.** Every request the host sent for the operation: each attempt, and the one resend after a
  `401` refresh (D5), which is not counted against `retry_max`.
- **Which requests are repeated**: GET, HEAD, OPTIONS, PUT and DELETE, idempotent by RFC 9110 section 9.2.2. A POST or
  a PATCH only with `idempotent` true, by which the document states that the server takes the key below; a POST or
  PATCH with `retry_max` above 0 and `idempotent` false is refused (`invalid_argument`) before anything is sent.
- **The key is the invocation's.** Every attempt of a request with `idempotent` true carries `Idempotency-Key`, a
  string of RFC 9651 holding the base64url, without padding, of the HMAC-SHA-256, under the key ring's idempotency key
  (a `mac` key the host generates once and keeps, section 13), of the deterministic CBOR array
  `[session id, incarnation, invoke id, start number]`: the identity of the invocation (section 1, ADR 0107 D4). Every
  attempt of one invocation sends one key; a restore that starts the invocation again under its number sends it again;
  another start sends another; another session never sends it. The key names no session to the server, as section
  14's token names none.
- **The request is kept, and a repeat sends the same bytes.** Before its first attempt is sent, the host writes the
  request it is about to send (method, URL, the document's header lines, the text body, or the address of a body blob
  whose grant the invocation holds, ADR 0153 D3) and the host clock's time under the identity, in the store that keeps
  the session's checkpoints, so a restore finds it wherever it finds the checkpoint. A start under an identity that
  has a kept request is a repeat, with `restarted` set or not (a start the document makes after a restore from a
  checkpoint older than the first start takes the same number): the host sends the kept request, not the one the
  start hands it, whose parameters were evaluated again against restored data. The kept request holds no secret, since
  a credential is a name and its header is written at each attempt (D5); it is sealed at rest under `Sealed`. It is
  dropped when the invocation ends (its result taken in, or its cancellation). A start under an identity with no kept
  request was never sent, since the keeping precedes the send, and is a first start.
- **A request without a key is sent at most once across a restore.** A repeat of a POST or a PATCH with `idempotent`
  false, a start under an identity whose kept request exists, is not sent: the operation completes with the failure
  `outcome_unknown` (Birrell and Nelson, 1984: the call may or may not have run). A repeat of a request of an
  idempotent method sends the kept request.
- **The server's window is named.** A server keeps a key for a time it publishes. The deployment writes
  `idempotency_window_ms`, the time the servers it names keep a key, with no default: a host that serves this
  operation is not made without it. No attempt of a request with `idempotent` true starts later than
  `idempotency_window_ms` after its kept request's time: a retry or a restore past it completes `outcome_unknown` and
  sends nothing. Keeping a record longer on SCE's side does not make the server's effect happen once (ADR 0113's
  Context).
- **`outcome_unknown` is the one failure that does not say nothing changed.** Section 1's failure row says nothing the
  operation was asked to change has changed; this code says the host cannot tell, and the document decides what to do,
  as it decides after a cancellation (section 5).
- **`sce-host-http-fetch` takes the same rule on a restore.** It keeps no request and has no key. A start with
  `restarted` set of a POST or a PATCH is not sent and completes `outcome_unknown`, a new code of its closed set on
  every lane that serves section 5; a start with `restarted` set of another method is sent as a first start. Today's
  hosts ignore `restarted` and send a POST again; that is the defect step 2 repays.
- **What it does not promise.** A server that does not keep the key may apply a repeat. The document says it does
  (`idempotent` true); the host cannot know. A server whose window is shorter than `idempotency_window_ms` says may
  apply a repeat inside it; the deployment states the window, the host cannot measure it.
- Section 1's rule becomes: a host does not repeat an operation by itself, except as the operation's request states
  and once after a credential refresh a `401` calls for (D5); every request it sends is counted in the result where the
  operation has `attempts`. `sce-host-http-fetch` still repeats nothing.

### D7. Lanes

D1, the incarnation and the restart rule of `sce-host-http-fetch` (D6) land on every lane that serves section 5, all
seven: the saved state is one text on every backend (SCE Accepted Subset §2.15), and section 5 is every lane's.
`sce-host-http-exchange` and the three blob operations are rows of the standard types table
(`sce-build/src/forge/standard_host.rs`) served by Rust, so a build for another lane refuses a machine that runs one
with the existing `codegen/standard-host-type-not-served`. ADR 0109 D0 builds a new facility for Rust and C++; this
record builds the new operations for Rust alone under the owner's scope rule of 2026-11-22 (built on the backend a
consumer needs, Rust first; the contract is general). C++, ADR 0109's second lane, joins in the change that lands its
host and its scenarios, when a consumer needs it.

### D8. The session store's layouts

D5's credential table and D6's kept requests are tables of the session store. ADR 0152 (the changelog) and ADR 0153
(the blob store) add tables too. Each takes the next layout number after the highest `sqlite.rs` defines when it
lands (`LAYOUT_14` at this acceptance), in landing order across ADR 0150, ADR 0152 and ADR 0153; no record fixes a
number in advance.

### Not in scope

- Signing or sealing a blob (ADR 0109 Open question 7), which can now name one.
- A blob from a person's file picker (a platform's host).
- Cookies, client certificates (mutual TLS), DPoP proof-of-possession, and a credential a document supplies at run
  time.
- Moving section 14's issued tokens out of machine data.
- `sce-host-http-fetch` refusing credential headers, which would change a surface every lane serves.

## Consequences

- A build moves the HTTP bounds as it moves every named bound, no lane's host disagrees with its machine's records,
  and a build whose transport bound disagrees with a record's is refused.
- A file of megabytes is sent and received without passing through a machine or a record, and a form with a file is
  one `<invoke>`. A blob lives as long as a grant holds it, and a reference crosses between sessions by translation.
- No saved state, outbox, kept request, log line or snapshot of a session holds an outbound credential; a token
  expires and is refreshed without the machine, and two processes do not spend one refresh token twice.
- A request whose response is lost is sent again with the same bytes under the same key, and a restore sends it again
  with the same bytes under the same key, so a server that keeps keys applies it once. No new session sends an old
  session's key.
- A POST that was running when the process stopped is no longer sent a second time: it completes `outcome_unknown`,
  on every lane, for `sce-host-http-fetch` as for the new operation.
- The saved state gains the incarnation on every backend; a token, a key and a put-if's first result are tagged with
  it.
- The session store gains two tables of this record, in layouts assigned in landing order (D8), and a host process the
  credentials and blobs a deployment gives it.

## Alternatives rejected

- **Widening `sce-host-http-fetch`.** Its records are every lane's; a field added there must be served by all seven
  at once (ADR 0109 D0), and streaming, credentials and blobs are needed on Rust.
- **A body as a list of chunks in the machine.** The machine would hold and checkpoint the bytes it only forwards.
- **A blob owned by its session** (this record as proposed). A fact outlives its session (ADR 0153), and a reference
  could not cross to another holder.
- **The digest as the name a party asks by.** It grants nothing by itself and must be checked against grants anyway; a
  handle per holder does that and ties no URL to the content (ADR 0153).
- **A secret marked `sce:sensitivity="secret"` and kept in the machine.** The mark redacts a log line; the checkpoint,
  the outbox and the snapshot would still hold the secret.
- **One refresh in flight per process as the only guard.** Two processes sharing the store would both redeem one
  rotating token.
- **The host retrying every failure, or every method.** A POST repeated without a key the server keeps can apply
  twice; the end decides.
- **A retry made by the statechart as a new start.** A new start is a new identity, so the key would change and the
  server could not tell the repeat.
- **A key drawn at random per attempt, or per start and kept in the machine.** The first breaks the guarantee; the
  second is machine data the identity already is.
- **The triple identity without an incarnation.** A session made again under an old id sends an old key for a new
  operation.
- **Evaluating the request again on a restart.** The data may have changed, and a server would see two requests under
  one key.
- **Sending a keyless POST again after a restore.** That is at least once where the document asked for nothing.
- **The session id in clear as the key.** It names the session to every server the machine talks to.
- **Retrying a `500`.** It reports a condition the server did not expect, which a wait does not change.

## Order of work

Each step lands with a test that fails without it.

1. This record, accepted.
2. **The restart rule of `sce-host-http-fetch` (D6), on every lane.** The defect is repaid first. Test: a scenario of
   section 8 that saves a session with a POST running, restores it, and expects `outcome_unknown` with no second
   request at the scripted server, and a GET in the same place sent again; replayed by every lane's driver.
3. **The incarnation (D6), on every lane.** The saved state, the identity handed to the host, and sections 3, 13 and
   14. Tests: a session removed and made again under the same id issues a different token and generates a different
   key than the first under the same invoke id and start (Rust `backends/rust/host-features/tests/credentials.rs`, and
   C++ `AStandardHostCredentials`, which agree byte for byte); a saved state of the format before restores with the
   identity it had.
4. **D1, the named bounds, on every lane.** The two library records, each host's reading of the numbers, the
   transport's bounds and their relations. Tests: `a_build_moves_the_http_bounds_and_the_host_judges_by_them` (Rust,
   `backends/rust/host-features/tests`: a host given `http_response_body=65536` answers a 20000-byte body that today
   fails `response_too_large`; C++ the same, a case of `AStandardHostNamedBounds`),
   `a_statechart_reads_a_bound_by_its_name_and_a_host_reads_its_constant`
   (`sce-build/tests/a_limit_moves_every_bound_that_names_it.rs`) extended to the five names on every backend, and a
   build with `http_response_headers` above `http_response_head` refused with `codegen/standard-host-bound-relation`.
   The two records are read by committed generated trees, whose source hashes move (ADR 0094).
5. **D3 with text bodies and D6.** The operation, its records, the retry, the kept request and the key. Test:
   `a_lost_response_does_not_write_twice` (a scripted server that keeps keys and counts its writes; the first response
   is lost after the server applied the request, and the second attempt is answered from the server's record; then the
   process stops after sending, the session is restored from its checkpoint, its data changed, and the invocation
   started again under its number; one write in each case, every attempt with one key and the same body; a POST with
   `retry_max` 1 and `idempotent` false refused before it is sent; a keyless POST restored completes `outcome_unknown`
   with one request seen; a restore past `idempotency_window_ms` completes `outcome_unknown` with nothing sent; a
   `409` repeated for a keyed request and a `422` answered; a `Retry-After` past `retry_max_ms` completes with that
   attempt's status). Without the operation the machine does not build.
6. **D5.** Tests: `no_saved_state_log_or_snapshot_holds_a_credential` (a credential whose secret is a canary; a
   machine that exchanges under it through a refresh, a `401` with `invalid_token` and the request sent again,
   `attempts` 2 with `retry_max` 0; every checkpoint, outbox, kept request, audit entry, log line, metric, `Debug` text
   of the host, the machine and the result, and every byte of a `Sealed` store's file, searched for the canary as
   written, in base64, base64url and hexadecimal; an unsealed store holds it in its outbound credential table and
   nowhere else); `a_credential_is_sent_only_to_its_origins`; `one_refresh_serves_every_exchange_that_waits_on_it`
   (two exchanges, one call to the token endpoint); `two_processes_redeem_a_refresh_token_once` (two hosts over one
   store, one call to the token endpoint, both using the token it answered).
7. **D2 and the streaming of D3, on ADR 0153's store.** Tests:
   `a_response_larger_than_a_record_streams_to_a_blob_within_its_bound` (64 MiB in chunks; the largest write the store
   sees is one chunk; one byte past `http_stream_body` fails and keeps nothing);
   `a_blob_put_repeated_under_one_identity_is_stored_once`;
   `a_blob_handle_is_its_holders_alone` (a handle's text given to another session is `unknown_blob`).
8. **D4.** Test: `a_form_is_sent_as_rfc_7578_multipart_with_its_blob_streamed` (a fixed random source, the body
   compared byte for byte with the expected one), and the refusal of a field of another type.
9. **D7's refusal** for each other lane, and the documents named in "Amends".

## Open questions

Decided, each with its reason; the acceptance kept each, amended where "Amended on acceptance" says.

1. **Does `sce-host-http-fetch` refuse credential headers?** No (D5): it is a surface every lane serves (ADR 0109 D0),
   and the new operation is the way to keep a credential out of the machine.
2. **Is a blob named by its content?** Inside the store, yes; to a party, no: a party asks by a handle under its
   grants (D2, ADR 0153 D1 and D3).
3. **Which outcomes are repeated?** No response, `408`, `429`, `502`, `503`, `504`, and `409` for a keyed request
   (D6): the transient ones.
4. **Who says a POST may be repeated?** The document, by `idempotent` (D6): the end decides, and RFC 9110 section
   9.2.2 asks the client to know.
5. **Does a refresh after a `401` count as an attempt?** It is not counted against `retry_max`: the request was not
   applied, and the refresh is the credential's, once per attempt. `attempts` records the resend (D6).
6. **The library numbers of the new bounds**: `http_stream_body` 64 MiB (below ADR 0153's ceiling of 2^30, on the
   `BlobRef` field's `sce:max-size`), `http_response_head` 65536, `blob_put` and `blob_read` 16384, `http_form`
   16384; each is named, so a build moves it.

## Sources

- RFC 9110, HTTP Semantics (sections 9.2.1, 9.2.2, 10.2.3, 15.5.2, 15.5.9, 15.6.3 to 15.6.5); RFC 6585 section 4.
- J. Jena and S. Dalal, "The Idempotency-Key HTTP Header Field", Internet-Draft
  `draft-ietf-httpapi-idempotency-key-header`, IETF HTTPAPI working group, work in progress. This record follows the
  revision current on 2026-11-28, the day of its acceptance; the landing of step 5 writes that revision's number and
  date here and in SCE_HOST_FEATURES.md. A later revision is taken by a record that amends this one.
- RFC 9651, Structured Field Values for HTTP.
- RFC 6749, The OAuth 2.0 Authorization Framework (sections 2.3, 4.4 and 6); RFC 6750, Bearer Token Usage (section
  3.1); RFC 8707, Resource Indicators for OAuth 2.0; RFC 9700, Best Current Practice for OAuth 2.0 Security (sections
  4.14 and 4.14.2); RFC 7617, Basic.
- RFC 7578, Returning Values from Forms: multipart/form-data (section 4.3); RFC 2046 section 5.1.1.
- A. D. Birrell and B. J. Nelson, "Implementing Remote Procedure Calls", ACM TOCS 2(1), 1984.
- C. Lee, S. J. Park, A. Kejriwal, S. Matsushita, J. Ousterhout, "Implementing Linearizability at Large Scale and Low
  Latency" (RIFL), SOSP 2015.
- J. H. Saltzer, D. P. Reed, D. D. Clark, "End-to-End Arguments in System Design", ACM TOCS 2(4), 1984.
- J. B. Dennis and E. C. Van Horn, "Programming Semantics for Multiprogrammed Computations", CACM 9(3), 1966.
- S. Quinlan and S. Dorward, "Venti: a new approach to archival storage", FAST 2002.
- D. Harnik, B. Pinkas and A. Shulman-Peleg, "Side Channels in Cloud Services: Deduplication in Cloud Storage", IEEE
  Security and Privacy 8(6), 2010.
- G. Hohpe and B. Woolf, *Enterprise Integration Patterns*, Addison-Wesley, 2003 (Claim Check).
- M. Brooker, "Exponential Backoff and Jitter", AWS Architecture Blog, 2015.
- ADR 0032, ADR 0050, ADR 0094, ADR 0107 D4 and D6, ADR 0109, ADR 0113, ADR 0114, ADR 0152, ADR 0153 and ADR 0158 in
  this repository.
