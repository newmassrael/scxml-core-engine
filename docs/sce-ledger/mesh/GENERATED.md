# GENERATED.md — atomic store derived view

this file `mnemosyne-cli generate-docs` output — direct no edit. atomic store (`docs/.atomic/workspace.atomic.json`) in mutate primitive (`set-section-*` / `append-changelog-entry`) pass and then re-generate.

Source: `docs/sce-ledger/mesh/.atomic/workspace.atomic.json`

---

## Sections

### §mesh-1. Vision













### §mesh-10. Event Ordering and Concurrency













### §mesh-10.1. Ordering Guarantees













### §mesh-10.10. `OutboundBuffer` — readiness-gated outbound admit











**Implementations**:
- sce/include/mesh/CommunicationError.h:ReasonCode
- sce/include/mesh/InvokeCorrelation.h:cancelAllPending
- sce/include/mesh/MeshUuidKey.h
- sce/include/mesh/OutboundBuffer.h:markNotReady
- sce/include/mesh/OutboundBuffer.h:markReady




### §mesh-10.2. Backpressure and Flow Control













### §mesh-10.3. Thread Safety Model













### §mesh-10.4. Transport Contract











**Implementations**:
- sce/include/mesh/transports/CustomTcpTransport.h




### §mesh-10.4.1. Transport Lifecycle Invariants











**Implementations**:
- sce/include/mesh/InvokeCorrelation.h:cancelAllPending
- sce/include/mesh/InvokeCorrelation.h:registerInvoke
- sce/include/mesh/OutboundBuffer.h:markNotReady




### §mesh-10.4.2. Transport Descriptor Interface













### §mesh-10.4.3. Conformance Verification













### §mesh-10.5. Duplicate Suppression











**Implementations**:
- sce/include/mesh/DedupRouter.h:DedupRouter
- sce/include/mesh/DedupRouter.h:kCapacity
- sce/include/mesh/OrderingBuffer.h
- sce/include/mesh/OutboundBuffer.h
- sce/include/mesh/RetryingDispatcher.h:send_with_retry




### §mesh-10.6. Sequence Ordering Buffer











**Implementations**:
- sce/include/mesh/OrderingBuffer.h
- sce/include/mesh/OutboundBuffer.h




### §mesh-10.6.1. deploy.yaml schema











**Implementations**:
- sce/include/mesh/OrderingBuffer.h:OrderingBuffer




### §mesh-10.6.2. Dispatch decision













### §mesh-10.6.3. Sequence stamping











**Implementations**:
- sce/include/mesh/CommunicationError.h:ReasonCode
- sce/include/mesh/MeshEnvelope.h:sequence_no
- sce/include/mesh/OrderingBuffer.h:admit




### §mesh-10.6.4. Receiver buffer











**Implementations**:
- sce/include/mesh/CommunicationError.h:ReasonCode




### §mesh-10.7. `_event` Field Wiring for Distributed Events











**Implementations**:
- sce/include/mesh/InvokeCorrelation.h:InvokeCorrelation




### §mesh-10.7.1. Structured `_event.data` for `error.*` events











**Implementations**:
- sce/include/mesh/CommunicationError.h:detail
- sce/include/mesh/CommunicationError.h:envelope_id
- sce/include/mesh/CommunicationError.h:source
- sce/include/mesh/CommunicationError.h:toJsonBytes
- sce/include/mesh/CommunicationError.h:transport_error
- sce/include/mesh/MeshDispatch.h:dispatchEnvelope
- sce/include/mesh/ShmChannel.h:ShmChannel




### §mesh-10.8. Delayed Send + Cancel (Cross-Process)













### §mesh-10.9. Origin Identity — `source` vs `routing_id`











**Implementations**:
- sce/include/mesh/MeshEnvelope.h:routing_id
- sce/include/mesh/OutboundBuffer.h




### §mesh-11. Performance Characteristics













### §mesh-11.1. AOT State Machine Cost (Local, No Transport)













### §mesh-11.2. Transport Overhead (Added to Local Cost)













### §mesh-11.3. Throughput at 60Hz Game Tick (16.6ms)













### §mesh-11.4. Memory per Instance













### §mesh-11.5. Interpreter vs AOT Comparison













### §mesh-12. Example: Same SCXML, Three Domains













### §mesh-13. Roadmap











**Implementations**:
- sce/include/mesh/MeshUuidKey.h
- sce/include/mesh/transports/SomeipScxmlInvokeEndpoint.h
- sce/include/mesh/transports/ZenohScxmlInvokeEndpoint.h




### §mesh-14. deploy.yaml Schema











**Implementations**:
- sce/include/mesh/CommunicationError.h:timeout_ms
- sce/include/mesh/ParallelCompletionTracker.h
- sce/include/static/StaticExecutionEngine.h:triggerParallelRegionRemoteSend




### §mesh-14.4. Binding value-field placeholders











**Implementations**:
- sce/include/mesh/transports/SomeipScxmlInvokeEndpoint.h:SCE::Mesh::Someip
- sce/include/mesh/transports/ZenohScxmlInvokeEndpoint.h:SCE::Mesh::Zenoh




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











**Implementations**:
- sce/include/mesh/CommunicationError.h:ReasonCode
- sce/include/mesh/transports/SomeipScxmlInvokeEndpoint.h:SCE::Mesh::Someip
- sce/include/mesh/transports/SomeipScxmlInvokeEndpoint.h:availabilityChangeSafely




### §mesh-16.5. Parallel `<final>` barrier











**Implementations**:
- sce/include/mesh/CommunicationError.h:ReasonCode
- sce/include/mesh/CommunicationError.h:parallel_id
- sce/include/mesh/MeshDispatch.h:SCE::Mesh
- sce/include/mesh/MeshEnvelope.h:parallel_id
- sce/include/mesh/MeshEnvelopeCodec.h:SCE::Mesh
- sce/include/mesh/ParallelCompletionTracker.h:ParallelCompletionTracker
- sce/include/mesh/ParallelCompletionTracker.h:onRegionComplete
- sce/include/mesh/ParallelCompletionTracker.h:reset
- sce/include/mesh/PatternKind.h:PatternKind
- sce/include/static/StaticExecutionEngine.h:isGlobalFinalState
- sce/include/static/StaticExecutionEngine.h:onParallelRegionLocalComplete_
- sce/include/static/StaticExecutionEngine.h:setParallelRegionLocalCompleteCallback
- sce/include/static/StaticExecutionEngine.h:setParallelRegionRemoteSendCallback
- sce/include/static/StaticExecutionEngine.h:tick
- sce/include/static/StaticExecutionEngine.h:triggerParallelRegionLocalComplete
- sce/include/static/StaticExecutionEngine.h:triggerParallelRegionRemoteSend
- sce/src/mesh/MeshEnvelopeCodec.cpp:isValidPatternKind




### §mesh-16.6. `<history>` in distributed parallel













### §mesh-16.7. `error.communication` raise policy











**Implementations**:
- sce/include/mesh/CommunicationError.h:ReasonCode
- sce/include/mesh/CommunicationError.h:SCE::Mesh
- sce/include/mesh/CommunicationError.h:attempts
- sce/include/mesh/CommunicationError.h:codec
- sce/include/mesh/CommunicationError.h:invoke_id
- sce/include/mesh/CommunicationError.h:last_seen_ms_ago
- sce/include/mesh/CommunicationError.h:lost_seq_hi
- sce/include/mesh/CommunicationError.h:lost_seq_lo
- sce/include/mesh/CommunicationError.h:machine
- sce/include/mesh/CommunicationError.h:missing_regions
- sce/include/mesh/CommunicationError.h:parallel_id
- sce/include/mesh/CommunicationError.h:partition
- sce/include/mesh/CommunicationError.h:position
- sce/include/mesh/CommunicationError.h:queue_depth
- sce/include/mesh/CommunicationError.h:reason
- sce/include/mesh/CommunicationError.h:target
- sce/include/mesh/CommunicationError.h:timeout_ms
- sce/include/mesh/CommunicationError.h:transport
- sce/include/mesh/CommunicationError.h:transport_error
- sce/include/mesh/CommunicationError.h:transport_status
- sce/include/mesh/CommunicationError.h:window_size
- sce/include/mesh/DedupRouter.h:DedupWindow
- sce/include/mesh/DedupRouter.h:admitWithSignal
- sce/include/mesh/InvokeCorrelation.h:InvokeCorrelation
- sce/include/mesh/InvokeCorrelation.h:cancelAllPending
- sce/include/mesh/InvokeCorrelation.h:cancelAllPendingForTarget
- sce/include/mesh/InvokeCorrelation.h:registerInvoke
- sce/include/mesh/OrderingBuffer.h:OrderingGapEvent
- sce/include/mesh/OutboundBuffer.h:OutboundBuffer
- sce/include/mesh/OutboundBuffer.h:SendResult
- sce/include/mesh/OutboundBuffer.h:admit
- sce/include/mesh/OutboundBuffer.h:markNotReady
- sce/include/mesh/OutboundBuffer.h:markReady
- sce/include/mesh/OutboundBuffer.h:retryable
- sce/include/mesh/ParallelCompletionTracker.h
- sce/include/mesh/RetryingDispatcher.h:RetryingDispatcher
- sce/include/mesh/ShmChannel.h:ShmChannel
- sce/include/mesh/ShmChannel.h:drain
- sce/include/mesh/ShmChannel.h:drainWith
- sce/include/mesh/third_party/AuthClassifier.h:isZenohAuthFailMessage
- sce/include/mesh/transports/CustomTcpTransport.h:ReadResult
- sce/include/mesh/transports/CustomTcpTransport.h:readLoop
- sce/include/mesh/transports/CustomTcpTransport.h:setDecodeErrorHandler
- sce/include/mesh/transports/SomeipScxmlInvokeEndpoint.h:SCE::Mesh::Someip
- sce/include/mesh/transports/SomeipScxmlInvokeEndpoint.h:availabilityChangeSafely
- sce/include/mesh/transports/SomeipScxmlInvokeEndpoint.h:registerWire
- sce/include/mesh/transports/SomeipScxmlInvokeEndpoint.h:setDecodeErrorHandler
- sce/include/mesh/transports/ZenohScxmlInvokeEndpoint.h:setDecodeErrorHandler
- sce/include/mesh/transports/ZenohScxmlInvokeEndpoint.h:start




### §mesh-16.8. Conformance test harness













### §mesh-16.8.1. Harness architecture













### §mesh-16.8.2. IRP distributable subset













### §mesh-16.8.3. Transport selection for harness











**Implementations**:
- sce/include/mesh/transports/CustomTcpTransport.h:parse_endpoint




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











**Implementations**:
- sce/include/mesh/CommunicationError.h:codec




### §mesh-7.6. What Developers Write













### §mesh-7.7. Build-Time Verification













### §mesh-8. Protocol Mapping













### §mesh-8.1. Communication Pattern Semantics











**Implementations**:
- sce/include/mesh/PatternKind.h:PatternKind
- sce/include/mesh/MeshDispatch.h:dispatchEnvelope




### §mesh-8.2. Transport Capability Matrix













### §mesh-8.3. Realization Status (2026-04-13)













### §mesh-9. Remote Invoke Semantics













### §mesh-9.1. Local vs Remote Invoke













### §mesh-9.2. Session ID Management













### §mesh-9.3. Remote Invoke Lifecycle













### §mesh-9.4. Limitations













### §mesh-9.5. `<invoke type="sce:mesh-rpc">` — short-lived RPC











**Implementations**:
- sce/include/mesh/CommunicationError.h:invoke_id
- sce/include/mesh/InvokeCorrelation.h:cancelAllPendingForTarget
- sce/include/mesh/MeshDeadlineScheduler.h:shutdown
- sce/include/mesh/MeshDispatch.h:dispatchEnvelope
- sce/include/mesh/MeshUuidKey.h
- sce/include/mesh/RetryingDispatcher.h:cancelEnvelopeRetry
- sce/include/mesh/RetryingDispatcher.h:onRetryFire
- sce/include/static/StaticExecutionEngine.h:currentEventInvokeId_
- sce/include/static/StaticExecutionEngine.h:onMeshCancel_
- sce/include/static/StaticExecutionEngine.h:onMeshInvoke_
- sce/include/static/StaticExecutionEngine.h:raiseExternal
- sce/include/static/StaticExecutionEngine.h:setMeshCancelCallback
- sce/include/static/StaticExecutionEngine.h:setMeshInvokeCallback




### §mesh-9.6. `<invoke type="scxml">` — full remote SCXML session (Session F)











**Implementations**:
- sce/include/mesh/ChildSessionAdapter.h
- sce/include/mesh/CommunicationError.h:invoke_id
- sce/include/mesh/IChildSession.h
- sce/include/mesh/MeshDispatch.h
- sce/include/mesh/PatternKind.h
- sce/include/mesh/ShmChannel.h:drain
- sce/include/mesh/transports/SomeipScxmlInvokeEndpoint.h:SCE::Mesh::Someip
- sce/include/mesh/transports/SomeipScxmlInvokeEndpoint.h:ScxmlInvokeEndpoint
- sce/include/mesh/transports/SomeipScxmlInvokeEndpoint.h:methodForPattern
- sce/include/mesh/transports/SomeipScxmlInvokeEndpoint.h:send
- sce/include/mesh/transports/ZenohScxmlInvokeEndpoint.h:SCE::Mesh::Zenoh
- sce/include/mesh/transports/ZenohScxmlInvokeEndpoint.h:ScxmlInvokeEndpoint
- sce/include/mesh/transports/ZenohScxmlInvokeEndpoint.h:start
- sce/include/static/StaticExecutionEngine.h:performScxmlInvokeStart
- sce/src/mesh/MeshEnvelopeCodec.cpp:isValidPatternKind




### §mesh-9.6.1. Session establishment











**Implementations**:
- sce/include/mesh/IChildSession.h:sessionId
- sce/include/mesh/IChildSession.h:tick
- sce/include/mesh/MeshEnvelope.h:child_session_id




### §mesh-9.6.2. Envelope extensions for full remote invoke











**Implementations**:
- sce/include/common/DoneDataHelper.h:emitContentLiteral
- sce/include/common/DoneDataHelper.h:evaluateContent
- sce/include/common/EventMetadataHelper.h:EventMetadataHelper
- sce/include/mesh/MeshDispatch.h:dispatchEnvelope
- sce/include/mesh/MeshEnvelope.h:child_session_id
- sce/include/mesh/MeshEnvelopeCodec.h:SCE::Mesh
- sce/include/mesh/PatternKind.h:PatternKind
- sce/include/mesh/ShmChannel.h:ShmChannel
- sce/include/mesh/transports/SomeipScxmlInvokeEndpoint.h:SCE::Mesh::Someip
- sce/include/mesh/transports/SomeipScxmlInvokeEndpoint.h:send
- sce/include/mesh/transports/ZenohScxmlInvokeEndpoint.h
- sce/include/static/StaticExecutionEngine.h:donedataAtFinal
- sce/include/static/StaticExecutionEngine.h:onScxmlInvokeCancel_
- sce/include/static/StaticExecutionEngine.h:onScxmlInvokeParentEvent_
- sce/include/static/StaticExecutionEngine.h:onScxmlInvokeStart_
- sce/include/static/StaticExecutionEngine.h:pendingDonedataAtFinal_
- sce/include/static/StaticExecutionEngine.h:setScxmlInvokeCancelCallback
- sce/include/static/StaticExecutionEngine.h:setScxmlInvokeParentEventCallback
- sce/include/static/StaticExecutionEngine.h:setScxmlInvokeStartCallback
- sce/include/static/StaticExecutionEngine.h:stashDonedataAtFinal
- sce/src/mesh/MeshEnvelopeCodec.cpp:isValidPatternKind




### §mesh-9.6.3. `_event` field wiring (W3C §5.10.2 compliance)











**Implementations**:
- sce/include/mesh/IChildSession.h:sessionId
- sce/include/mesh/MeshDispatch.h:dispatchEnvelope
- sce/include/mesh/MeshEnvelope.h:child_session_id
- sce/include/static/StaticExecutionEngine.h




### §mesh-9.6.4. `<finalize>` semantics preserved













### §mesh-9.6.5. `autoforward="true"` semantics













### §mesh-9.6.6. Inline `<content>` and child SCXML precompilation













### §mesh-9.6.7. Foreign processor compatibility













## Changelog (atomic ledger)

(empty — first atomic entry will populate this section.)

