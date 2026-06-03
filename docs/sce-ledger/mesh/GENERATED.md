# GENERATED.md — atomic store derived view

this file `mnemosyne-cli generate-docs` output — direct no edit. atomic store (`docs/.atomic/workspace.atomic.json`) in mutate primitive (`set-section-*` / `append-changelog-entry`) pass and then re-generate.

Source: `docs/sce-ledger/mesh/.atomic/workspace.atomic.json`

---

## Sections

### §mesh-1. Vision














### §mesh-10. Event Ordering and Concurrency














### §mesh-10.1. Ordering Guarantees














### §mesh-10.10. `OutboundBuffer` — readiness-gated outbound admit











**Bindings**:
- [references] sce/include/mesh/CommunicationError.h:ReasonCode
- [references] sce/include/mesh/InvokeCorrelation.h:cancelAllPending
- [references] sce/include/mesh/MeshUuidKey.h
- [implements] sce/include/mesh/OutboundBuffer.h:markNotReady
- [implements] sce/include/mesh/OutboundBuffer.h:markReady





### §mesh-10.2. Backpressure and Flow Control














### §mesh-10.3. Thread Safety Model














### §mesh-10.4. Transport Contract











**Bindings**:
- [implements] sce/include/mesh/transports/CustomTcpTransport.h





### §mesh-10.4.1. Transport Lifecycle Invariants











**Bindings**:
- [implements] sce/include/mesh/InvokeCorrelation.h:cancelAllPending
- [implements] sce/include/mesh/InvokeCorrelation.h:registerInvoke
- [implements] sce/include/mesh/OutboundBuffer.h:markNotReady





### §mesh-10.4.2. Transport Descriptor Interface














### §mesh-10.4.3. Conformance Verification














### §mesh-10.5. Duplicate Suppression











**Bindings**:
- [implements] sce/include/mesh/DedupRouter.h:DedupRouter
- [references] sce/include/mesh/DedupRouter.h:kCapacity
- [implements] sce/include/mesh/OrderingBuffer.h
- [implements] sce/include/mesh/OutboundBuffer.h
- [references] sce/include/mesh/RetryingDispatcher.h:send_with_retry





### §mesh-10.6. Sequence Ordering Buffer











**Bindings**:
- [implements] sce/include/mesh/OrderingBuffer.h
- [implements] sce/include/mesh/OutboundBuffer.h





### §mesh-10.6.1. deploy.yaml schema











**Bindings**:
- [references] sce/include/mesh/OrderingBuffer.h:OrderingBuffer





### §mesh-10.6.2. Dispatch decision














### §mesh-10.6.3. Sequence stamping











**Bindings**:
- [references] sce/include/mesh/CommunicationError.h:ReasonCode
- [references] sce/include/mesh/MeshEnvelope.h:sequence_no
- [references] sce/include/mesh/OrderingBuffer.h:admit





### §mesh-10.6.4. Receiver buffer











**Bindings**:
- [references] sce/include/mesh/CommunicationError.h:ReasonCode
- [implements] sce/include/mesh/OrderingBuffer.h:admit
- [implements] sce/include/mesh/OrderingBuffer.h:OrderingBuffer





### §mesh-10.7. `_event` Field Wiring for Distributed Events











**Bindings**:
- [references] sce/include/mesh/InvokeCorrelation.h:InvokeCorrelation





### §mesh-10.7.1. Structured `_event.data` for `error.*` events











**Bindings**:
- [references] sce/include/mesh/CommunicationError.h:detail
- [references] sce/include/mesh/CommunicationError.h:envelope_id
- [references] sce/include/mesh/CommunicationError.h:source
- [implements] sce/include/mesh/CommunicationError.h:toJsonBytes
- [references] sce/include/mesh/CommunicationError.h:transport_error
- [references] sce/include/mesh/MeshDispatch.h:dispatchEnvelope
- [references] sce/include/mesh/ShmChannel.h:ShmChannel





### §mesh-10.8. Delayed Send + Cancel (Cross-Process)














### §mesh-10.9. Origin Identity — `source` vs `routing_id`











**Bindings**:
- [references] sce/include/mesh/MeshEnvelope.h:routing_id
- [implements] sce/include/mesh/OutboundBuffer.h





### §mesh-11. Performance Characteristics














### §mesh-11.1. AOT State Machine Cost (Local, No Transport)














### §mesh-11.2. Transport Overhead (Added to Local Cost)














### §mesh-11.3. Throughput at 60Hz Game Tick (16.6ms)














### §mesh-11.4. Memory per Instance














### §mesh-11.5. Interpreter vs AOT Comparison














### §mesh-12. Example: Same SCXML, Three Domains














### §mesh-13. Roadmap











**Bindings**:
- [references] sce/include/mesh/MeshUuidKey.h
- [implements] sce/include/mesh/transports/SomeipScxmlInvokeEndpoint.h
- [implements] sce/include/mesh/transports/ZenohScxmlInvokeEndpoint.h





### §mesh-14. deploy.yaml Schema











**Bindings**:
- [references] sce/include/mesh/CommunicationError.h:timeout_ms
- [implements] sce/include/mesh/ParallelCompletionTracker.h
- [references] sce/include/static/StaticExecutionEngine.h:triggerParallelRegionRemoteSend





### §mesh-14.4. Binding value-field placeholders











**Bindings**:
- [implements] sce/include/mesh/transports/SomeipScxmlInvokeEndpoint.h:SCE::Mesh::Someip
- [implements] sce/include/mesh/transports/ZenohScxmlInvokeEndpoint.h:SCE::Mesh::Zenoh





### §mesh-14.5. Cross-transport auto-bridging — rejected














### §mesh-15. Zenoh Transport Template Specification














### §mesh-15.1. Key Expression Mapping














### §mesh-15.2. Session Management














### §mesh-15.3. QoS Configuration (deploy.yaml → Generated Code)














### §mesh-15.4. Deployment Topology














### §mesh-15.5. Zenoh SHM and `shm_transport` Template Relationship














### §mesh-15.6. SCXML Concept → Zenoh Primitive Mapping














### §mesh-15.7. Build Dependencies














### §mesh-15.8. Example: Complete Automotive deploy.yaml with Zenoh














### §mesh-16. Distributed W3C SCXML Conformance














### §mesh-16.1. Conformance claim














### §mesh-16.10. Relationship to W3C SCXML 1.0 Normative Text














### §mesh-16.2. Distributed equivalence (weak)














### §mesh-16.3. Parallel region distributability rule














### §mesh-16.4. Cross-region transition auto-merge











**Bindings**:
- [references] sce/include/mesh/CommunicationError.h:ReasonCode
- [implements] sce/include/mesh/transports/SomeipScxmlInvokeEndpoint.h:SCE::Mesh::Someip
- [implements] sce/include/mesh/transports/SomeipScxmlInvokeEndpoint.h:availabilityChangeSafely





### §mesh-16.5. Parallel `<final>` barrier











**Bindings**:
- [references] sce/include/mesh/CommunicationError.h:ReasonCode
- [references] sce/include/mesh/CommunicationError.h:parallel_id
- [implements] sce/include/mesh/MeshDispatch.h:SCE::Mesh
- [references] sce/include/mesh/MeshEnvelope.h:parallel_id
- [implements] sce/include/mesh/MeshEnvelopeCodec.h:SCE::Mesh
- [implements] sce/include/mesh/ParallelCompletionTracker.h:ParallelCompletionTracker
- [implements] sce/include/mesh/ParallelCompletionTracker.h:onRegionComplete
- [implements] sce/include/mesh/ParallelCompletionTracker.h:reset
- [references] sce/include/mesh/PatternKind.h:PatternKind
- [implements] sce/include/static/StaticExecutionEngine.h:isGlobalFinalState
- [references] sce/include/static/StaticExecutionEngine.h:onParallelRegionLocalComplete_
- [implements] sce/include/static/StaticExecutionEngine.h:setParallelRegionLocalCompleteCallback
- [implements] sce/include/static/StaticExecutionEngine.h:setParallelRegionRemoteSendCallback
- [implements] sce/include/static/StaticExecutionEngine.h:tick
- [implements] sce/include/static/StaticExecutionEngine.h:triggerParallelRegionLocalComplete
- [implements] sce/include/static/StaticExecutionEngine.h:triggerParallelRegionRemoteSend
- [references] sce/src/mesh/MeshEnvelopeCodec.cpp:isValidPatternKind





### §mesh-16.6. `<history>` in distributed parallel














### §mesh-16.7. `error.communication` raise policy











**Bindings**:
- [references] sce/include/mesh/CommunicationError.h:ReasonCode
- [implements] sce/include/mesh/CommunicationError.h:SCE::Mesh
- [references] sce/include/mesh/CommunicationError.h:attempts
- [references] sce/include/mesh/CommunicationError.h:codec
- [references] sce/include/mesh/CommunicationError.h:invoke_id
- [references] sce/include/mesh/CommunicationError.h:last_seen_ms_ago
- [references] sce/include/mesh/CommunicationError.h:lost_seq_hi
- [references] sce/include/mesh/CommunicationError.h:lost_seq_lo
- [references] sce/include/mesh/CommunicationError.h:machine
- [references] sce/include/mesh/CommunicationError.h:missing_regions
- [references] sce/include/mesh/CommunicationError.h:parallel_id
- [references] sce/include/mesh/CommunicationError.h:partition
- [references] sce/include/mesh/CommunicationError.h:position
- [references] sce/include/mesh/CommunicationError.h:queue_depth
- [references] sce/include/mesh/CommunicationError.h:reason
- [references] sce/include/mesh/CommunicationError.h:target
- [references] sce/include/mesh/CommunicationError.h:timeout_ms
- [references] sce/include/mesh/CommunicationError.h:transport
- [references] sce/include/mesh/CommunicationError.h:transport_error
- [references] sce/include/mesh/CommunicationError.h:transport_status
- [references] sce/include/mesh/CommunicationError.h:window_size
- [implements] sce/include/mesh/DedupRouter.h:DedupWindow
- [implements] sce/include/mesh/DedupRouter.h:admitWithSignal
- [implements] sce/include/mesh/InvokeCorrelation.h:InvokeCorrelation
- [implements] sce/include/mesh/InvokeCorrelation.h:cancelAllPending
- [implements] sce/include/mesh/InvokeCorrelation.h:cancelAllPendingForTarget
- [implements] sce/include/mesh/InvokeCorrelation.h:registerInvoke
- [references] sce/include/mesh/OrderingBuffer.h:OrderingGapEvent
- [implements] sce/include/mesh/OutboundBuffer.h:OutboundBuffer
- [references] sce/include/mesh/OutboundBuffer.h:SendResult
- [implements] sce/include/mesh/OutboundBuffer.h:admit
- [implements] sce/include/mesh/OutboundBuffer.h:markNotReady
- [implements] sce/include/mesh/OutboundBuffer.h:markReady
- [references] sce/include/mesh/OutboundBuffer.h:retryable
- [implements] sce/include/mesh/ParallelCompletionTracker.h
- [implements] sce/include/mesh/RetryingDispatcher.h:RetryingDispatcher
- [implements] sce/include/mesh/ShmChannel.h:ShmChannel
- [references] sce/include/mesh/ShmChannel.h:drain
- [implements] sce/include/mesh/ShmChannel.h:drainWith
- [implements] sce/include/mesh/third_party/AuthClassifier.h:isZenohAuthFailMessage
- [references] sce/include/mesh/transports/CustomTcpTransport.h:ReadResult
- [implements] sce/include/mesh/transports/CustomTcpTransport.h:readLoop
- [implements] sce/include/mesh/transports/CustomTcpTransport.h:setDecodeErrorHandler
- [implements] sce/include/mesh/transports/SomeipScxmlInvokeEndpoint.h:SCE::Mesh::Someip
- [implements] sce/include/mesh/transports/SomeipScxmlInvokeEndpoint.h:availabilityChangeSafely
- [implements] sce/include/mesh/transports/SomeipScxmlInvokeEndpoint.h:registerWire
- [implements] sce/include/mesh/transports/SomeipScxmlInvokeEndpoint.h:setDecodeErrorHandler
- [implements] sce/include/mesh/transports/ZenohScxmlInvokeEndpoint.h:setDecodeErrorHandler
- [implements] sce/include/mesh/transports/ZenohScxmlInvokeEndpoint.h:start
- [implements] sce/include/mesh/RetryingDispatcher.h:send_with_retry
- [implements] sce/include/mesh/RetryingDispatcher.h:onRetryFire





### §mesh-16.8. Conformance test harness














### §mesh-16.8.1. Harness architecture














### §mesh-16.8.2. IRP distributable subset














### §mesh-16.8.3. Transport selection for harness











**Bindings**:
- [implements] sce/include/mesh/transports/CustomTcpTransport.h:parse_endpoint





### §mesh-16.8.4. Harness build integration














### §mesh-16.9. Incremental delivery: Sessions E1, E2, F














### §mesh-17. Distributed-Friendly SCXML Design Principles














### §mesh-17.1. Why design matters for AOT + distributed














### §mesh-17.2. Five principles














### §mesh-17.3. Good vs bad patterns














### §mesh-17.4. Data locality rules of thumb














### §mesh-17.5. When to pick `<parallel>` vs `<invoke>`














### §mesh-17.6. Design checklist before distribution














### §mesh-17.7. Heterogeneous deployment as a first-class use case














### §mesh-2. Architecture














### §mesh-3. Three Abstraction Axes














### §mesh-3.1. Scheduler — When to Execute














### §mesh-3.2. Transport Codegen — How to Deliver














### §mesh-3.3. Discovery — Where to Find














### §mesh-4. Discovery Modes and Conflict Resolution














### §mesh-4.1. Static Mode (Build-Time Resolved)














### §mesh-4.2. Scoped Mode (Domain-Partitioned)














### §mesh-4.3. Dynamic Mode (Priority-Based Resolution)














### §mesh-4.4. Event Deduplication














### §mesh-4.5. Instance Lifecycle














### §mesh-5. QoS Model: Deploy-Time Realization














### §mesh-6. Build Profiles














### §mesh-6.1. Vehicle Profile














### §mesh-6.2. IntraECU Profile














### §mesh-6.3. DDS Profile (Full QoS)














### §mesh-6.4. Custom Transport














### §mesh-7. Build Pipeline














### §mesh-7.1. Inputs














### §mesh-7.2. Build Tool Analysis














### §mesh-7.3. Outputs














### §mesh-7.4. Generated Transport Code














### §mesh-7.5. Generated Event Serialization











**Bindings**:
- [references] sce/include/mesh/CommunicationError.h:codec





### §mesh-7.6. What Developers Write














### §mesh-7.7. Build-Time Verification














### §mesh-8. Protocol Mapping














### §mesh-8.1. Communication Pattern Semantics











**Bindings**:
- [references] sce/include/mesh/PatternKind.h:PatternKind
- [implements] sce/include/mesh/MeshDispatch.h:dispatchEnvelope





### §mesh-8.2. Transport Capability Matrix














### §mesh-8.3. Realization Status (2026-04-13)














### §mesh-9. Remote Invoke Semantics














### §mesh-9.1. Local vs Remote Invoke














### §mesh-9.2. Session ID Management














### §mesh-9.3. Remote Invoke Lifecycle














### §mesh-9.4. Limitations














### §mesh-9.5. `<invoke type="sce:mesh-rpc">` — short-lived RPC











**Bindings**:
- [references] sce/include/mesh/CommunicationError.h:invoke_id
- [implements] sce/include/mesh/InvokeCorrelation.h:cancelAllPendingForTarget
- [implements] sce/include/mesh/MeshDeadlineScheduler.h:shutdown
- [implements] sce/include/mesh/MeshDispatch.h:dispatchEnvelope
- [references] sce/include/mesh/MeshUuidKey.h
- [implements] sce/include/mesh/RetryingDispatcher.h:cancelEnvelopeRetry
- [references] sce/include/mesh/RetryingDispatcher.h:onRetryFire
- [references] sce/include/static/StaticExecutionEngine.h:currentEventInvokeId_
- [references] sce/include/static/StaticExecutionEngine.h:onMeshCancel_
- [references] sce/include/static/StaticExecutionEngine.h:onMeshInvoke_
- [references] sce/include/static/StaticExecutionEngine.h:raiseExternal
- [implements] sce/include/static/StaticExecutionEngine.h:setMeshCancelCallback
- [implements] sce/include/static/StaticExecutionEngine.h:setMeshInvokeCallback
- [implements] sce/include/mesh/InvokeCorrelation.h:InvokeCorrelation





### §mesh-9.6. `<invoke type="scxml">` — full remote SCXML session (Session F)











**Bindings**:
- [implements] sce/include/mesh/ChildSessionAdapter.h
- [references] sce/include/mesh/CommunicationError.h:invoke_id
- [references] sce/include/mesh/IChildSession.h
- [implements] sce/include/mesh/MeshDispatch.h
- [references] sce/include/mesh/PatternKind.h
- [implements] sce/include/mesh/ShmChannel.h:drain
- [implements] sce/include/mesh/transports/SomeipScxmlInvokeEndpoint.h:SCE::Mesh::Someip
- [implements] sce/include/mesh/transports/SomeipScxmlInvokeEndpoint.h:ScxmlInvokeEndpoint
- [implements] sce/include/mesh/transports/SomeipScxmlInvokeEndpoint.h:methodForPattern
- [implements] sce/include/mesh/transports/SomeipScxmlInvokeEndpoint.h:send
- [implements] sce/include/mesh/transports/ZenohScxmlInvokeEndpoint.h:SCE::Mesh::Zenoh
- [implements] sce/include/mesh/transports/ZenohScxmlInvokeEndpoint.h:ScxmlInvokeEndpoint
- [implements] sce/include/mesh/transports/ZenohScxmlInvokeEndpoint.h:start
- [implements] sce/include/static/StaticExecutionEngine.h:performScxmlInvokeStart
- [implements] sce/src/mesh/MeshEnvelopeCodec.cpp:isValidPatternKind





### §mesh-9.6.1. Session establishment











**Bindings**:
- [implements] sce/include/mesh/IChildSession.h:sessionId
- [implements] sce/include/mesh/IChildSession.h:tick
- [references] sce/include/mesh/MeshEnvelope.h:child_session_id





### §mesh-9.6.2. Envelope extensions for full remote invoke











**Bindings**:
- [references] sce/include/common/DoneDataHelper.h:emitContentLiteral
- [references] sce/include/common/DoneDataHelper.h:evaluateContent
- [references] sce/include/common/EventMetadataHelper.h:EventMetadataHelper
- [implements] sce/include/mesh/MeshDispatch.h:dispatchEnvelope
- [references] sce/include/mesh/MeshEnvelope.h:child_session_id
- [implements] sce/include/mesh/MeshEnvelopeCodec.h:SCE::Mesh
- [references] sce/include/mesh/PatternKind.h:PatternKind
- [implements] sce/include/mesh/ShmChannel.h:ShmChannel
- [implements] sce/include/mesh/transports/SomeipScxmlInvokeEndpoint.h:SCE::Mesh::Someip
- [implements] sce/include/mesh/transports/SomeipScxmlInvokeEndpoint.h:send
- [implements] sce/include/mesh/transports/ZenohScxmlInvokeEndpoint.h
- [references] sce/include/static/StaticExecutionEngine.h:donedataAtFinal
- [references] sce/include/static/StaticExecutionEngine.h:onScxmlInvokeCancel_
- [references] sce/include/static/StaticExecutionEngine.h:onScxmlInvokeParentEvent_
- [references] sce/include/static/StaticExecutionEngine.h:onScxmlInvokeStart_
- [references] sce/include/static/StaticExecutionEngine.h:pendingDonedataAtFinal_
- [implements] sce/include/static/StaticExecutionEngine.h:setScxmlInvokeCancelCallback
- [implements] sce/include/static/StaticExecutionEngine.h:setScxmlInvokeParentEventCallback
- [implements] sce/include/static/StaticExecutionEngine.h:setScxmlInvokeStartCallback
- [references] sce/include/static/StaticExecutionEngine.h:stashDonedataAtFinal
- [implements] sce/src/mesh/MeshEnvelopeCodec.cpp:isValidPatternKind





### §mesh-9.6.3. `_event` field wiring (W3C §5.10.2 compliance)











**Bindings**:
- [implements] sce/include/mesh/IChildSession.h:sessionId
- [implements] sce/include/mesh/MeshDispatch.h:dispatchEnvelope
- [references] sce/include/mesh/MeshEnvelope.h:child_session_id
- [implements] sce/include/static/StaticExecutionEngine.h





### §mesh-9.6.4. `<finalize>` semantics preserved














### §mesh-9.6.5. `autoforward="true"` semantics














### §mesh-9.6.6. Inline `<content>` and child SCXML precompilation














### §mesh-9.6.7. Foreign processor compatibility














## Changelog (atomic ledger)

(empty — first atomic entry will populate this section.)

