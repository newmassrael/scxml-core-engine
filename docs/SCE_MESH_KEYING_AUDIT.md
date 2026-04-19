# SCE Mesh Runtime Keying Audit

**Scope**: every keyed container used by the mesh runtime for dispatch or
per-peer state is enumerated below, with its current axis (what the
insert / find / erase call sites actually key on) compared to the axis
its consumer assumes. Drift — where the two disagree — is called out
explicitly so future fixes can target the specific container rather
than re-derive the audit.

This is a Phase α read-only audit (per `next_session_arch_debt_roadmap.md`
Gap 2). No runtime code is modified; the one known clobber
(`zenoh_subscribers_`) and the machine-name-as-identity axes shared with
Gap 3 are documented for subsequent fix sessions.

## Enumeration scope

**Template-generated containers** live inside each `TransportRouter`
specialization emitted by `tools/codegen/templates/mesh/cpp/mesh_transport.h.jinja2`.
They are gated on the transport mix and pattern mix of the particular
SCXML document being compiled.

**Runtime-library containers** live inside the reusable helpers under
`sce/include/mesh/` and are instantiated once per TransportRouter
owning an instance of the helper.

Containers outside these two scopes (e.g. vsomeip internal tables,
Zenoh session-level routing) are the transport's responsibility and
not audited here.

## Container catalog

### Template-generated (`mesh_transport.h.jinja2`)

| # | Container | File:Line | Current key axis | Assumed axis | Drift |
|---|---|---|---|---|---|
| T1 | `linked_` | 629–633 | SCXML target string | SCXML target string | Clean |
| T2 | `zenoh_subscribers_` | 658 | target string | (target, event) | **DRIFT** (confirmed) |
| T3 | `peer_last_seen_` | 687 | peer `machine_name` | peer identity | Linked to Gap 3 |
| T4 | `pending_rpcs_` | 834 | `CorrelationKey` (UUIDv7 16B) | unique per RPC | Clean |
| T5 | `active_invokes_` | 906 | `InvokeSiteKey{target, field_suffix}` | one live invoke per site | Invariant-bound |
| T6 | `subscription_refcount_` | 918 | `"target\|event"` composite | `(target, event)` | Clean |
| T7 | `pending_server_requests_` | 929 | `CorrelationKey` | unique per inbound SOME/IP request | Layered (dedup + UUIDv7 fallback) |
| T8 | `pending_server_queries_` | 941 | `CorrelationKey` | unique per inbound Zenoh query | Layered (dedup + UUIDv7 fallback) |

Line numbers reference the declaration site in the template; see the
"Insert / erase call sites" subsection below for the mutation sites.

### Runtime library (`sce/include/mesh/`)

| # | Container | File:Line | Current key axis | Assumed axis | Drift |
|---|---|---|---|---|---|
| R1 | `InvokeCorrelation::pending_` | `InvokeCorrelation.h:154` | UUIDv7 16B | unique per invoke | Clean |
| R2 | `DedupRouter::windows_` | `DedupRouter.h:150` | `env.source` (sender `machine_name`) | per-sender window | Linked to Gap 3 |
| R3 | `MeshDeadlineScheduler::active_` | `MeshDeadlineScheduler.h:219` | UUIDv7 16B | unique per deadline | Clean |
| R4 | `OrderingBuffer::state_` | `OrderingBuffer.h:240` | `env.source` (sender `machine_name`) | per-source reorder buffer | Linked to Gap 3 |
| R4' | `OrderingBuffer::PerSender::pending` (nested) | `OrderingBuffer.h:219` | `sequence_no` (uint64) | sequence-ordered, per-source scope | Clean |

### Insert / erase call sites (template)

Per-container mutation sites, used below in the drift judgments:

| Container | Insert | Erase |
|---|---|---|
| `linked_` | 1752 (`linkTo`, setup only) | (never — setup-only) |
| `zenoh_subscribers_` | 2839 (`insert_or_assign` on `EventSubscribe`) | 2843 (`erase` on `EventUnsubscribe`); 1712 (shutdown clear) |
| `peer_last_seen_` | 1344 (liveliness PUT) | 1361 (liveliness DELETE) |
| `pending_rpcs_` | 1088 (`RpcRequest` outbound) | 94 (inbound `RpcReply`) |
| `active_invokes_` | 2299 (`operator[]=` on invoke entry) | 2485 (`erase` on cancel) |
| `subscription_refcount_` | 1103 (`[key]++` on `EventSubscribe`) | 1122 (`erase` at 1→0 on `EventUnsubscribe`) |
| `pending_server_requests_` | 1420, 1478 (`operator[]=` on SOME/IP inbound) | 2131 (`erase` on response) |
| `pending_server_queries_` | 1519 (`insert_or_assign` on Zenoh query) | 2169 (normal reply), 2586 (Z2 timeout) |

## Drift classification

Three categories are used, kept narrow on purpose:

* **Clean** — the current key axis equals the assumed axis, and the
  consumer has no alternative interpretation. Safe to leave alone.
* **Invariant-bound** — axes match *if* an author-facing invariant holds
  (e.g. "one live invoke per site"). The invariant is not enforced by
  the data model itself; it is an assumption the template comment
  makes. Worth documenting so a future change that breaks the
  invariant (e.g. a new SCXML shape) is flagged.
* **Linked to Gap 3** — the container's axis is `machine_name` but
  `machine_name` also serves as the routing identity (see
  `next_session_arch_debt_roadmap.md` Gap 3 — "Origin identity overload").
  Correctness is clean under today's "one instance per machine_name"
  invariant; multi-instance deployments (Gap 7) break it. Not a Gap 2
  fix — the resolution is Gap 3's routing_id split.
* **Layered** — the axis is correct under a defence-in-depth chain
  (dedup + UUIDv7 freshness fallback). The chain's breakage mode is
  worth naming because a change to any one link silently exposes
  clobber on the others.

### Drift — confirmed: T2 `zenoh_subscribers_`

**Axis disagreement**: inserted at `mesh_transport.h.jinja2:2839` with
`zenoh_subscribers_.insert_or_assign(target_name, …)` — keyed by
SCXML target string. The in-tree comment at jinja2:2809–2816 names
the drift directly: "the refcount key is `target|event`, but
`zenoh_subscribers_` is keyed by `target` alone. Two EventSubscribes
on the same target but different events both reach this dispatcher
with distinct `key` expressions, and the second `insert_or_assign`
clobbers the first entry."

**Reproduction (pseudo-scenario, not executed this session)**:

1. Author SCXML with two `<send>` actions to the same mesh target:
   * `<send target="#bus" event="event.subscribe.brake_status"/>`
   * `<send target="#bus" event="event.subscribe.lane_state"/>`
   emitted from two separate states (or from two parallel regions,
   both active) so both subscribes are live simultaneously.
2. The subscribe dispatcher runs once per event. Each call reaches
   `subscription_refcount_`, sees a refcount transition `0→1`
   (different `(target, event)` keys), and falls through to the
   `zenoh::Session::declare_subscriber` branch — correct so far.
3. First `insert_or_assign(target_name, …)` stores the `brake_status`
   subscriber handle under key `"#bus"`. RAII: the handle owns the
   Zenoh subscription.
4. Second `insert_or_assign(target_name, …)` stores the `lane_state`
   subscriber handle under the same key `"#bus"` — the first handle
   is destroyed by the move-assign, which calls into Zenoh to
   undeclare the `brake_status` subscription.
5. Observable end state: only `lane_state` is subscribed at the
   transport layer. `subscription_refcount_` still reports `1` for
   `"#bus|event.subscribe.brake_status"`, so any later
   `EventUnsubscribe` on `brake_status` drives refcount `1→0` and
   *calls erase on `"#bus"`*, which now destroys the `lane_state`
   subscriber — the surviving subscriber is lost, not the one being
   unsubscribed.

**No in-tree fixture reproduces this today**. All existing Zenoh
pub/sub fixtures (`mesh_zenoh_base_subscribe_verification`,
`mesh_zenoh_machine_lifetime_subscribe_verification`,
`mesh_zenoh_multipattern_runtime_verification`) use one event per
target, so `zenoh_subscribers_` never holds more than one live entry
per target and the clobber path is never exercised. The fix + its
reproduction fixture both belong to a Gap 2 follow-up session.

### Drift — invariant-bound: T5 `active_invokes_` (revised 2026-04-19)

**Axis**: `InvokeSiteKey{target, field_suffix}` → latest invoke UUID.
Inserted at jinja2:2299 via plain `operator[]=`:

```cpp
active_invokes_[InvokeSiteKey{target, fieldSuffix}] = uuid;
```

**Revised analysis** (replaces the original "parallel regions clobber
by construction" claim). `field_suffix` is derived in `parser.rs:1249`
as `invoke_id.trim_start_matches('_')`, where `invoke_id` is either:

* The author's `<invoke id="…">` attribute, or
* An auto-generated `_invoke_<N>` whose `N` comes from a parser-local
  monotonic counter (`invoke_counter`).

For two `<invoke>` elements to land on the same `(target, field_suffix)`
key, the author must write the same literal `id=` on both. W3C SCXML
§3.14 forbids duplicate ids across an SCXML document, so the failing
shape named in the original audit (two parallel regions carrying the
same `<invoke id="X">`) is **already spec-invalid input**. Under
W3C-valid SCXML — including auto-id parallel regions, which always
yield distinct `invoke_0` / `invoke_1` suffixes — the map invariant
holds by construction.

Empirical probe: codegen on a `<parallel>` with two identical-target
`<invoke>` elements (no author ids) emits `fieldSuffix == "invoke_0"`
and `"invoke_1"` at the two site-match arms — distinct keys, no
clobber. An explicit W3C §3.14-violating fixture (same author id on
both invokes) currently passes the SCXML parser without diagnostic,
so the *enforcement* of §3.14 is the residual concern, not the mesh
map shape.

**Gap reclassification**: the T5 fix is W3C §3.14 enforcement at the
SCXML parser layer (orthogonal to mesh — any code that keys on
invoke identity inherits the same assumption). Runtime key widening
with `originating_state_id` would solve a symptom, not the root
cause, and would not prevent the adjacent AOT-side breakage of
`done.invoke.<id>` / `error.invoke.<id>` event matching that
duplicate invoke ids already produce regardless of mesh. Codegen
refusing "two-region same-target invokes" would be strictly more
restrictive than W3C §3.14 — rejecting legitimate auto-id shapes
that never collide — and is the wrong axis.

**Action (landed 2026-04-19, commit `452e6791`)**: parser now
rejects duplicate `<invoke>` ids (author-supplied, auto-shadowed, or
author-vs-auto collision) via `ValidationError::DuplicateId` with
`ForgeKind::Statechart`. `SCXMLParser::invoke_ids_seen` tracks all
parsed ids in one set; 3 unit tests pin the coverage. Other SCXML
id namespaces (state / parallel / data) keep their implicit
behavior — out of scope for the invoke-site invariant this audit
section documents.

### Linked to Gap 3 — T3, R2, R4

Three containers key on `machine_name`:

* `peer_last_seen_` (T3) — liveliness registry by peer `machine_name`.
* `DedupRouter::windows_` (R2) — dedup window by sender `machine_name`
  (populated from `env.source`).
* `OrderingBuffer::state_` (R4) — reorder buffer by sender
  `machine_name` (populated from `env.source`).

In all three the axis is correct under the current "one instance per
`machine_name`" invariant. Gap 7 (multi-session server pool) lifts
that invariant, which makes these three containers collide between
instances of the same machine: two instances with the same
`env.source` would share one liveliness entry, one dedup window,
and one reorder buffer. The resolution is Gap 3 — split routing
identity (`routing_id`, per-session UUIDv7) from document identity
(`machine_name`). Once Gap 3 lands, these three containers' axes
migrate from `machine_name` to `routing_id` with no structural
rework beyond the rename.

These are **not** Gap 2 fixes. They are Gap 3 consumers and are
called out here so the Gap 3 fix scope is visible: the axis change
lives in two places (template T3 + runtime R2/R4) plus the envelope
`source` field semantics.

### Layered — T7, T8

`pending_server_requests_` and `pending_server_queries_` key on
`CorrelationKey` derived as `invoke_id ?? correlation_id ?? fresh UUIDv7`.
The axis is correct under a two-layer chain:

1. The §10.5 dedup filter (`admitInbound`) drops duplicate envelope
   ids before the pending-map insert runs.
2. If dedup is bypassed (the dedup filter is opt-in per transport
   per `supplies_dedup`), the UUIDv7 freshness of
   `invoke_id`/`correlation_id`/fresh-fallback makes same-key collision
   statistically negligible.

Both maps use `operator[]=` / `insert_or_assign`, so a hypothetical
key collision silently clobbers. The template comment at
jinja2:1427–1430 acknowledges this is tolerated: "on a duplicate,
the next insert overwrites with the same correlation_id and
handleServerResponse still finds a valid entry when the engine
replies."

Breakage mode worth naming: if a future transport is added with
`supplies_dedup=false` and the client path for that transport starts
re-using `correlation_id` across requests (e.g. a buggy client
stamp), the server-side pending maps lose the first request. The
invariant "client stamps fresh ids" is not enforced at the server
ingress — today no test exercises server-side receive of identical
cids.

No immediate drift. Flagged so a change to either dedup coverage or
client id generation lands with the server-side implication
visible.

### Clean — T1, T4, T6, R1, R3, R4'

No drift, no invariant load-bearing beyond the axis itself.

* T1 `linked_` — setup-only (ctor-path), never mutated after `init()`.
* T4 `pending_rpcs_` — UUIDv7 per RPC, 1:1 insert/erase.
* T6 `subscription_refcount_` — composite `"target|event"` key matches
  the intended semantics; the drift this creates against T2 is
  T2's, not T6's.
* R1 `InvokeCorrelation::pending_` — `emplace` returns `inserted=false`
  on duplicate key; caller contract is explicit.
* R3 `MeshDeadlineScheduler::active_` — `count()` guard before
  `emplace`; duplicate registration is rejected, not clobbered.
* R4' `OrderingBuffer::PerSender::pending` — inner map under R4 per-
  source scope; sequence numbers are unique per sender by sender
  contract.

## §16.7 axes not audited

Two dispatch paths read state that is not a keyed container per the
audit's scope but is worth naming so future audits can decide
whether to extend:

* **Per-target `seq_counter_{target}_`** (jinja2:763–765): a scalar
  counter, not a container. Incremented on the engine's single step
  thread; no concurrency, no keying question.
* **vsomeip internal routing tables** (service / instance / method
  maps inside `vsomeip::application`): opaque to this codebase.
  Mesh relies on vsomeip's documented dispatch semantics; keying
  correctness is vsomeip's responsibility.

## Suggested spec invariant (proposal — not a spec edit)

SCE_MESH.md §10 has no section naming the keying invariants that
the runtime containers above rely on. A future spec commit could
add a short §10.9 (or an extension to §10.3 "Thread Safety Model")
that codifies:

1. **Per-router keyed state axes are deploy-time bounded.** Every
   container keyed on a sender identifier (liveliness, dedup,
   reorder) must be indexable within the deploy.yaml machine
   roster; runtime unbounded keys are out of scope. This closes
   R2 / R4 / T3 documentation debt.
2. **Envelope-uniqueness axes must use UUIDv7.** Correlation tables
   that key on an RPC/query identifier assume 122 bits of entropy
   per key. Reusing `correlation_id` across requests is caller
   contract violation and unspecified runtime behaviour. This
   closes T4 / T7 / T8 / R1 / R3 axis documentation.
3. **Subscription state must key on `(target, event)`.** A container
   that tracks the *transport-level* subscriber keyed on target
   alone must either (a) encode the event into the key explicitly,
   or (b) prove that the declaration path guarantees at most one
   subscription per target. This closes T2 (by forcing a fix) and
   formalises T6 as the SSoT.

Proposal form only. The spec edit lands in a follow-up session,
bundled with the T2 fix so the spec and the implementation move
together.

## What this audit does NOT do

* **Not a fix.** T2 clobber is documented, not resolved. T5 invariant
  holds under W3C-valid input; the residual §3.14 parser enforcement
  is orthogonal to mesh and tracked separately. T3 / R2 / R4 are
  deferred to Gap 3.
* **Not a spec edit.** SCE_MESH.md §10 is unchanged; the "suggested
  spec invariant" section is a proposal, not a spec commit.
* **Not a coverage review.** The existing mesh ctest fixtures were
  not re-run; no new fixtures were added to reproduce the T2
  clobber. Reproduction is described at the pseudo-scenario level
  only.
* **Not bundled with other gaps.** Gap 1 (target synthesis
  asymmetry) and Gap 3 (origin identity) are referenced only where
  their resolution touches an axis in this audit. Their fixes are
  separate preflights.
