# Standard host features

A statechart that runs under `datamodel="sce-static"` has no operating system to
call. What it needs of one (the time, a place to keep a small value, a
notification, an HTTP exchange) it asks of its host, and the host answers. Every
application needs these four, and an interface each application invents for them
is an interface each host re-implements and each statechart is tied to.

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
| Read | `sce-host-kv-get` | `kv_get_request`: `key` (256) | `kv_get_result`: `found` bool, `value` string (4096) |
| Write | `sce-host-kv-put` | `kv_put_request`: `key` (256), `value` (4096) | `kv_put_result`: `replaced` bool |
| Remove | `sce-host-kv-delete` | `kv_delete_request`: `key` (256) | `kv_delete_result`: `existed` bool |
| Walk | `sce-host-kv-next-key` | `kv_next_key_request`: `after` (256), `prefix` (256) | `kv_next_key_result`: `found` bool, `key` string (256) |

Failure codes: `unavailable` (the store cannot be reached or opened) for all four,
and `quota_exceeded` for a write that would make the store larger than the host
allows, which leaves the store unchanged.

Refusal (`invalid_argument`): a key that is empty or contains U+0000. A `prefix`
and an `after` may be empty.

- A key is compared as its UTF-8 bytes. There is no normalisation: two spellings
  of one character are two keys.
- **Get** answers `found` false and an empty `value` for a key nothing was written
  under. A written empty text is `found` true with an empty `value`. Idempotent.
- **Put** replaces any value. `replaced` is true when the key held a value, whether
  or not it was the same text. Repeating a put with the same arguments leaves the
  same stored state. When it completes, a get started after it on this host
  answers the new value; whether the value survives the host's restart is the
  host's.
- **Delete** answers `existed` false for a key that was not there, which is not a
  failure. Idempotent in effect.
- **Next-key** answers the first key, in ascending order of UTF-8 bytes, that is
  greater than `after` and begins with `prefix`; `after` is excluded, and an empty
  `after` starts at the least key. A statechart enumerates by asking again with the
  key it was given. It is not a snapshot: a key written or removed between two asks
  is seen or missed by the later one, and an enumeration never repeats or skips a
  key that was neither written nor removed during it.

Left to the host: durability (the reference hosts keep the store in memory), the
quota, whether the stores of two applications on one device are kept apart, and
encryption at rest.

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
TLS, or the connection ended), `response_too_large` (the headers or the body do not
fit the record), `invalid_body` (the body is not UTF-8).

Refusal (`invalid_argument`): a `method` that is not one of `GET`, `HEAD`, `POST`,
`PUT`, `PATCH`, `DELETE`, `OPTIONS` (spelled in capitals); a `url` that is not an
absolute `http` or `https` URL with a host, or that holds an ASCII space or control
character (U+0000 to U+0020, U+007F); a header line without a colon, or whose name
is not an RFC 9110 token.

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

## 6. What the build does

- **The types are served without a declaration.** A document that runs an
  `<invoke>` of a standard type needs no `--host-invoker`; every host serves the
  standard types. `sce-codegen check` reports them under `host_invoker_types`.
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
  (`scxml/static-datamodel-rule`): an integer whole in the field's range, a real no
  wider than the field's, a bool, and a text held by a variable whose
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
  library by `the_standard_host_features_are_one_set`.

## 7. Conformance and reference hosts

`tests/host_features/scenarios/` holds one scenario for each feature, and the four
machines that ask for it are
`sce-build/tests/fixtures/static_datamodel/static_host_{kv,clock,notify,http}.scxml`.
A scenario is a list of steps. A step may set what the reference host is like
(`host`), send an event, and state what the machine published and what the host
was asked:

| Member | What it says |
|---|---|
| `host` | directives applied before the step: `clock` (`utc_ms`, `monotonic_ms`, `zone_id`, `utc_offset_s`, each optional), `permission` (what the platform says), `user_decision` (what the user would choose if asked: `granted`, `denied`, `none`), `fail` (operation type to the failure code it fails with; an empty object ends them), `quota` (the most keys the store holds), `http` (what the transport answers: `status`, `headers` as name and value pairs, and `body` as text, `body_hex` as bytes or `body_repeat` as a text and a count; or `fail`, a transport failure; or `hold`, which does not answer) |
| `advance_ms` | moves the engine's manual clock on, which is how a deadline passes |
| `event`, `data` | an external event and its payload |
| `expect.state` | the atomic state the machine is in |
| `expect.variables` | what the machine published |
| `expect.host_invokes` | the operations the host was started with in this step, refused ones included, oldest first: `{"type", "id", "request"}`, the request as the record |
| `expect.host_state` | what the host holds afterwards, for the members named: `store` (key to value), `waiting` and `shown` (the notification ids, in the order posted), `http` (what the transport was given in all: method, URL, request header lines as name and value, body), `cancelled` (the invocations the host was told to stop) |

A reference host is the whole of a feature's semantics, as one object that is told
a start (the type and the request's text parameters) and answers a completion, a
refusal, or nothing yet. Each lane has one, with its own clock and transport, and
a driver that registers it as the invoker of every standard type of a generated
machine and replays the scenarios:

| Lane | Reference host | Driver | Clock | HTTP transport |
|---|---|---|---|---|
| Rust | `backends/rust/host-features` | `backends/rust/scenarios/tests/host_features.rs` | system, scripted | none |
| Kotlin (JVM) | `backends/kotlin/host-features` | `backends/kotlin/tests/src/test/kotlin/com/sce/integration/HostFeaturesTest.kt` | system, scripted | JDK client |
| Go | `backends/go/host-features` | `backends/go/tests/integration/static_datamodel/host_features_test.go` | system, scripted | `net/http` |
| Python | `backends/python/host-features` | `backends/python/tests/integration/static_datamodel/test_host_features.py` | system, scripted | `http.client` |
| C++ | `sce/include/host/StandardHost.h` | `tests/integration/AStandardHostScenarioTest.cpp` | system, scripted | cpp-httplib (`HttplibTransport.h`) |
| C | `backends/c/host-features` | `backends/c/tests/integration/test_host_features.c` | system, scripted | none |
| Swift | `backends/swift/host-features` | `backends/swift/tests/Tests/StaticDatamodelTests/HostFeaturesTests.swift` | system, scripted | none |

Each driver runs the machine on a manual clock and the host on a scripted one, so a
scenario is deterministic. The transports that exist are held by a test against a
local server (carrying headers and a body, no redirect followed, an error status
an answer, a server that is gone `network`). Each lane's host is also held, apart
from the scenarios, to the system clock's monotonic reading never decreasing, to
serving none of a type outside the standard set, and, where a lane's strings can
hold U+0000 (a C string cannot), to refusing a key with one. Every lane is held to
the order of the keys' UTF-8 bytes by a pair of keys that sort differently by
UTF-16 code unit, which is the order a Kotlin or JVM string compares in.

Three things differ by lane and are not hidden:

- **The order of a response's fields.** The Python transport gives them in the
  order received. The Go, Kotlin and C++ transports give them sorted by name
  because their clients keep fields in a map.
- **The Kotlin JVM transport cannot carry a request header the JDK client
  reserves** (`host`, `content-length`, `connection`, `expect`, `upgrade`): a
  request that names one fails `network`.
- **The C host is a hosted reference** (it allocates), and a result record travels
  in one data slot of the machine, `SCE_MAX_DATA_LEN`, which defaults to 256 bytes.
  A C build that serves the HTTP operation sets it to at least 40000
  (`SCE_HF_MIN_DATA_LEN`) and `SCE_MAX_HOST_INVOKERS` to at least 11. A result that
  does not fit the slot is refused as `error.execution`, not cut.

## 8. Not provided

These are not built, and nothing here stands in for them:

- **Swift and iOS.** The build machines carry a Swift toolchain, and `Language`
  names the backend, but it writes only a statechart of `datamodel="null"` so
  far, and every Forge kind is refused for it with
  `codegen/generic-kind-backend-emit-missing`. So there is no iOS host, no Swift
  generated interface for these types, and no `UserNotifications` binding.
- **Android.** No `NotificationManager` host, no Android key-value store
  (`SharedPreferences` or DataStore) and no Android HTTP stack. The Kotlin
  reference host keeps its semantics in common code and the JVM's clock and HTTP
  client beside it; it is not an Android host.
- **A production server host.** No durable store, no quota policy, no outbound HTTP
  with a trust policy, and no notification delivery.
- **A durable store.** The reference stores are in memory. Durability is left to
  the host (section 3).
- **A network HTTP transport for Rust and C.** Their reference hosts are the full
  semantics over a transport the embedding supplies, driven by a scripted one. The
  Rust runtime links no HTTP client, and the C runtime's client posts a form for a
  `<send>` and carries no general exchange.
- **A zone for the Rust host's system clock on a platform other than Unix**, which
  answers `unavailable` for `sce-host-clock-zone`.
- **Delivery of a scheduled notification while the application is not running**,
  and any user interface for a permission request. A reference host answers the
  permission as the scenario sets it.
