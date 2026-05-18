# SCE Mesh Conformance Coverage Matrix

**Scope**: this document catalogs the 44 ctest fixtures under `tests/mesh/`
(fixture list obtained via `ctest --test-dir build -R '^mesh_' -N`) against the
SCE_MESH.md sections each fixture verifies. It is not the §16.8 IRP distributed
conformance suite — that suite is Session E2 scope per §16.9 and has not been
built. See "§16.8 status and delta" below.

The matrix exists so the daily mesh ctest coverage is visible from a single
page, so `§10.4 conformance-complete` / `conformance-degraded` classifications
have observable backing, and so the drift between spec promises and ctest
evidence is named rather than implied.

## Bucket definitions

Classification is by the **runtime property under test**, not by the transport.
Every fixture fits exactly one bucket.

| Bucket | Property under test |
|---|---|
| 1. Core primitives (unit) | A helper class (`DedupRouter`, `OrderingBuffer`, `InvokeCorrelation`, etc.) exercised in isolation, no codegen. Evidence for transport-independent §10 / §16.7 invariants. |
| 2. Codegen compile / reject | `sce-codegen` produces code that compiles (or a negative test rejecting an invalid deploy.yaml). No runtime behaviour exercised. Evidence for §10.4.3 gates 1 & 2 (build-time). |
| 3. Two-machine runtime | Two SCXML engines (client + server, or brake + motor) wired through a transport end-to-end. Same OS process unless noted. Evidence for §9.5 mesh-rpc lifecycle, §10.4 per-sender FIFO, §13 topology, §14 pattern dispatch. |
| 4. Ordering / dedup / gap-recovery | Runtime enforcement of §10.5 duplicate suppression and §10.6 sequence ordering, including §16.7 rows 11 (`MISSING_SEQUENCE`) and 12 (`ORDERING_GAP`). |
| 5. Liveness / availability | Runtime detection of transport-level faults that raise `error.communication` (§16.7). |

## Per-fixture table

Column legend:
- **Fixture** — `add_test(NAME …)` from `tests/CMakeLists.txt`.
- **Bucket** — 1–5 per the table above.
- **Spec anchor** — the load-bearing SCE_MESH.md section. Secondary anchors in parentheses.
- **Property verified** — what a failure of this fixture would surface.
- **Process shape** — `1P` = single OS process (threads only); `2P` = fork or spawn; `hybrid` = ctest binary + sidecar daemon.

### Bucket 1 — Core primitives (8 fixtures)

| Fixture | Spec anchor | Property verified | Shape |
|---|---|---|---|
| mesh_envelope_codec_verification | §7.5 (CBOR wire format) | Canonical envelope encode/decode round-trip. | 1P |
| mesh_invoke_correlation_verification | §9.5 (mesh-rpc correlation) | `InvokeCorrelation` insert / match / deadline / cancel semantics. | 1P |
| mesh_dedup_router_verification | §10.5 | `DedupRouter::admit` (source, id) window semantics. | 1P |
| mesh_ordering_buffer_verification | §10.6 | `OrderingBuffer::admit` + `tick` gap-timeout; per-source state isolation. | 1P |
| mesh_outbound_buffer_verification | §10.10, §16.7 row 9 | `OutboundBuffer::admit` overflow drops newest + raises `BACKPRESSURE_DROP`; no coalescing across sustained overflow. | 1P |
| mesh_communication_error_verification | §10.7.1, §16.7 | Structured `error.communication` JSON builder covers all reason codes. | 1P |
| mesh_deadline_scheduler_verification | §9.5, §10.8 | `DeadlineScheduler` register / fire / cancel ordering. | 1P |
| mesh_dispatch_verification | §3.2, §14 | Generated dispatch table selects correct per-pattern handler. | 1P |

### Bucket 2 — Codegen compile / reject (12 fixtures)

| Fixture | Spec anchor | Property verified | Shape |
|---|---|---|---|
| mesh_compile_verification | §7 (build pipeline) | `brake.scxml` + `deploy.yaml` → `brake_transport.h` compiles. | build |
| mesh_invoke_compile_verification | §9.5 | `<invoke type="sce:mesh-rpc">` codegen compiles. | build |
| mesh_srcexpr_compile_verification | §14.4 | `srcexpr` placeholder substitution codegen compiles. | build |
| mesh_shm_compile_verification | §3.3 (shm transport) | SHM transport codegen compiles. | build |
| mesh_someip_compile_verification | §3.3, §8.2 | SOME/IP transport codegen compiles. | build |
| mesh_someip_multipattern_compile_verification | §8.2 (capability matrix) | Multi-pattern SOME/IP codegen compiles. | build |
| mesh_zenoh_compile_verification | §3.3 | Zenoh transport codegen compiles. | build |
| mesh_zenoh_multipattern_compile_verification | §8.2 | Multi-pattern Zenoh codegen compiles. | build |
| mesh_pool_compile_verification | §14 (pool config) | Pool deploy.yaml codegen compiles. | build |
| mesh_event_coverage_rejection | §13, §14.1 | Event unreachable in topology → `sce-build` hard error. | build-reject |
| mesh_pattern_capability_rejection | §8.2 | Pattern unsupported by transport → `sce-build` hard error. | build-reject |
| mesh_middleware_switch_demo | §6 (build profiles) | Shell-level demo: single SCXML + two deploy.yaml → two binaries. | script |

### Bucket 3 — Two-machine runtime (20 fixtures)

| Fixture | Spec anchor | Property verified | Shape |
|---|---|---|---|
| mesh_local_runtime_verification | §10.4 (local transport) | `<send>` across two in-process engines via local callback router. | 1P |
| mesh_invoke_rpc_runtime_verification | §9.5 | mesh-rpc request / reply / correlation; `done.invoke.<id>` raised. | 1P |
| mesh_invoke_deadline_expiry_verification | §9.5, §10.8 | mesh-rpc deadline expiry fires `error.invoke.<id>` with `rpc_status = DeadlineExceeded` per §9.5 L1347. Not a §16.7 catalog condition. | 1P |
| mesh_invoke_cancel_abort_verification | §9.5 | `<cancel>` on an in-flight mesh-rpc aborts correlation entry. | 1P |
| mesh_shm_runtime_verification | §3.3, §10.4 | SHM ring + fork: parent brake → child motor, state change observed. | 2P (fork) |
| mesh_custom_tcp_runtime_verification | §16.8.3, §10.4 (per-sender FIFO) | `custom_tcp` FireForget E2E plus wire-level FIFO assertion over 100 envelopes. | 1P (TCP loopback) |
| mesh_shm_payload_runtime_verification | §7.5 (SHM payload arena) | SHM control ring + payload arena delivers non-trivial payload. | 2P (fork) |
| mesh_someip_runtime | §3.3, §8.2 | vsomeip RPC + FireForget + FieldRead + FieldWrite across brake/motor. | hybrid |
| mesh_someip_rpc_engine_driven | §9.5, §14 | Engine-driven RPC (no manual `handleServerResponse`) over SOME/IP. | hybrid |
| mesh_someip_eventgroup_engine_driven | §8.3 (field notify) | SOME/IP eventgroup notification reaches subscribed client. | hybrid |
| mesh_someip_unsubscribe | §13 (subscription lifecycle) | SOME/IP unsubscribe flows through refcount and stops delivery. | hybrid |
| mesh_zenoh_runtime | §3.3 | Zenoh peer-mode pub/sub across brake/motor. | 1P |
| mesh_zenoh_base_subscribe_verification | §13.8 (state-entry subscribe) | State-entry auto-subscribe + auto-unsubscribe symmetry. | 1P |
| mesh_zenoh_machine_lifetime_subscribe_verification | §13, §14 (Z5a) | deploy.yaml `subscriptions:` → route_send at machine lifetime. | 1P |
| mesh_zenoh_multipattern_runtime | §8.2 | Zenoh multi-pattern (pub/sub + RPC + field) dispatch. | 1P |
| mesh_zenoh_server_runtime | §8.3 | Zenoh queryable server-side getter / setter / field.notify. | 1P |
| mesh_zenoh_rpc_e2e | §9.5 | Zenoh session.get / queryable RPC round-trip. | 1P |
| mesh_zenoh_rpc_engine_driven | §9.5 | Engine-driven RPC over Zenoh (no manual `handleServerResponse`). | 1P |
| mesh_zenoh_eventgroup_engine_driven | §8.3 | Zenoh session.put triggers field.notify without prior get/set. | 1P |
| mesh_pool_zenoh_runtime_verification | §14 (pool) | Pool-deployed motor serves multiple brake clients over Zenoh. | 1P |

### Bucket 4 — Ordering / dedup / gap-recovery (2 fixtures)

| Fixture | Spec anchor | Property verified | Shape |
|---|---|---|---|
| mesh_dedup_injection_verification | §10.5 | Generated `admitInbound` suppresses same-(source,id) duplicates. Mutation-verified. | 1P |
| mesh_order_injection_verification | §10.6, §16.7 rows 11 + 12 | `OrderingBuffer` reorders out-of-order, gap-times out, raises `MISSING_SEQUENCE` / `ORDERING_GAP`. | 1P |

### Bucket 5 — Liveness / availability (3 fixtures)

| Fixture | Spec anchor | Property verified | Shape |
|---|---|---|---|
| mesh_zenoh_liveliness_verification | §16.7 row 8 (`PEER_PARTITIONED`) | Zenoh liveliness token drop → `error.communication` with `target` and `last_seen_ms_ago`. | 1P |
| mesh_zenoh_on_drop_verification | §16.7, §9.5 | Zenoh `session.get` early-cancel → `RpcStatus::Unavailable` → reply drop during shutdown. | 1P |
| mesh_zenoh_server_timeout_verification | §9.5, §10.8 | Zenoh queryable server-side `query_timeout_ms` silently drops stale queries. | 1P |

## Section coverage summary

Coverage is recorded per SCE_MESH.md section. `F` = fully verified by the
named fixtures; `P` = partial coverage with known gaps listed in "§16.8 status
and delta"; `S` = spec-only, no runtime evidence.

| Section | Status | Evidence |
|---|---|---|
| §7.5 CBOR wire format | F | `mesh_envelope_codec_verification`. |
| §8.2 Transport capability matrix | P | Negative path `mesh_pattern_capability_rejection` + positive paths per transport (`mesh_{someip,zenoh}_multipattern_compile_verification`). Not every (transport, pattern) cell is individually asserted. |
| §8.3 Field get/set/notify | F (Zenoh, SOME/IP) | `mesh_zenoh_server_runtime`, `mesh_zenoh_eventgroup_engine_driven`, `mesh_someip_runtime`, `mesh_someip_eventgroup_engine_driven`. |
| §9.5 mesh-rpc lifecycle | F | Correlation (`mesh_invoke_correlation_verification`), runtime (`mesh_invoke_rpc_runtime_verification`), deadline (`mesh_invoke_deadline_expiry_verification`), cancel (`mesh_invoke_cancel_abort_verification`), per-transport engine-driven variants. |
| §9.6 full remote `<invoke type="scxml">` | S | Session F scope (§16.9). Zero fixtures. |
| §10.4 Transport Contract (per-sender FIFO, at-least-once, duplicate tolerance, fault signal) | P | FIFO: `mesh_custom_tcp_runtime_verification` (wire-level, 100 envelopes). Duplicate tolerance: §10.5 fixtures. Fault signal: bucket 5 fixtures. At-least-once: implied by underlying transport (TCP, SOME/IP-TCP, Zenoh reliable); no cross-process retry test. |
| §10.4.1 Transport lifecycle | P | `connect()` / `send()` / `shutdown()` exercised via the runtime fixtures; `reconnect()` path not directly tested. |
| §10.4.2 Transport descriptor interface | F (build-time) | `mesh_pattern_capability_rejection`, `mesh_event_coverage_rejection`, and every `*_compile_verification` fixture. |
| §10.4.3 Conformance verification | P | Gates 1 & 2 (build-time): F — bucket 2. Gates 3 & 4 (runtime): S — the IRP distributed harness and seeded fault injection are Session E2 scope. |
| §10.5 Duplicate suppression | F | Unit (`mesh_dedup_router_verification`) + integration (`mesh_dedup_injection_verification`). Mutation-verified. |
| §10.6 Sequence ordering buffer | F | Unit (`mesh_ordering_buffer_verification`) + integration (`mesh_order_injection_verification`). |
| §10.7 `_event` field wiring | P | Covered indirectly by every runtime fixture that raises `error.communication` or `done.invoke.<id>`. No dedicated field-by-field assertion suite. |
| §10.7.1 Structured `error.*` data | F | `mesh_communication_error_verification` covers every catalog row. |
| §10.8 Delayed send + cancel | P | Deadline scheduler unit test + RPC deadline expiry; `<send delay>` cross-process sender-hold not exercised (Session E2 scope). |
| §13 Topology (inferred pairing, subscription dual-lifecycle) | F | `mesh_zenoh_base_subscribe_verification`, `mesh_zenoh_machine_lifetime_subscribe_verification`, `mesh_someip_unsubscribe`. |
| §14 Pattern dispatch + pool + srcexpr | F | Multi-pattern runtime fixtures + `mesh_pool_zenoh_runtime_verification` + `mesh_srcexpr_compile_verification`. |
| §14 `partitions:` schema (rules 1/2/5/6–12) | F | Rules 1/2/5–11 via `mesh_partition_rule{1,2,6,7,8,9,10,11}_rejection`; rule 12 (five violations) via `mesh_partition_rule12{a,b,c,d,e}_rejection`. Cross-reference enforced by `sce-build/src/mesh/partitions.rs::validate_parallel_root_designation`. |
| §16.3 / §16.4 Distributability analyzer (R1–R4) | S | Session E2 scope. Zero fixtures. |
| §16.5 Parallel `<final>` barrier + `ParallelRegionDone` wire value 21 | F (shm) / S-rejected (custom_tcp) | Tracker primitive at `sce/include/mesh/ParallelCompletionTracker.h` + `mesh_parallel_completion_tracker_verification`; wire-21 dispatch via `MeshDispatch::dispatchEnvelope`; codegen emits Root / NonRoot / SinglePartition branches via `parallel_final.jinja2`. End-to-end inter-partition delivery (NonRoot send → Root tracker → `done.state.<parallel>`) for the `transport_binding: shm` path verified via `mesh_partition_rule12_e2e` (two OS processes, fork+exec). The `transport_binding: custom_tcp` codegen surface has no wire-21 emitter yet; `validate_parallel_root_designation` Pass 2b rejects such configurations at deploy time via `mesh/partition-wire21-custom-tcp-unimplemented` (regression: `mesh_partition_rule12f_rejection`). Finite `barrier_timeout_ms` runtime firing (§16.5 L3500 `PARALLEL_BARRIER_TIMEOUT`) is scheduler-fired and observable end-to-end: `mesh_barrier_timeout_fires_e2e` proves the timer pops `error.communication` and the SM transitions into `<final id="timeout_failed">` when a region never reports completion; `mesh_barrier_timeout_cancels_e2e` proves the cancel-wins-race invariant when a wire-21 `ParallelRegionDone` arrives before the timer. |
| §16.7 `error.communication` catalog | P | Row 1 (`TRANSPORT_UNAVAILABLE`) raised by `OutboundBuffer::markNotReady` on the `true → false` transition (the §10.4.1 "Active → Disconnected" lifecycle edge), driven by SOME/IP `register_availability_handler(is_available=false)` and Zenoh `declare_matching_listener(matching=false)`; the initial `ready_=false` seed state does not emit because no Active phase preceded it (`mesh_outbound_buffer_verification` adds `MarkNotReadyFromInitialStateDoesNotRaise` / `MarkNotReadyAfterReadyRaisesRow1` / `RepeatedMarkNotReadyRaisesPerTransitionOnly` to pin the per-transition discipline). Scope is the SOME/IP + Zenoh subset because those are the transports with native Active↔Disconnected detection; shm / custom_tcp / local are best-effort per §10.4.1 (no `Disconnected` state to enter) and remain Row 1-exempt by construction. Row 2 (`SEND_FAILED`) raised by `OutboundBuffer` at the two dispatcher-fail observation points — `admit` fast path (when the transport API declines a direct send under ready+empty) and `markReady` drain (per declined envelope, captured under `mu_` and emitted in a post-drain loop to preserve §10.10 lock-discipline). The §10.4.1 "Enqueued-but-unsent envelopes are failed individually on Active→Disconnected" clause is satisfied vacuously by OutboundBuffer's `ready_=true ⟹ queue.empty()` invariant: admits under ready fast-path and drain holds `mu_` for its entire scope so no admit can land in the "ready + non-empty queue" branch during drain; the queue at the disconnect moment is therefore empty by construction. Tests `AdmitFastPathDispatchFailRaisesRow2` / `AdmitFastPathDispatchSuccessDoesNotRaise` / `MarkReadyDrainDispatchFailRaisesRow2PerEnvelope` / `MarkReadyDrainMixedSuccessAndFailureRaisesPerFailure` pin the dispatcher-fail contract. Today's emit covers SOME/IP because the SOME/IP dispatcher returns `app.send()`'s bool; Zenoh's dispatcher always returns true (`publisher.put` has no boolean return) and its api-fail surface lands with a Stage 2 atomic that also adds the `transport_error` field for the underlying API error string. Row 4 (`ENVELOPE_CORRUPT`) raised at every codegen TransportRouter `decodeEnvelope` call site via the `raise_envelope_corrupt` macro in `tools/codegen/templates/mesh/cpp/mesh_transport.h.jinja2` (9 sites: SOME/IP and Zenoh subscribers, queryables, RPC replies, eventgroup notifies). Hand-written endpoints (`ShmChannel::drainWith`, `CustomTcpTransport::read_envelope`, `ZenohScxmlInvokeEndpoint`, `SomeipScxmlInvokeEndpoint`) still silently drop malformed CBOR — extending those interfaces to surface decode failures upward is a separate atomic. Row 5 (`INVOKE_CHILD_LOST`, §9.5 mesh-rpc half) raised by `TransportRouter::shutdown` via `InvokeCorrelation::cancelAllPending` per §10.4.1 row 1704 "Outstanding RPC entries are cancelled with reason: INVOKE_CHILD_LOST" — the correlation table now stores `(target, deliver)` per entry so the shutdown callback can surface both `invoke_id` (the entry's UUID v7 key) and `target` (the peer machine name passed at `registerInvoke`) without a parallel reverse index; `active_invokes_` is cleared in the same shutdown block so a racing state-exit `cancelMeshRpc` is a no-op on already-failed invokes. The §9.6 `<invoke type="scxml">` peer-down case (parent-side `activeInvokes_` in generated SCXML documents, signalled by Zenoh liveliness DELETE or SOME/IP availability=false rather than transport shutdown) is a separate future atomic with a different emit signal. Today's emit fires only at transport Shutdown (TransportRouter dtor or explicit `shutdown()` call); a post-init peer-drop fast-path that would fail outstanding RPCs sooner is also Stage 2 scope. `InvokeCorrelationTest` adds `CancelAllPending_IteratesEntriesWithTargetAndErases` / `CancelAllPending_OnEmptyTable_IsNoOp` / `CancelAllPending_NullCallback_IsTolerated` to pin the new API. Row 6 (`PARALLEL_BARRIER_TIMEOUT`) via `mesh_barrier_timeout_fires_e2e` / `mesh_barrier_timeout_cancels_e2e` (runtime fire + cancel-wins-race). Row 7 (`DEDUP_WINDOW_OVERFLOW`) raised at the `admitInbound` and `driveOrderingTick` dedup sites in `mesh_transport.h.jinja2` when `DedupRouter::admitWithSignal` returns `NovelWithEviction` (the runtime cannot retain unbounded history to confirm a leaked duplicate, so eviction at full capacity is the closest operational proxy for "sustained rate exceeds capacity"); `mesh_dedup_router_test` adds `ObserveWithSignalEnumPathing` / `AdmitWithSignalRaisesOverflowOnFirstEviction` / `BoolAdmitContractUnchanged`. Row 8 (`PEER_PARTITIONED`) via `mesh_zenoh_liveliness_verification`. Row 9 (`BACKPRESSURE_DROP`) via `mesh_outbound_buffer_verification` (admit-time overflow + sustained-overflow no-coalescing). Rows 11 / 12 (`MISSING_SEQUENCE` / `ORDERING_GAP`) via `mesh_order_injection_verification`. Row 13 (`REGION_PARTITIONED`) via `mesh_zenoh_region_liveness_verification` (3-segment liveliness DELETE) and `mesh_someip_region_liveness_verification` (vsomeip `register_availability_handler` per RFC F.X-3). Byte-shape pins in `mesh_communication_error_verification` cover eleven rows (`TransportUnavailableShape` / `SendFailedShape` / `EnvelopeCorruptShape` + `EnvelopeCorruptShapeWithPosition` / `InvokeChildLostShape` / `BarrierTimeoutShape` / `DedupWindowOverflowShape` / `PeerPartitionedShape` / `MissingSequenceMinimalShape` / `OrderingGapFullShape` / `RegionPartitionedShape` / `BackpressureDropShape`); rows 3 (`DELIVERY_EXHAUSTED`, needs retry-layer infrastructure) and 10 (`UNAUTHORIZED`, needs auth-layer infrastructure) have neither runtime emit nor end-to-end transport-fault fixture. |
| §16.8 IRP distributed harness | S | Session E2 scope. Zero fixtures. See below. |

## §16.8 status and delta

The §16.8 harness ("run the full W3C IRP suite once single-process, once
distributed; identical verdicts is the pass criterion") is declared but not
built. Concretely, the following artifacts named in §16.8.1–16.8.4 do not
exist in the tree as of commit `1c414320`:

- `tests/w3c_distributed_manifest.yaml` — the `distributable: {yes,
  merged_single_partition, no, forbidden}` classification file.
- `tests/w3c/dist/` — per-test per-partition binaries. The directory is not
  created by any CMake target.
- `tests/w3c/dist/run_distributed.py` — the harness driver.
- The CTest label `w3c_distributed_conformance`.
- The R1–R4 distributability analyzer in `sce-build` (§16.3).
- (closed for `transport_binding: shm` as of `mesh_partition_rule12_e2e`;
  `custom_tcp` rejected at deploy time as of `mesh_partition_rule12f_rejection`)
  Inter-partition transport delivery of wire-21 `ParallelRegionDone`
  envelopes for the `custom_tcp` binding — the wire-21 channel
  emitter only materializes shm. `validate_parallel_root_designation`
  Pass 2b rejects `transport_binding: custom_tcp` on any partition
  participating in a distributed `<parallel>` route via the
  `mesh/partition-wire21-custom-tcp-unimplemented` diagnostic, so the
  configuration gap surfaces at build instead of as a runtime throw.

None of the 44 mesh ctest fixtures satisfy the §16.8 architecture, because
none spawn per-partition OS processes and none cross-compare a distributed
run against a single-process reference run over W3C IRP documents. The
fixtures verify mesh transport primitives and runtime machinery the IRP
harness would consume — necessary, not sufficient.

§16.9 classifies the §16.8 harness as Session E2 scope. This matrix is the
day-0 artifact that makes "what is verified today vs what §16.8 promises"
observable so the drift cannot hide across sessions.

## Reproduction rule for new fixtures

When adding a fixture under `tests/mesh/`:

1. Pick the **narrowest** bucket that matches the property under test. A
   fixture that exercises both a transport primitive and an ordering invariant
   belongs in bucket 4 (the sharper invariant), not bucket 3.
2. Name the SCE_MESH.md section in the fixture's header comment — the same
   section cited here. Secondary anchors go in parentheses in this matrix.
3. Extend this matrix in the same commit that lands the fixture. The matrix is
   the single source of truth for per-section coverage; a fixture without a
   matrix row cannot be cited by §10.4.3 gate 1 / 2, §10.7.1, or §16.7.
4. Update the "Section coverage summary" row if the new fixture moves a row
   from `S` → `P` or `P` → `F`.
5. If the fixture exercises a §16.8 obligation (IRP distributed harness), it
   does not belong in `tests/mesh/` — the IRP harness lives under
   `tests/w3c/dist/` per §16.8.4 and is a separate build target.
