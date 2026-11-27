# ADR 0150 — An outbound HTTP exchange is bounded by name, carries large bodies and credentials by handle, and is retried only under its invocation's key

- Status: Proposed (2026-11-27), for the owner's acceptance. Written under the owner's standing rule: the most
  long-term correct structure, textbook in its foundations, cost ignored; decided, not asked. Each open question is
  decided below by the textbook rule, and the acceptance may overturn any of them.
- Date: 2026-11-27
- Amends, once accepted (each amended document gains an "Amended by: ADR 0150" line in the landing that moves it):
  `docs/SCE_HOST_FEATURES.md` section 1 (a host repeats an operation only as its request states, D6), section 5 (the
  named bounds, D1; the new operation, D3 to D6), sections 7 to 9 (the lanes, the scenarios, what is not provided), and
  two new sections, blobs (D2) and outbound credentials (D5), numbered after ADR 0148's section 16;
  `stdlib/host/http_fetch_request.scxml` and `http_fetch_result.scxml` (D1); ADR 0109 Open question 7 (a store of
  blobs now exists; signing one stays a record of its own).
- Scope: the contract names no language. D1 changes two records every lane's reference host serves, so it lands on
  all seven. D2 to D6 are built in the Rust host core (`sce-rust-host-features`, `sce-rust-host-http`,
  `sce-rust-session-store`) and `sce-build`; every other lane refuses the new operations by name (D7).
- Related: ADR 0050 (a bound read by name), ADR 0077 (a bound is part of a type), ADR 0107 D4 (the identity of an
  invocation, a repeat answered with the first answer), ADR 0109 (D3 credentials, D4 the destination policy, D7 the key
  ring, D8 sealing at rest, D14 the audit log and `sce:sensitivity`, D17 named bounds of host records, Open question
  7), ADR 0113 (at most once by the operation's identity).

## Context

An adapter that speaks to a service over HTTPS needs four things the standard HTTP operation does not give.

**The bounds are literals, written eight times.** `sce-host-http-fetch` (SCE_HOST_FEATURES.md section 5) takes
`stdlib/host/http_fetch_request.scxml` (`url` `string[2048]`, `headers` `string[2048]`, `body` `string[16384]`) and
answers `http_fetch_result.scxml` (`headers` `string[4096]`, `body` `string[16384]`). Each lane's reference host
repeats the numbers: Rust's `HEADERS_BOUND` and `HTTP_BODY_MAX_BYTES` (`backends/rust/host-features/src/http.rs`),
read again by the network transport (`backends/rust/host-http/src/lib.rs`, `max_response_bytes`); C++'s response
check (`sce/include/host/StandardHost.h`, `joined.size() > 4096 || payload.size() > 16384`) and
`CurlTransport.h`'s `maxResponseBytes = 16384`; and the Go, Kotlin, Python, Swift and C hosts (`http.go`, `Http.kt`,
`http.py`, `Http.swift`, `standard_host.c`). The records of signing and sealing had the same shape and were given
named bounds by ADR 0109 D17 (`bytes[cose_payload=4096]`, read by the hosts through `NamedBounds`,
`backends/rust/host-features/src/bounds.rs`); the HTTP records were not.

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

SCE's earlier answer to this request, which this record keeps: the idempotency key is ADR 0107 D4's identity, and the
named bounds may land first.

## The textbook structure

- **A bound a deployment chooses is named once** and read by every party from one number (ADR 0050), so a build that
  moves it moves the buffer, the check and the host together.
- **Data larger than a message is passed by reference.** A large value lives in a store, and what moves between
  parties is a handle to it, with its size and digest; the bytes flow from store to network in bounded chunks with
  back-pressure, never whole in memory (stream processing; content stores named by a digest, as ADR 0032 names a
  picture).
- **A secret is held by the party that uses it, and others hold a reference** (the capability discipline; a
  credential's holder presents it, a caller names it). An access token is short-lived and refreshed by its holder
  (OAuth 2.0, RFC 6749 sections 4.4 and 6; RFC 9700, the current security practice, section 4.14 on rotating refresh
  tokens). A credential is bound to the resource it is for, so it is not presented elsewhere (RFC 8707).
- **A request may be repeated when repeating it is the same as doing it once.** RFC 9110 section 9.2.2 names the
  idempotent methods (GET, HEAD, OPTIONS, PUT, DELETE, TRACE); a POST or a PATCH is made idempotent by an identity of
  the operation the server keeps (Birrell and Nelson, 1984; RIFL, Lee et al., SOSP 2015, which ADR 0113 follows), on
  HTTP the `Idempotency-Key` header field (an Internet-Draft of the IETF HTTPAPI working group,
  `draft-ietf-httpapi-idempotency-key-header`). The same key must be sent for every repetition of one operation and a
  different one for a different operation.
- **Which failures may be repeated is the protocol's.** No response at all leaves the outcome unknown; `408`, `429`
  (RFC 6585 section 4), `502`, `503` and `504` are answers that the request may be sent again, `Retry-After` (RFC 9110
  section 10.2.3) says when; a `401` says the request "has not been applied" (RFC 9110 section 15.5.2). Waits grow
  exponentially and are jittered (Brooker, "Exponential Backoff and Jitter", 2015), as the Mesh's retries already are.
- **The end decides** (Saltzer, Reed and Clark, 1984): whether the server honours a key is known to the author of the
  document, not to the transport, so the document states it.
- **A multipart form is RFC 7578's**: one part per field, `Content-Disposition: form-data; name=...`, a file's part
  with its file name and media type, parts separated by a boundary that occurs in no part (RFC 2046 section 5.1.1).

## Decision

### D1. The bounds of the HTTP records are named, and every host reads them

The records' bounds become named bounds, with today's numbers as the library's, so no byte moves:

| Record | Field | Type |
|---|---|---|
| `http_fetch_request` | `url` | `string[http_url=2048]` |
| `http_fetch_request` | `headers` | `string[http_request_headers=2048]` |
| `http_fetch_request` | `body` | `string[http_request_body=16384]` |
| `http_fetch_result` | `headers` | `string[http_response_headers=4096]` |
| `http_fetch_result` | `body` | `string[http_response_body=16384]` |

`method` (8) and `error` (24) keep their numbers: each is the length of the longest member of a closed set. Every
lane's reference host judges a request and an answer by the numbers the build resolved, in place of its literals:
Rust through `NamedBounds` and `ReferenceHost::set_named_bounds`, the network transport's `max_response_bytes` and
`max_header_bytes` taken from the same two; C++ through its bounds struct as `CoseHost.h` holds its own; Go, Kotlin,
Python, Swift and C each by the form its lane gives a machine's bounds (`StaticTarget::limits_def`,
`sce-build/src/forge/static_lowering.rs`). This is ADR 0109 D17 for HTTP, and it lands first and alone.

### D2. A body larger than a record is a blob the host keeps, named by a handle

- **A blob** is an immutable byte string with a media type, a size and its SHA-256, in the host's blob store. A
  statechart holds a `record:BlobRef` (`sce:std/host/blob_ref.scxml`, new): `handle` (64), `size` uint64, `sha256`
  (64, lowercase hexadecimal), `media_type` (255), `name` (255, the file name a form part gives it, empty for none).
  The bytes never enter the machine.
- **A handle is a grant, not a name of content.** It is `blob:` and 32 lowercase hexadecimal digits of 16 bytes the
  host draws (section 11), valid in the session it was answered to and nowhere else: a handle carried to another
  session or a child is `unknown_blob` there. A digest anyone can compute grants nothing.
- **Three operations** (Rust): `sce-host-blob-put` (`bytes` up to the named bound `blob_put`, `media_type`, `name`;
  answers the `BlobRef`), `sce-host-blob-read` (`handle`, `offset`, `length` up to `blob_read`; answers the bytes of
  that range) and `sce-host-blob-release` (`handle`; answers `released`). Failure codes: `unknown_blob`,
  `quota_exceeded`, `unavailable`.
- **A blob ends with its session.** It is released by the statechart or deleted in the transaction that deletes its
  session. A checkpointed session's blobs are kept in the session store's file beside its checkpoint (`SqliteStore`, a
  new layout), sealed at rest under `Sealed` (ADR 0109 D8). A deployment states the bytes a session may hold
  (`blob_bytes`, no default), past which a put or a streamed body fails `quota_exceeded` and keeps nothing.
- **A blob made under an identity is made once.** The store keeps the invocation's identity (section 1) with the blob
  it made, so a put or a streamed response repeated under one identity, as a restore starts it again, answers the
  handle of the first and stores nothing more, as key generation does (section 13).

### D3. `sce-host-http-exchange`: one exchange that may stream

A new operation beside `sce-host-http-fetch`, which keeps its contract on every lane:

| Request `http_exchange_request` | Result `http_exchange_result` |
|---|---|
| `method` (8), `url` (`http_url`), `headers` (`http_request_headers`) | `error` (24), `status` uint16 |
| at most one of `body` (`http_request_body`), `body_blob` (64, a handle), `form` (D4) | `headers` (`http_response_headers`) |
| `response_to_blob` bool | `body` (`http_response_body`) or `body_blob` (`record:BlobRef`, its handle empty when unused) |
| `credential` (64, D5) | `attempts` uint8 (D6) |
| `retry_max` uint8, `retry_initial_ms` uint32, `retry_max_ms` uint32, `retry_multiplier_milli` uint16, `idempotent` bool (D6) | |

- **A request body from a blob is streamed** from the store in chunks, with `content-length` its size.
- **With `response_to_blob`, the response body is streamed into a new blob** in chunks, up to the named bound
  `http_stream_body` (library number 67108864); a longer one fails `response_too_large` and keeps nothing. The body
  may be any bytes; `invalid_body` is for a text body only. The host holds at most one chunk of a body at a time,
  whatever its size.
- **The rest is section 5's**: the methods, the refusals, the destination policy (ADR 0109 D4), no redirect followed,
  no cookie kept, `_sce_deadline_ms` bounding the whole operation, retries included.
- **New failure codes** of the operation: `unknown_blob`, `quota_exceeded`, `credential_refused`,
  `credential_unavailable` (D5).

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
  refresh token the operator provisioned).
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
  again: RFC 9110 section 15.5.2 says the first was not applied. One refresh per credential is in flight at a time, and
  every exchange waiting on it uses its result, so a rotating refresh token is spent once (RFC 9700 section 4.14); the
  new refresh token replaces the old in the store in one transaction before the access token it came with is used. A
  token endpoint that answers `invalid_grant`, or no answer, fails `credential_unavailable`.
- **Where a secret is.** In the host's outbound credential store (a contract of `sce-rust-session-store`, `SqliteStore`
  in a new layout, sealed at rest under `Sealed`) and in the host's memory while it is used, in a type whose `Debug`
  writes `sce-secret` and which is wiped when dropped, as the key ring keeps its material (section 13). Never in a
  result record, a checkpoint, an outbox, a log line, the audit log, or a metric's label or value.

Section 14's tokens are another matter, and this record does not move them: a token a statechart issues to a client
passes through the statechart to be sent, and so through its checkpoint and outbox, sealed by ADR 0109 D8.

### D6. A request is repeated only as it states, and every repeat carries its invocation's key

- **The document states the retry**: `retry_max` (0, none, to 10), `retry_initial_ms`, `retry_max_ms` and
  `retry_multiplier_milli` (at least 1000). The host computes each wait with the standard library's
  `retry_next_backoff` and `retry_jittered` (`stdlib/mesh/`), the functions the Mesh's sends use, drawing from its
  random source; a longer `Retry-After` is honoured; no attempt starts past `_sce_deadline_ms`.
- **What is repeated**: an attempt that got no response (`network`) and the statuses `408`, `429`, `502`, `503`
  and `504`. Nothing else: a `500` may have applied the request, and any other status is the answer. The last
  attempt's outcome is the operation's, and `attempts` says how many were made.
- **Which requests are repeated**: GET, HEAD, OPTIONS, PUT and DELETE, idempotent by RFC 9110 section 9.2.2. A POST or
  a PATCH only with `idempotent` true, by which the document states that the server takes the key below; a POST or
  PATCH with `retry_max` above 0 and `idempotent` false is refused (`invalid_argument`) before anything is sent.
- **The key is the invocation's.** Every attempt of a request with `idempotent` true carries `Idempotency-Key`, a
  string of RFC 9651 holding the base64url, without padding, of the HMAC-SHA-256, under the key ring's idempotency key
  (a `mac` key the host generates once and keeps, section 13), of the deterministic CBOR array
  `[session id, invoke id, start number]`: the identity of the invocation (section 1, ADR 0107 D4). Every attempt of
  one invocation sends one key; a restore that starts the invocation again under its number sends it again; another
  start sends another. The key names no session to the server, as section 14's token names none.
- **What it does not promise.** A server that does not keep the key may apply a repeat. The document says it does
  (`idempotent` true); the host cannot know.
- Section 1's rule becomes: a host does not repeat an operation by itself, except as the operation's request states.
  `sce-host-http-fetch` still repeats nothing.

### D7. Lanes

D1 lands on every lane that serves section 5, all seven. `sce-host-http-exchange` and the three blob operations are
rows of the standard types table (`sce-build/src/forge/standard_host.rs`) served by Rust, so a build for another lane
refuses a machine that runs one with the existing `codegen/standard-host-type-not-served`. C++, ADR 0109's second
lane, joins in the change that lands its host and its scenarios, when a consumer needs it.

### Not in scope

- Signing or sealing a blob (ADR 0109 Open question 7), which can now name one.
- A blob shared between sessions or handed to a child; a blob from a person's file picker (a platform's host).
- Cookies, client certificates (mutual TLS), DPoP proof-of-possession, and a credential a document supplies at run
  time.
- Moving section 14's issued tokens out of machine data.
- `sce-host-http-fetch` refusing credential headers, which would change a surface every lane serves.

## Consequences

- A build moves the HTTP bounds as it moves every named bound, and no lane's host disagrees with its machine's records.
- A file of megabytes is sent and received without passing through a machine or a record, and a form with a file is
  one `<invoke>`.
- No saved state, outbox, log line or snapshot of a session holds an outbound credential; a token expires and is
  refreshed without the machine.
- A request whose response is lost is sent again under the same key, and a restore sends it again under the same key,
  so a server that keeps keys applies it once.
- The session store gains two tables and a layout, and a host process the credentials and blobs a deployment gives it.

## Alternatives rejected

- **Widening `sce-host-http-fetch`.** Its records are every lane's; a field added there must be served by all seven
  at once (ADR 0109 D0), and streaming, credentials and blobs are needed on Rust.
- **A body as a list of chunks in the machine.** The machine would hold and checkpoint the bytes it only forwards.
- **A blob named by its digest.** Anyone who knows the content could name it in another session.
- **A secret marked `sce:sensitivity="secret"` and kept in the machine.** The mark redacts a log line; the checkpoint,
  the outbox and the snapshot would still hold the secret.
- **The host retrying every failure, or every method.** A POST repeated without a key the server keeps can apply
  twice; the end decides.
- **A retry made by the statechart as a new start.** A new start is a new identity, so the key would change and the
  server could not tell the repeat.
- **A key drawn at random per attempt, or per start and kept in the machine.** The first breaks the guarantee; the
  second is machine data the identity already is.
- **The session id in clear as the key.** It names the session to every server the machine talks to.
- **Retrying a `500`.** It may have been applied.

## Order of work

Each step lands with a test that fails without it.

1. This record, accepted.
2. **D1, the named bounds, on every lane.** The two library records, each host's reading of the numbers. Tests:
   `a_build_moves_the_http_bounds_and_the_host_judges_by_them` (Rust, `backends/rust/host-features/tests`: a host
   given `http_response_body=65536` answers a 20000-byte body that today fails `response_too_large`; C++ the same,
   a case of `AStandardHostNamedBounds`), and `a_statechart_reads_a_bound_by_its_name_and_a_host_reads_its_constant`
   (`sce-build/tests/a_limit_moves_every_bound_that_names_it.rs`) extended to the five names on every backend. The
   two records are read by committed generated trees, whose source hashes move (ADR 0094).
3. **D3 with text bodies and D6.** The operation, its records, the retry and the key. Test:
   `a_lost_response_does_not_write_twice` (a scripted server that keeps keys and counts its writes; the first
   response is lost after the server applied the request, and the second attempt is answered from the server's record;
   then the process stops after sending, the session is restored from its checkpoint and the invocation started
   again under its number; one write in each case, every attempt with one key; and a POST with `retry_max` 1 and
   `idempotent` false refused before it is sent). Without the operation the machine does not build.
4. **D5.** Tests: `no_saved_state_log_or_snapshot_holds_a_credential` (a credential whose secret is a canary; a
   machine that exchanges under it through a refresh, a `401` with `invalid_token` and the request sent again; every
   checkpoint, outbox, audit entry, log line, metric, `Debug` text of the host, the machine and the result, and every
   byte of a `Sealed` store's file, searched for the canary as written, in base64, base64url and hexadecimal; an
   unsealed store holds it in its outbound credential table and nowhere else);
   `a_credential_is_sent_only_to_its_origins`; `one_refresh_serves_every_exchange_that_waits_on_it` (two exchanges,
   one call to the token endpoint).
5. **D2 and the streaming of D3.** Tests: `a_response_larger_than_a_record_streams_to_a_blob_within_its_bound` (64 MiB
   in chunks; the largest write the store sees is one chunk; one byte past `http_stream_body` fails and keeps
   nothing); `a_blob_is_gone_with_its_session`; `a_blob_put_repeated_under_one_identity_is_stored_once`.
6. **D4.** Test: `a_form_is_sent_as_rfc_7578_multipart_with_its_blob_streamed` (a fixed random source, the body
   compared byte for byte with the expected one), and the refusal of a field of another type.
7. **D7's refusal** for each other lane, and the documents named in "Amends".

## Open questions

Decided here, each with its reason; the acceptance may overturn any.

1. **Does `sce-host-http-fetch` refuse credential headers?** No (D5): it is a surface every lane serves, and the new
   operation is the way to keep a credential out of the machine.
2. **Is a blob named by its content?** No (D2): a handle is a grant to one session.
3. **Which outcomes are repeated?** No response, `408`, `429`, `502`, `503`, `504` (D6): the ones the protocol says
   may be sent again.
4. **Who says a POST may be repeated?** The document, by `idempotent` (D6): the end decides.
5. **Does a refresh after a `401` count as an attempt?** No: the request was not applied, and the refresh is the
   credential's, once per attempt.
6. **The library numbers of the new bounds**: `http_stream_body` 64 MiB, `blob_put` and `blob_read` 16384, `http_form`
   16384; each is named, so a build moves it.

## Sources

- RFC 9110, HTTP Semantics (sections 9.2.1, 9.2.2, 10.2.3, 15.5.2, 15.5.9, 15.6.3 to 15.6.5); RFC 6585 section 4.
- `draft-ietf-httpapi-idempotency-key-header`, The Idempotency-Key HTTP Header Field (Internet-Draft).
- RFC 9651, Structured Field Values for HTTP.
- RFC 6749, The OAuth 2.0 Authorization Framework (sections 4.4 and 6); RFC 6750, Bearer Token Usage (section 3.1);
  RFC 8707, Resource Indicators for OAuth 2.0; RFC 9700, Best Current Practice for OAuth 2.0 Security; RFC 7617, Basic.
- RFC 7578, Returning Values from Forms: multipart/form-data; RFC 2046 section 5.1.1.
- A. D. Birrell and B. J. Nelson, "Implementing Remote Procedure Calls", ACM TOCS 2(1), 1984.
- C. Lee, S. J. Park, A. Kejriwal, S. Matsushita, J. Ousterhout, "Implementing Linearizability at Large Scale and Low
  Latency" (RIFL), SOSP 2015.
- J. H. Saltzer, D. P. Reed, D. D. Clark, "End-to-End Arguments in System Design", ACM TOCS 2(4), 1984.
- M. Brooker, "Exponential Backoff and Jitter", 2015.
- ADR 0050, ADR 0094, ADR 0107 D4, ADR 0109, ADR 0113 in this repository.
