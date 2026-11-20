# Standard host features

- Amended by: ADR 0107 (D8): section 7, a standard type is served on the lanes
  the table names, and a build for another lane refuses a machine that runs it
  (`docs/adr/0107-a-server-keeps-a-durable-session-per-key-and-a-client-reaches-it-by-mesh-over-http.md`).
- Amended by: ADR 0107 (D6): section 3, a read answers the key's version and
  `sce-host-kv-put-if` writes only over the version it names; section 8, how
  the reference stores number versions; section 9, the identity of an
  invocation a repeat would be answered by.
- Amended by: ADR 0107 (D7): section 10, signing and sealing as COSE operations,
  served by the Rust and C++ reference hosts; sections 7 to 9 name them where
  they list the types, the scenarios and what is not provided.
- Amended by: ADR 0107 (D4): section 1, the identity of an invocation, which a
  restore keeps and every runtime hands a host; section 3, a put-if repeated
  under one identity is answered with its first result; section 8, the
  reference hosts keep that result; section 9 no longer lists it.
- Amended by: ADR 0107 (its documentation step): section 9, the server's parts
  built in the Rust host core leave the list, which names what a server still
  lacks and points to the runbook, `docs/SCE_SERVER_DEPLOY.md`; the Swift item
  narrowed to iOS, since the Swift lane serves sections 2 to 6.
- Amended by: ADR 0109 (D4): section 5, the destination policy a server's
  transport is built with and the failure code `destination_refused`;
  section 8, the network transports of the Rust and C++ reference hosts;
  section 9, Rust leaves the list of lanes without a network transport, and a
  server's outbound HTTP leaves what a server lacks
  (`docs/adr/0109-a-server-on-the-public-network-keeps-what-it-owes-bounds-what-its-clients-cost-and-is-upgraded-in-place.md`).
- Amended by: ADR 0109 (D5 to D7): new sections 11 (random bytes), 12 (keyed
  digests) and 13 (keys), served by the Rust and C++ reference hosts; section
  10, a derived key named by what derived it, a key of the key ring signing and
  sealing beside the configured ones, and `key_retired`; sections 7 to 9 name
  them where they list the types, the scenarios and what is not provided.
- Amended by: ADR 0109 (step 4 of its order of work): section 13, `key_use` the
  library's enum `key_use`, refused where it is written; section 5, `method`
  stays a text the host judges, because the operation is served on every lane
  and only the Rust and C++ lanes read an enum field of a request yet.
- Amended by: ADR 0109 (D3): new section 14 (credentials), served by the Rust
  and C++ reference hosts; section 13, the key ring's issuing key; section 8,
  the scenario's `credentials` host state; sections 7 and 9 name them where they
  list the lanes and what is not provided, and section 9 no longer lists the
  durable keeping of the `install` authenticator's pins.
- Amended by: ADR 0109 (D13): section 3, "Left to the host: durability" becomes
  "Durability": a host states whether its store is durable, and the reference
  durable store is the session store's SQLite file, which keeps each put-if's
  first result in the transaction of its write; section 8, the Rust and C++
  reference hosts take their store at construction; section 9 narrows the
  durable key-value store to the lanes other than Rust.
- Amended by: ADR 0109 (D14): new section 15 (the audit log), served by the
  Rust and C++ reference hosts; section 8, the scenario's `audit` directive and
  host state; sections 7 and 9 name it where they list the lanes and what is
  not provided.
- Amended by: ADR 0109 (D9, its record-level half; ADR 0107, Open question
  1): section 10, a record signed, verified, sealed and opened by name
  (`sce:payload`), which crosses as its deterministic CBOR and comes back as
  the record, and the failure `not_a_record` of a verification or an opening
  whose bytes are no value of it; the C++ reference host's trust sets name
  public keys it holds no seed of (`CoseFeature::setTrustedKeys`).
- Amended by: ADR 0109 (D17): section 10, the bounds of the signing and
  sealing records are named (`cose_payload`, `cose_signed`, `cose_plaintext`,
  `cose_sealed`), `--limit` moves them, a build that breaks their relations is
  refused (`codegen/standard-host-bound-relation`), and the Rust and C++
  reference hosts judge by the numbers the build gave them, as for
  `mac_message` and `random_bytes` of sections 11 and 12; section 8, how a
  reference host is given them.
- Amended by: ADR 0109 (D16): section 8, what the Rust and C++ reference hosts
  report through the metrics port a server's program gives them.
- Amended by: ADR 0109 (step 15 of its order of work): section 9, a server
  program as the parts that landed make it: its stores in one file sealed at
  rest, the reference hosts' constructors and setters, the metrics port and the
  named bounds; the orderly stop leaves what a server lacks.

A statechart that runs under `datamodel="sce-static"` has no operating system to
call. What it needs of one (the time, a place to keep a small value, a
notification, an HTTP exchange, a text a model generates) it asks of its host,
and the host answers. An interface each application invents for these is an
interface each host re-implements and each statechart is tied to.

This document defines them once. Each feature is a closed set of **host
operations**: an `<invoke>` of a named type, with a request record and a result
record that `stdlib/host` holds as event-schema documents, a closed set of
failure codes, and the guarantees a host gives. A statechart that asks for a
standard feature runs on any host that serves it, and the same scenario judges
every host.

It sits on the typed host interface of SCE Accepted Subset §2.12 (`sce:request`,
`sce:result`, the generated invoker and adapter) and adds nothing to the engine
beyond what "What the build does" records.

## 1. The contract every operation shares

**An operation is a host-run `<invoke>`.** Its `type` is the operation's name
(`sce-host-kv-get`), its `id` is the author's, and its request and result are
the records the operation names, imported from `sce:std/host/`:

```xml
<sce:import kind="event-schema" src="sce:std/host/kv_get_request.scxml" as="KvGetRequest"/>
<sce:import kind="event-schema" src="sce:std/host/kv_get_result.scxml" as="KvGetResult"/>
<invoke type="sce-host-kv-get" id="get" sce:request="KvGetRequest" sce:result="KvGetResult">
  <param name="key" expr="key"/>
</invoke>
<transition event="done.invoke.get" cond="_event.data.error === ''" target="idle">
  <assign location="got" expr="_event.data.value"/>
</transition>
<transition event="done.invoke.get" target="failed">
  <assign location="why" expr="_event.data.error"/>
</transition>
```

A build holds the invoke to the library's records. An operation that takes no
argument names no request and carries no `<param>`.

**There are four ends, and only four.**

| End | The document receives | Meaning |
|---|---|---|
| Completion | `done.invoke.<id>`, the result record with `error` empty | The operation ran and this is its answer. A negative answer that is part of the operation (a key that is not there, a notification that was not allowed) is a completion, not a failure. |
| Failure | `done.invoke.<id>`, the result record with `error` one code of the operation's closed set | The operation started and could not be carried out. Nothing it was asked to change has changed, and every other field of the record is at its zero. |
| Refusal | `error.execution`, `_event.data` the JSON `{"reason":"invalid_argument"}` | The host would not start it: an argument the record's type holds but the operation does not accept (an empty key, a method that is not one of the listed ones). The host did nothing. |
| Deadline | `error.invoke.<id>`, `_event.data` `"deadline"` | The `_sce_deadline_ms` the document gave passed first. The host was told to stop, and a report it makes later is refused. |

**A failure is a value, not an event.** Every result record has `error` as its
first field: a text of at most 24 bytes, empty when the operation ran and one
code of the operation's closed set when it could not. A machine of this data
model reads the data of an event only through the schema the event declares, and
the data of `error.invoke.<id>` is text with none, so a failure that arrived there
could be told apart from another by nobody. As a field of the result it is read
like any other (`_event.data.error === ''`), and `error.invoke.<id>` has the one
meaning the table gives it.

A host that starts an operation reports exactly one completion (a failure
included), or is cancelled. A statechart that leaves the state that holds the
`<invoke>` cancels it: the host stops, and what it reports afterwards is ignored by
the engine (SCE Accepted Subset §2.12, "Cancel"). A host does not report an
operation it was not asked for, and a statechart reads no event a host raises on
its own: everything a host tells a machine is the end of an operation it began.

**Ordering.** Operations of one feature, on one host, take effect in the order
they were started, and an operation that starts after another completed sees its
effect. Two operations of one feature started in the same macrostep (two regions
of a `<parallel>`) take effect in document order. Completions reach the machine in
the order the host reports them, which for a host that answers while it is started
is the order of the starts.

**Idempotency.** Each operation says whether repeating it is the same as doing it
once. A host does not repeat an operation by itself: an HTTP request that failed
is not sent again unless the document sends it again.

**Identity** (ADR 0107, D4). An invocation is named by three things, and every
runtime hands all three to the host with the request: the id of the session that
started it, the `<invoke>`'s id, and its start number, which start of that
`<invoke>` in that session it is, counted from 1. The engine counts the starts of
each host-run `<invoke>` of a session, and a state entered again makes the next
start, after it cancelled the one before. A saved state holds each running
invocation with its start number and each session's count of starts (the
`sce-saved-state` format 4, SCE Accepted Subset §2.15), and a restore starts a
running invocation again under the number it had, with `restarted` set: the same
invocation asked a second time, which a host may answer as it answered the first.
A start the document makes after a restore is numbered on from the saved count.
The `token` a request carries is not part of the identity: it is distinct for
every start within one engine and given anew to a start a restore makes, so a
report from the process that saved the state cannot complete the one that
restored it.

**Deadlines.** Any operation may carry `<param name="_sce_deadline_ms" expr="N"/>`.
It is the engine's, not a field of the request, and a host does not receive it.

**Text and numbers.** Every text is UTF-8. A bound written below is in UTF-8 bytes
and is the `sce:max-size` of the field in the record: a value past it never
reaches a host, because the typed request refuses it where the invocation starts,
and a host never answers one. A 64-bit integer of a result is written as the text
of its digits, as every payload carries one (SCE Accepted Subset §2.15,
"Params").

## 2. Clock

| Operation | `type` | Request | Result |
|---|---|---|---|
| Read the time | `sce-host-clock-now` | none | `clock_now_result`: `utc_ms` int64, `monotonic_ms` int64 |
| Read the zone | `sce-host-clock-zone` | none | `clock_zone_result`: `zone_id` string (64), `utc_offset_s` int32 |

Failure code: `unavailable`, for both.

- `utc_ms` is milliseconds since 1970-01-01T00:00:00Z, every day 86400 seconds
  (POSIX time). It may go backwards when the host's clock is set.
- `monotonic_ms` is milliseconds since an origin the host fixes for the life of its
  process. Across the operations of one host it never decreases, whatever the
  source it reads does. Its origin is not a date and is not comparable between
  hosts or between runs.
- Both readings of one `sce-host-clock-now` are taken at one instant.
- `zone_id` is an IANA time zone identifier, empty when the host knows only an
  offset. `utc_offset_s` is seconds east of UTC in force at the instant the host
  answered, and for no other instant: a statechart that must convert another
  instant asks the host for its transition table and calls
  `sce:std/time/utc_offset_at` and `utc_from_local`.
- Both operations are idempotent and change nothing.

Left to the host: where the time comes from, its resolution (a host may round both
readings to a coarser step), and whether it follows the user's time zone setting
while running.

## 3. Key-value store

A store maps keys to texts. There is one store per host; a document that needs
several keeps them apart by a prefix of its choosing.

| Operation | `type` | Request | Result |
|---|---|---|---|
| Read | `sce-host-kv-get` | `kv_get_request`: `key` (256) | `kv_get_result`: `found` bool, `value` string (4096), `version` string (20) |
| Write | `sce-host-kv-put` | `kv_put_request`: `key` (256), `value` (4096) | `kv_put_result`: `replaced` bool |
| Write over a version | `sce-host-kv-put-if` | `kv_put_if_request`: `key` (256), `value` (4096), `expected_version` (20) | `kv_put_if_result`: `written` bool, `version` string (20) |
| Remove | `sce-host-kv-delete` | `kv_delete_request`: `key` (256) | `kv_delete_result`: `existed` bool |
| Walk | `sce-host-kv-next-key` | `kv_next_key_request`: `after` (256), `prefix` (256) | `kv_next_key_result`: `found` bool, `key` string (256) |

Failure codes: `unavailable` (the store cannot be reached or opened) for all five,
and `quota_exceeded` for a put or a put-if that would make the store larger than
the host allows, which leaves the store unchanged.

Refusal (`invalid_argument`): a key that is empty or contains U+0000, and an
`expected_version` that is empty or holds a character other than the digits `0`
to `9`. A `prefix` and an `after` may be empty.

- A key is compared as its UTF-8 bytes. There is no normalisation: two spellings
  of one character are two keys.
- **A version** is a 64-bit unsigned integer, written as the text of its digits
  (section 1). A key nothing was written under, or whose value was removed, is at
  version `0`. Every put, every put-if that wrote and every delete that removed
  the key gives the key a new version, greater than any it had before, so a
  version once read names one stored state of the key and is never given to
  another: a key removed and written again does not return to a version it had.
  A version says nothing else: it is not a count of writes, and two keys' versions
  are not comparable.
- **Get** answers `found` false, an empty `value` and `version` `0` for a key
  nothing was written under. A written empty text is `found` true with an empty
  `value`. `version` is the key's version, the one a put-if names to write only
  over the state this read saw. Idempotent.
- **Put** replaces any value. `replaced` is true when the key held a value, whether
  or not it was the same text. Repeating a put with the same arguments leaves the
  same stored value, at a new version. When it completes, a get started after it
  on this host answers the new value; whether the value survives the host's
  restart is the host's.
- **Put-if** writes `value` under `key` only when the key's version is
  `expected_version`, `0` meaning that the key must be absent, and answers
  `written` true with the key's new version. When the version is another it
  writes nothing and answers `written` false with the key's version as it is, `0`
  for an absent key: a lost race is an answer, not a failure, and the document
  decides whether to read again and retry. `expected_version` is compared as a
  number, so leading zeros do not count, and a number past 2^64 - 1 names no
  version and is never matched. The quota is judged only for a write that would be
  made: a put-if whose version does not match answers `written` false whatever
  the quota. Not idempotent by itself: asked again by another invocation after
  it wrote, it finds the version it gave and answers `written` false. A put-if
  started again under the identity of one already answered (section 1, a restore
  of a state saved before the machine took the answer) is that invocation asked
  twice: the host answers it with the first result, a failure included, and
  changes nothing. A refusal is no result, and a refused start asked again is
  judged again.
- **Delete** answers `existed` false for a key that was not there, which is not a
  failure. Idempotent in effect.
- **Next-key** answers the first key, in ascending order of UTF-8 bytes, that is
  greater than `after` and begins with `prefix`; `after` is excluded, and an empty
  `after` starts at the least key. A statechart enumerates by asking again with the
  key it was given. It is not a snapshot: a key written or removed between two asks
  is seen or missed by the later one, and an enumeration never repeats or skips a
  key that was neither written nor removed during it.

**Durability** (docs/adr/0109, D13). A host states whether its store is
durable: whether a write that completed survives the host's restart, and with
it the first result of each put-if, which a durable store keeps in the
transaction of the write, so that a put-if a restore starts again is answered
with its first result whether the host stopped before or after its answer
left. A store that is not durable forgets both when its host stops. The
reference hosts take their store when they are built, in memory unless the
embedding gives another (section 8); the reference durable store is the
session store's SQLite file (`sce_rust_session_store::SqliteStore`, layout 7),
held with the store in memory to one contract (`KvStore`), and the removal of a
session from it deletes the first results of that session's put-ifs in the same
transaction. A server keeps it sealed at rest (`Sealed`,
docs/SCE_SEALED_RECORD.md): each key as its keyed digest, each value sealed.

Left to the host: the quota, whether the stores of two applications on one
device are kept apart, and encryption at rest.

## 4. Notifications

| Operation | `type` | Request | Result |
|---|---|---|---|
| Ask the permission | `sce-host-notify-permission` | none | `notify_permission_result`: `state` string (16) |
| Request the permission | `sce-host-notify-request-permission` | none | `notify_permission_result` |
| Post | `sce-host-notify-post` | `notify_post_request`: `id` (64), `title` (128), `body` (512), `at_ms` int64 | `notify_post_result`: `status` string (16) |
| Withdraw | `sce-host-notify-cancel` | `notify_cancel_request`: `id` (64) | `notify_cancel_result`: `existed` bool |

`state` is one of `granted`, `denied`, `undetermined`. `status` is one of
`posted`, `scheduled`, `denied`. Failure code: `unavailable` for all four (the
platform's notification service did not answer).

Refusal (`invalid_argument`): an `id` that is empty.

- **Ask the permission** changes nothing. **Request the permission** may show the
  user a choice and answers the decision. Asked again once the user has decided, it
  answers the decision and shows nothing. A host with no user to ask answers
  `denied`.
- **Post** with a permission that is not `granted` completes with status `denied`
  and keeps nothing. That is an answer, not a failure: the document chooses what to
  do. Otherwise, with `at_ms` zero or not after the host's `utc_ms`, the
  notification is handed to the platform to show now (`posted`); with a later
  `at_ms` it is kept to be shown then (`scheduled`).
- Posting an `id` again replaces what that id named, whether waiting or shown, and
  a replaced waiting notification is not shown. The replacement is the latest post:
  the host lists what it holds in the order of the post that put each there.
- **Withdraw** removes a waiting notification, or a shown one from wherever the
  platform keeps it, and answers `existed` true; an `id` that names nothing
  answers `existed` false. Idempotent in effect.
- Text is shown as given. A host does not interpret markup in `title` or `body`.

Left to the host: how a notification looks and sounds, what the user can do with
it, and whether a scheduled one is shown while the application is not running. A
statechart that needs a scheduled notification after a restart posts it again from
what it saved.

## 5. HTTP

| Operation | `type` | Request | Result |
|---|---|---|---|
| One exchange | `sce-host-http-fetch` | `http_fetch_request`: `method` (8), `url` (2048), `headers` (2048), `body` (16384) | `http_fetch_result`: `status` uint16, `headers` string (4096), `body` string (16384) |

Failure codes: `network` (no response was received: name resolution, connection,
TLS, a timeout, or the connection ended), `response_too_large` (the headers or the
body do not fit the record, or pass a bound of the transport), `invalid_body` (the
body is not UTF-8), `destination_refused` (the transport's destination policy
refused the target before anything was sent: it is not on the policy's allow-list,
or every address its name resolved to is refused; see "Destination policy" below).

Refusal (`invalid_argument`): a `method` that is not one of `GET`, `HEAD`, `POST`,
`PUT`, `PATCH`, `DELETE`, `OPTIONS` (spelled in capitals); a `url` that is not an
absolute `http` or `https` URL with a host, or that holds an ASCII space or control
character (U+0000 to U+0020, U+007F); a header line without a colon, or whose name
is not an RFC 9110 token.

**`method` is a text the host judges.** It is no enum: an enum field of a request
is read on the Rust and C++ lanes alone (SCE Accepted Subset §2.12), and this
operation is served on every lane, so an enum would take it from the others,
which docs/adr/0109 (D0, its first exception) does not allow of a surface every
lane shares. It becomes an enum in the change that gives every lane serving this
operation an enum field of a request, so that no lane loses it.

- `headers` of a request are zero or more `Name: value` lines separated by a line
  feed; an empty line is ignored and a value is without the spaces and tabs around
  it. `headers` of the result are one `name: value` line for each field, names in
  lower case, in the order the transport reports them (the order received where the
  transport keeps it; a transport whose client keeps fields in a map gives them
  sorted by name, and the reference hosts say which they do).
- Every response is a completion, an error status included: `404` and `500` are
  answers. The host does not follow redirects, so `301` arrives as `301` with its
  `location` header, and it does not retry.
- A body that does not fit, or headers that do not fit, are never cut: the exchange
  fails `response_too_large`; the size is judged before the encoding. A response to
  `HEAD` has an empty body.
- The host adds no header the document did not write, except those the transport
  cannot do without (`host`, `content-length`). It keeps no cookies and answers no
  authentication challenge.
- The exchange is not idempotent by itself; whether repeating it is safe is the
  method's and the server's. A statechart that cancels it (by leaving its state)
  does not learn whether the server saw the request.
- A document that wants a bound on the wait writes `_sce_deadline_ms`; with none,
  the host's own timeout applies and surfaces as `network`.

Left to the host: the trust store and certificate policy, proxies, connection
reuse, compression, HTTP version, and the host's own timeout.

**Destination policy** (docs/adr/0109, D4). A transport that reaches the network
for a server, where the URL a document fetches may come from a client, is built
with a destination policy and has no constructor without one. The policy is
either an allow-list (each entry a scheme, a host or `*.` before a name for every
name below it, and a port) or the public internet (any host and port, `https`
only), and judges each request before anything is sent:

- A target the policy does not admit is refused before its name is resolved.
- The name is resolved once for the exchange, by the policy, and every address it
  resolves to is judged against the special-purpose ranges of RFC 6890 and the
  IANA registries it set up: unspecified, "this network" `0.0.0.0/8`, loopback,
  RFC 1918, shared `100.64.0.0/10`, link-local `169.254.0.0/16` and `fe80::/10`,
  unique local `fc00::/7`, the IETF protocol assignments `192.0.0.0/24` and
  `2001::/23`, documentation, benchmarking `198.18.0.0/15`, multicast, broadcast,
  discard-only `100::/64`, the local-use NAT64 prefix `64:ff9b:1::/48`, reserved
  `240.0.0.0/4`, the deprecated `192.88.99.0/24` and `fec0::/10`, and every IPv6
  address outside `2000::/3`. An IPv6 address that carries an IPv4 address
  (IPv4-mapped, IPv4-translated, IPv4-compatible, 6to4, the well-known NAT64 prefix
  `64:ff9b::/96`) is judged by the address it carries. An address literal is
  judged without being resolved.
- An address in such a range is refused unless the allow-list entry the target
  matched names a network that holds it (an internal service the deployment names,
  and no other); the addresses left are where the transport may connect, and a
  name with none left is refused `destination_refused`. A name that resolves to no
  address fails `network`.
- The transport connects only to an address the policy admitted, never to a
  second resolution of the name, so a name that answers another address between
  the check and the connect cannot pass; `Host` and the TLS server name stay the
  URL's host, and TLS is verified against it.
- No redirect is followed. A connection, a TLS handshake, the response's head and
  the whole exchange each have a bound, which fails `network`; a body or a head
  past the transport's bound fails `response_too_large`.
- No proxy is read from the environment. A proxy the deployment configures is
  reached as itself: its own address is not judged, the target still is, as
  without one. Every exchange through it is tunnelled with `CONNECT`, for `http`
  as for `https`, to the address the policy admitted and not to the name, so the
  proxy never resolves the name either; TLS is still verified against the URL's
  host. The proxy's credentials travel in the `CONNECT` alone and never reach the
  target, and a proxy that refuses the tunnel fails `network`.

## 6. Generated text

A host may hold a model that writes a text from a prompt. The engine calls no
model: the host performs the generation, as it performs an HTTP exchange, and the
statechart receives the text as data and decides from it (docs/adr/0087,
sections 0 and 4).

| Operation | `type` | Request | Result |
|---|---|---|---|
| Generate a text | `sce-host-model-generate` | `model_generate_request`: `prompt` (`model_prompt`=8192), `answer_schema` (128), `max_tokens` uint32, `temperature_milli` uint16 | `model_generate_result`: `text` string (`model_answer`=4096), `constrained` bool, `model` string (128), `prompt_tokens` uint32, `answer_tokens` uint32, `finish` string (16) |

Failure codes: `unavailable` (no model is loaded, or the provider cannot be
reached), `busy` (the provider serves another caller and this host does not
queue), `network` (a remote provider sent no response), `context_exceeded` (the
prompt and `max_tokens` do not fit the model's context), `answer_too_large`
(the answer is past the bound of `text`), `declined` (the provider declined to
answer).

Refusal (`invalid_argument`): an empty `prompt`, `max_tokens` zero,
`temperature_milli` above 2000, or an `answer_schema` that is not empty and is
not an id of the machine's table of answer schemas.

- **The answer schema is named by the build.** `<invoke
  type="sce-host-model-generate" sce:answer="Answer">` names an imported event
  schema; the build writes the request's `answer_schema` from it, the schema's
  id `urn:sce:schema:<document name>:<revision>`, and the document does not
  (a `<param name="answer_schema">` is `validation/typed-invoke-request`).
  Without `sce:answer` the field is empty and the provider answers freely.
- **The machine carries the table of its answer schemas.** For each schema its
  invokes name, the generated machine declares the id, the schema's JSON Schema
  (2020-12) and its GBNF grammar, which counts every text's bound in bytes: Rust
  `ANSWER_SCHEMAS`, Go `AnswerSchemas`, Kotlin `<Machine>AnswerSchemas.all`,
  Python `ANSWER_SCHEMAS`, C++ `ANSWER_SCHEMAS`, C11 `<machine>_answer_schemas()`
  with `<MACHINE>_ANSWER_SCHEMA_COUNT`, Swift `<Machine>AnswerSchemas.all`, each
  entry the runtime's `AnswerSchema`. A host resolves a request's id there, with
  no file and no second copy of a grammar, and gives its provider the form it
  takes. The table's JSON Schema is the schema unit's, without its
  `x-sce-source-hash`: the machine's own header names the source set the table
  was written from. A machine that names no answer schema declares no table.
- **A provider that decodes under a grammar** is given the form it takes, and
  the result says `constrained` true; one that cannot ignores the schema and says
  false. `model` names what answered, as the host spells it (the model, its
  variant and its runtime). `temperature_milli` is the sampling temperature in
  thousandths; 0 asks for the most likely token at each step.
- **`finish`** is `stop` when the provider ended its answer and `length` when it
  reached `max_tokens`. An answer cut by `length` is still a completion; the
  statechart's lift of the text (`<sce:lift>`) is what refuses it. A host never
  cuts `text`: an answer past its bound fails `answer_too_large`.
- The operation changes nothing; repeated, it may answer differently. Starts of
  one host are served in the order started.

Left to the host: the provider (on the device or remote), its model, its
decoding engine, its own timeouts, and whether a cancelled generation stops at
once. Not provided here: any binding to a real provider.

## 7. What the build does

- **The types are served without a declaration.** A document that runs an
  `<invoke>` of a standard type needs no `--host-invoker`: the host of a lane
  that serves the type serves it. `sce-codegen check` reports them under
  `host_invoker_types`.
- **Each type is served on the lanes the table names.** Each entry of the table
  lists the lanes whose reference host (section 8) serves the type: the
  backends `-l` names, and the Interpreter, whose build is the document
  `sce-codegen lower` writes and whose host is the C++ one its driver registers
  on the `StateMachine`. A build for another lane, `generate`, `check` and
  `lower` alike, refuses a machine that runs an
  `<invoke>` of it as `codegen/standard-host-type-not-served`, on that
  `<invoke>`, naming the type, the lane and the lanes that serve it; a child the
  document carries is held to the same lane. Every type of sections 2 to 6 is
  served on every lane; the types of sections 10 to 15 are served on Rust and
  C++ (docs/adr/0109, D0).
  That every lane serves every type stays the goal, and is
  not a precondition of adding one: a type is added with the lanes whose
  reference host it lands in, and a lane joins its entry in the change that
  lands that host's support and its scenarios. A backend added to SCE serves
  none of the types until its reference host lands.
- **The records are the library's.** `sce:result` (and `sce:request`, for an
  operation that has one) must name the record the standard library gives the
  operation, imported from `sce:std/host/`; a record of another document, however
  alike, is refused as `validation/typed-invoke-schema`. An operation that takes
  no argument takes no `<param>`, no `namelist` and no `<content>`, and a
  `<param>` is refused as `validation/typed-invoke-request`. The reserved
  `_sce_deadline_ms` is the engine's and is allowed.
- **A computed argument is checked at build time.** Under `sce-static` a `<param>`
  of a typed request may be an expression of the machine's fields. Its type is
  known before the machine runs, so it is held to its field where it is written
  (`scxml/static-datamodel-rule`): an integer whole in the field's range (a
  literal takes the field's type, and one past its range is
  `expression/literal-out-of-range`), a real no wider than the field's, a bool,
  and a text held by a variable whose
  `sce:capacity` is no more than the field's `sce:max-size`. A computed value that
  fails at run time (a checked operation overflowing) refuses the request:
  `error.execution`, and nothing starts. Before this, a typed request could carry
  literals only under `sce-static`, and every backend's template had no path for a
  computed value of a typed request.
- **A guard reads the payload of an event that declares one.** Under `sce-static`
  a `cond` that reads `_event.data` on an event with no schema is refused
  (`scxml/static-datamodel-rule`). It was accepted before, and written into the
  generated machine as the text the author wrote, which names no value there.
- **The table is one list** (`sce-build/src/forge/standard_host.rs`), held to the
  library by `the_standard_host_features_are_one_set`, and its lanes to the order
  and the refusal by the unit tests beside it.

## 8. Conformance and reference hosts

`tests/host_features/scenarios/` holds one scenario for each feature, and the six
machines that ask for it are
`sce-build/tests/fixtures/static_datamodel/static_host_{kv,clock,notify,http,model,cose}.scxml`.
A lane whose reference host does not serve a feature (section 7) does not replay
its scenario, and its driver names the scenario as one it does not serve.
A scenario is a list of steps. A step may set what the reference host is like
(`host`), send an event, and state what the machine published and what the host
was asked:

| Member | What it says |
|---|---|
| `host` | directives applied before the step: `clock` (`utc_ms`, `monotonic_ms`, `zone_id`, `utc_offset_s`, each optional), `permission` (what the platform says), `user_decision` (what the user would choose if asked: `granted`, `denied`, `none`), `fail` (operation type to the failure code it fails with; an empty object ends them), `quota` (the most keys the store holds), `http` (what the transport answers: `status`, `headers` as name and value pairs, and `body` as text, `body_hex` as bytes or `body_repeat` as a text and a count; or `fail`, a transport failure; or `hold`, which does not answer), `model` (what the scripted provider is like, each member optional: `answers`, the answers it gives, one a start, in order, each `text` or `text_repeat` (a text and a count) with `finish` (`stop` when not written), replacing those not given yet; `constrained`, whether it decodes under a grammar; `name`, what it says answered; `context`, the most tokens a prompt and `max_tokens` may take together; `hold`, true for a provider that does not answer), `cose_keys` (the keys of sections 10 and 12, replacing those configured before: `sign`, a key id to the hexadecimal 32-byte seed of an Ed25519 key; `aead`, a key id to a hexadecimal 32-byte ChaCha20/Poly1305 key; `mac`, a key id to a hexadecimal HMAC-SHA-256 key of any length; `trust`, a trust set's name to the ids of `sign` keys of the same directive, whose public keys it holds; a table not named is empty, and the keys the host generated and derived are kept), `random` (hexadecimal bytes the host's random source gives next, before the operating system's: a draw, a generation, the naming key of the first derivation, a seal's nonce, and the audit log's pseudonymisation key and audit key at their first use take them in the order the host draws, section 13), `audit` (the streams of the audit log, replacing those kept before: `streams`, a stream's name to its `checkpoint_every`, its `retain_days` and, optionally, `personal`, `pseudonymised` or `clear`, section 15) |
| `advance_ms` | moves the engine's manual clock on, which is how a deadline passes |
| `event`, `data` | an external event and its payload |
| `expect.state` | the atomic state the machine is in |
| `expect.variables` | what the machine published |
| `expect.host_invokes` | the operations the host was started with in this step, refused ones included, oldest first: `{"type", "id", "request"}`, the request as the record |
| `expect.host_state` | what the host holds afterwards, for the members named: `store` (key to value), `waiting` and `shown` (the notification ids, in the order posted), `http` (what the transport was given in all: method, URL, request header lines as name and value, body), `cancelled` (the invocations the host was told to stop), `credentials` (each name the host issued a token for, to its tokens as `issued_ms`, `expires_ms` and `revoked`, in the order of those three; a token's own bytes depend on the session's id, which differs by lane, section 14), `audit` (for each stream named, its rows as `sequence`, `at_ms` and `entry`, the sequences of its checkpoints, and whether a reader verifies it under the host's audit key, section 15) |

**The reference stores' versions.** Every lane's store numbers versions from one
counter of the store, which starts at `0`: each put, each put-if that wrote and
each delete that removed a key takes the counter's next number, and a write gives
it to the key as its version (a delete's number is given to no stored value, so a
key written again after it is past both). A failure, a refusal, a put-if that did
not write and a delete of an absent key take none. That keeps section 3's rule
and makes a scenario's versions exact on every lane.

**The scripted provider.** A reference host answers a generation from a
provider the embedding supplies, and every lane has a scripted one: it gives the
answers it was told, in order, and counts a token as one byte of UTF-8 (a
prompt's `prompt_tokens` is its length in bytes, an answer's `answer_tokens`
the length of its text). With no answer left it fails `unavailable`, as a host
with no model loaded does; an answer past the bound of `text` fails
`answer_too_large`; a prompt whose length and `max_tokens` pass its `context`
fails `context_exceeded`; it is `constrained` only when the request names an
answer schema. The other failures are the `fail` directive's.

**The exhaustive provider** (docs/adr/0087, section 9.4). The second provider
every lane has stands for a model that may give any outcome at any call: driven
by the test, each start takes the next outcome class of a sequence the test
names, and answers what the class says. The classes are one table every lane
reads, `tests/host_features/model_outcome_classes.json`: a record read, a record
read whose id is not one the reference harness offers (which only a harness's
check tells apart from the first), a text for each refusal code of
`sce:std/lift/refusal_code.scxml` that a schema of fields reaches, an answer cut
by `length`, each failure code, the refusal, a deadline that passes and a cancel
by leaving the state. The same table's `harness` section names the reference
harness the exhaustive run of docs/adr/0087 step 7 drives. Each lane runs
`static_host_model` against every sequence of classes up to the table's
`bound`, and holds what the machine publishes after each call to what the class
says it publishes. The host still judges the request first: a request it
refuses is refused whatever the class.

**The reference harness** (docs/adr/0087, section 6 and step 7). The harness,
`static_model_harness`, is replayed against the scripted provider by five
scenarios, `static_model_harness_*.json`, one a case of section 9.1 item 8: a run
of the harness ends at one of its finals, so a case is a run of its own and a
driver replays a scenario by its file's name against the machine it names. Each
lane also runs the harness against the exhaustive provider for every sequence of
the harness section's `bound` classes (each a run, so every shorter sequence the
harness can consume is a prefix of one), sets the transition observer of
docs/adr/0089, and requires the runs together to take every row of the
harness's emitted transition table, to reach every refusal code (the harness keeps
each refusal it answered in `refusals`), and to reach every host outcome at every
call site, a row whose `targets` is the state that holds the call; the test fails
naming the rows, codes or outcomes never reached.

A reference host is the whole of a feature's semantics, as one object that is told
a start (the type and the request's text parameters) and answers a completion, a
refusal, or nothing yet. Each lane has one, with its own clock and transport, and
a driver that registers it as the invoker of every standard type of a generated
machine and replays the scenarios:

| Lane | Reference host | Driver | Clock | HTTP transport |
|---|---|---|---|---|
| Rust | `backends/rust/host-features` | `backends/rust/scenarios/tests/host_features.rs` | system, scripted | hyper over rustls, behind a destination policy (`backends/rust/host-http`) |
| Kotlin (JVM) | `backends/kotlin/host-features` | `backends/kotlin/tests/src/test/kotlin/com/sce/integration/HostFeaturesTest.kt` | system, scripted | JDK client |
| Go | `backends/go/host-features` | `backends/go/tests/integration/static_datamodel/host_features_test.go` | system, scripted | `net/http` |
| Python | `backends/python/host-features` | `backends/python/tests/integration/static_datamodel/test_host_features.py` | system, scripted | `http.client` |
| C++ | `sce/include/host/StandardHost.h` | `tests/integration/AStandardHostScenarioTest.cpp` | system, scripted | cpp-httplib (`HttplibTransport.h`); libcurl, behind a destination policy (`CurlTransport.h`, `DestinationPolicy.h`) |
| C | `backends/c/host-features` | `backends/c/tests/integration/test_host_features.c` | system, scripted | none |
| Swift | `backends/swift/host-features` | `backends/swift/tests/Tests/StaticDatamodelTests/HostFeaturesTests.swift` | system, scripted | none |
| C++ Interpreter | `sce/include/host/StandardHost.h` | `tests/integration/AStaticDatamodelRunsLoweredUnderTheInterpreterTest.cpp` | scripted | none |

The two C++ drivers read a scenario, set the host and judge it through one replay
(`tests/integration/StandardHostScenarioReplay.h`); the Interpreter's runs the
document `sce-codegen lower` makes of the machine, with the reference host registered
on the `StateMachine` as it is on the generated engine (`SCE::StandardHost::serve`;
docs/adr/0084, D7), and moves its scheduler's logical clock for `advance_ms`. The
lowering refuses a model's generation by name (docs/adr/0087, section 7.1), so the
Interpreter's driver holds `static_host_model` and the reference harness
`static_model_harness` to that refusal instead of replaying them. The Interpreter
is not among the lanes of section 10's operations: its driver adds no COSE feature
to its host, so the lowering refuses `static_host_cose` as
`codegen/standard-host-type-not-served` (section 7, docs/adr/0107, D8), and the
driver holds it to that refusal; its scenario is replayed on the Rust and C++
lanes.

Each driver runs the machine on a manual clock and the host on a scripted one, so a
scenario is deterministic. The transports that exist are held by a test against a
local server (carrying headers and a body, no redirect followed, an error status
an answer, a server that is gone `network`). The two built with a destination
policy (section 5) are held besides, against a server on a loopback port and a
resolver the test scripts, to refusing a name that resolves to loopback before a
byte is sent while serving one the allow-list lets reach it, to resolving a name
once per exchange and connecting to the address checked, to answering a redirect
to a private address as the `3xx` it is, to their bounds of time and size, and,
through a `CONNECT` proxy of the test, to judging the target, tunnelling to the
address checked and keeping the proxy's credentials from the target
(`backends/rust/host-http/tests/`, `tests/integration/AStandardHostCurlTransportTest.cpp`);
each lane's policy is held to one table of addresses in every special-purpose
range (`backends/rust/host-features/src/destination.rs`,
`tests/integration/AStandardHostDestinationPolicyTest.cpp`). Each lane's host is also held, apart
from the scenarios, to the system clock's monotonic reading never decreasing, to
serving none of a type outside the standard set, and, where a lane's strings can
hold U+0000 (a C string cannot), to refusing a key with one. Every lane is held to
the order of the keys' UTF-8 bytes by a pair of keys that sort differently by
UTF-16 code unit, which is the order a Kotlin or JVM string compares in.

Each lane's host keeps the first result of each put-if by the identity of its
invocation (section 1) and answers a repeat with it (section 3). Each lane's
driver holds that against the machine: a put-if the host carried out whose answer
the machine did not take before the state was saved is answered, after the
restore, with the first result and writes nothing, and a put-if the document
starts afterwards is the next start, judged against the store as it is. The
starts the host records carry the session's id and the start number beside the
`<invoke>`'s id.

The Rust and C++ reference hosts keep their store, with those first results, in
the store they were built with (docs/adr/0109, D13): Rust's
`ReferenceHost::with_kv_store` takes a `sce_rust_session_store::KvStore`, and
C++'s `ReferenceHost(clock, transport, store)` a `KvStore` of
`host/StandardHost.h`; each builds one in memory when given none. The Rust
driver replays section 3's scenario against the SQLite store too, and holds a
put-if whose process stopped before its answer reached the machine to its first
result, answered by a host of the next process on the same file
(`backends/rust/scenarios/tests/host_features.rs`).

**The named bounds a host judges by** (docs/adr/0109, D17). The Rust and C++
reference hosts hold the records of sections 10 to 12 to the numbers of their
named bounds (`cose_payload`, `cose_signed`, `cose_plaintext`, `cose_sealed`,
`mac_message`, `random_bytes`) the build of the machine they serve gave them,
which the machine declares as `<Machine>Limits`: Rust's
`ReferenceHost::set_named_bounds` takes a `NamedBounds`, and C++'s
`CoseFeature(crypto, bounds)` a `NamedBounds` of `host/CoseHost.h`, each the
library's numbers (`NamedBounds::LIBRARY`, a `NamedBounds{}`) when given none,
which are those of a build given no limit. An embedding fills one from its
machine's constants:

```rust
host.set_named_bounds(NamedBounds {
    cose_payload: StaticHostCoseLimits::COSE_PAYLOAD as usize,
    cose_signed: StaticHostCoseLimits::COSE_SIGNED as usize,
    ..NamedBounds::LIBRARY
});
```

A host given numbers below its machine's refuses a request the machine may
make; one given numbers above them may answer with an object the machine's
record cannot hold; so the two are given one build's numbers. Each lane drives the fixture `static_host_cose` built a second
time with `--limit cose_payload=8192 --limit cose_signed=8448 --limit
cose_plaintext=8448 --limit cose_sealed=8576` through a host given its
constants, signing, verifying, sealing and opening at those bounds: Rust's
module `static_datamodel_large` of the tree `rust-static-datamodel`
(`a_build_with_larger_bounds_signs_and_seals_at_them_through_its_host`), C++'s
executable `standard_host_named_bounds_test`
(`tests/integration/AStandardHostNamedBoundsTest.cpp`).

**What a reference host reports** (docs/adr/0109, D16). A Rust or C++
reference host given a metrics port (Rust's `ReferenceHost::set_metrics`, an
`Arc<dyn sce_rust_metrics::Metrics>`; C++'s `ReferenceHost::setMetrics` and
`AuditFeature::setMetrics`, a `SCE::Metrics::Metrics` of `host/Metrics.h` that
outlives the host) reports, by the operation's type, how long each start it
answered at once took (`sce_host_operation_duration_seconds`; a start it holds
is not observed) and the code of each failure and refusal
(`sce_host_operation_failures_total`, a code no operation names counted as
`other`), and each entry appended to the audit log
(`sce_host_audit_appends_total`; a repeat under one identity appends none). A
host given none reports nothing. The names are a registry, `sce-metrics-1`,
which `docs/SCE_SERVER_DEPLOY.md` section 10 lists.

Three things differ by lane and are not hidden:

- **The order of a response's fields.** The Python transport and the C++ libcurl
  one give them in the order received. The Go, Kotlin, C++ cpp-httplib and Rust
  transports give them sorted by name because their clients keep fields in a map.
- **The Kotlin JVM transport cannot carry a request header the JDK client
  reserves** (`host`, `content-length`, `connection`, `expect`, `upgrade`): a
  request that names one fails `network`. The Rust and C++ network transports
  likewise carry none of the fields that frame or route a message (`host`,
  `content-length`, `transfer-encoding`, `connection`, `upgrade`, `expect`, `te`).
- **The C host is a hosted reference** (it allocates), and a result record travels
  in one data slot of the machine, `SCE_MAX_DATA_LEN`, which defaults to 256 bytes.
  A C build that serves the HTTP operation sets it to at least 40000
  (`SCE_HF_MIN_DATA_LEN`) and `SCE_MAX_HOST_INVOKERS` to at least 13. A result that
  does not fit the slot is refused as `error.execution`, not cut.

## 9. Not provided

These are not built, and nothing here stands in for them:

- **iOS.** The Swift lane generates a machine of this data model and its
  reference host (`backends/swift/host-features`) serves the types of sections 2
  to 6 (section 8), but it is not an iOS host: it has no `UserNotifications`
  binding, no iOS key-value store and no HTTP transport (see the network HTTP
  transport below).
- **Android.** No `NotificationManager` host, no Android key-value store
  (`SharedPreferences` or DataStore) and no Android HTTP stack. The Kotlin
  reference host keeps its semantics in common code and the JVM's clock and HTTP
  client beside it; it is not an Android host.
- **A server program.** No server program is built here: each deployment
  writes the program that joins SCE's parts to its machine, its keys and its
  credentials, and `docs/SCE_SERVER_DEPLOY.md` is the runbook for one. The parts
  are built in the Rust host core (docs/adr/0107, docs/adr/0109): the keyed pool
  of sessions with its bounds, its orderly stop and its group commit
  (`sce_rust_mesh::pool`, `sce_rust_mesh::commit`, SCE_MESH.md section 14.4),
  the checkpoint of a session (SCE_MESH.md section 10.11), the server of the
  `https` binding with its edge limits, its durable outboxes and its message
  signatures (`sce_rust_mesh::https::server`, SCE_MESH.md section 22), the
  health checks, and the metrics port (`sce-rust-metrics`). The reference host
  of these features serves such a program's machine as it serves any other:
  - **Its stores.** One SQLite file holds every store a server keeps
    (`sce_rust_session_store::SqliteStore`, layout 7): the sessions the pool
    keeps, and the host's key-value store (section 3), key ring (section 13),
    credentials (section 14) and audit log (section 15); `SqliteStore` is a
    `SessionStore`, a `KvStore`, a `KeyStore`, a `CredentialStore` and an
    `AuditStore`. A server keeps it sealed at rest: `Sealed<S>`
    (`sce_rust_crypto::sealed`, docs/SCE_SEALED_RECORD.md) implements each of
    those contracts over the file, every value a sealed record and every key
    its keyed digest, under keys of a key ring whose key-encryption key the
    program holds apart from the file.
  - **Its constructor and setters.** Rust's `ReferenceHost::new(clock,
    transport)` keeps its key-value store in memory, and
    `ReferenceHost::with_kv_store(clock, transport, store)` in the store it is
    given; `set_keyring`, `set_credentials`, `set_audit_store` and
    `set_audit_streams` give it the other stores and the streams it keeps,
    each in memory until given; `set_cose_keys` its configured keys;
    `set_random_source` and `set_nonce_source` the source of what it draws;
    `set_named_bounds` the numbers of the named bounds its machine's build
    resolved (section 8); `set_metrics` the metrics port (section 8). A
    server's transport is `NetworkTransport` of `sce-rust-host-http`, built
    with a destination policy (section 5). The audit log's passes are the
    program's to call: `audit_retain` on its schedule and `audit_stop` at the
    orderly stop. The C++ host is built the same way, `ReferenceHost(clock,
    transport)` or `ReferenceHost(clock, transport, store)`, with each feature
    of sections 10 to 15 added (`addFeature`): `CoseFeature(crypto, bounds)`,
    `CredentialFeature(keyRing, crypto, clock)` and `AuditFeature(crypto,
    clock)` with `setStreams`; `ReferenceHost::setMetrics` and
    `AuditFeature::setMetrics` give the metrics port. Its key ring, tokens and
    audit log are in memory (below), since the server is the Rust host
    core's.
  Beside it there is no quota policy and no delivery of notifications.
- **A durable key-value store on the lanes other than Rust.** The Rust host
  keeps section 3's values in a durable store when it is given one, the session
  store's SQLite file (section 3, "Durability"); the reference stores of the
  other lanes are in memory, and so is the table of first results by which each
  answers a put-if repeated under one identity (section 8): a host of those
  lanes that stops forgets both. The C++ host takes a store at construction
  (`KvStore`), and SCE gives it none that is durable.
- **A network HTTP transport for C and Swift.** Their reference hosts are
  the full semantics over a transport the embedding supplies, driven by a
  scripted one. The C runtime's client posts a form for a `<send>` and carries
  no general exchange. The adapters of the Mesh `https` binding (hyper over
  rustls for Rust; the system's libcurl and WinHTTP for C++; SCE_MESH.md
  section 22.6) carry that binding's exchanges through its own port and are not
  a transport of this feature; Rust's (`backends/rust/host-http`) and C++'s
  (`CurlTransport.h`) are, on the same stacks (section 5, "Destination
  policy").
- **A zone for the Rust host's system clock on a platform other than Unix**, which
  answers `unavailable` for `sce-host-clock-zone`.
- **Delivery of a scheduled notification while the application is not running**,
  and any user interface for a permission request. A reference host answers the
  permission as the scenario sets it.
- **A durable audit log for C++.** The C++ reference host keeps its audit log
  in memory, for its own life, with an audit key and a pseudonymisation key it
  draws anew (section 15); the durable log is the Rust host core's.
- **Signing and sealing, random bytes, keyed digests, keys, credentials and the
  audit log on the lanes other than Rust and C++** (sections 10 to 15). Each is a later step
  of docs/adr/0107 and docs/adr/0109 (D0), taken when a consumer needs it; until
  it lands, a build for that lane refuses a machine that asks for one of the
  operations, by name (section 7).
- **The COSE structures as codec documents** (docs/adr/0107, D9). A CBOR codec
  writes one map whose keys are unsigned integers of one head byte and whose
  entries hold no signed integer (SCE_FORGE.md section 4.6.1; docs/adr/0101's
  step 4, which gives each backend's codec the whole of its form, has landed on
  no backend): it has no spelling for a tagged array (`COSE_Sign1`,
  `COSE_Encrypt0`), for a map carried inside a byte string (the protected
  header), or for a negative algorithm (`-8`). The reference hosts write and
  read the structures themselves, through their language's CBOR heads, and are
  held to one encoding by the shared scenario's bytes.
- **A record signed or sealed by name** (docs/adr/0107, Open question 1, decided:
  `<invoke type="sce-host-cose-sign1" sce:payload="DeviceFile">` encodes a record
  of the schema it names into the request's `payload`, `sce-host-cose-encrypt0`
  into its `plaintext`, and on `sce-host-cose-verify1` and
  `sce-host-cose-decrypt0` reads the result's bytes back into that record,
  refusing bytes that are no value of it). The build reads no `sce:payload`, so a
  document signs and seals bytes it holds. No machine of any lane can yet write
  a record it holds as CBOR or read one from CBOR: a CBOR codec document
  (SCE_FORGE.md section 4.6.1) writes and reads a map of its own entries, and
  reads a record only as an entry of its own list, in the schema unit's type, not
  in the record type a machine declares. The mechanism needs, on Rust and C++
  first: one deterministic CBOR form of a record, docs/adr/0101 section 2's (a
  field keyed by its id, an absent optional left out), which no backend writes
  until docs/adr/0101's step 4 lands on it (the
  one record encoder that exists, `sce_rust_trust::cbor::encode`, writes
  docs/adr/0086 section 9.2's form, an absent optional `null` under its key,
  encodes only, and is no dependency of a generated machine); a writer of a
  machine's record in that form beside the JSON and wire writers of the composed
  lowering (`RustCarrier`, `CppCarrier`), so the build writes the request's field
  at the invoke site as it writes `answer_schema` from `sce:answer`; a reader of
  that form into the machine's record that refuses with the lift's codes
  (docs/adr/0087, section 2.3), where each runtime's reader reads JSON only
  (Rust `sce_rust_runtime::event_payload`, C++ `common/EventPayloadLift.h`); and
  a completion that carries the record read or its refusal, a result record the
  build derives from the library's.
- **A durable key store for C++.** The Rust host keeps the keys it generates and
  derives in a key store it is given (section 13), durable on SQLite, its
  material wrapped under a key-encryption key; the C++ reference host keeps them
  in memory, for its own life, each wiped when it is dropped. A key a C++ host
  generated is gone with its process; a derived key's id is computed from what
  derived it on both hosts, so a C++ host that derives the same key again after
  a restart names it as before only when its naming key is the same, which a
  host in memory draws anew.
- **A durable credential store for C++.** The C++ reference host keeps the
  tokens it issues in memory, for its own life, under an issuing key it draws
  anew (section 14); it keeps no installation pin, since the `https` server is
  the Rust host core's, so its revocation counts tokens only.
- **The removal of expired and revoked tokens.** A credential store keeps every
  token it was given, a revoked one as revoked, so that an issue repeated under
  its identity cannot make it live again; no operation and no call of the store
  removes one yet.
- **Rotation of a key, and its destruction.** A key is retired by a statechart
  and stays (section 13); no operation makes a successor to a retired key or
  destroys one, which is the operator's.

## 10. Signing and sealing

A host signs and seals documents as COSE objects (RFC 9052), and verifies and
opens them, with keys it holds. No key enters a statechart: a signing key, a
symmetric key and a trust set are the host's configuration, or a key of the
host's key ring, which it generated or derived (section 13), and a statechart
names each by its id. Served by the Rust and C++ reference hosts (section 7;
docs/adr/0107, D7).

| Operation | `type` | Request | Result |
|---|---|---|---|
| Sign | `sce-host-cose-sign1` | `cose_sign1_request`: `key_id` (64), `payload` bytes (`cose_payload`=4096), `external_aad` bytes (256) | `cose_sign1_result`: `cose` bytes (`cose_signed`=4352) |
| Verify | `sce-host-cose-verify1` | `cose_verify1_request`: `trust_set` (64), `cose` bytes (`cose_signed`=4352), `external_aad` bytes (256) | `cose_verify1_result`: `valid` bool, `key_id` string (64), `payload` bytes (`cose_payload`=4096) |
| Seal | `sce-host-cose-encrypt0` | `cose_encrypt0_request`: `key_id` (64), `plaintext` bytes (`cose_plaintext`=4352), `external_aad` bytes (256) | `cose_encrypt0_result`: `cose` bytes (`cose_sealed`=4480) |
| Open | `sce-host-cose-decrypt0` | `cose_decrypt0_request`: `key_id` (64), `cose` bytes (`cose_sealed`=4480), `external_aad` bytes (256) | `cose_decrypt0_result`: `opened` bool, `plaintext` bytes (`cose_plaintext`=4352) |
| Derive a key | `sce-host-key-derive` | `key_derive_request`: `secret` bytes (256), `input_key_id` (64), `salt` bytes (64), `info` bytes (256) | `key_derive_result`: `key_id` string (64) |

A byte string travels, in a request and in a result, as its byte-exact Latin-1
text (SCE Accepted Subset §2.12): each byte the character of that code point.

Failure codes: `unknown_key` for all five (the signing key, the trust set, the
symmetric key or the input key the request names is not one the host holds, or
is a key of the key ring of a use the operation does not take, section 13),
`key_retired` for a signing, a seal and a derivation (the key of the key ring
the request names is retired: it still verifies and opens, and makes nothing
new), `unavailable` (the host's key service cannot be reached, or the random
source gave no nonce for a seal or no naming key for a derivation), and
`not_a_record` for a verification and an opening whose invoke names a record
(`sce:payload`, below): the object verified or opened, and the bytes it holds
are no value of that record.

Refusal (`invalid_argument`): an empty `key_id` (sign, seal, open) or
`trust_set` (verify); a derivation given both a `secret` and an `input_key_id`, or
neither; a byte string's text holding a character past U+00FF, or a value past
its field's bound, which the typed request already refuses before a host sees it.

**The algorithms are fixed, not negotiated.** EdDSA on Ed25519 (RFC 9053 section
2.2, COSE algorithm -8); ChaCha20/Poly1305 (COSE algorithm 24) under a 96-bit
nonce the host draws from the operating system's random source for each seal;
HKDF with SHA-256 (RFC 5869), 32 bytes of output.

**The objects.** Every item is written in its deterministic form (RFC 8949
section 4.2.1), so two hosts given the same key and payload write the same
signed object:

- A signature is the tagged `COSE_Sign1` (tag 18) `[protected, unprotected,
  payload, signature]`: the protected header `{1: -8, 4: key_id}` (the key id as
  the bytes of its text), an empty unprotected header, the payload attached, and
  the signature over `Sig_structure` `["Signature1", protected, external_aad,
  payload]` (RFC 9052 section 4.4).
- A seal is the tagged `COSE_Encrypt0` (tag 16) `[protected, unprotected,
  ciphertext]`: the protected header `{1: 24}`, the unprotected header `{4:
  key_id, 5: nonce}`, and the ciphertext with its 16-byte tag, authenticating
  `Enc_structure` `["Encrypt0", protected, external_aad]` (RFC 9052 section 5.3).

**Verify.** `valid` is true when a signer of the trust set signed the object over
`external_aad`; `key_id` is that signer's id and `payload` what it signed. A key
id the object carries, in either header, selects the signer of that id alone; an
object without one is tried against each signer of the set, in its order.
Anything else is an answer, `valid` false with `key_id` and `payload` empty: a
signature that does not verify, an object that is not a tagged `COSE_Sign1` of
definite lengths with nothing after it, a protected header whose algorithm is not
-8, a header label given twice or in both headers, a `crit` header (label 2), a
detached payload, and a payload past the bound of `payload` (`cose_payload`).

**Open.** `opened` is true when `key_id` opens the object over `external_aad`, and
`plaintext` is then what was sealed. The key id the object carries is not read:
the request names the key. Anything else is an answer, `opened` false with
`plaintext` empty: a key that does not open it, an object that is not a tagged
`COSE_Encrypt0` of definite lengths with nothing after it, a protected algorithm
that is not 24, a nonce that is not 12 bytes, a partial nonce (label 6), a header
label given twice or in both headers, a `crit` header, a detached ciphertext, and
a plaintext past the bound of `plaintext` (`cose_plaintext`).

**The bounds are named, and a build moves them** (docs/adr/0109, D17). What is
signed and the object signed, what is sealed and the object sealed, are bounded
by the named bounds `cose_payload`, `cose_signed`, `cose_plaintext` and
`cose_sealed`, at the numbers the table gives; `sce-codegen --limit name=N`
moves each, as it moves every named bound (SCE Accepted Subset, "A bound a
deployment chooses"), in the records and in every variable of a machine that
names it. Two relations hold between them, which the table of standard types
checks: an object takes up to 256 bytes more than the payload it signs (a
`COSE_Sign1` adds at most 146: its tag and heads, a protected header of the
algorithm and a key id at its bound of 64, an empty unprotected one, the
signature of 64), and up to 128 more than the plaintext it seals (a
`COSE_Encrypt0` adds at most 110: its tag and heads, the algorithm, a key id of
64 and a nonce of 12 unprotected, the tag of 16). A build whose records hold a
`cose_signed` below `cose_payload` and 256, or a `cose_sealed` below
`cose_plaintext` and 128, is refused on each `<invoke>` of an operation that
reads both, as `codegen/standard-host-bound-relation`, naming the two bounds,
their numbers and the `--limit` that repairs it. A build that signs payloads of
8192 bytes is therefore built with

```
sce-codegen --limit cose_payload=8192 --limit cose_signed=8448 generate ...
```

and a build given no limit holds every record, and every object signed or
sealed, to the numbers it held before the bounds were named. The generated
machine declares each bound it names as a constant of the number its build
resolved (`<Machine>Limits::COSE_PAYLOAD` and its neighbours, on Rust and
C++), and a reference host is given those
numbers (section 8): a request past one is refused, and a verification or an
opening whose payload or plaintext is past one is answered as one that does not
verify or open. A payload of megabytes, a detached payload over a stored blob,
is not this mechanism (docs/adr/0109, Open question 7).

**A record signed and sealed by name** (docs/adr/0109, D9, its record-level
half, as docs/adr/0107, Open question 1 decided it). A document that signs or
seals a document of its own does not hold its bytes: it names the record's
event schema on the invoke, and the build carries the record across.

```xml
<invoke type="sce-host-cose-sign1" id="sign_note" sce:payload="Note"
        sce:request="Sign1Request" sce:result="Sign1Result">
  <param name="key_id" expr="'device-a'"/>
  <param name="payload" expr="note"/>
  <param name="external_aad" expr="''"/>
</invoke>
<invoke type="sce-host-cose-verify1" id="verify_note" sce:payload="Note"
        sce:request="Verify1Request" sce:result="Verify1Result">
  <param name="trust_set" expr="'devices'"/>
  <param name="cose" expr="object"/>
  <param name="external_aad" expr="''"/>
</invoke>
<transition event="done.invoke.verify_note" cond="_event.data.error === ''" target="idle">
  <assign location="verified_note" expr="_event.data.payload"/>
</transition>
```

- **Sign and seal.** On `sce-host-cose-sign1` and `sce-host-cose-encrypt0`,
  the `<param>` of `payload` (`plaintext`) is a variable, or a field of a record
  variable, of the record the attribute names, and nothing else. Where the
  invocation starts, the machine writes it as its deterministic CBOR
  (docs/adr/0101, section 2: a map of the fields present, each keyed by its id,
  the keys in the order their encodings sort, every head shortest), and those
  bytes are the request's byte string: the host signs or seals them as any
  payload, and its request is what it would be had the document written the
  bytes itself. A record no encoding holds (a list below its lower count) is a
  value its field cannot hold: `error.execution`, and the host is not asked.
- **Verify and open.** On `sce-host-cose-verify1` and
  `sce-host-cose-decrypt0`, the completion's `payload` (`plaintext`) is an
  `optional<record:...>` of that record, never bytes: the build derives the
  completion's record from the library's result, the byte string replaced by
  the record, and binds `done.invoke.<id>` to it. Where the completion is read,
  the bytes the host answered are read as the record: present when `valid`
  (`opened`) is true and `error` empty, absent otherwise. Bytes that are no
  value of the record (an item of another type, a key given twice, a field
  missing, a value outside its type, a variant no enum declares, bytes after
  the record; a float that is not finite and a text that holds U+0000, which a
  payload spells none of) make the completion the operation's failure
  `not_a_record`: `error` the code, `valid` (`opened`) false, every other field
  at its zero, the record absent. The object verified or opened, so the
  failure is not an answer about the signature or the seal: it says the signer
  or the sealer did not state a record of the kind the machine asked for.
- **Refused at parse** (`validation/payload-record-schema`): the attribute on
  an `<invoke>` of any other type, an empty one, one that names no event schema
  the document imports (an enum or a union included, which no record codec
  writes), a record whose type has no CBOR form, and a record whose largest
  deterministic CBOR is past the bound of the byte string it crosses in: `C` of
  the record (docs/adr/0101, section 4.1), the `<NAME>_MAX_BYTES` a codec
  holding it states, against `cose_payload` for a signing and a verification
  and `cose_plaintext` for a seal and an opening, as this build resolved them,
  so `--limit` that raises the bound admits the record. A `<param>` that is no
  record of the schema is refused where it is written
  (`scxml/static-datamodel-rule`).
- **The lanes.** Rust and C++ serve it (docs/adr/0109, D0): the record codec
  is `sce_rust_runtime::record_cbor` (the runtime's feature `record-cbor`) and
  `common/RecordCbor.h`, each the one map between the record's JSON, as a
  machine writes and reads a payload, and its CBOR, under the description of
  the record's type the build writes once. Every other lane refuses the
  machine by name: `codegen/standard-host-type-not-served` while it serves no
  COSE operation, and `generate/unsupported-feature` naming `sce:payload` once
  it serves them and not the record codec.
- **Offline verification.** A client verifies what its server signed holding
  nothing but the server's public key: a Rust trust set holds public keys
  (`TrustedKey`), and a C++ trust set names a signing key the host holds or a
  public key it was given (`CoseFeature::setTrustedKeys`), so no seed and no
  server is in the path. `AMachineVerifiesASignedRecordFromAFileWithNoServer`
  (`tests/integration/APayloadRecordTest.cpp`) reads a `COSE_Sign1` from a
  file and receives the record.
- **The two lanes agree byte for byte** on
  `tests/host_features/payload_record_cross_language.json`: the description of
  the note of `static_host_cose`, the note's deterministic CBOR, its
  `COSE_Sign1` under an Ed25519 key and its `COSE_Encrypt0` under a fixed
  nonce, and bytes that are no note with their objects, computed by an
  implementation that shares nothing with either lane. Rust is held to it by
  `backends/rust/host-features/tests/payload_record_cross_language.rs`, C++ by
  `tests/integration/APayloadRecordTest.cpp`, and the scenario of
  `static_host_cose` signs, verifies, seals and opens the note on both.

**Derive.** The input keying material is `secret` or the key `input_key_id`
names: a configured symmetric key, a derived key, or a generated key of use
`derive` (section 13); a signing key is not one. An empty `salt` is no salt. The
host keeps the derived key in its key ring and answers its id, named by what
derived it (docs/adr/0109, D7): `derived:` and the first 16 bytes, in lowercase
hexadecimal, of the HMAC-SHA-256 under the key ring's naming key of the
deterministic CBOR array `[input, salt, info]`, the input the text of
`input_key_id` or the byte string of the SHA-256 of `secret`. The naming key is
32 bytes the key ring keeps, drawn from the host's random source at its first
derivation. Deriving the same material again is the same id, and the key ring
keeps one key under it; a `derived:` id a saved session holds names the same
key in any process that shares the key store. A derivation that is refused or
fails keeps nothing. A derived key is a symmetric key from then on: sealing,
opening and a further derivation take it by its id, and no configured key has an
id beginning `derived:` or `generated:`.

Signing, verifying and opening change nothing, and repeated give the same
answer. A seal draws a new nonce each time, so two seals of one plaintext are two
objects. A derivation repeated answers the id it answered the first time.

Left to the host: where its configured keys come from and how they are stored,
the key store its key ring keeps its keys in (section 13), and whether
`unavailable` is ever answered for a key service it does not have.

The reference hosts are Rust's `sce_rust_host_features::cose` over the objects of
`sce_rust_crypto::cose`, on the RustCrypto crates, its keys given by
`ReferenceHost::set_cose_keys`, and C++'s on libsodium.
`static_host_cose` and its scenario hold both to the objects of the COSE working
group's examples for these algorithms (`eddsa-examples/eddsa-sig-01.json`,
`chacha-poly-examples/chacha-poly-enc-01.json`; the second's
`intermediates.AAD_hex` spells the context `Encrypt1` and does not open its own
output, which opens under `Encrypt0`), to the bytes of a signature as the
deterministic encoding writes them, and to keys derived by HKDF opening objects
sealed under the same keys elsewhere.

**The two hosts agree byte for byte.** Each writes every object of
`tests/host_features/cose_cross_language.json` through its operations, from the
keys the file gives and, for a seal, the nonce its case fixes, and verifies and
opens every object of it: Rust in
`backends/rust/host-features/tests/cose_cross_language.rs`, C++ in
`AStandardHostCose.ACoseObjectCrossesBetweenTheHosts`
(`tests/integration/AStandardHostCoseTest.cpp`). An object either host signs or
seals is then one the other verifies or opens, in either direction. The file's
objects were computed by an implementation that shares nothing with either
host. A seal's nonce comes from the host's random source (section 11), the
operating system's unless the embedding gives another, which is how a test fixes
it: Rust's `ReferenceHost::set_random_source`, or a source of nonces alone,
`ReferenceHost::set_nonce_source` (a `NonceSource`; `SystemNonces` is the
operating system's), C++'s `CoseCrypto::random`. A source that gives one nonce
twice under one key gives both plaintexts away, so one that is not random is for
such a test.

## 11. Random bytes

A host draws random bytes for a statechart (docs/adr/0109, D5). A machine is a
function of its inputs, and a random value is one of them, as the time of
section 2 is: it is asked of the host, so a scenario, a replay and the judge see
it as an input. Served by the Rust and C++ reference hosts (section 7).

| Operation | `type` | Request | Result |
|---|---|---|---|
| Draw | `sce-host-random-bytes` | `random_bytes_request`: `length` uint16 | `random_bytes_result`: `bytes` bytes (`random_bytes`=256) |

Failure code: `unavailable` (the random source gave no bytes).

Refusal (`invalid_argument`): a `length` of 0, or past the bound of `bytes`.

- `bytes` holds `length` bytes from the host's random source: the operating
  system's cryptographic generator (RFC 4086; `getrandom(2)`,
  `BCryptGenRandom`, libsodium's `randombytes_buf`) unless the embedding gives
  another, which is how a test fixes them.
- Not idempotent: a draw repeated draws again. An invocation a restore starts
  again draws again, which no one can tell, since a completion the machine took
  is in its saved state and one it did not take was seen by nothing.
- The bound of `bytes` is the named bound `random_bytes`, which `--limit` moves;
  a reference host judges `length` by the number its build gave it (section 8).

The same source gives the host's other draws: the id and the material of a
generated key, the naming key of the key ring, and, unless the embedding gives a
source of nonces of its own, a seal's nonce (section 10). The reference hosts
are Rust's `ReferenceHost::set_random_source` (a `RandomSource` of
`sce_rust_crypto::random`; `SystemRandom` is the operating system's,
`ScriptedRandom` gives a test's bytes first) and C++'s `CoseCrypto::random`.

## 12. Keyed digests

A host tags a message with a key it holds and verifies a tag
(docs/adr/0109, D6). A statechart compares no secret: where it would compare a
tag, it asks the host that holds the key to verify it, and the host compares in
time that does not depend on where two tags differ. Served by the Rust and C++
reference hosts (section 7).

| Operation | `type` | Request | Result |
|---|---|---|---|
| Tag | `sce-host-mac` | `mac_request`: `key_id` (64), `message` bytes (`mac_message`=4096) | `mac_result`: `tag` bytes (32) |
| Verify | `sce-host-mac-verify` | `mac_verify_request`: `key_id` (64), `message` bytes (`mac_message`=4096), `tag` bytes (32) | `mac_verify_result`: `valid` bool |

Failure codes: `unknown_key` (the key `key_id` names is not a MAC key the host
holds), `key_retired` (a tag under a retired key of the key ring, which still
verifies), and `unavailable`.

Refusal (`invalid_argument`): an empty `key_id`, or a value past its field's
bound.

- The digest is HMAC-SHA-256 (RFC 2104, FIPS 198-1), the whole 32-byte tag; no
  truncated tag is made or verified.
- `valid` is true when `tag` is the tag of `message` under the key. A tag that
  does not verify, a tag of another length included, is an answer, `valid`
  false, as a signature that does not verify is in section 10.
- A MAC key is a configured one, an HMAC key of any length, or a key of the key
  ring of use `mac` (section 13). A sealing or a signing key does not tag.
- Both operations change nothing, and repeated give the same answer.
- An unkeyed digest is a pure function, and belongs to the standard library,
  not here.

Every comparison of a tag, a digest or another secret in the reference hosts is
one helper of each language: Rust's `sce_rust_crypto::equal::same_secret` (on
`subtle`'s `ConstantTimeEq`), C++'s `CoseCrypto::sameSecret` (on libsodium's
`sodium_memcmp`). The two hosts are held to RFC 4231's test cases 1 to 4, 6
and 7 (case 5 truncates its tag, which this operation does not) by
`tests/host_features/keys_cross_language.json`.

## 13. Keys

A host generates keys, exports a signing key's public key, and retires a key,
for the statechart that names it (docs/adr/0109, D7). The keys a host generates
and derives are its **key ring**'s; the keys of its configuration (section 10)
are not, and keep their meaning. Served by the Rust and C++ reference hosts
(section 7).

| Operation | `type` | Request | Result |
|---|---|---|---|
| Generate | `sce-host-key-generate` | `key_generate_request`: `key_use` enum `key_use`, `label` (64) | `key_generate_result`: `key_id` string (64) |
| Export | `sce-host-key-public` | `key_public_request`: `key_id` (64) | `key_public_result`: `cose_key` bytes (128) |
| Retire | `sce-host-key-retire` | `key_retire_request`: `key_id` (64) | `key_retire_result`: `retired` bool |

Failure codes: `unknown_key` (export: the key is no signing key the host holds;
retire: the key is no key of the key ring, a configured key included) and
`unavailable` (the key store, the key-encryption key or the random source could
not be used).

Refusal (`invalid_argument`): a `key_use` that is not `sign`, `seal`, `mac` or
`derive` (which a typed request cannot carry, see below); an empty `key_id`; a
value past its field's bound.

**`key_use` is an enum, refused where it is written.** The field is a value of
the library's enum `key_use` (`sce:std/host/key_use.scxml`, beside the record),
whose variants are the four uses (docs/adr/0109, step 4 of its order of work). A
machine imports that enum and writes one of its variants, or a variable of it,
at the `<param>`; a text, a value of another enum, and a value of an enum the
machine does not import are refused where they are written
(`scxml/static-datamodel-rule`, or `validation/typed-invoke-request` for a text
literal), so no request names a use that is none of the four (SCE Accepted
Subset §2.12, an enum field of a request). The value crosses to the host as the
name the enum declares for it, which is what the reference hosts read: they
still refuse any other text (`invalid_argument`), which only a request made
without the generated interface can carry. The generated Rust and C++ interfaces
hand the host the variant itself. Since the operation is served by the Rust and
C++ reference hosts alone, which are the lanes that read an enum field of a
request, no lane serves the operation and refuses the field. It is named
`key_use`, not `use`, so that every target language can spell it as a field
(`use` is a keyword of Rust).

**One use.** A generated key has the one use it was generated for (key
separation, NIST SP 800-57 Part 1): a `sign` key signs and exports (section 10),
a `seal` key seals and opens, a `mac` key tags and verifies (section 12), a
`derive` key is the input of a derivation. A key of the key ring asked for
another use is `unknown_key`, as a signing key is when it is asked to seal. A
derived key seals, opens and derives (section 10).

**Generate.** The host draws, in one draw from its random source (section 11),
16 bytes and then 32: the id is `generated:` and the 16 in lowercase hexadecimal,
the key the 32 (an Ed25519 seed, a ChaCha20/Poly1305 key, an HMAC key or input
keying material, by its use). The key ring keeps the key with its use, its
algorithm, the `label`, the moment it was made and its state, and keeps the
identity of the invocation (section 1) with it in one transaction: a generation
repeated under one identity, as a restore starts it again, answers the id of the
key it generated the first time and draws nothing.

**Export.** `cose_key` is the public key of the signing key `key_id` names,
configured or generated, retired or not, as a `COSE_Key` (RFC 9052 section 7;
RFC 9053 section 7.2) in its deterministic encoding: the map `{1: 1, 2: kid,
-1: 6, -2: x}`, key type OKP, the key id as the bytes of its text, curve
Ed25519 and the 32-byte public key. It is what a trust set of another host is
made of, and changes nothing.

**Retire.** `retired` is true when the key was active and is retired now, false
when it was retired already. A retired key verifies, opens and exports, and no
longer signs, seals, tags or derives (`key_retired`). Destroying a key is the
operator's. A retirement repeated changes nothing more.

**The key ring.** A key's material is never in a statechart, in a log or in what
a debugger prints of the host's types. The Rust host keeps its key ring
(`sce_rust_crypto::keyring::Keyring`) in a key store
(`sce_rust_session_store::KeyStore`): in memory unless the embedding gives
another with `ReferenceHost::set_keyring`; the reference durable store is the
session store's SQLite file (`SqliteStore`, layout 7). Each key's material is
kept wrapped under a key-encryption key the store never holds, bound to the
key's id and use, so a file read without that key holds no key and a row's
material moved to another row does not unwrap. The program supplies the
key-encryption key through `KeyWrap`, whose reference adapter
(`KeyEncryptionKey`) reads 32 bytes from a file or a variable the operator names
and wraps with XChaCha20-Poly1305 under a 192-bit random nonce; rotating it
rewraps every key (`Keyring::rewrap_all`). Material is unwrapped for each use
into memory that is wiped when it is dropped (`zeroize`), as the configured keys
are held. The C++ host keeps its key ring in memory, for its own life, each key
wiped with `sodium_memzero` when it is dropped (section 9).

**The issuing key.** Beside the keys a statechart generates and derives, the key
ring holds the key the host issues bearer tokens under (section 14): a MAC key
of the id `issuing:credentials`, 32 bytes drawn from the host's random source
at the first issue and kept as any key of the ring is, wrapped under the
key-encryption key, so that every process that shares the store issues the same
token for the same invocation. No statechart names it: an id a statechart gives
reaches the key ring only when it is `generated:` or `derived:`. Retired by the
operator, it issues nothing more (`key_retired`).

The two hosts agree byte for byte on `tests/host_features/keys_cross_language.json`:
every tag of RFC 4231, every derived key's id and key under the file's naming
key, every generated key's id, `COSE_Key`, signature and tag from the file's
random bytes, and every configured signing key's `COSE_Key`: Rust in
`backends/rust/host-features/tests/keys_cross_language.rs`, C++ in
`AStandardHostKeys.KeysAndTagsCrossBetweenTheHosts`
(`tests/integration/AStandardHostKeysTest.cpp`). `static_host_cose` and its
scenario hold both hosts to sections 10 to 15 through a machine.

## 14. Credentials

A statechart decides who gets a bearer token and whose credentials are taken
away; its host keeps them (docs/adr/0109, D3). The authority is the document's
and the storage the host's: the host keeps each credential as the SHA-256 of its
secret, never the secret, in the credential store the server of the `https`
binding reads (SCE_MESH.md section 22.5), so a token a statechart issued is the
one the server takes. Served by the Rust and C++ reference hosts (section 7).

| Operation | `type` | Request | Result |
|---|---|---|---|
| Issue | `sce-host-credential-issue` | `credential_issue_request`: `name` (256), `ttl_s` uint32 | `credential_issue_result`: `token` string (64) |
| Revoke | `sce-host-credential-revoke` | `credential_revoke_request`: `name` (256) | `credential_revoke_result`: `revoked` uint32 |

Failure codes: `unavailable` (the credential store, the key ring, the random
source or the clock could not be used) and, for an issue, `key_retired` (the
operator retired the issuing key, section 13).

Refusal (`invalid_argument`): an empty `name`, or one past its bound; a
`ttl_s` of 0.

**Issue.** `token` is `sce1.` followed by the base64url (RFC 4648 section 5),
without padding, of the HMAC-SHA-256, under the key ring's issuing key, of the
deterministic CBOR array `[session id, invoke id, start, name]`: the identity of
the invocation (section 1) and the name. It is 48 bytes of text. The host keeps
the SHA-256 of the text with the name, the moment the host's clock read
(`issued_ms`) and the moment `ttl_s` seconds later (`expires_ms`), not revoked.
The token passes through the statechart once, to be sent to its client, and so
through that session's checkpoint and the client's outbox: a deployment that
issues tokens this way stores those encrypted (docs/adr/0109, D8).

- An issue repeated under one identity, as a restore starts the invocation
  again, answers the same token, and keeps the row of the first issue: its
  expiry, and its revocation if it was revoked since. The host keeps no token in
  the clear to answer it with: the token is the tag of the identity.
- Another start of the `<invoke>`, or another name, is another token. A name may
  hold several tokens at once.
- The name is part of the tag, so a token issued for one name is never the token
  of another, whatever the identity.

**Revoke.** `revoked` is how many credentials of `name` the revocation took
away: its tokens not revoked before, which are kept as revoked, and its
installation pin, which is removed, so that the next secret the name brings is
pinned. Both are one transaction of the store. A revocation repeated takes
nothing more and answers 0. The C++ host keeps no pin (section 9), and counts
tokens only.

**Checking a token.** A server's `bearer` check over the store
(`StoredTokens`, `sce_rust_crypto::credential`) digests a presented token, finds
the row of its digest, compares the row's name with the name the request claims
in constant time, and takes it before `expires_ms` and only while it is not
revoked; anything else is `401`. A statechart compares no token: where it would,
the host that holds the store does (section 12).

**The store.** The Rust host keeps its credentials in a credential store
(`sce_rust_session_store::CredentialStore`): in memory (`MemoryCredentials`)
unless the embedding gives another with `ReferenceHost::set_credentials`; the
reference durable store is the session store's SQLite file (`SqliteStore`,
layout 7), which the server's authenticator is given too. The C++ host keeps its
tokens in memory, for its own life (`CredentialFeature`, host/CredentialHost.h).
A token's text is written in time that does not depend on its bytes (`base64ct`
in Rust, libsodium's `sodium_bin2base64` in C++), since a token is a secret.

The two hosts agree byte for byte on the token of one identity and name under
one issuing key: Rust in `backends/rust/host-features/tests/credentials.rs`, C++
in `AStandardHostCredentials.ATokenIsTheTagOfTheIdentityAndTheNameUnderTheIssuingKey`
(`tests/integration/AStandardHostCredentialsTest.cpp`).

## 15. Audit log

A statechart appends what happened to a durable log its host keeps
(docs/adr/0109, D14): append-only, each entry chained on the one before,
checkpointed under the host's audit key, redacted by the schema of the entry,
and kept for a time each stream states. A reader who holds the audit public key
proves that an entry was changed, removed or cut from the end. It is not the
bounded log of docs/adr/0062, which forgets its oldest entry, nor `<log>`, which
is diagnostic output. Served by the Rust and C++ reference hosts (section 7).

| Operation | `type` | Request | Result |
|---|---|---|---|
| Append | `sce-host-audit-append` | `audit_append_request`: `stream` (64), `entry` (`audit_entry`=4096), `sensitivity` (`audit_marks`=1024) | `audit_append_result`: `sequence` string (20) |

Failure codes: `unknown_stream` (the host keeps no policy for the stream),
`key_retired` (the operator retired the audit key or the pseudonymisation key)
and `unavailable` (the log, the clock, a key or the random source could not be
used).

Refusal (`invalid_argument`): an empty `stream`, or a value past its bound; an
`entry` that is not a JSON object, or that lacks a member its marks name, or a
`personal` mark on a value that is no text (which a typed request cannot carry,
see below).

**The entry is a record of a schema.** `<invoke type="sce-host-audit-append"
sce:entry="SignIn">` names an imported event schema, and the `<param
name="entry">` is a variable, or a field of a record variable, of that record:
the build writes it as its JSON payload, the form every lane writes an event's
payload in, and refuses anything else where it is written (SCE Accepted Subset
§2.12). The document writes `stream` and `entry`; `sensitivity` is the build's.

**Redaction is declared once, on the schema.** A field may carry
`sce:sensitivity="secret"`, never written to the log (the entry holds
`sce-secret` in its place), or `sce:sensitivity="personal"`, on a text or an
optional text, written as `sce-personal:` and the lowercase hexadecimal of its
HMAC-SHA-256 under the log's pseudonymisation key, so that equal values still
correlate and none is in the clear. The build writes the schema's marks into
the request's `sensitivity` (`/password=secret;/email=personal`), and the host
redacts by them before anything is written. A stream may keep personal values in
the clear (`personal: clear`); a secret is never written by any stream. A record
nested in an entry is not redacted by its own marks yet, so the build refuses an
entry schema that holds one.

**The marks hold in a `<log>` too** (docs/adr/0109, Open question 9). Under
`datamodel="sce-static"` a `<log expr>` that reads a field a schema marks, at any
depth, or a value computed from one, writes `sce-secret` for a secret and
`sce-personal` for a personal value in its place: the build lowers the marker
where the expression stood, so every lane and the Interpreter's lowering write
the same text and no host is asked anything. A log line writes no keyed digest:
a diagnostic log has no pseudonymisation key, and a value that correlates there
would still name the person to whoever reads the log. A statechart under any
other data model that imports a schema marking a field is refused at the
import, since no value of it has a type a mark can follow (SCE Accepted Subset
§2.15, "What a `<log>` shows").

**Durable before it answers.** The host writes the entry in its canonical form,
chains it (`chain_n = SHA-256(chain_(n-1) || [stream, n, at_ms, entry])`), and,
when `n` is a multiple of the stream's `checkpoint_every`, signs a checkpoint, a
`COSE_Sign1` over `["sce-audit-1", stream, n, chain_n]` under its audit key, all
in one transaction of its store, and answers `sequence` only once it is durable.
At its orderly stop a host signs the head of every stream. The bytes are
docs/SCE_AUDIT_LOG.md.

- An append repeated under one identity (section 1), as a restore starts the
  invocation again, answers the first sequence and appends nothing: the store
  keeps the identity in the transaction of the append.
- **Retention.** A stream keeps its entries for its `retain_days`; a host's pass
  deletes older entries by whole segments, each ending at a checkpoint, which
  then anchors what remains, so the rest stays verifiable.
- **The keys.** The audit key is a signing key the host draws at its first
  checkpoint and the pseudonymisation key a MAC key it draws at its first
  personal value, both from its random source (section 11); the Rust host keeps
  them in its key ring (`audit:sign`, `audit:pseudonym`, section 13), the C++
  host in memory for its own life.

**The streams.** A host keeps the streams its embedding configures, each with
its `checkpoint_every`, its `retain_days` and how it writes a personal value:
Rust's `ReferenceHost::set_audit_streams`, C++'s `AuditFeature::setStreams`. The
Rust host appends to the audit store it is given (`set_audit_store`, an
`sce_rust_session_store::AuditStore`: in memory unless the embedding gives
another; the reference durable store is the session store's SQLite file,
`SqliteStore`, layout 7, which a server keeps sealed at rest), signs at its stop
with `audit_stop` and retains with `audit_retain`; a reader verifies a stream
with `sce_rust_crypto::audit::verify`. The C++ host keeps its log in memory
(`AuditFeature`, host/AuditHost.h; section 9).

The two hosts agree byte for byte on `tests/host_features/audit_cross_language.json`:
every redaction, chain and checkpoint of the file. Rust in
`backends/rust/crypto/tests/audit.rs` and
`backends/rust/host-features/tests/audit.rs`, C++ in
`tests/integration/AStandardHostAuditTest.cpp`.
