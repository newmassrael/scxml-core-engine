# SCE Mesh: Distributed SCXML State Machine Framework

## 1. Vision

### Problem

Distributed systems require state machines that span multiple devices, processes, and networks. Today, developers hand-code state logic on top of communication middleware (Zenoh, SOME/IP, gRPC), resulting in:

- Fragile if/else state management scattered across codebases
- No static checking of cross-device state interactions (topology, event coverage, pattern/transport compatibility)
- Tight coupling between application logic and transport protocols
- Impossible to reason about system-wide behavior from a single artifact

### Solution

SCE Mesh extends the SCXML Core Engine with **location-transparent state machine communication**. The same SCXML source can be deployed across devices, processes, and networks by swapping `deploy.yaml` — subject to the distributed-friendly design principles in §17 (authors who violate them see the analyzer either auto-merge partitions or fail the build in strict mode).

```
SCXML Author sees:           sce-build generates:
  <send target="#motor"/>      Local? → direct call (inlined)
                               Remote? → transport-native API call
                               Same ECU? → shared memory write
                               Different ECU? → SOME/IP, Zenoh, CAN native call
                               Cloud? → gRPC stub call
```

### Core Principle

**SCXML authors write business logic. Platform engineers configure deploy.yaml. Neither needs to know the other's domain.** SCXML declares behavioral intent; deploy.yaml declares platform-specific realization; sce-build generates transport-native code that directly calls each middleware's API. A minimal shared runtime (`sce_mesh_common`: scheduler concepts, MPSC event bridge, and optional dedup/ordering/outbound-buffer primitives — see §10.5/§10.6/§10.10) supplies only what cannot be resolved at build time.

### Design Principle: Build-Time Resolution

SCE's core philosophy is: **resolve at build time what can be resolved at build time.** The AOT engine compiles state machines into switch/case at build time. The expression transpiler compiles ECMAScript expressions into target-language code at build time. Transport dispatch follows the same principle — `deploy.yaml` determines routing at build time, and sce-build generates code that calls transport APIs directly. No vtable-based transport abstraction, no dispatch indirection on the hot path, and transport-native features remain reachable (DDS QoS policies, SOME/IP service model, D-Bus object paths, etc.).

### Positioning

SCE Mesh does not compete with communication middleware — it generates code that calls middleware APIs directly. It sits between SCXML authoring and the transport libraries.

Value SCE Mesh adds on top of existing middleware:

- **Build-time topology and pattern-compatibility validation** — `deploy.yaml` + SCXML are checked together; unsupported pattern/transport combinations fail the build (§8.2)
- **AOT code generation** — transport dispatch compiled to direct native API calls (not interpreted at runtime)
- **Single-source multi-deployment** — one SCXML document, many `deploy.yaml` targets (single-process, multi-host, hybrid edge/cloud), subject to §17
- **Build-time artifacts** — routing tables, event serialization, and transport proxies generated automatically from the two inputs

### Scope and Status

SCE Mesh is a **build-time + minimal-runtime** layer. It is explicitly **not**:

- A continuous state replication layer (no shared mutable datamodel, no synchronous remote reads — §8.1)
- A game-netcode framework (MMO-style `UPROPERTY(Replicated)` / Unity NetCode magic variable sync is an external `sce-game-netcode` concern — §8.1)
- A persistence, area-of-interest, or client-prediction layer (external adapters — see project scope boundaries)

Current realization state (see §8.3 and §13 for authoritative detail):

- **Language coverage**: mesh runtime targets **C++ only**. The other five codegen backends (Kotlin, Rust, Python, Go, plus interpreter path) ship state-machine codegen without mesh. Per-language mesh expansion is evaluated case-by-case, not a parity obligation.
- **Pattern coverage**: `service.fire_forget` is realized end-to-end across local/shm/someip/zenoh. `service.request` / `event.subscribe` / `field.get` pass build-time validation but degrade to FireForget shape at runtime until Phase 3.5 closes the Pattern Realization Gap (§8.3).
- **Conformance scope**: distributed W3C SCXML conformance is a weak-equivalence claim with explicit deferrals; see §16 and `SCE_MESH_CONFORMANCE_MATRIX.md`.

Claims in the rest of this document are contractual relative to this scope. Sections below add, not retract, from it.

---

## 2. Architecture

### Layer Diagram

```
+-----------------------------------------------------------------+
|                      SCXML Documents                             |
|              (design-time, domain-agnostic)                      |
+-----------------------------------------------------------------+
|                      deploy.yaml                                 |
|        (deployment-time, transport-native configuration)         |
+=================================================================+
|                                                                  |
|   +-------------------------------------------------------+     |
|   |          sce-build (AOT Code Generator)                |     |
|   |                                                        |     |
|   |  +---------------+  +--------------------------------+|     |
|   |  | SM Codegen    |  | Transport Codegen               ||     |
|   |  | (existing)    |  | (NEW — per-transport templates) ||     |
|   |  +---------------+  +--------------------------------+|     |
|   |  +---------------+  +--------------------------------+|     |
|   |  | Forge Codegen |  | Topology Analyzer               ||     |
|   |  | (existing)    |  | (NEW — deploy.yaml → routing)  ||     |
|   |  +---------------+  +--------------------------------+|     |
|   +-------------------------------------------------------+     |
|                            |                                     |
|                    generates                                     |
|                            v                                     |
|   +-------------------------------------------------------+     |
|   |           Generated Code (per device)                  |     |
|   |                                                        |     |
|   |  +----------+  +-----------+  +--------------------+  |     |
|   |  | SM Code  |  | Transport |  | Routing Table      |  |     |
|   |  | (AOT)    |  | Code      |  | (constexpr)        |  |     |
|   |  |          |  | (native   |  |                    |  |     |
|   |  |          |  |  API call)|  |                    |  |     |
|   |  +----------+  +-----------+  +--------------------+  |     |
|   +-------------------------------------------------------+     |
|                            |                                     |
|                      links against                               |
|                            v                                     |
|   +-------------------------------------------------------+     |
|   |           Transport Libraries (user-provided)          |     |
|   |                                                        |     |
|   |  +--------+  +-------+  +-----+  +------+  +------+  |     |
|   |  | Zenoh  |  |vsomeip|  | DDS |  | gRPC |  | SHM  |  |     |
|   |  +--------+  +-------+  +-----+  +------+  +------+  |     |
|   +-------------------------------------------------------+     |
|                                                                  |
|   +-------------------------------------------------------+     |
|   |           sce_mesh_common (minimal shared runtime)     |     |
|   |                                                        |     |
|   |  +------------+  +-------------------+                 |     |
|   |  | IScheduler |  | EventQueueBridge  |                 |     |
|   |  | (concept)  |  | (MPSC queue)      |                 |     |
|   |  +------------+  +-------------------+                 |     |
|   +-------------------------------------------------------+     |
+-----------------------------------------------------------------+
```

### Dependency Rule

- SCXML documents reference no platform code and no transport specifics
- deploy.yaml contains all transport-native configuration (QoS, addresses, protocol settings)
- sce-build reads both and generates code that directly calls transport APIs — no runtime indirection
- Generated transport code links against the user-provided transport library (zenoh-c, vsomeip, etc.)
- `sce_mesh_common` provides only the minimal runtime components that cannot be codegen'd: scheduler concepts (OS-dependent timing) and the MPSC event queue bridge (thread synchronization)
- Cross-transport bridging is handled at codegen time: sce-build generates a bridge function that calls both transport APIs, translating events between wire formats. No runtime bridge abstraction needed

### Relationship to Existing SCE Architecture

SCE Mesh extends the existing codegen pipeline within sce-build, following the same pattern as SCE Forge:

```
sce-build (existing Rust binary)
   |
   +-- SM codegen     (existing — SCXML → state machine code)
   +-- Forge codegen  (existing — sce:kind → transform/codec/... code)
   +-- Mesh codegen   (NEW — SCXML + deploy.yaml → transport-native code)

Runtime libraries:
sce_core           (existing — AOT engine, W3C algorithms)
sce_base           (existing — utilities, logging)
sce_scripting      (existing — Lua/JS engines)
sce_runtime        (existing — interpreter)
sce_mesh_common    (NEW — scheduler concepts, MPSC queue, event bridge)
```

`sce_mesh_common` is a thin runtime library (~500-1000 LOC) providing only what cannot be determined at build time: OS-level scheduling and thread-safe event queue bridging. All transport dispatch, routing, serialization, and QoS configuration are resolved at build time by sce-build.

**Consistency with SCE Forge**: Forge adds kind-specific Jinja2 templates to sce-build. Mesh adds transport-specific Jinja2 templates to sce-build. Both follow the same pattern: domain-specific SCXML extensions → build-time analysis → target-language code generation.

```
tools/codegen/templates/
  forge/
    cpp/codec.h.jinja2            # Forge: kind × language templates
    cpp/transform.h.jinja2
    ...
  mesh/
    cpp/mesh_transport.h.jinja2   # Mesh: unified transport routing template
                                  # Handles all transports via {% elif %} dispatch
                                  # per target.transport (local, shm, someip, zenoh, ...)
```

### Relationship to SCE Forge

SCE Mesh and SCE Forge are orthogonal extensions that compose naturally:

```
SCE Forge  = extends WHAT scxml-core-engine can generate (kinds: codec, transform, procedure, ...)
SCE Mesh   = extends WHERE generated code can execute (transports: SOME/IP, Zenoh, SHM, ...)
```

Key integration points:
- **Forge `codec` kind → External protocol adaptation**: When a mesh target communicates with a non-SCE system that requires a specific binary wire format (CAN DBC frame, legacy sensor protocol), the transport template can optionally call Forge codec `encode()`/`decode()` for protocol-native byte packing. This is an opt-in adapter for external systems, not a general event serialization mechanism (see Section 7.5)
- **Forge `procedure` kind → Mesh remote `<invoke>`**: Procedure state machine classes work unchanged with Mesh's remote invoke codegen (see Section 9)
- **Forge `observer` kind → Mesh routing**: Observer-generated threshold events are routed via generated transport code
- **Shared `sce:` namespace**: Both use `http://sce.dev/ext`, both processed at build time by sce-build (see Section 5)
- **Same codegen pattern**: Forge adds kind-specific Jinja2 templates. Mesh adds transport-specific Jinja2 templates. Both are sce-build extensions

**CMake integration**: Generated transport code links against user-provided transport libraries. `sce_mesh_common` provides only the scheduler concepts and MPSC event queue:

```cmake
option(SCE_ENABLE_MESH "Build sce_mesh_common (scheduler + event bridge)" OFF)

# sce_mesh_common is a thin library (~500-1000 LOC):
#   - Scheduler concepts (TickScheduling, EventDrivenScheduling)
#   - MPSC event queue bridge (thread-safe event injection)
# Generated transport code links against:
#   - sce_mesh_common (scheduler)
#   - User-provided transport libraries (zenoh-c, vsomeip, etc.)
```

---

## 3. Three Abstraction Axes

### 3.1 Scheduler — When to Execute

Controls how and when state machines process events. Two fundamentally different scheduling models exist — tick-based batch processing and event-driven immediate dispatch. These are separated into **two distinct C++20 concepts**, consistent with the existing `sce_core` patterns (CRTP + Concepts with C++17 fallback).

#### Why Concepts, Not Virtual Interfaces

The AOT engine's core principle is "zero runtime overhead." Using virtual interfaces for the scheduler would introduce vtable indirect calls on every event dispatch — contradicting this principle. The existing codebase already uses this pattern (`StaticExecutionEngine` via CRTP, `StatePolicyConcepts.h`, `EventQueueConcept.h`), so concept-based separation is the natural choice.

#### Concept Definitions

```cpp
// Tick-based: batch processing at fixed intervals
template<typename S>
concept TickScheduling = requires(S& s, InstanceBatch batch, EventBatch events) {
    { s.tick(batch, events) };
    { s.deadline() } -> std::convertible_to<Duration>;
};

// Event-driven: immediate dispatch on arrival
template<typename S>
concept EventDrivenScheduling = requires(S& s, Instance& inst, Event event) {
    { s.onEvent(inst, event) };
};
```

Concept names use the `-ing` suffix (`TickScheduling`, `EventDrivenScheduling`) to avoid collision with implementation class names (`GameLoopScheduler`, `EventDrivenScheduler`).

The scheduler is the only runtime component in `sce_mesh_common` — OS timing primitives (RTOS periodic tasks, epoll, game loop timers) cannot be resolved at build time. Generated transport and routing code is wired to the scheduler at the application level:

```cpp
// Generated mesh_main.h provides the wiring
// Scheduler is the only template parameter — transport/routing are fully codegen'd
template<typename Scheduler>
void run_mesh(Scheduler& scheduler) {
    // Generated: init transport connections (native API calls)
    init_transports();

    if constexpr (TickScheduling<Scheduler>) {
        while (running_) {
            // Generated: collect events from all configured transports
            auto events = collect_transport_events();
            scheduler.tick(instances_, events);
        }
    } else if constexpr (EventDrivenScheduling<Scheduler>) {
        // Generated: register callbacks on each transport's native notification mechanism
        register_transport_callbacks([&](Event e) {
            scheduler.onEvent(resolve_instance(e.target), e);
        });
        scheduler.run();
    }
}

// Mismatched calls are compile errors, not runtime errors
```

For the rare case where the scheduler type must be chosen at runtime (e.g., from a config file), a `std::variant`-based wrapper provides type erasure without vtable overhead:

```cpp
using AnyScheduler = std::variant<
    GameLoopScheduler,       // satisfies TickScheduling
    RealTimeScheduler,       // satisfies TickScheduling
    EventDrivenScheduler,    // satisfies EventDrivenScheduling
    CooperativeScheduler     // satisfies TickScheduling
>;
// Dispatched via std::visit — no virtual call
```

#### Implementations

| Scheduler | Concept | Behavior | Domain |
|-----------|---------|----------|--------|
| `GameLoopScheduler` | `TickScheduling` | Fixed-rate tick (e.g. 60Hz), batch processing | MMORPG, simulation |
| `RealTimeScheduler` | `TickScheduling` | RTOS periodic task, WCET guarantee, priority inheritance | Automotive ECU |
| `EventDrivenScheduler` | `EventDrivenScheduling` | Process on event arrival (epoll/kqueue) | Cloud, microservices |
| `CooperativeScheduler` | `TickScheduling` | Single-thread round-robin | Bare-metal MCU, AUTOSAR Runnable |

### 3.2 Transport Codegen — How to Deliver

Transport dispatch is resolved at **build time**, not runtime. sce-build reads `deploy.yaml` bindings and generates code that directly calls each transport's native API. There is no `ITransport` runtime interface — a unified **Jinja2 codegen template** dispatches per-target to transport-native code.

#### Transport Groups

Transports are classified into three architectural groups. Each group has its own codegen path because the wire semantics are fundamentally different:

| Group | Examples | Wire Model | Codegen Path |
|-------|----------|-----------|--------------|
| **Byte-stream** | local, shm, someip, zenoh, dds, grpc, kafka, nats, udp | opaque bytes (name + data payload) | Mesh-native wire format (Section 7.5) |
| **Signal-based** | can, lin, flexray | fixed frame + bit-packed signals | Schema-driven from `.dbc`/`.arxml` (Section 7.5) |
| **Field-oriented** | opc_ua, dbus_property | named field + typed value | Pattern-driven (`field.get`/`field.set`, Section 8.1) |

The three groups do **not** share a wire format. They share only the logical `(event_name, data)` model at the SCXML layer. Each group generates a different transport send/receive code path:

- **Byte-stream**: canonical CBOR `MeshEnvelope` (see Section 13 Phase 3.5). `MeshEnvelope.data` carries event payload as bytes.
- **Signal-based**: `.dbc` schema drives bit-packed signal encoding. `event_name` maps to CAN ID via deploy.yaml. `MeshEnvelope.data` is ignored — signals are packed from SCXML `<param>` values through schema-aware codegen.
- **Field-oriented**: event becomes a property get/set. `event_name` maps to node_id/object_path. `data` is marshaled as the protocol's typed value (OPC UA Variant, D-Bus Variant).

**Why three groups, not one**: SCE Mesh's logical model `(name, data)` is universal. The wire format is not — CAN has no name field (frame ID is the routing key), OPC UA has no event concept (only property access). Attempting a single wire format across all groups either constrains byte-stream transports unnecessarily (fixed frame sizes to fit CAN) or loses signal-level type information (opaque bytes over CAN). Each group preserves its native semantics.

Within a group, deploy.yaml-only middleware switching works (e.g., someip ↔ zenoh, both byte-stream). Across groups, build-time pattern inference checks that the transport supports each event's inferred pattern (Section 8.2 Transport Capability Matrix). If unsupported, the build fails with a clear diagnostic.

#### Template Architecture

```
tools/codegen/templates/mesh/
  cpp/
    mesh_transport.h.jinja2      # Unified transport routing template
```

A single template generates `TransportRouter` with per-target send functions. Each target's transport type determines the generated code via `{% elif %}` dispatch:

| Transport | Generated Send Pattern | Field Layout |
|-----------|----------------------|--------------|
| `local` | Direct `engine.processEvent()` call (inlined) | Engine reference (per-target) |
| `shm` | `ShmChannel::send()` | ShmChannel (per-target) |
| `someip` | `vsomeip::application::send()` | vsomeip application + thread (per-target) |
| `zenoh` | `zenoh::Session::put()` | Session (device-shared) |
| `dds` | Template slot (Phase 4) | Per-target |
| `can` | Template slot (Phase 4) | Per-target |

The unified template avoids duplicating common boilerplate (namespace, `TransportRouter` struct, constructor, `route_send()` dispatch) across per-transport files. Transport-specific code lives in clearly marked `{% elif %}` blocks with `NEW TRANSPORT` extension points. Each transport receives the full transport-native configuration from deploy.yaml — no abstraction loss.

#### Generated Code Pattern

For each `<send>` target in an SCXML document, sce-build generates a target-specific send function:

```cpp
// [generated] brake_transport.h
namespace SCE::Generated::brake {

// deploy.yaml: "#motor" → transport: dds, topic: "motor/cmd", qos: { ... }
void send_to_motor(const EventDescriptor& event) {
    // DDS native API — full QoS preserved
    static dds_qos_t* qos = make_motor_qos();  // all 22 DDS QoS policies
    auto payload = serialize_event(event);
    dds_write(motor_writer_, &payload);
}

// deploy.yaml: "#dashboard" → transport: can, address: "can0:0x100"
void send_to_dashboard(const EventDescriptor& event) {
    // CAN native API — frame packing, priority
    struct can_frame frame;
    frame.can_id = 0x100;
    pack_event_to_can(event, frame.data, &frame.can_dlc);
    write(can_socket_, &frame, sizeof(frame));
}

// Routing: compile-time dispatch
void route_send(const char* target, const EventDescriptor& event) {
    // constexpr hash or if-else chain — no vtable, no map lookup
    if (__builtin_strcmp(target, "#motor") == 0) send_to_motor(event);
    else if (__builtin_strcmp(target, "#dashboard") == 0) send_to_dashboard(event);
}

}  // namespace SCE::Generated::brake
```

#### Error Propagation Contract

Transport errors must be surfaced to the SCXML state machine as W3C-compliant `error.communication` events. Each transport template generates error handling code that converts transport-native errors into SCXML events:

```
Protocol error (e.g., SOME/IP TIMEOUT, CAN bus-off, gRPC UNAVAILABLE)
    |
    v
Generated error handler (transport-native):
    DDS:     on_subscription_matched(0)
    vsomeip: availability_handler(NOT_AVAILABLE)
    CAN:     read() returns -1, errno == ENETDOWN
    |
    v
Generated code creates SCXML event:
    name:   "error.communication"        (W3C SCXML 4.9.1)
    data:   { transport: "dds",
              target: "#motor",
              reason: "PEER_LOST",
              original_event: "brake.activate" }
    |
    v
Event enters external event queue of the sending state machine
    |
    v
SCXML <transition event="error.communication"> handles it
```

QoS violation behavior is generated per-transport from deploy.yaml configuration:

| Situation | Generated Behavior |
|-----------|----------|
| Reliable binding send fails | Transport-native retry (DDS: reliability QoS, SOME/IP: method retry), then `error.communication` |
| `<invoke>` deadline exceeded | Timer-based enforcement in generated code, `error.invoke.ID` with `RpcStatus::DeadlineExceeded` |
| Transport disconnected | Transport-native disconnect detection, `error.communication`, instance lifecycle → DRAINING |
| Best-effort binding send fails | Silent drop, no error event (fire-and-forget semantics) |

#### Supported Transports

**Intra-ECU (same device, different processes)**

| Template | Mechanism | Latency | Native Features Preserved |
|----------|-----------|---------|---------------------------|
| `shm_transport` | Ring buffer in shared memory | < 1 us | Zero-copy, lock-free MPSC |
| `dbus_transport` | D-Bus session/system bus | < 100 us | Object paths, interfaces, signals, method calls |

**Vehicle Network (ECU to ECU)**

| Template | Mechanism | Latency | Native Features Preserved |
|----------|-----------|---------|---------------------------|
| `someip_transport` | SOME/IP over Ethernet | < 1 ms | Service model, event groups, SD, method call/fire-and-forget |
| `zenoh_transport` | Zenoh pub/sub + query | < 1 ms | Key expressions, SHM, QoS reliability/congestion, scouting |
| `can_transport` | CAN bus frames | < 1 ms | Frame ID priority, DBC signal packing, CAN FD support |
| `dds_transport` | DDS (Cyclone/Connext) | < 1 ms | **All 22 QoS policies**, typed topics, content filters, partitions |

**Cloud / Game / IoT**

| Template | Mechanism | Latency | Native Features Preserved |
|----------|-----------|---------|---------------------------|
| `grpc_transport` | gRPC stubs | < 10 ms | Unary/streaming RPCs, metadata, interceptors, TLS |
| `zenoh_transport` | Zenoh (reusable) | < 1 ms | Peer/client/router modes, key wildcards |

### 3.3 Discovery — Where to Find

Discovery determines how logical `#target` IDs are resolved to physical addresses. Like transport, discovery follows the build-time-first principle:

**Static Discovery (codegen'd):** deploy.yaml bindings are compiled into `constexpr` routing tables. Zero runtime overhead. This is the primary mode for Phase 1-3.

**Runtime Target Selection (Phase 5):** SCE does not reimplement transport-native service discovery. When a client wants to select among runtime instances of an already-declared binding (cloud auto-scaling, game zone migration), binding value-field placeholders (§14.4) substitute a `<param>` value into the transport's native address at the send site; the transport stack (Zenoh scouting / vsomeip SD) handles peer availability. No SCE-maintained peer table, no `IDiscovery` trait, no `runtime_targets_` map.

```
Phase 1-3: Static (build-time)       — constexpr routing tables
Phase 5:   Runtime target selection  — binding placeholders + transport-native routing
Phase 6:   Multi-session (deferred)  — server-side instance pool
```

#### Static Discovery (Phase 1-3)

deploy.yaml bindings are resolved at build time. Generated code contains compile-time constant routing:

```cpp
// [generated] brake_routing.h — no runtime lookup
namespace SCE::Generated::brake {
constexpr auto route_target(const char* target) {
    if (eq(target, "#motor"))     return &send_to_motor;
    if (eq(target, "#dashboard")) return &send_to_dashboard;
    return &send_error_unknown_target;
}
}
```

#### Runtime Target Selection via Binding Placeholders (Phase 5)

SCE Mesh does not reimplement transport discovery. When a transport has a native routing layer (Zenoh's KeyExpr matching with scouting and gossiping, vsomeip's SD with availability handlers, DDS participant discovery), SCE emits the transport-specific identifier (KeyExpr string or `(service_id, instance_id, method_id)` triple) and hands off to the transport. Peer availability tracking, endpoint resolution, and failover live inside the transport stack.

The one primitive SCE adds above the transport is **binding value-field placeholders**: deploy.yaml binding values may carry `{name}` tokens that are substituted at `<send>` / `<invoke>` time from `<param>` values. This lets a single binding represent a family of runtime targets without SCE maintaining a peer table.

```yaml
# deploy.yaml excerpt
bindings:
  "#player":
    transport: zenoh
    key: "sce/player/{id}"    # {id} resolved at runtime
```

```xml
<!-- SCXML -->
<invoke type="sce:mesh-rpc" src="#player">
  <param name="_mesh_event" expr="'service.request.damage'"/>
  <param name="id" expr="targetPlayerId"/>
</invoke>
```

Codegen emits `zenoh::KeyExpr("sce/player/" + std::to_string(targetPlayerId))` and calls `session.put` or `session.get`. Zenoh's native routing delivers the envelope to whichever peer has declared a matching subscriber; SCE does not enumerate peers.

Transports without a native routing layer (local, shm, custom_tcp, can) do not support placeholder bindings — see §4.3 and §16.8.3. The spec-level capability flag is `TransportDescriptor::supports_pool`.

Cross-transport automatic bridging is explicitly rejected (not deferred) — see §14.5. A machine that receives over one transport and forwards over another does so through explicit SCXML transitions, not middleware-level envelope translation.

---

## 4. Discovery Modes and Conflict Resolution

When multiple transports coexist, their native service discovery mechanisms can conflict. SCE Mesh prevents this through three discovery modes.

### 4.1 Static Mode (Build-Time Resolved)

All bindings are determined at build time. No runtime discovery. Zero ambiguity.

```yaml
# deploy.yaml
topology:
  ecu_a:
    machines:
      brake:
        bindings:
          "#motor":     { transport: someip, address: "service:0x1001" }
          "#dashboard": { transport: can,    address: "can0:0x100" }
```

Generated code contains compile-time constant routing tables. Protocol-native SD is not used.

**Best for**: Deterministic automotive, embedded systems.

### 4.2 Scoped Mode (Domain-Partitioned)

Each protocol owns a URI scheme. No overlap, no competition. **Importantly, URI schemes are declared only in `deploy.yaml`, never in SCXML documents.** SCXML always uses logical `#id` targets to preserve location transparency.

```yaml
# deploy.yaml — URI scopes are a deployment concern, not an authoring concern
discovery:
  mode: scoped
  scopes:
    someip: "ecu://**"       # ECU-to-ECU: SOME/IP only
    dbus:   "proc://**"      # IPC: D-Bus only
    grpc:   "cloud://**"     # Cloud: gRPC only
    zenoh:  "robot://**"     # Robotics: Zenoh only

  # Map logical SCXML targets to scoped URIs
  bindings:
    "#motor":     "ecu://motor"
    "#logger":    "proc://logger"
    "#analytics": "cloud://analytics"
```

SCXML remains domain-agnostic:

```xml
<!-- SCXML always uses #id — no protocol knowledge -->
<send target="#motor" .../>      <!-- deploy.yaml maps to ecu://motor -->
<send target="#logger" .../>     <!-- deploy.yaml maps to proc://logger -->
<send target="#analytics" .../>  <!-- deploy.yaml maps to cloud://analytics -->
```

The runtime resolves `#id` -> scoped URI -> protocol-specific discovery at startup. This preserves the Vision's core promise: the same SCXML runs unchanged across domains.

**Best for**: Multi-protocol systems with clear domain boundaries.

### 4.3 Dynamic Mode (Priority-Based Resolution)

Multiple protocols may discover the same instance. SCE Registry is the single authority.

```yaml
discovery:
  mode: dynamic
  resolution:
    strategy: priority
    priority_order:
      - local       # same process: direct call
      - shm         # same machine: shared memory
      - someip      # same network: SOME/IP
      - zenoh       # same network: Zenoh
      - udp         # remote: UDP
      - grpc        # remote: gRPC (fallback)
  dedup:
    key: instance_id
    ttl: 5s
    tiebreak: priority_order
```

**Best for**: Game server auto-scaling, IoT fleet management.

**Phase 5 scope**: only the "transport-native runtime target selection" subset of dynamic mode is landed. Priority-based cross-transport resolution (`resolution.strategy: priority`), per-instance dedup (`dedup.key: instance_id`), external registries (Consul, etcd, mDNS), and an SCE-level peer table are all deferred or rejected. What IS landed is the binding-value-field placeholder mechanism described in §3.3 and formalised in §14.4 — a deploy.yaml binding may carry `{name}` tokens resolved at `<send>` / `<invoke>` time from `<param>` values. Per-transport capability gating is enforced at build time (`TransportDescriptor::supports_pool`): today Zenoh supports open placeholder substitution, SOME/IP supports a bounded `instances:` list (because vsomeip's `request_service(ANY_INSTANCE)` does not actually subscribe to every instance — see §14.4), and custom_tcp / shm / local / can do not support placeholders at all.

### 4.4 Event Deduplication

Event deduplication applies **only when the discovery mode or transport can produce duplicates** (Dynamic mode with failover, multi-path delivery). It is not required for Static or Scoped modes with single-path routing.

**When enabled** (Dynamic mode, or a deploy.yaml binding with reliable QoS + failover):

```
SCE Event Header (optional, added by generated transport code):
  source:     instance_id
  seq:        per-source sequence number (lightweight counter)
```

Receiver drops events where `seq <= last_seen_seq[source]`.

**When disabled** (Static mode, single-path Scoped mode):

No header overhead. Events are delivered as-is, matching the zero-overhead principle for safety-critical and resource-constrained environments.

### 4.5 Instance Lifecycle

Prevents events from being sent to partially-initialized instances:

```
REGISTERED  -> declared in deploy.yaml
DISCOVERED  -> found by at least one protocol
READY       -> state machine initialized, accepting events
ACTIVE      -> normal operation
DRAINING    -> shutting down, rejecting new events
GONE        -> removed
```

Events arriving before READY are buffered (configurable timeout).

---

## 5. QoS Model: Deploy-Time Realization

QoS is a **deployment concern**, not a state machine concern. A single SCXML document must be deployable across domains (vehicle ECU with DDS QoS profiles, IntraECU with SHM, MMORPG with UDP + best-effort) without rewriting the state logic. Placing QoS in SCXML couples the machine to a specific deployment, which contradicts SCE Mesh's core principle: **the same SCXML runs in three different domains by swapping deploy.yaml**.

**Decision (Session E1 path B/C, 2026-04-14)**: QoS lives entirely in deploy.yaml. SCXML carries only event names and transitions.

### Rationale

- **Location transparency**: SCXML author writes `<send event="brake.activate" target="#motor"/>`. The QoS policy ("reliable, 1ms deadline, highest priority") is a property of the *deployment*, not the message. Same `<send>` in a test harness may have zero QoS; in production may have full DDS reliability+deadline+history.
- **Single source of truth**: QoS is already native to each transport (DDS QoS policies, SOME/IP protocol selection, Zenoh reliability). Declaring it in SCXML creates a second source that must stay in sync.
- **W3C purity**: No SCXML extensions for QoS means graceful execution in any conforming SCXML 1.0 processor.

### Deprecated: SCXML QoS Attributes

`sce:qos`, `sce:deadline`, `sce:priority` (and related `sce:*` send-time attributes) are **removed in Session E1**. The parser rejects them as hard errors (`validation/removed-attribute`); the full migration map and the authoritative migration timeline (Stage 1 warning retired, Stage 2 hard error is the current state) live in §13 "Session C/D attribute deprecation". See there for authoritative wording.

### deploy.yaml QoS (Realization Layer)

Transport-native QoS configuration with full feature access:

```yaml
topology:
  brake_ecu:
    machines:
      brake:
        bindings:
          "#motor":
            transport: dds
            topic: "vehicle/powertrain/motor/cmd"
            qos:
              # Full DDS QoS — all 22 policies available
              reliability: RELIABLE
              durability: TRANSIENT_LOCAL
              deadline: 10ms
              liveliness: AUTOMATIC
              lease_duration: 1s
              history:
                kind: KEEP_LAST
                depth: 5
              resource_limits:
                max_samples: 100
                max_instances: 1
              transport_priority: 7

          "#dashboard":
            transport: someip
            service: 0x1001
            instance: 0x01
            # Full SOME/IP settings — service model preserved
            protocol: TCP          # TCP for reliable, UDP for fire-and-forget
            serializer: someip
            event_group: 0x01
```

sce-build reads the native QoS and generates code that passes these values directly to the transport API:

```cpp
// [generated] — DDS QoS applied natively, not mapped through abstraction
static dds_qos_t* make_motor_qos() {
    auto* q = dds_create_qos();
    dds_qset_reliability(q, DDS_RELIABILITY_RELIABLE, DDS_MSECS(100));
    dds_qset_durability(q, DDS_DURABILITY_TRANSIENT_LOCAL);
    dds_qset_deadline(q, DDS_MSECS(10));
    dds_qset_liveliness(q, DDS_LIVELINESS_AUTOMATIC, DDS_SECS(1));
    dds_qset_history(q, DDS_HISTORY_KEEP_LAST, 5);
    dds_qset_resource_limits(q, 100, 1, DDS_LENGTH_UNLIMITED);
    dds_qset_transport_priority(q, 7);
    return q;
}
```

### Build-Time Validation

With QoS in deploy.yaml only, validation is a one-sided check: each binding's transport-native QoS must be valid for that transport (e.g., a DDS binding must have valid DDS policies). sce-build catches typos via `deny_unknown_fields` on the per-transport config structs.

Cross-transport policy consistency (e.g., "all brake-critical events across DDS and SOME/IP must be reliable") is expressed via deploy.yaml `pattern_defaults` or binding group labels — not via SCXML hints.

### `sce:` Namespace Scope

SCE Mesh introduces **exactly one** SCXML extension: `<invoke type="sce:mesh-rpc">` (spec in §9.5; wire encoding in §13 MeshEnvelope schema). This is a type *value*, not a new attribute, and W3C SCXML §6.4 explicitly reserves invoke `type` as implementation-defined.

SCE Forge continues to use `sce:` attributes for its codegen (`sce:kind`, `sce:type`, `sce:service`, etc.). See SCE_FORGE.md §3.6. SCE Mesh does not introduce any attributes in the `sce:` namespace.

### Future Direction: Distributed Transaction Patterns

The following patterns address distributed consistency but are **out of scope for the initial specification**. They may be defined in a separate extension specification after Phase 1-3 are validated:

- **Saga pattern** (`sce:saga`, `sce:compensate`) — orchestrated compensation transactions
- **Consistency modes** (`sce:consistency="strong|eventual"`) — cross-instance consistency guarantees

These are orchestration-layer concerns, not state machine concerns. Mixing them into the SCXML namespace risks violating W3C compatibility and the core simplicity of the state machine model. If needed, they should be implemented as a higher-level orchestration layer that uses SCE Mesh as its execution substrate.

---

## 6. Build Profiles

A Build Profile is a deploy.yaml configuration that selects a Scheduler type and declares transport bindings. sce-build reads the profile and generates all transport-specific code. The scheduler remains a runtime concept (OS-dependent timing); transport dispatch is fully codegen'd.

### 6.1 Vehicle Profile

```yaml
# deploy.yaml
profile: vehicle
scheduler:
  type: real_time
  cycle_ms: 1

topology:
  brake_ecu:
    platform: qnx
    target: aarch64
    machines:
      brake:
        bindings:
          "#motor":     { transport: someip, service: 0x1001, instance: 0x01 }
          "#dashboard": { transport: can, address: "can0:0x100" }

discovery:
  mode: static

qos:
  defaults:
    qos: reliable
    deadline: 10ms
```

Generated application code:

```cpp
int main() {
    // Scheduler is the only runtime component — OS-dependent timing
    SCE::Mesh::RealTimeScheduler scheduler{.cycle_ms = 1};

    // Transport init is generated — calls vsomeip/SocketCAN APIs directly
    SCE::Generated::brake::init_transports();

    // SM + routing are generated — no runtime abstraction
    SCE::Generated::brake::BrakeSM sm;
    scheduler.run([&](auto events) {
        sm.processEvents(events);
    });
}
```

### 6.2 IntraECU Profile

```yaml
profile: intra_ecu
scheduler:
  type: event_driven

topology:
  this_machine:
    machines:
      brake:
        bindings:
          "#motor": { transport: shm, address: "/sce_motor", size: "4MB" }

discovery:
  mode: static
```

### 6.3 DDS Profile (Full QoS)

```yaml
profile: dds_vehicle
scheduler:
  type: real_time
  cycle_ms: 5

topology:
  brake_ecu:
    machines:
      brake:
        bindings:
          "#motor":
            transport: dds
            topic: "vehicle/powertrain/motor/cmd"
            domain: 0
            qos:
              reliability: RELIABLE
              durability: TRANSIENT_LOCAL
              deadline: 10ms
              liveliness: AUTOMATIC
              lease_duration: 1s
              history: { kind: KEEP_LAST, depth: 5 }
              resource_limits: { max_samples: 100 }
              transport_priority: 7
              # All 22 DDS QoS policies available here

discovery:
  mode: static
```

sce-build generates code that calls `dds_create_qos()` with every specified policy — no feature loss.

### 6.4 Custom Transport

To add a new transport, update three registries (authoritative list lives in `codegen.rs`):

1. **`codegen::transport_shape()`** — declare the field layout: per-target field (`true` for local/shm/someip/dds/can) or device-shared session (`true` for zenoh). Unknown transports cause a build error (`CodegenError::UnsupportedTransport`).

2. **`pattern::transport_capabilities()`** — declare supported communication patterns (Section 8.2). Returns `None` for unknown transports (conservative: validation skipped).

3. **`mesh_transport.h.jinja2` `{% elif %}` blocks** — add transport-specific code at four `NEW TRANSPORT` extension points:
   - (A) Includes
   - (B) Per-target constants
   - (C) Per-target send functions
   - (D1-D5) TransportRouter fields, constructor, init/shutdown, route_send

4. If the transport has device-shared session config (like Zenoh), add a typed struct field to `deploy::TransportConfigs`. `serde` + `deny_unknown_fields` then reject invalid values at parse time.

5. Thread the new session config through `generate_mesh()` in `lib.rs`, pre-escaping for C++ via `cpp_string_literal()` before inserting into the template context.

The `transport_shape_and_capabilities_in_sync` test catches drift between steps 1 and 2. The template's `#error` fallback catches step 3 drift at C++ compile time.

```
// Unknown transport in Rust pipeline:
// "transport 'my_transport' not yet supported (target '#motor')"
//
// Unknown transport in generated C++ code:
// #error "SCE Mesh: unsupported transport 'my_transport' for target '#motor'..."
```

---

## 7. Build Pipeline

### 7.1 Inputs

```
project/
  scxml/
    brake.scxml           # state machine definitions
    motor.scxml
    dashboard.scxml
  deploy.yaml             # topology + transport bindings
  events.yaml             # event payload type definitions (buffer-based transports)
  vehicle.dbc             # CAN signal definitions (signal-based transports, optional)
```

### 7.2 Build Tool Analysis

```
sce-codegen generate scxml/*.scxml --deploy deploy.yaml -o generated/

Step 1: Parse all SCXML documents
  brake.scxml   -> name="brake"
  motor.scxml   -> name="motor"
  dashboard.scxml -> name="dashboard"

Step 2: Build topology map
  brake  --send--> motor       ("motor.cut_power", "motor.resume")
  brake  --send--> dashboard   ("brake.indicator.on/off")
  motor  --send--> dashboard   ("motor.status.*")

Step 3: Apply deployment map (deploy.yaml)
  device_a: [brake]
  device_b: [motor]
  device_c: [dashboard]

Step 4: Boundary analysis
  brake->motor:     cross-device -> generate transport-native send code
  brake->dashboard: cross-device -> generate transport-native send code
  (same-device targets: generate direct call, inlined away)

Step 5: Select codegen template per transport
  brake->motor (someip):    someip_transport.h.jinja2
  brake->dashboard (can):   can_transport.h.jinja2

Step 6: Generate per-device artifacts (SM + transport + serialization)

Step 7: Build-time validation
  - All <send> targets resolve in deploy.yaml
  - All deploy.yaml named entities (service/method/event_group) resolve in external infra config (vsomeip.json / zenoh.json5)
  - Every inferred request↔response pair is complete on both sender and receiver
  - Event coverage: every sent event has at least one receiver
```

### 7.3 Outputs

```
generated/
  device_a/
    brake_sm.h              # AOT state machine (existing codegen)
    brake_transport.h       # transport-native send/subscribe (generated from templates)
    brake_events.h          # event serialization/deserialization
    brake_mesh_main.h       # transport init + scheduler wiring

  device_b/
    motor_sm.h
    motor_transport.h
    motor_events.h
    motor_mesh_main.h

  device_c/
    dashboard_sm.h
    dashboard_transport.h
    dashboard_events.h
    dashboard_mesh_main.h
```

### 7.4 Generated Transport Code

Each target binding generates a dedicated send function that calls the transport API directly:

```cpp
// [generated] brake_transport.h
namespace SCE::Generated::brake {

// Generated from someip_transport.h.jinja2
// deploy.yaml: "#motor" → { transport: someip, service: 0x1001, ... }
void send_to_motor(const EventDescriptor& event) {
    auto msg = vsomeip::runtime::get()->create_request(/*reliable=*/true);
    msg->set_service(0x1001);
    msg->set_instance(0x01);
    msg->set_method(0x01);
    msg->set_payload(serialize_event(event));
    app_->send(msg);
}

// Generated from can_transport.h.jinja2
// deploy.yaml: "#dashboard" → { transport: can, address: "can0:0x100" }
void send_to_dashboard(const EventDescriptor& event) {
    struct can_frame frame;
    frame.can_id = 0x100;
    pack_event_to_can(event, frame.data, &frame.can_dlc);
    write(can_socket_, &frame, sizeof(frame));
}

// Compile-time routing — no map lookup, no vtable
void route_send(const char* target, const EventDescriptor& event) {
    if (eq(target, "#motor"))     return send_to_motor(event);
    if (eq(target, "#dashboard")) return send_to_dashboard(event);
}

}  // namespace SCE::Generated::brake
```

### 7.5 Generated Event Serialization

Event serialization has two distinct layers with different purposes:

1. **Mesh-native serialization (SCE ↔ SCE)**: Automatic, schema-free. The mesh transport carries SCXML event data as-is between SCE-generated state machines. Both sides run SCE-generated code, so the wire format is an internal concern — no user-defined schema required.
2. **External protocol adaptation (SCE → non-SCE)**: Opt-in, schema-required. When a mesh target is a non-SCE system with a fixed binary protocol (CAN ECU, legacy sensor), a Forge codec or signal database provides the wire format translation.

#### Mesh-Native Serialization (Default, Byte-Stream Group)

For SCE-to-SCE communication over byte-stream transports, the transport carries event data inside a `MeshEnvelope` (see Section 13 Phase 3.5). The SCXML runtime assembles `<param>`, `<content>`, and `namelist` data into a JSON string (via `EventDataHelper::buildJsonFromParams`). The generated `TransportRouter::wireTo()` lambda packs this into a `MeshEnvelope` with UUID v7, source machine name, pattern kind, and payload codec tag, then routes through the target transport.

##### Canonical CBOR Wire Format

All byte-stream transports share a single canonical wire format: **CBOR-encoded `MeshEnvelope`** (RFC 8949 §4.2.1 canonical, integer-keyed map). The schema and key map are pinned in Section 13 Phase 3.5.

The shared codec is `SCE::Mesh::encodeEnvelope()` / `SCE::Mesh::decodeEnvelope()` in `sce/include/mesh/MeshEnvelopeCodec.h`. For SHM, encode/decode are internal to `ShmChannel` (symmetric API: `send(MeshEnvelope)` / `drain()`). For SOME/IP and Zenoh, the generated `send_to_X()` functions call `encodeEnvelope()` directly.

Pattern-based dispatch is handled by `SCE::Mesh::dispatchEnvelope<Policy>()` in `sce/include/mesh/MeshDispatch.h` — single source of truth for envelope-to-engine event delivery, used by `ShmChannel::drain()`, `TransportRouter::onIncoming()`, and `route_send()` local branch.

##### SHM Wire Layout (Control Ring + Payload Arena)

Shared memory has a unique constraint: the lock-free MPSC ring buffer (Vyukov algorithm) requires fixed-size slots, but SCXML events are variable-length. SCE Mesh resolves this through a **control-plus-arena** layout — the textbook approach used by high-performance shm IPC systems (iceoryx, DDS implementations):

```
┌─────────────────────────────────────────┐
│ POSIX shm segment                        │
├─────────────────────────────────────────┤
│ Layout header:                          │
│   ready_magic (atomic uint64)            │ ← startup handshake
├─────────────────────────────────────────┤
│ Control ring buffer (Vyukov MPSC)        │
│   fixed-size slots: {offset, length}    │ ← 8 bytes each, lock-free
│   capacity: power of 2                  │
├─────────────────────────────────────────┤
│ Payload arena (circular byte buffer)    │
│   variable-size [name\0data] entries    │ ← producer advances head
│   configurable size (default 64KB)      │   consumer advances tail
└─────────────────────────────────────────┘
```

**Producer path**:
1. Reserve `sizeof(cbor_bytes)` in the arena via CAS on head cursor.
2. Write `encodeEnvelope(env)` output to reserved arena region.
3. Push `{arena_offset, length}` to the control ring.

**Consumer path**:
1. Pop `{offset, length}` from the control ring.
2. Read wire bytes from arena at `offset`.
3. Advance arena tail cursor after the entry is consumed.

**Failure modes**:
- Control ring full → `send()` returns false (current behavior preserved).
- Arena has insufficient contiguous space → `send()` returns false. Producer treats identically to ring-full.
- Reader lags → arena fills up → producer back-pressure. No silent truncation.

**Configuration** (deploy.yaml, per-binding):
```yaml
"#motor":
    transport: shm
    shm_arena_bytes: 65536   # default; override for high-throughput bindings
    shm_ring_capacity: 256   # default; must be power of 2
```

This layout eliminates the two shortcomings of fixed-slot shm:
- No waste on small events (name-only events cost ~name_length bytes in arena, 8 bytes in ring).
- No silent truncation on large events (up to `arena_bytes` is accepted).

##### Receive Path Semantics (W3C-Compliant)

The drain-to-engine path must respect W3C SCXML macrostep semantics. Specifically, the mesh transport layer **does not drive macrostep execution** — the application or scheduler owns `step()` timing:

```
Mesh receive flow:
  transport wire → decodeEnvelope → MeshEnvelope
                                         ↓
  dispatchEnvelope<Policy>(env, engine)  ← switch on PatternKind
                                         ↓
  engine.raiseExternal(event, data)      ← enqueues to external queue only
                                         ↓
  (scheduler or application calls step() at its chosen boundary)
                                         ↓
  processEventQueues() → EventMetadataHelper → _event.data in Lua/JS
```

**Responsibilities**:
- **Transport drain**: calls `raiseExternal` for each received event, then returns. No `step()`.
- **Scheduler** (Section 3.1 `TickScheduling` / `EventDrivenScheduling`): decides when macrosteps run.
- **Application**: owns the event loop if not using a scheduler.

**Why this separation matters** (W3C SCXML 3.12):
External events are processed in the "next stable configuration" — i.e., between macrosteps, not during one. If the transport layer called `step()` per event, it would force micro-level macrosteps, breaking batch processing semantics and contradicting the Scheduler abstraction. It would also make event ordering dependent on transport arrival order rather than scheduler policy.

The transport layer's job ends at the external queue. The engine's step/tick boundary is a scheduler concern.

##### End-to-End Verification

Each buffer-based transport's runtime test must verify:
1. SCXML with `<param>` on the sender → event data reaches `_event.data` on the receiver's Lua/JS engine.
2. Empty `<param>` case — payload-less events work without overhead.
3. Payload size near arena boundary (shm only) — back-pressure, not truncation.

These scenarios close the gap between "wire format defined" and "data reaches application logic."

##### Summary

```
Byte-stream wire format:
  [event_name\0data_bytes]    — single canonical format, all transports

SHM layout:
  control ring (fixed slots) + payload arena (variable bytes)

Receive path:
  transport drain → raiseExternal → scheduler-owned step → _event.data

Compact binary encoding (future):
  MessagePack/CBOR for data portion — transparent to SCXML/deploy.yaml
```

No `events.yaml`, no codec SCXML, no external schema. The JSON payload from the SCXML runtime is the data. This is sufficient for the majority of mesh use cases where both ends are SCE-generated machines.

#### External Protocol Adaptation (Opt-In)

When a mesh target is a non-SCE system that requires a specific binary wire format, two opt-in mechanisms provide protocol adaptation:

**Forge codec** — for custom binary protocols:

When deploy.yaml specifies a `wire_codec` on a binding, the transport template generates code that calls the Forge-generated codec's `encode()`/`decode()` instead of passing JSON:

```yaml
# deploy.yaml — opt-in codec for external system communication
"#legacy_sensor":
    transport: someip
    service_id: "0x1234"
    wire_codec: sensor_frame    # Forge codec SCXML name — external protocol
```

```cpp
// [generated] — codec adapts SCE event data to external binary format
void send_to_legacy_sensor(const MeshEnvelope& env) {
    std::string json_data(env.data.begin(), env.data.end());
    auto frame = SensorFrameCodec::from_json(json_data);  // JSON → typed struct
    auto bytes = frame.encode();                           // struct → wire bytes
    vsomeip_send(sensor_service_, bytes.data(), bytes.size());
}
```

This is explicitly for interfacing with systems that do not run SCE-generated code and require a specific byte layout. Forge codec remains a protocol-native binary serialization tool — it is not a general event serialization mechanism.

**Signal database** — for automotive signal-based protocols (CAN, LIN):

```yaml
# deploy.yaml — DBC signal import for CAN target
"#motor_ecu":
    transport: can
    address: "can0:0x100"
    signals: "vehicle.dbc"     # DBC file provides bit-level signal layout
```

Types, scaling, offsets, and bit layouts are imported from standard automotive database files (`.dbc`, `.arxml`). The transport template generates bit-packing code from the signal definition:

```cpp
// [generated] brake_can_signals.h — layout derived from vehicle.dbc
namespace SCE::Generated::brake::signals {

struct MotorCutPower {
    static constexpr auto NAME = "motor.cut_power";
    static constexpr uint32_t CAN_ID = 0x100;
    static constexpr uint8_t START_BIT = 0;
    static constexpr uint8_t LENGTH = 16;
    static constexpr float SCALE = 0.1f;
    static constexpr float OFFSET = 0.0f;

    void pack(uint8_t frame[8]) const;
    static MotorCutPower unpack(const uint8_t frame[8]);
};

}  // namespace SCE::Generated::brake::signals
```

#### Summary

```
SCE ↔ SCE communication:   mesh-native (JSON, automatic, no schema)
SCE → external system:      wire_codec (Forge codec, opt-in per binding)
SCE → CAN/LIN:              signal database (.dbc/.arxml, opt-in per binding)
```

SCXML documents never reference serialization details. The choice between mesh-native and protocol adaptation is a deploy.yaml concern — consistent with the core principle that SCXML authors write business logic and platform engineers configure deployment.

### 7.6 What Developers Write

```cpp
#include "generated/brake_sm.h"           // existing AOT state machine
#include "generated/brake_transport.h"    // generated transport code (native API calls)
#include "generated/brake_mesh_main.h"    // generated init + scheduler wiring

int main() {
    // Scheduler is the only runtime component — OS timing is not codegen-able
    SCE::Mesh::RealTimeScheduler scheduler{.cycle_ms = 1};

    // Transport initialization is generated — calls vsomeip/SocketCAN/etc. directly
    SCE::Generated::brake::init_transports();

    // SM is generated (existing), routing is generated (new)
    SCE::Generated::brake::BrakeSM sm;
    scheduler.run([&](auto events) {
        sm.processEvents(events);
    });
}
```

No `TransportSet`, no `StaticRegistry`, no `make_runtime()`. The generated code handles transport initialization, routing, serialization, and error propagation. The developer only provides the scheduler and wires the SM.

### 7.7 Build-Time Verification

The build tool performs conservative static analysis across all SCXML documents. Since distributed state machine verification is generally undecidable (data-dependent conditional sends make exact analysis impossible), all checks use **over-approximation** — they may report false positives but will not miss real issues.

| Check | Description | Precision | Phase |
|-------|-------------|-----------|-------|
| Topology completeness | All `<send>` targets resolve in deploy.yaml | Exact | 2 |
| Event coverage | Every sent event has at least one receiver | Exact — name matching only | 2 |
| Interface match | Sender event names match receiver transition triggers | Exact — name matching only | 2 |
| Cross-transport ordering | Warn when order-dependent events route through different transports to the same target | Conservative | 3 |
| Pattern capability | SCXML communication pattern is supported by bound transport (see 8.2) | Exact | 3 |
| Reachability | Every declared state is reachable via some event path | Exact for unconditional, conservative for guarded | 5 |
| Circular dependency detection | Detect potential circular wait in cross-device send/invoke patterns | Conservative — may flag safe cycles | 5 |

---

## 8. Protocol Mapping

How SCXML concepts map to each transport protocol. Each cell represents the native API call that the corresponding transport codegen template generates:

| SCXML Concept | SOME/IP | Zenoh | DDS | gRPC | CAN | Shared Mem |
|--------------|---------|-------|-----|------|-----|------------|
| `<send>` | Request/Fire&Forget | Put(key) | Write(topic) | Unary RPC | Frame TX | Ring buffer write |
| `<send>` + response | Request/Reply | Get(key) | Request/Reply | Unary RPC | N/A | Futex signal |
| `<invoke>` | Service Offer | Declare Publisher | Create Writer | Bidi Stream | N/A | Fork + SHM |
| Event receive | Notification | Subscribe(key) | Read(topic) | Server Stream | Frame RX | Ring buffer read |
| `done.invoke` | Service Down | Undeclare | Dispose | Stream End | N/A | Process exit |
| `error.communication` | TIMEOUT/NACK | Disconnected | Lost Writer | UNAVAILABLE | Bus-off | SIGPIPE |
| Target format | service:instance | key/expression | domain/topic | host:port/svc | bus:id | /dev/shm/name |

### 8.1 Communication Pattern Semantics

Protocol Mapping (above) shows per-transport API calls. Communication Pattern Semantics define the **transport-agnostic event vocabulary** that SCXML authors use. Each transport template maps these patterns to native API calls.

| SCXML Event Pattern | Semantics | Example |
|---|---|---|
| `service.request` | Request/Response — expects reply | Method call, RPC |
| `service.response` | Reply to a prior request | Method return |
| `service.fire_forget` | One-way send, no reply expected | Notification, command |
| `event.subscribe` | Register interest in a topic/event group | Pub/Sub setup |
| `event.notification` | Received event from subscription | Pub/Sub delivery |
| `field.get` / `field.set` | Read/write a named data field | Property access |

These patterns enable **deploy.yaml-only middleware switching**: the same SCXML event patterns map to different native APIs depending on the transport binding. SCXML documents never reference transport-specific concepts.

**Stream patterns (wire values 10-13) are reserved for a future class of communication patterns that pair a subscription with an initial state snapshot delivered as a one-shot event, followed by delta-encoded change events.** Stream patterns are **wire-layer optimizations of the W3C event model**: they materialize at the SCXML receiver as ordinary discrete events consumable via `<transition event="..."><assign>` — the platform's external event queue (W3C SCXML §5.10) delivers the snapshot and each delta as injected events. The same SCXML is valid whether the subscription is realized via FireForget or a future Stream pattern.

SCE does **not** introduce continuous cross-session state sharing, synchronous remote state reads, or shared mutable datamodel. Those primitives would violate W3C §3.12 run-to-completion and the datamodel isolation that the "Same SCXML, Three Domains" claim (§1) rests on. MMO-style "replication" in the sense of Unreal Engine `UPROPERTY(Replicated)` or Unity NetCode — magic shared variables auto-synced across the network — is an **external game-netcode layer** responsibility (e.g., a `sce-game-netcode` adapter repo), not an SCE core feature. The terms collide; the semantics do not.

### 8.2 Transport Capability Matrix

Not all transports support all communication patterns. sce-build validates pattern compatibility at build time:

| Pattern | Req/Reply transports | Pub/Sub transports | Signal transports |
|---|---|---|---|
| `service.request` (req/resp) | Supported | N/A | N/A |
| `service.fire_forget` | Supported | Supported | Supported |
| `event.subscribe` | N/A | Supported | N/A |
| `field.get` / `field.set` | Supported | Via topic read/write | Via signal read/write |
| Reliable delivery | Transport-dependent | Transport-dependent | N/A |

If SCXML uses a pattern that the bound transport does not support, sce-build emits a **build error** with the specific pattern/transport mismatch.

### 8.3 Realization Status (2026-04-13)

The capability matrix above reflects design intent. Current runtime realization is incomplete:

- **`service.fire_forget`**: realized end-to-end across local/shm/someip/zenoh.
- **All other patterns**: build-time validation passes, but the runtime degrades to FireForget shape (no correlation, no reply routing, no subscription lifecycle).

This is tracked as the **Pattern Realization Gap** and is closed by Phase 3.5 (Section 13). Until then, do not rely on `service.request`/`event.subscribe`/`field.get` semantics for production deployments — they are syntactically accepted but semantically incomplete.

---

## 9. Remote Invoke Semantics

`<send>` is fire-and-forget — it maps naturally to remote messaging. `<invoke>` is fundamentally different: it creates a **stateful parent-child session** with lifecycle management. This section defines how `<invoke>` works across device boundaries.

### 9.1 Local vs Remote Invoke

| Aspect | Local `<invoke>` | Remote `<invoke>` |
|--------|-----------------|-------------------|
| Child creation | Same process, new SM instance | Remote process, coordinated via transport |
| Session ID | Runtime-assigned, in-memory | Globally unique, shared across devices |
| `<param>` passing | Direct memory reference | Serialized via generated transport code |
| `<finalize>` data | Direct memory access | Deserialized from transport payload |
| `done.invoke.*` | Internal event queue | Transport-delivered event |
| `<cancel>` | Direct call to child | Transport-delivered cancel request |
| Parent crash | Child auto-terminates (same process) | Child must detect via heartbeat/timeout |

### 9.2 Session ID Management

Remote invoke requires globally unique session IDs to correlate parent-child relationships across devices:

```
Parent (Device A)                    Child (Device B)
  |                                    |
  |-- invoke(session_id: "A:brake:1") -->|
  |   type="scxml"                     |-- creates child SM
  |   src="motor_control.scxml"        |   with session "A:brake:1"
  |                                    |
  |<-- events with session_id ---------| 
  |   (finalize can extract data)      |
  |                                    |
  |<-- done.invoke.A:brake:1 ----------|  (child reaches final state)
```

Session ID format: `<origin_device>:<parent_machine>:<counter>` — deterministic, no UUID overhead in Static mode.

### 9.3 Remote Invoke Lifecycle

```
Parent sends INVOKE_REQUEST via generated transport code
    |
    v
Child device receives INVOKE_REQUEST
    |
    +-- Child SCXML not found on target device
    |   -> error.execution with reason: "INVOKE_SRC_NOT_FOUND"
    |      (delivered to parent via transport, enters parent's external queue)
    |
    +-- Child SCXML found, creates child SM instance
        |
        +-- Child fails during initialization (e.g., datamodel error)
        |   -> error.execution with reason: "INVOKE_CHILD_INIT_FAILED"
        |      + done.invoke.<id> (child never reached a stable state)
        |
        +-- Child SM runs normally, sends events back to parent via generated transport code
            |
            v
        Parent receives child events into external queue, <finalize> processes them
            |
            +-- Child reaches <final>: sends done.invoke.<id> to parent
            |
            +-- Parent exits invoking state: sends INVOKE_CANCEL to child
            |
            +-- Child device crashes: parent detects via transport error
            |   -> error.communication with reason: "INVOKE_CHILD_LOST"
            |
            +-- Transport failure during child operation
                -> error.communication with reason: "INVOKE_TRANSPORT_FAILED"
                   (parent must decide whether to retry or transition to error state)
```

Invoke-specific errors follow W3C SCXML error naming:
- `error.execution` — invoke setup failures (src not found, init failed)
- `error.communication` — transport/runtime failures after child is running

### 9.4 Limitations

Remote `<invoke>` inherits all distributed system constraints:

- `<finalize>` data arrives asynchronously — parent may have transitioned before data arrives. Per W3C SCXML semantics, if the parent has already exited the invoking state when child data arrives, `<finalize>` is not executed and the data is discarded. This behavior applies identically to remote invoke: the runtime drops late-arriving child events for sessions whose parent has exited the invoking state.
- `done.invoke` delivery is not guaranteed unless the deploy.yaml binding for the invoked machine is configured with reliable QoS
- `<cancel>` is best-effort over unreliable transports
- Child cannot access parent's data model (no shared memory across devices)

### 9.5 `<invoke type="sce:mesh-rpc">` — short-lived RPC

`sce:mesh-rpc` is an **implementation-defined invoke type** (W3C §6.4 explicitly reserves `type` for implementation URIs). Unlike `type="scxml"` which spawns a full child session, `mesh-rpc` is a single request/single reply RPC modeled on top of W3C invoke lifecycle events (`done.invoke.ID`, `error.invoke.ID`, `<cancel>`).

**Semantics**:

| Aspect | `type="scxml"` (§9.6) | `type="sce:mesh-rpc"` |
|---|---|---|
| Child session | Full state machine, long-lived | None — stateless RPC handler |
| `_event` stream from child | Multiple events possible | Single reply event only |
| `<finalize>` | Called per child event | Not invoked (no event stream) |
| `done.invoke.ID` | Raised when child reaches `<final>` | Raised when RPC reply arrives |
| `error.invoke.ID` | Raised on child error or transport loss | Raised on timeout or `RpcStatus != Ok` |
| `<cancel>` | Terminates child session | Emits `RpcStatus::Cancelled` envelope |
| Correlation | `invoke_id` throughout session | `invoke_id` single round trip |
| Reserved `<param>` names | None | `_mesh_event` (request event name), `_mesh_deadline_ms` (timeout). See reserved-name rule below. |

**Reserved `<param>` names** (codegen metadata, stripped from the request payload):

| Name | Meaning | Required |
|---|---|---|
| `_mesh_event` | SCXML event name of the request (e.g. `'service.request.compute_force'`) | Yes |
| `_mesh_deadline_ms` | Request timeout in milliseconds (integer). Absent ⇒ no deadline | No |

The `_mesh_` prefix is W3C-identifier-safe and highly unlikely to collide with natural SCXML author payloads. Any additional `_mesh_*` name is reserved for future metadata; `sce-build` rejects **unknown** `_mesh_*` names at build time.

**Reserved-name conflict is a build-time hard error**: an author cannot shadow a `_mesh_*` reserved name with a business payload. `sce-build` fails:
```
error: <param name="_mesh_event"> is reserved metadata for <invoke type="sce:mesh-rpc">.
       Rename your payload parameter (for example, "event_data") or nest it under
       a non-reserved object name.
```

The mesh envelope's `type` (CBOR key 2) is always populated from `_mesh_event`; it is not taken from any author-named `<param>`.

**Target selection — `src` vs `srcexpr`**:

`<invoke type="sce:mesh-rpc">` accepts exactly one of `src` (static `#<machine_name>`, resolved at build time) or `srcexpr` (datamodel expression, evaluated at `<invoke>` entry). Both absent or both present is a build-time hard error. This mirrors W3C §6.4 for `type="scxml"` but applies narrowly to `sce:mesh-rpc`; the two invoke types do not share semantics — a `sce:mesh-rpc` `srcexpr` selects a remote target by name, not a child SCXML session.

| Form | Resolution point | Target must match |
|---|---|---|
| `src="#<name>"` | Build time | A static deploy.yaml binding |
| `srcexpr="<expr>"` | Runtime at `<invoke>` entry | A static deploy.yaml binding whose key the expression resolves to |

The `srcexpr` expression must evaluate to a string of the form `"#<machine_name>"`. Evaluation follows the datamodel's standard expression pipeline; the resulting name is looked up in static topology. If the name does not match any binding at runtime, `error.execution` is raised immediately with `_event.data.reason = "INVOKE_SRC_NOT_FOUND"` (§10.7.1) — no retry or wait. No envelope is emitted, so no wire `RpcStatus` applies. Authors who need wait-for-peer semantics must encode the wait in SCXML (`<transition cond="...">` gating the `<invoke>`).

**Error event class for `sce:mesh-rpc`** — three tiers, keyed on whether an envelope reached the wire:

| Failure class | Event | Where the status surfaces |
|---|---|---|
| Pre-envelope setup failure (binding miss, pool instance out-of-range, srcexpr shape violation, unknown method on target) | `error.execution` | `_event.data.reason` per §10.7.1 (catalogue: `INVOKE_SRC_NOT_FOUND`, etc.) |
| Reply arrived with `rpc_status != Ok` (including synthetic deadline reply) | `error.invoke.<id>` | `rpc_status` on the delivered envelope (CBOR key 10) surfaces via the runtime's §10.7 wiring |
| Transport-layer fault after send (peer partitioned, backpressure drop, delivery exhausted) | `error.communication` | `_event.data.reason` per §16.7 catalogue |

The W3C foreign-processor fallback (see the **Graceful degradation** paragraph below) raises `error.execution` on unknown invoke type; the pre-envelope tier above is the native counterpart, so author handlers of the form `<transition event="error.execution">` work identically whether the processor is SCE or a reference W3C 1.0 impl for this class of fault.

**`srcexpr` does not imply runtime peer discovery.** It only allows the author to pick among already-declared bindings at runtime. For a runtime-varying *instance of a bound service*, use the binding-value-field placeholder mechanism (§14.4) together with a static `src`.

**Deadline precedence** (per-invoke `<param>` vs deploy.yaml binding-level):

| Source | Scope | Precedence |
|---|---|---|
| `<param name="_mesh_deadline_ms">` on `<invoke>` | This specific invoke instance | **Authoritative** — wins on conflict |
| `deploy.yaml bindings.<target>.deadline_ms` | All traffic to this target | Fallback when `<param>` is absent |
| Transport-native deadline (e.g. DDS `deadline:` QoS) | All traffic through this transport | Applied in parallel by the transport stack; **not** combined with the two above |

Rule: the `<param>` value, if present, **overrides** any deploy.yaml binding-level deadline for this one invocation. Transport-native deadlines from QoS are independent — they enforce delivery SLAs, not RPC response timeouts. Diagnostic: when both `<param>` and deploy.yaml deadline exist with different values, `sce-build` emits an informational notice; it is not an error because per-invoke override is expected usage.

**Wire mapping**:
- Request: envelope with `pattern=RpcRequest`, `invoke_id=SCXML invoke id (UUID v7)`, `type=<value of _mesh_event>`, `data=<param payload excluding _mesh_* reserved names>`, `deadline_unix_ms=<now + effective deadline>` (effective deadline = `_mesh_deadline_ms` if present, else deploy.yaml binding-level, else absent).
- Reply: envelope with `pattern=RpcReply`, `invoke_id=<matching>`, `rpc_status=Ok|…`, `data=<reply payload>`.
- Cancel: envelope with `pattern=RpcReply`, `invoke_id=<matching>`, `rpc_status=Cancelled`, empty data.

**Runtime mapping to `_event`** (on reply delivery to parent):
- `_event.name` = `done.invoke.<id>` (success) or `error.invoke.<id>` (non-Ok status)
- `_event.data` = deserialized reply payload (for success) or the structured error object defined in §10.7 (for error)
- `_event.invokeid` = the SCXML invoke id
- `_event.origin` = `mesh://<envelope.source>` (URI form per §10.7)
- `_event.origintype` = `"sce:mesh-rpc"`
- `_event.sendid` = undefined (RPC reply, not a `<send>`)

**Graceful degradation**: a foreign W3C SCXML 1.0 processor that does not understand `sce:mesh-rpc` raises `error.execution` per §6.4.1. The document author may catch this with `<transition event="error.execution">` for local fallback logic. The structured `_event.data` payload for such errors follows the convention in §10.7.

### 9.6 `<invoke type="scxml">` — full remote SCXML session (Session F)

When `type="scxml"` (or the default, which equals `"http://www.w3.org/TR/scxml/"`) is used with `src="#<machine_id>"` referring to a mesh-registered peer, SCE Mesh provides **full W3C SCXML 1.0 invoke semantics across a transport**. This is not an abbreviation — the parent/child session lifecycle and all its derived behaviors (`<finalize>`, `autoforward`, child→parent events, inline content) are preserved end-to-end.

**Implementation status (Session F)**: the full lifecycle is active over same-device shm transport. Wires 14/15/16/17/18/19 carry the W3C §6.4 parent/child session edges (diagram below); wire 20 `InvokeError` remains for instantiation-failure and transport-unavailable paths. Parent-side `<invoke type="scxml" src="#peer">` emits wire-14 `InvokeStart` at state entry, registers a child-session record keyed on the compile-time invoke id, and expects wire-15 `InvokeStarted` (URI stash), zero or more wire-16 `ChildEvent` envelopes (delivered via `_event.origin = child_session_id` per §9.6.3), and terminating wire-18 `InvokeDone` (raises `done.invoke.<id>`) or wire-19 `InvokeCancel` (from onexit). Worker-side `WorkerSessionHost` instantiates the AOT-generated child engine on each wire-14 and ticks it at the worker's cadence, so W3C §6.4.1 ordering (child macrosteps observe as external events on the parent's next macrostep) holds by construction. Absent a deploy.yaml transport binding to the peer's device, the parent raises `error.execution` with `_event.data.reason == "SESSION_F_TRANSPORT_UNAVAILABLE"` (per the structured convention in §10.7.1) without wire traffic. Cross-device transports for scxml-remote invoke now land transport-by-transport: `custom_tcp` arrived via Sessions 1-3 of the cross-device roll-out below (L1395-L1398); `someip` arrived via Sessions 4 + 4b of the same roll-out (L1401-L1402), with single-process wire-14/18 runtime proof and the `<machine>_scxml_invoke_app_` OEM-boundary split documented in Session 4b; `zenoh` arrived via Session 5 (L1404), riding the device-shared `zenoh_session_` because the §13 OEM boundary that motivated SOME/IP's dedicated application has no Zenoh equivalent — SCE-reserved §9.6 traffic is namespaced via the `sce/scxml_invoke/` key-expression prefix instead. The four-transport split (shm + custom_tcp + someip + zenoh) is now complete and the design reserves wire values 14-20 so that each follow-on transport (dds / can / future) adds binding wiring only, not envelope changes.

**Cross-device transport declaration — schema-side (Session 1 of §9.6 cross-device rollout, 2026-04-24)**: the parent's own `machines.<parent>.bindings["#<peer>"].transport` entry is the single source of truth for which transport targets the peer, reusing the field `<send target="#<peer>">` already consumes — no new deploy.yaml key. Classifier `classify_remote_scxml_invokes` records the declared transport on `ScxmlInvokeInfo.remote_mesh_transport` when a peer classifies as remote-mesh (cross-partition); `collect_scxml_remote_peers` publishes the typed `ScxmlRemotePeerBinding { name, transport }` on `SCXMLModel.scxml_remote_{outbound,inbound}_peers`. `validate_scxml_invoke_transport` then layers the device-identity check on top: a peer whose deploy.yaml device differs from the parent's device requires a transport declaration, and the declared transport must be both capable of crossing devices AND wired by the Session F C++ dispatch. Three failure shapes are discriminated on one diagnostic code (`mesh/deploy-scxml-invoke-cross-device-transport`) via `ScxmlInvokeCrossDeviceFailure`: `MissingBinding` (no `bindings["#peer"]` at all), `TransportIncapable` (shm / local — cannot cross a device boundary), and `TransportUnwired` (structurally capable transport like someip / zenoh / dds, but the C++ wire-14/20 dispatch has not yet landed for it — mirrors the §16.5 `partition-wire21-custom-tcp-unimplemented` precedent so a build-time rejection beats a runtime silent fallback). Same-device cross-partition peers keep today's implicit shm path with no declaration required — remains green across the existing ctest matrix (`mesh_session_f_*`). Session 2 of this roll-out extends `mesh_transport.h.jinja2` past the `ShmChannel<>` hardcode with per-transport wire-14/20 branches; the `TransportUnwired` rejection converts to acceptance one transport at a time as the dispatch lands, without further schema or classifier changes. Session 3 adds cross-device E2E fixtures (the donedata-matrix subset from §9.6 L1379 folded into the cross-device test axis). The runtime `SESSION_F_TRANSPORT_UNAVAILABLE` raise (above) narrows to "binding declared and wired but peer not reachable right now" once Session 2 lands — the current "no binding" case is caught earlier at build time by this validator.

**Cross-device wire-14/20 codegen — custom_tcp (Session 2 of §9.6 cross-device rollout, 2026-04-24)**: `mesh_transport.h.jinja2` drops the `using ScxmlInvokeChannel = ShmChannel<>` alias and branches per-peer on `peer.transport`. shm peers keep today's two `ShmChannel<>` members (one per direction) driven by the polling `pumpScxmlInvokeRequests` / `pumpScxmlInvokeReplies` loop. custom_tcp peers emit a single outbound `SCE::Mesh::CustomTcp::Client` (`p2c_to_<peer>_` on parent, `c2p_to_<peer>_` on worker) that dials the peer's `transports.custom_tcp.listen:` endpoint resolved by the classifier into `ScxmlRemotePeerBinding.connect_endpoint`; inbound envelopes ride the device-shared `custom_tcp_server_` whose receive callback grew a pattern switch: wire-14/17/19 stage into a per-peer mutex-guarded queue drained by the worker's pump cadence (keeps `WorkerSessionHost`'s child session map single-threaded), while wire-15/16/18/20 inline into `dispatchToSession` since `dispatchEnvelope` is already documented thread-safe for callback threads (§14.4 convention, mirrors SOME/IP / Zenoh parents). `validate_scxml_invoke_transport` removes custom_tcp from the `TransportUnwired` branch and gains a fourth failure `TransportListenMissing { transport, device }` — scxml-remote is bidirectional so both parent and peer devices need `transports.custom_tcp.listen:` for their side of the inbound stream (silent send-only otherwise). The schema reuses the existing machine-level `transports.custom_tcp.{listen,connect}` from partition wire-21 (§16.5); currently that couples one endpoint per machine (safe today because every SCE deployment is 1-machine-per-device — if multi-machine-per-device ever lands, lift the scope to `devices.X.transports.custom_tcp` without touching the binding-level shape). Session 3 adds cross-device E2E fixtures (the donedata-matrix subset from §9.6 L1379 folded into the cross-device test axis); until then the runtime code paths are exercised only by unit-level shm regressions and the validator's acceptance / rejection tests.

**Cross-device E2E fixtures (Session 3 of §9.6 cross-device rollout, 2026-04-24)**: the Session 2 L1397 promissory "runtime code paths exercised only by unit-level shm regressions" is closed here. Stage A of Session 3 landed the ephemeral-port infrastructure: `CustomTcp::parse_endpoint` accepts the `:0` sentinel so `bind()` delegates port assignment to the kernel; `Server::local_endpoint()` reads the assigned port back via `getsockname`; `TransportRouter::custom_tcp_local_endpoint()` surfaces it at the router level (narrow forwarding getter — avoids callers reaching into the private `custom_tcp_server_` unique_ptr); `CustomTcp::PortOverride::peer_connect_endpoints` + `Client::set_connect_endpoint` plumb runtime endpoints into `TransportRouter::init(PortOverride)` per custom_tcp peer; `run_two_process_fixture.sh` + `tests/cmake/two_process_test.cmake`'s `sce_register_two_process_mesh_test()` orchestrate a worker-first handshake with a `LISTEN_READY` barrier and multi-peer `LISTEN_ENDPOINT_<peer>=` fan-out, exporting each as `MESH_PEER_ENDPOINT[_<peer>]` to the parent. Stage B then wired the two gtest fixtures: `mesh_session_f_crossdev_lifecycle` splits `parent_session_f_wired` + `worker_session_f_wired` across `ecu_parent` + `ecu_worker` so the classifier emits custom_tcp wire-14/20 (SCXMLs reused verbatim from the shm baseline — crossdev variant differs only in deploy.yaml topology + per-peer `transport: custom_tcp` binding); the parent threads the worker's kernel-ephemeral port into `init(PortOverride)` (first TransportRouter-level consumer of the Stage A3 plumbing) and reaching `State::Pass` via `<transition event="done.invoke.*" target="pass"/>` confirms the full wire-14 `InvokeStart` → wire-15 `InvokeStarted` → wire-18 `InvokeDone` round-trip survives the TCP boundary. `mesh_session_f_crossdev_donedata` scales this to three worker shapes (`<param>`, primitive `<content expr>`, nested object/array/integer from the `deploy_session_f_donedata.yaml` SCXMLs) each binding its own kernel-ephemeral Server; the parent seeds a three-entry `PortOverride::peer_connect_endpoints` map in one shot and reaching Pass proves all three decoded `_event.data` payloads satisfied the existing SCXML conds — wire-byte identity to the shm baseline is structurally impossible (CBOR + length-prefix framing diverges by transport), so the equivalence is verified at the decoded-payload level. Parent Server uses a static CMake-configurable port per fixture (`SCE_TEST_CROSSDEV_LIFECYCLE_PORT` / `SCE_TEST_CROSSDEV_DONEDATA_PORT`, each `RESOURCE_LOCK` keyed so parallel ctest runs serialise on the port) because today's `PortOverride` is init-time-only; bilateral ephemeral would require a post-init endpoint-update hook on `TransportRouter`, deferred until a fixture actually needs it — the Session F path's only remaining cross-device gap after this landing.

**SomeIP scxml-invoke foundation — module boundary (Session 4 of §9.6 cross-device rollout, 2026-04-25)**: the per-transport codegen module split called out in §9.6 L1399 (b) below is introduced now, in anticipation of the third wired transport — `sce-build/src/mesh/transport.rs` becomes `transport/mod.rs` with sibling `shm.rs` / `custom_tcp.rs` / `someip.rs` submodules; the only helper with a current consumer (`resolve_connect_endpoint`) migrates into `transport::custom_tcp` and the sibling modules document their boundary shape so `zenoh` (the fourth transport) plugs in without a four-module simultaneous split. What this session deliberately does **not** land and defers to Session 4b (below): (i) the validator-accept flip for `transport: someip` on scxml-remote bindings, (ii) the C++ helper header `mesh/transports/SomeipScxmlInvokeEndpoint.h`, (iii) the `mesh_transport.h.jinja2` someip scxml-remote emission branch, and (iv) the single-process `mesh_someip_scxml_invoke_roundtrip` fixture. The four parts are coupled: validator-accept without codegen would silently route through the `else`-branch shm emission at peer member declaration and produce a runtime fault rather than a build-time diagnostic; landing the helper header without a template consumer violates the "verify before ship" rule because the helper's API shape is only pinned once a working fixture exercises it. Session 4b (L1402) lands (i)+(ii)+(iii)+(iv) as a tightly-sequenced set of commits on the same branch so the contract holds end-to-end at the merge boundary.

**SomeIP scxml-invoke runtime — wire-14/18 landing (Session 4b of §9.6 cross-device rollout, 2026-04-25)**: Session 4's four deferred parts (i)-(iv) all landed here. The C++ helper `sce/include/mesh/transports/SomeipScxmlInvokeEndpoint.h` (B1) exposes a per-peer endpoint that offers the local machine's service, registers per-wire `register_message_handler`s for wire-14..20 methods 0x0014..0x0020 (BCD-style identity mapping, drift-guarded by compile-time `static_assert`s on `methodForPattern(PatternKind) ↔ SCXML_INVOKE_METHOD_WIRE*`), and dispatches `MT_REQUEST_NO_RETURN` fireAndForget requests via `methodForPattern(env.pattern)`. Service IDs derive from machine names via the compile-time `serviceIdForMachine(std::string_view)` constexpr — FNV-1a 32-bit hash low byte ORed with the SCE-reserved base `SCXML_INVOKE_SERVICE_BASE = 0x8100`, yielding 256 distinct IDs in `[0x8100, 0x81FF]`. A Rust mirror (`sce-build/src/mesh/transport/someip.rs`, B2) pins the same constants + hash through unit tests so a future drift in either side trips `cargo test -p sce-build --lib` before the C++ side rebuilds. The codegen template emits one `ScxmlInvokeEndpoint` member per §9.6 someip peer plus a per-machine `<machine>_scxml_invoke_app_` (`std::shared_ptr<vsomeip::application>`) that every endpoint on that machine shares; `init()` calls `app->init()`, installs per-peer receive handlers (parent-side inline `dispatchToSession` for wire-15/16/18/20 mirroring the custom_tcp parent pattern, worker-side mutex-guarded staging for wire-14/17/19 mirroring the custom_tcp worker pattern) and spawns `<machine>_scxml_invoke_thread_` running `app->start()`. Validator `validate_scxml_invoke_transport` accepts `someip` as a fourth arm before the catch-all `TransportUnwired` (B4) — `vsomeip.json` application/service-ID collision validation remains §13's responsibility and is not duplicated. The single-process fixture `mesh_someip_scxml_invoke_roundtrip` (B5) proves the full wire-14 → wire-15 → wire-18 round-trip over vsomeip internal routing with SD disabled and parent nominated as routing manager. **Dedicated `<machine>_scxml_invoke_app_` is textbook, not incrementalism**: three forces drive the split from any per-`<send>`-target vsomeip application. (1) §13 OEM boundary — `vsomeip.json applications[*]` is OEM-owned territory; SCE does not register SCE-reserved services `0x8100..0x81FF` inside an OEM-declared application, so dedicating an SCE-named application keeps the registration boundary observable at the routing layer via the `(application, service)` tuple. (2) Failure isolation — a §9.6 peer disconnect or handler exception stays contained inside the dedicated application's callback thread and cannot block the `<send>` SOME/IP path that may carry safety-relevant traffic (e.g. brake control); vsomeip's routing_manager dispatches per `(application, service)` tuple, so the two applications are also isolated at the routing layer. (3) Service-ID responsibility split — SCE-reserved range collision detection is SCE codegen's responsibility; OEM service-ID collision detection is OEM `vsomeip.json`'s responsibility; dedicated applications make the boundary observable at the routing layer. A future "one app per machine, multiplex SCE + OEM services" alternative would violate (1) by design and is rejected. The 256-ID collision boundary (birthday paradox crosses 50% near 16 machines) is a known MVP limit — a §9.6 4c+ deploy-time validator will reject deployments whose §9.6 peer set collides; no fixture exercises that corner in Session 4b because every current fixture has ≤2 machines and the pinned hashes are collision-free. Two-process cross-device someip is queued alongside the zenoh wire landing.

**Zenoh scxml-invoke runtime — wire-14/18 landing (Session 5 of §9.6 cross-device rollout, 2026-04-25)**: the four-transport split closes here. The C++ helper `sce/include/mesh/transports/ZenohScxmlInvokeEndpoint.h` exposes a per-peer endpoint that takes a non-owning reference to the device-wide `zenoh_session_`, declares one `zenoh::Publisher` on the local role's key-expression and one `zenoh::Subscriber` on the peer role's key, and dispatches via `Publisher::put` with reliability options pinned to `Z_CONGESTION_CONTROL_BLOCK + Z_PRIORITY_DATA` — §9.6 wire-14/18 cannot recover from a silent drop because there is no resend protocol on the parent/child staging queues, and explicit options keep a future C-ABI default flip from silently downgrading reliability. Per-direction key-expressions assemble at construction via `keyExprP2C(parent, child) → "sce/scxml_invoke/p2c/<parent>/<child>"` and `keyExprC2P(child, parent) → "sce/scxml_invoke/c2p/<child>/<parent>"`; pattern discrimination remains on `MeshEnvelope::pattern` in the receive callback (parent inline `dispatchToSession`, worker mutex-guarded staging — same shape as the someip §9.6 / custom_tcp §9.6 inbound paths). A Rust mirror (`sce-build/src/mesh/transport/zenoh.rs`) pins the prefix and key-derivation through unit tests so a future drift in either side trips `cargo test -p sce-build --lib` before the C++ side rebuilds. The codegen template emits one `std::optional<ScxmlInvokeEndpoint>` member per §9.6 zenoh peer (optional because `zenoh_session_` is created in `init()`, not the ctor — the endpoint stores a reference); `init()` emplaces each endpoint inside the same `try { … } catch (zenoh::ZException&)` block that opens the session, installs receive handlers, and `start()` declares the Publisher + Subscriber, so any declare failure surfaces as `init()` returning false the same way `<send>`-target zenoh publishers already do (template L2456+ §10.10 outbound buffer pattern). Validator `validate_scxml_invoke_transport` accepts `zenoh` as a fifth arm before the catch-all `TransportUnwired`; the previously-zenoh-rejection regression test re-points to `dds` as the next structurally-capable-but-unwired transport. The single-process fixture `mesh_zenoh_scxml_invoke_roundtrip` (single-ECU two-machines deployment, peer mesh anchored on a relay session that listens at the address both routers dial via `transports.zenoh.connect:`) proves the full wire-14 → wire-15 → wire-18 round-trip; convergence uses the existing `wait_for_peer_ready` liveliness pattern from `mesh_zenoh_runtime` plus a 200 ms subscriber-propagation settle matching `test_mesh_zenoh_eventgroup_engine_driven`'s precedent. **Shared `zenoh_session_` is textbook, not incrementalism**: three forces motivate it. (1) No §13 OEM boundary on the Zenoh side — `vsomeip.json applications[*]` is OEM-owned territory; Zenoh has no equivalent OEM-allocated identifier whose registration must stay outside SCE-named spaces, so the SCE-reserved §9.6 namespace is carved out via key-expression prefix (`sce/scxml_invoke/`) rather than session identity. (2) No 256-ID hashing or service_id collision domain — SOME/IP's `serviceIdForMachine` FNV-low-byte mapping required a dedicated application to keep its 256-machine MVP boundary observable on its own routing tuple; Zenoh routes by full key-expression with no analogous bounded namespace, so collision validation is moot at this layer. (3) Failure isolation — a §9.6 peer disconnect surfaces on the same Zenoh runtime callback thread as `<send>` traffic; sharing the session matches the existing `zenoh_subscribers_` map that already dispatches on the same callback thread without per-pattern isolation. Two-process cross-device zenoh (paired with the corresponding someip 2-process landing) is queued as a Session 5b candidate, mirroring the custom_tcp Session 3 Stage B pattern; the existing `mesh_session_f_crossdev_*` orchestrator handles the boundary case once a fixture demands it.

**Two-host scxml-invoke fixtures — SOME/IP + Zenoh (Session 5b of §9.6 cross-device rollout, 2026-04-25)**: the queued Session 5b candidate from L1405 lands here. Two new fixtures (`mesh_someip_scxml_invoke_crossdev`, `mesh_zenoh_scxml_invoke_crossdev`) drive the full wire-14 → wire-15 → wire-18 round-trip across distinct Linux network namespaces wired by a veth pair, so the §9.6 lifecycle traverses a real network stack — multicast SD for SOME/IP, peer-mesh TCP for Zenoh — that loopback alone cannot reproduce. The boundary infrastructure (`tests/mesh/setup_crossdev_netns.sh` + `cleanup_crossdev_netns.sh` + `run_two_host_fixture.sh` + `tests/cmake/two_host_test.cmake`'s `sce_register_two_host_mesh_test()`) mirrors the tc8-harness `mock_dut/env/setup-netns.sh` precedent — parent runs in `sce-mesh-parent` netns at 172.16.10.1, worker runs in `sce-mesh-worker` at 172.16.10.2, `224.0.0.0/4` multicast route lets vsomeip's SD reach the peer over veth (without it the routing manager would flip into a "no peers" state on first sendto). The orchestrator is a sister to `run_two_process_fixture.sh`: same worker-first stderr `LISTEN_READY` barrier, plus a 500 ms `SCE_TWO_HOST_SETTLE_MS` window before launching the parent so SD multicast / Zenoh subscriber declarations have time to converge before wire-14 fires. **Both transports diverge from their 1-process variants on three axes**: (1) **SD enable** — Session 4b's `vsomeip_scxml_invoke.json` keeps `service-discovery.enable: false` because both apps share one in-process routing manager; Session 5b's `vsomeip_scxml_invoke_crossdev_{parent,worker}.json` flip it to `true` and pin `multicast: 224.244.224.245 / port: 30490 / protocol: udp` so the two routing managers find each other through SD multicast over the wire — the textbook two-ECU pattern. (2) **Routing manager identity** — each side runs its own RM in its own netns with distinct `network` field (`sce_scxml_invoke_someip_crossdev_{parent,worker}`) so the per-network UNIX socket paths under /tmp do not collide on the shared filesystem; vsomeip applications and routing IDs follow the same prefix split. (3) **Topology** — Zenoh's `deploy_scxml_invoke_zenoh_crossdev.yaml` puts `transports.zenoh.listen: tcp/172.16.10.1:17449` on the parent ECU and `transports.zenoh.connect: tcp/172.16.10.1:17449` on the worker ECU, so the parent's own listen socket is the rendezvous (no third relay process); Zenoh's connect-retry on the worker side handles the worker-comes-up-first race that the orchestrator's launch order forces. **Per-binary VSOMEIP_CONFIGURATION** is baked into the SOME/IP test binaries via CMake `target_compile_definitions(... VSOMEIP_CONFIG_PATH=...)` and `setenv()` in `main()` before `TransportRouter::init()`, because ctest's `ENVIRONMENT` test property cannot vary by side and the two halves need distinct vsomeip.json files; this preserves single-source-of-truth ownership inside each binary. **Both parent test drivers sleep 500 ms after `init()` before `initialize()`** — the orchestrator's settle covers worker→parent direction (worker has been advertising for 500 ms before parent launch) but the parent→worker SD/peer-mesh discovery still needs a window after parent's RM comes up before wire-14 has a routing target; the same 500 ms post-init pattern the 1-process Session 4b roundtrip uses for both directions. Fixtures are gated on the new CMake option `SCE_ENABLE_NETNS_TESTS=ON` (default OFF) and registered with `SKIP_RETURN_CODE 77` so a non-root `cmake .. && ctest` in a fresh checkout reports them as Skipped, not Failed — the developer / CI runs `sudo tests/mesh/setup_crossdev_netns.sh` once before turning the option on. The custom_tcp fixture (Session 3 Stage B at L1399) intentionally stays on loopback because TCP works over `127.0.0.1` without multicast or distinct routing tables; the two cross-device tiers (loopback custom_tcp / netns someip + zenoh) reflect what each transport's discovery mechanism requires, not arbitrary fragmentation.

**Abstraction triggers (§9.6 cross-device rollout, governance notes).** Three candidate abstractions were considered and explicitly deferred or rejected so follow-on transport landings do not re-litigate the decisions: (a) **Runtime channel concept / adapter** — rejected. Wrapping shm + custom_tcp + someip + zenoh in a common `ScxmlInvokeChannelLike` concept would force a single receive model (polling or callback) on all transports, but the receive models are inherently divergent (shm = polling, tcp/someip/zenoh = callback) and the unification would discard each transport's native latency characteristic. C++ duck typing on `.send(env) -> bool` is enough at send sites; receive paths fork per-transport by design. (b) **Per-transport codegen module split** — **landed in Session 4** (see L1401); Session 4b (L1402) populated `transport::someip` with the wire-14/18 runtime mirror of `SomeipScxmlInvokeEndpoint.h`; Session 5 (L1404) populated `transport::zenoh` with the key-expression mirror of `ZenohScxmlInvokeEndpoint.h`. `sce_build/src/mesh/transport/{shm,custom_tcp,someip,zenoh}.rs` modules now exist and `resolve_connect_endpoint` lives in `transport::custom_tcp` — the four-transport split is complete, and any further transport (dds / can / future) plugs in as a fifth sibling without disturbing the four already-landed shapes. (c) **Unified `TransportConfig` schema** — deferred. custom_tcp's `listen: "host:port"` fits today's `Option<String>` shape; someip's `service_id / instance_id / method_id` triple and zenoh's `KeyExpr` don't. Introduce a common config trait when the someip codegen landing surfaces the structural diff, not before.

#### 9.6.1 Session establishment

```
Parent P on device D_P                  Child template resolved on device D_C
  |                                       |
  |-- InvokeStart { invoke_id,            |
  |                 type="scxml",         |
  |                 src="#motor",         |
  |                 params, content,      |
  |                 namelist_snapshot } ->|
  |                                       |-- instantiate new child session C
  |                                       |   init datamodel from params + content + namelist
  |                                       |   child session id = "<D_P>:<P.machine>:<invoke_id>"
  |                                       |
  |<-- InvokeStarted { invoke_id,         |   (success) parent stashes child endpoint
  |                    session_id } ------|
  |                                       |
  |<-- ChildEvent { invoke_id,            |   (repeated — every child <send target="#_parent">
  |                 event_name,           |    AND every event raised inside child)
  |                 data, sendid,         |
  |                 origintype } ---------|
  |                                       |
  | (parent: parent executes <finalize>   |
  |  on the transition triggered by       |
  |  the child event, if the transition   |
  |  is in the invoking state and a       |
  |  <finalize> is declared on the        |
  |  <invoke>)                            |
  |                                       |
  |-- ParentEvent { invoke_id,            |-- child external queue receives event
  |                 event_name, data,     |   (fired by autoforward="true" OR explicit
  |                 sendid } ------------>|    <send target="#_invoke_<id>">)
  |                                       |
  |<-- Done { invoke_id,                  |   child reached <final>
  |           donedata } -----------------|
  |                                       |
  |-- Cancel { invoke_id } -------------->| (parent exits invoking state or explicit <cancel>)
  |                                       |-- child terminated, child final events discarded
```

#### 9.6.2 Envelope extensions for full remote invoke

Additional wire-stable pattern values (reserved in §13 CBOR key map):

| Pattern | Wire value | Direction | Envelope content |
|---|---|---|---|
| `InvokeStart` | 14 | Parent → Child | `invoke_id` (correlation UUID v7), `subject`=compile-time invoke id, `data`=CBOR map `{src, params, content, namelist, autoforward}` |
| `InvokeStarted` | 15 | Child → Parent | `invoke_id`, `subject`=compile-time invoke id, `child_session_id` (CBOR key 18, child session URI per §9.6.1 L1410) |
| `ChildEvent` | 16 | Child → Parent | `invoke_id`, `subject`=sendid, `child_session_id`, `type`=event name, `data`=payload, codec tag |
| `ParentEvent` | 17 | Parent → Child | `invoke_id`, `subject`=compile-time invoke id, `type`=event name, `data`=payload |
| `InvokeDone` | 18 | Child → Parent | `invoke_id`, `subject`=compile-time invoke id, `child_session_id`, `data`=donedata payload |
| `InvokeCancel` | 19 | Parent → Child | `invoke_id` omitted on cancel; `subject`=compile-time invoke id, empty `data` |
| `InvokeError` | 20 | Child → Parent or Parent → Child | `invoke_id`, `subject`=compile-time invoke id, `rpc_status`, `rpc_error_message` |

Wire values 10-13 remain reserved for Stream patterns (§13 Phase 4); 14-20 are assigned to the full remote invoke lifecycle (Session F) and all seven are active as of this landing; 21 is `ParallelRegionDone` (Session E2). Future additions must use unused integers (22+). **No wire value may ever be reused or overloaded — see §13 "Adding a variant requires a new wire value, never reuse."**

**Codegen-shape exclusivity.** A machine registered in `deploy.yaml` as a remote invoke peer (i.e. at least one sibling machine's SCXML contains `<invoke type="scxml" src="#<this>">`) MUST NOT simultaneously appear as a local-path `<invoke type="scxml" src="<this>.scxml">` target elsewhere in the same deployment. The mesh peer shape is default-constructible so `ChildSessionAdapter<Engine>` can own the child engine, while the local-path shape carries a `ParentStateMachine` template parameter and a `parent_` pointer threaded through the ctor for direct enum-based parent notification; the two shapes are structurally incompatible on a single generated SM class. Violations are rejected at build time with `mesh/deploy-scxml-invoke-target-conflict` (`DeployError::ScxmlInvokeTargetConflict`). Fix by flipping the local-path invoker to the `#<peer>` mesh shape, or by removing the machine from the deploy topology so it ceases to be a mesh peer.

#### 9.6.3 `_event` field wiring (W3C §5.10.2 compliance)

When a child event arrives at the parent as `ChildEvent`:

| Standard `_event` field | Wiring source |
|---|---|
| `_event.name` | envelope `type` |
| `_event.type` | `"external"` (per §5.10.1, all invoke-sourced events are external) |
| `_event.sendid` | envelope `subject` (child's sendid, transparent) |
| `_event.origin` | child session endpoint URI: `mesh://<child_device>/<child_machine>/<child_session_id>` |
| `_event.origintype` | `"http://www.w3.org/TR/scxml/#SCXMLEventProcessor"` (standard; remote transport transparent per §6.2) |
| `_event.invokeid` | envelope `invoke_id` (UUID v7 as hex string) |
| `_event.data` | deserialized payload according to envelope `datacontenttype` |

The reverse applies to `ParentEvent` arriving at the child: `_event.origin` points at the parent endpoint, `_event.invokeid` is the same UUID v7, and `_event.type = "external"`.

#### 9.6.4 `<finalize>` semantics preserved

Per W3C §6.4.4, `<finalize>` executes **in the context of the invoking state**, **before** the transition triggered by the child event fires, and modifies the parent's datamodel. Remote invoke preserves this ordering:

1. `ChildEvent` arrives at parent's transport layer.
2. Mesh runtime enqueues the event into the parent's external event queue at macrostep boundary.
3. During the parent's next macrostep, the event is selected.
4. The runtime identifies the `<invoke id="X">` matching `_event.invokeid`.
5. If the parent is still in the invoking state and `<invoke>` has a `<finalize>` child, the runtime executes `<finalize>` **before** evaluating transition selection for this event. The `<finalize>` body may read `_event.data` and write to the parent datamodel.
6. Transition selection proceeds normally; any transition whose `event` matches may fire.

If the parent has already exited the invoking state when the event arrives (step 4 fails), the event is **discarded silently** per W3C §6.4.5 (finalize not executed, transition not considered). Late events for terminated sessions do not re-activate the invoking state.

#### 9.6.5 `autoforward="true"` semantics

Per W3C §6.4.2, `autoforward="true"` causes the parent to forward every external event in its queue to the invoked child. In remote invoke:

- Parent runtime observes each external event selected during macrostep.
- For each active `<invoke autoforward="true">`, runtime emits `ParentEvent` envelope to the child with `invoke_id` matching the invoke and the event copied verbatim.
- `ParentEvent` delivery preserves per-sender FIFO ordering (Transport Contract §10.1).
- Foreword is **at-most-once-per-parent-event**; if the parent's own transition consumes the event before autoforward emits, forward is still emitted (per §6.4.2 — autoforward is orthogonal to parent's own handling).

Runtime cost: each autoforwarded event adds one envelope emission. Transport Contract reliability/ordering apply. `autoforward` with an unreliable transport is explicitly permitted but the author should expect occasional forward loss (same contract as any mesh `<send>`).

#### 9.6.6 Inline `<content>` and child SCXML precompilation

Per W3C §6.4, `<invoke>` may carry the child machine's SCXML inline via `<content>`:

```xml
<invoke type="scxml" id="worker">
  <content>
    <scxml>
      <state id="s">
        ...
      </state>
    </scxml>
  </content>
</invoke>
```

**Problem**: remote invoke requires the target device to have executable code for the child. Shipping SCXML text at runtime requires a remote interpreter and violates "build-time resolution" (§1 Core Principle).

**Resolution**: at build time, `sce-build` scans every `<invoke type="scxml">` with inline `<content>` and **extracts the inner `<scxml>` document as a separate logical machine**. The extracted machine is treated exactly like a named peer:

1. Build tool synthesizes a stable machine name: `<parent_machine_id>__sce_synth_invoke__<invoke_id>`. The `__sce_synth_invoke__` infix is reserved: `sce-build` **rejects any author-declared machine id containing this infix** with a build error, so name collisions with synthesized machines are structurally impossible. If a collision is nonetheless detected (e.g., two invokes sharing the same id under the same parent — which is already a W3C violation), `sce-build` fails with:
   ```
   error: synthesized machine name '<parent>__sce_synth_invoke__<invoke_id>' collides with
          another machine. Causes: duplicate <invoke id> under the same parent, or author
          machine id using the reserved '__sce_synth_invoke__' infix.
   ```
2. The inline content is removed from the parent source; the `<invoke>` is rewritten in-memory to `src="#<parent>__sce_synth_invoke__<invoke_id>"` (content field dropped).
3. The synthesized machine is placed into the same partition as the parent by default. `deploy.yaml` `partitions:` may explicitly reassign it to a different partition (enabling distributed inline invoke).
4. AOT codegen produces a state machine class for the synthesized machine identical to named peers.

Peer enumeration for the synthesized machine follows the reserved infix. The parser rewrites inline `<content>` to `src="#<synth>"` **only in the in-memory model** — the parent's on-disk SCXML still carries inline `<content>` with no `src=` attribute, so the sibling-file regex that resolves `scxml_remote_inbound_peers` for named peers cannot observe the rewrite when the synthesized machine is the codegen target. `sce-build` closes this by inverting the `__sce_synth_invoke__` infix: when `resolved_name` matches the infix and the extracted parent id is a declared topology machine with a distinct partition per rule 3, the parent is added to the synthesized machine's inbound peer list. Same-partition synthesized machines stay off the mesh inbound set (per rule 3 default) and the local child-session shape is preserved.

At runtime, the rewritten `<invoke src="#<parent>__sce_synth_invoke__<invoke_id>">` resolves through the normal remote invoke path. The author's intent ("inline content") is preserved — the child is semantically the inline SCXML — but the execution path is the same as named-peer invoke.

Deploy.yaml override example:
```yaml
partitions:
  worker_off_device:
    machines: [brake]
    contains:
      invokes: ["brake__sce_synth_invoke__worker"]   # synthesized from inline <content>
```

Cross-partition execution of a rule-3 override is pinned end-to-end by
`mesh_synth_invoke_override_e2e` (see §16.9 F exit criterion #4) — the
parent's inline `<content>` is materialised as a distinct synth
machine, deploy.yaml places the synth on a different partition, and
the §9.6.2 wire-14/15/18 round-trip drives `done.invoke.<id>` on the
parent identically to the author-declared peer case.

#### 9.6.7 Foreign processor compatibility

A W3C SCXML 1.0 processor that does not understand SCE Mesh's distribution layer will still execute `<invoke type="scxml">` — it will create a **local** child session in the same process. For SCXML documents intended for dual deployment (local-only test + distributed production), this is graceful: the document runs as a self-contained monolith under a foreign processor, and distributes only when loaded via `sce-build` with a matching `deploy.yaml`. No source modification needed.

---

## 10. Event Ordering and Concurrency

### 10.1 Ordering Guarantees

W3C SCXML guarantees within a single instance: internal events are processed before external events. SCE Mesh extends this to distributed events:

| Guarantee | Scope | Provided by |
|-----------|-------|-------------|
| Internal before external | Single instance | W3C SCXML RTC model (unchanged) |
| FIFO per source | Same source, same transport | Transport implementation |
| FIFO per source, cross-transport | Same source, different transports | **Not guaranteed** |
| Total ordering across sources | Multiple sources | **Not guaranteed** |

**Per-source FIFO** means: if Device A sends event1 then event2 to Device B over the same transport, Device B receives event1 before event2. This is a natural property of TCP-based transports (SOME/IP, gRPC) and can be enforced on UDP via sequence numbers.

**Cross-transport ordering is not guaranteed**: if Device A sends event1 via SOME/IP and event2 via CAN to Device B, arrival order depends on transport latency. SCXML authors must not depend on cross-transport ordering.

If causal ordering is required across transports, it must be implemented at the application level (e.g., explicit acknowledgment events in the SCXML design).

### 10.2 Backpressure and Flow Control

When a state machine cannot process events fast enough:

| Scheduler Mode | Queue Full Behavior | Rationale |
|----------------|-------------------|-----------|
| TICK_BASED (Game) | Drop oldest events, keep newest | Games need current state, stale events are harmful |
| TICK_BASED (RT) | `error.communication` to sender | Safety-critical: sender must know receiver is overloaded |
| EVENT_DRIVEN | Configurable: drop / block / error | Depends on deployment requirements |

Queue overflow policies are configured per-scheduler:

```cpp
RealTimeScheduler{
    .cycle_ms = 1,
    .overflow_policy = SCE::Mesh::OverflowPolicy::ERROR,  // signal to sender
    .max_queue_depth = 64
};

GameLoopScheduler{
    .tick_rate = 60,
    .overflow_policy = SCE::Mesh::OverflowPolicy::DROP_OLDEST,
    .max_queue_depth = 1024
};
```

### 10.3 Thread Safety Model

A single state machine instance is **single-threaded at the processing level**. The runtime enforces this:

```
Multiple transports may deliver events concurrently
    |
    v
EventRouter serializes all incoming events into instance's event queue
(lock-free MPSC queue: Multiple Producer, Single Consumer)
    |
    v
Scheduler dispatches from queue to instance — one event at a time
(W3C RTC guarantee: no concurrent processing within one instance)
```

For batch schedulers (Game, RT), the runtime distributes **different instances** across worker threads. Each instance is owned by exactly one worker per tick — no shared mutable state between instances.

```
Tick N:
  Worker 0: [instance 0, instance 4, instance 8, ...]
  Worker 1: [instance 1, instance 5, instance 9, ...]
  Worker 2: [instance 2, instance 6, instance 10, ...]
  Worker 3: [instance 3, instance 7, instance 11, ...]

  No two workers touch the same instance in the same tick.
```

If Instance A generates an event for Instance B during tick N, the event is always delivered at tick **N+1**, regardless of whether Instance B has already been processed in tick N. This rule is absolute:

- **Deterministic**: result does not depend on instance processing order within a tick
- **Simple to implement**: double-buffered event queues — tick N reads from buffer A, writes to buffer B; tick N+1 swaps
- **W3C consistent**: W3C SCXML requires external events to be processed in the "next stable configuration" after the current macrostep. In tick-based scheduling, the tick boundary is the macrostep boundary — delivering at tick N+1 satisfies this requirement

### 10.4 Transport Contract

A transport is **conformant** for distributed W3C SCXML 1.0 execution iff it provides:

| Property | Rationale (W3C clause) | Canonical provider |
|---|---|---|
| **Per-sender FIFO ordering** | §3.13 external event queue is FIFO; each sender is a single queue from the receiver's perspective | TCP stream, SOME/IP-TCP, DDS reliable, Zenoh reliable, gRPC |
| **At-least-once delivery** | §5.10 no loss permitted — W3C assumes successful receipt or an `error.communication` event | TCP, DDS reliable, application ACK on unreliable transport |
| **Duplicate tolerance** | §5.10 does not forbid duplicates but the runtime must suppress them (dedup at mesh layer; §10.5) | envelope id (UUID v7) |
| **Fault signal emission** | §3.2 `error.communication` is raised on transport failure; transport must surface disconnection | all transports (native or wrapper) |

A transport meeting all four is **conformance-complete**. Missing any property makes the transport **conformance-degraded**, and deploy.yaml must flag it:

```yaml
topology:
  game_server:
    transports:
      udp_fast:
        kind: udp
        conformance: degraded      # opt-in explicit declaration
        degraded_aspects: [ordering, delivery]
```

`sce-build` fails the build if a binding uses a degraded transport without the explicit `conformance: degraded` declaration on that transport. This prevents accidental conformance loss.

**Reference implementations in-tree**:
- `local`, `shm`, `someip` (TCP mode), `zenoh` (reliable mode): conformance-complete.
- `custom_tcp` (Session E2 CI reference): conformance-complete, zero external deps.
- `udp` (planned): conformance-degraded by default; opt-in sequence-id wrapper can lift to complete.

#### 10.4.1 Transport Lifecycle Invariants

Every transport implementation must honour the following lifecycle phases. The generated `TransportRouter` orchestrates these phases — transport authors provide the per-phase implementations.

```
 ┌──────────┐    init()    ┌───────────┐   connect()  ┌──────────┐
 │ Created  │ ──────────▶  │ Ready     │ ──────────▶  │ Active   │
 └──────────┘              └───────────┘              └──────────┘
                                │                        │    ▲
                                │ shutdown()             │    │ reconnect()
                                ▼                        │    │
                           ┌───────────┐   disconnect() │    │
                           │ Shutdown  │ ◀──────────────┘    │
                           └───────────┘                     │
                                                     ┌──────────────┐
                                                     │ Disconnected │
                                                     └──────────────┘
```

| Phase | Invariant |
|---|---|
| **Created → Ready** (`init()`) | Transport allocates resources (session handle, file descriptor, SHM segment). No I/O occurs. Errors are configuration errors, not network errors. |
| **Ready → Active** (`connect()`) | Transport establishes network presence (TCP connect, SOME/IP offer, Zenoh open). Blocking is permitted for session-shared transports; per-target transports may defer connection to first `send()`. |
| **Active → Active** (`send(envelope)` / `receive()`) | The four conformance properties (§10.4) apply. `send` returns success or an error that triggers `error.communication`. `receive` delivers envelopes to the engine's external queue via callback. |
| **Active → Disconnected** (transport fault) | Transport detects loss (TCP RST, SOME/IP availability change, Zenoh disconnect). Must emit `error.communication` with `reason: TRANSPORT_UNAVAILABLE` (§16.7). Enqueued-but-unsent envelopes are failed individually with `reason: SEND_FAILED`. |
| **Disconnected → Active** (`reconnect()`) | Transparent to SCXML author. Pending RPC correlation entries survive reconnection; the deadline timer (§10.8) is unaffected by transport-layer reconnect. A successful reconnect does NOT raise an event. |
| **Active/Disconnected → Shutdown** (`shutdown()`) | Transport releases resources. Outstanding RPC entries are cancelled with `reason: INVOKE_CHILD_LOST`. The mesh runtime calls `shutdown()` exactly once per engine lifetime; double-shutdown is a programming error. |

**Best-effort transports** (`conformance: degraded`) relax the `send → error.communication` guarantee: `send` may return success for a payload that is silently dropped. The `Disconnected` state may never be entered if the transport has no connection concept (UDP). These relaxations must be declared in `TransportDescriptor::degraded_aspects`.

#### 10.4.2 Transport Descriptor Interface

`sce-build` reads transport metadata from the single registry (`mesh/transport.rs`). Each transport entry declares:

| Field | Type | Purpose |
|---|---|---|
| `shape` | `TransportShape` | Codegen layout: per-target field vs device-shared session. |
| `capabilities` | `[TransportCapability]` | Supported communication patterns. Build-time validation rejects pattern/transport mismatches (§8.2). |
| `implemented` | `bool` | Template exists. `false` → build error at codegen stage (not deferred to C++ `#error`). |
| `required_binding_fields` | `[&str]` | deploy.yaml fields that must be present. `[]` for transports with no binding-level config (local, shm). |

Adding a transport requires exactly **two changes** (Rust registry entry + Jinja2 template block); the template's `#error` fallback catches drift at C++ compile time.

**Future extensions** (E2): `supplies_dedup: bool` (skip dedup layer for inherently-duplicate-free transports), `supplies_ordering: bool` + `ordering_representable: bool` (gate the §10.6 sequence-ordering runtime buffer and the CAN-style topology reject), `conformance_level: Complete | Degraded` (validated against deploy.yaml declarations), `max_payload_bytes: Option<usize>` (envelope size validation at build time).

#### 10.4.3 Conformance Verification

A transport is verified against the §10.4 contract by:

1. **Build-time (sce-build)**: Pattern capability check — `TransportDescriptor::capabilities` must include the pattern's required category. Violation is a hard build error.
2. **Build-time (sce-build)**: Required binding fields — `TransportDescriptor::required_binding_fields` must all be present in deploy.yaml. Violation is `TopologyError::MissingBindingField`.
3. **Runtime (mesh conformance suite, Session E2)**: The IRP distributed harness (§16.8) runs identical tests over single-process and distributed modes. A transport that drops, reorders, or duplicates events (without dedup suppression) will produce verdict mismatches, surfacing the conformance violation.
4. **Runtime (seeded fault injection, Session E2)**: Disconnect the transport mid-test. The engine must receive `error.communication` within one macrostep. The test verifies event delivery, `reason` payload, and macrostep boundary compliance.

**Status**: gates 1 and 2 are active. Gates 3 and 4 are Session E2 scope (§16.9) and not yet implemented. Today's runtime evidence for transport conformance is the 44 mesh ctest fixtures catalogued in [`docs/SCE_MESH_CONFORMANCE_MATRIX.md`](../docs/SCE_MESH_CONFORMANCE_MATRIX.md) — per-sender FIFO is asserted wire-level by `mesh_custom_tcp_runtime_verification`, duplicate tolerance by the §10.5 fixtures, and fault-signal emission by the §16.7 liveness fixtures. These ctests verify the transport primitives that gate 3 will consume; they are not the gate 3 suite itself.

### 10.5 Duplicate Suppression

Per-envelope duplicate suppression is a **mesh runtime responsibility**, not a transport-layer one. Runtime maintains a **per-sender recent-id window**:

```cpp
struct DedupWindow {
    // Ring buffer of UUID v7 envelope ids from this sender. Window size is
    // configurable; default 256 entries ~= 4KB per sender.
    std::array<std::array<uint8_t, 16>, 256> recent_ids;
    size_t head;
};
std::unordered_map<std::string /*sender*/, DedupWindow> dedup_;
```

On envelope receipt:
1. Look up `dedup_[envelope.source]`.
2. If `envelope.id` is already in the window → drop (duplicate).
3. Otherwise, insert into window, forward to engine.

UUID v7's monotonic ms-prefix makes the window a time-bounded sliding filter: a 256-entry window at 1000 events/sec per sender covers 256 ms — longer than any practical retransmit window.

Transports that **inherently suppress duplicates** (TCP single stream, SOME/IP over TCP) may skip the dedup layer; a transport descriptor flag `supplies_dedup: true` disables the runtime check. In-tree transports:

| Transport | `supplies_dedup` |
|---|---|
| local, shm | `true` (in-process queueing cannot duplicate) |
| custom_tcp | `true` (single TCP stream per sender) |
| someip (TCP) | `true` |
| zenoh (reliable) | `false` (application id dedup still runs — router reordering possible) |
| udp, generic unreliable | `false` |

### 10.6 Sequence Ordering Buffer

§10.1 classifies per-source FIFO as the transport's responsibility: TCP-based substrates deliver in order by construction, UDP-based substrates may reorder. SCXML state transitions are order-sensitive — `brake.press` arriving after `brake.release` leaves the system in the braking state until another press. The mesh runtime closes this gap for UDP-backed bindings via a sequence-ordered admit layer that sits alongside §10.5's dedup filter.

The design mirrors §10.5: per-binding declaration in deploy.yaml + runtime buffer emitted only when the transport cannot provide the guarantee natively. TCP-based bindings pay zero runtime cost.

#### 10.6.1 deploy.yaml schema

```yaml
topology:
  ecu1:
    machines:
      brake:
        ordering:                   # NEW — per-machine receiver buffer timings
          gap_timeout_ms: 100       # required if section present (no field-level default)
          tick_period_ms: 50        # required if section present
        bindings:
          "#motor":
            transport: zenoh
            key: motor/cmd
            ordering: required      # per-binding; default "none"
```

The two `ordering` keys answer different questions:

- **Per-binding `ordering:`** (under `bindings.<target>`) — *whether* the route enforces FIFO. Values: `none` (default — engine sees arrival order) | `required` (per-source FIFO guaranteed by either the transport or the runtime buffer).
- **Per-machine `ordering:`** (sibling of `bindings:`) — *how* the receiver buffer behaves once activated. Both fields are required when the section is present so a partial override cannot leave the relationship between the two values implicit; omit the section entirely to accept the defaults (100 ms / 50 ms — module constants `DEFAULT_GAP_TIMEOUT_MS` / `DEFAULT_TICK_PERIOD_MS` in `sce-build/src/mesh/deploy.rs`, the single source of truth). Validation (`mesh/deploy-invalid-ordering-timings`): both fields must be positive AND `tick_period_ms` must be strictly less than `gap_timeout_ms` (Nyquist), so a missed sequence is detected within `gap_timeout + tick_period`.

#### 10.6.2 Dispatch decision

| Transport | `ordering` | Behavior |
|---|---|---|
| local, shm, custom_tcp | `required` or `none` | Transport supplies order natively — zero runtime cost. |
| SOME/IP `protocol: tcp` | `required` or `none` | Transport supplies order — zero runtime cost. |
| Zenoh | `required` | Runtime `OrderingBuffer` activated; envelopes sorted by sender-stamped `sequence_no` before dispatch. |
| Zenoh | `none` | Current §10.1 contract — arrival order. |
| SOME/IP (UDP default) | `required` | Runtime `OrderingBuffer` activated. |
| SOME/IP (UDP default) | `none` | Current §10.1 contract. |
| DDS (any QoS) | `required` | Runtime `OrderingBuffer` activated. |
| CAN | `required` | **Rejected at topology stage** — broadcast bus cannot represent a per-(sender, receiver) sequence domain. Use `ordering: none` or switch transport. |
| CAN | `none` | Current §10.1 contract. |

The registry predicate pair is `supplies_ordering` (transport provides native FIFO) and `ordering_representable` (receiver, given a sender-stamped `sequence_no`, can reconstruct per-sender order). CAN is the sole `ordering_representable = false` transport: every frame reaches every bus participant, so a per-(source, target) sequence domain does not exist on the wire.

#### 10.6.3 Sequence stamping

Each sender maintains a per-target monotonic counter. The codegen-generated mesh-send-callback stamps `env.sequence_no = ++seq_counter_<target>` immediately before `route_send`, and only when the target binding has `ordering: required` on a non-ordered transport.

- Scope: per-(sender machine, target binding). Different targets on the same machine have independent counters.
- Type: `uint64_t`. At 1 kHz per target the counter wraps after ~5.8 × 10⁸ years; no wrap guard.
- Concurrency: mesh-send-callback runs on the engine's single step thread — no mutex.
- Wire: envelope field `sequence_no` (CBOR integer key 14, optional). Absent when the sender is not on an ordered route. A receiver with an active `OrderingBuffer` that observes an envelope without `sequence_no` drops the envelope and raises `error.communication.missing_sequence`. Topology guarantees all senders on an ordered route stamp the field; a missing value signals an out-of-sync sender. Pre-release wire format, no backward-compat shim.

#### 10.6.4 Receiver buffer

`OrderingBuffer` holds per-source state: `next_expected_seq` (starts at 1) and a sorted map of buffered envelopes. On `admit(source, env)`:

1. If `env.sequence_no == next_expected_seq` — dispatch immediately, increment `next_expected_seq`, then drain any contiguous higher sequences already buffered.
2. If `env.sequence_no > next_expected_seq` — buffer with an arrival timestamp.
3. If `env.sequence_no < next_expected_seq` (post-fast-forward straggler) — drop silently.

`tick()` fires gap timeouts: for each source whose `next_expected_seq` has been blocked longer than `gap_timeout`, fast-forward past the gap, raise `error.communication` with `reason = "ORDERING_GAP"` and the lost range carried in `_event.data.lost_seq_lo` / `lost_seq_hi` (§16.7 row 12), and drain buffered envelopes now contiguous with the new `next_expected_seq`. The generated `TransportRouter` owns a periodic tick thread that drives `tick()` at the deploy-supplied `tick_period_ms` cadence; cross-source gaps and gap-at-stream-end (no further inbound traffic) are recovered within one tick plus `gap_timeout`. `gap_timeout` and the tick cadence both come from the per-machine `ordering:` block in deploy.yaml (§10.6.1) — the receiver runtime carries no fallback constant, the values are emitted into the generated router verbatim. Defaults (100 ms / 50 ms) apply when the `ordering:` section is omitted; the parser enforces Nyquist (`tick_period_ms < gap_timeout_ms`).

An envelope that reaches `admitOrdered` on an ordered route without `sequence_no` is dropped and `error.communication` with `reason = "MISSING_SEQUENCE"` (§16.7 row 11) is raised, carrying `source` and `envelope_id` in `_event.data` for diagnosis. Topology guarantees all senders on the route stamp `sequence_no`; the event surfaces a sender drift condition.

Interaction with §10.5 dedup: the generated admit path is `admitOrdered`, which chains dedup → ordering → dispatch internally. A single generated method body; the `ordering: required` + `needs_dedup` composition introduces no code duplication.

Interaction with correlation-keyed patterns: `RpcReply` and `FieldNotify` envelopes carry `correlation_id` and match pending requests by that field, so their order across the wire is irrelevant to correctness. `admitOrdered` bypasses the buffer for those patterns (direct `dispatchToSender`) to avoid paying reply latency for an order invariant no handler depends on.

Per-source state (`state_` map inside `OrderingBuffer`) is bounded by the `deploy.yaml` machine roster — same argument as §10.5's DedupRouter; no eviction policy.

### 10.7 `_event` Field Wiring for Distributed Events

W3C §5.10.2 defines the standard `_event` fields. SCE Mesh populates them deterministically from envelope fields:

| `_event` field | Single-process single instance | Distributed (from inbound envelope) |
|---|---|---|
| `name` | dispatched event name | envelope `type` |
| `type` | `"internal"` or `"external"` | `"external"` (all mesh-delivered events are external per §5.10.1) |
| `sendid` | `<send>`'s id attribute or generated | envelope `subject` (or unset if not `<send>`-originated) |
| `origin` | unset (internal) | `mesh://<envelope.source>` (URI form; portable target spec) |
| `origintype` | unset (internal) | `"http://www.w3.org/TR/scxml/#SCXMLEventProcessor"` for inter-SCXML mesh traffic; transport-specific URIs for bridged traffic (e.g., `"sce:mesh/someip"` for raw bus events) |
| `invokeid` | unset | envelope `invoke_id` as hex string, or unset if no invoke context |
| `data` | payload | deserialized per envelope `datacontenttype` |

These fields are surface-compatible with local execution — an author's `<transition cond="_event.origin == 'mesh://chassis'">` works identically whether the event arrives locally or via any transport.

#### 10.7.1 Structured `_event.data` for `error.*` events

W3C SCXML 1.0 does not prescribe a `_event.data` schema for `error.execution` / `error.communication`; the spec only fixes the event names (§5.10.1). SCE Mesh pins a **JSON-shaped convention** so authors have a stable contract:

```
_event.data = {
  "errorName":    "execution" | "communication",
  "reason":       "<machine-readable reason code>",       // required
  "detail":       "<human-readable detail, optional>",
  "source":       "<envelope.source or null>",            // communication only
  "sendid":       "<originating sendid or null>",         // when applicable
  "envelope_id":  "<UUID v7 hex or null>",                // communication only
  "invoke_id":    "<UUID v7 hex or null>"                 // invoke-related only
}
```

Reason code catalog for `error.communication` is in §16.7. `error.execution` reason codes are SCE-internal; the canonical list is:

| `reason` | Origin |
|---|---|
| `INVOKE_TYPE_UNSUPPORTED` | `<invoke type=…>` type URI not recognized (e.g., foreign processor or `sce:mesh-rpc` on a reference impl without the extension) |
| `INVOKE_SRC_NOT_FOUND` | `<invoke type="sce:mesh-rpc">` setup could not resolve its target to a live dispatch path: `src="#X"` references a machine not registered in deploy.yaml, or `srcexpr` evaluated to a name / shape the static topology does not cover (§9.5), or an `instance_from:` placeholder for a SOME/IP pool binding resolved to an instance outside the declared `instances: [...]` set (§14.4). No envelope reached the wire. |
| `INVOKE_CHILD_INIT_FAILED` | remote child raised an error before reaching its first stable configuration |
| `RESERVED_PARAM_CONFLICT` | build-time: `<param>` shadows a `_mesh_*` reserved name (build fails before runtime; reason surfaces only if an out-of-band document bypasses the build tool) |
| `SESSION_F_NOT_IMPLEMENTED` | parser/model accepts full remote `<invoke type="scxml">` but runtime path is not yet implemented (Session E1/E2 transitional) |

Foreign W3C SCXML 1.0 processors that do not implement SCE Mesh will raise `error.execution` with their own `_event.data` shape (if any). Documents that must be portable between foreign and SCE processors should guard on `_event.name == 'error.execution'` only, not on `_event.data.reason`. SCE's own error handlers may read `_event.data.reason` reliably for diagnostics and recovery logic.

### 10.8 Delayed Send + Cancel (Cross-Process)

Per W3C §6.2.4, `<send delay="5s" id="later">` queues an event for future delivery, cancellable by `<cancel sendid="later">`. Distribution extends this as follows:

**Sender-hold model**: the delay timer lives at the **sender** side. The envelope is not emitted until the delay expires:

```
t=0    <send delay="5s" id="later" target="#peer" event="e1">
         parent scheduler queues { emit_at=t+5s, id="later", envelope=... }
         no wire traffic yet
t=1s   <cancel sendid="later"/>
         parent scheduler removes the queued entry
         no wire traffic at all — perfectly cancelled
```

**Emit boundary**: if the delay expires before cancel, the envelope is sent. Once on the wire, `<cancel>` downgrades to **best-effort**:

```
t=0    <send delay="50ms" id="fast" target="#peer" event="e2">
         scheduler queues with emit_at=t+50ms
t=50ms scheduler emits envelope to transport
         wire traffic begins
t=51ms <cancel sendid="fast"/>
         local record removed; a BestEffortCancel control envelope is
         emitted with the same envelope id and rpc_status=Cancelled.
         Peer may or may not have already processed the original event.
```

**Guarantees**:
- Cancel **before** emit_at: deterministic, no peer observes the event.
- Cancel **after** emit_at: peer *may* have processed the event. The cancel control envelope tells the peer to *stop processing if not yet delivered to the SCXML engine* — specifically, if the event is still in the receiver's transport-side inbox (queued but not raised), the runtime removes it; if already delivered to the engine, cancel is a no-op.
- Cross-process cancels **cannot unwind** effects of an already-processed event. Author must design for this (e.g., compensating transitions).

**Tracking**: the sender's scheduler maintains `{sendid → (emit_at, envelope)}` for unfired delays. Cancel is a local hash-table removal. Emitted sends are tracked in a short-lived `{sendid → envelope.id}` so cancel after emit can generate the best-effort control envelope.

### 10.9 Origin Identity — `source` vs `routing_id`

Mesh distinguishes **document identity** from **session identity** on every envelope:

- **`source`** (CBOR key 1, required, string) — the sending machine's stable document name, i.e. the deploy.yaml `machines.<name>` key and the SCXML `name=` attribute. This is the identity that survives across restarts of the same machine and is the axis SCXML authors reason about (`_event.data.source`, `error.communication.target`, liveliness `sce/live/<machine_name>` keyexpr).
- **`routing_id`** (CBOR key 15, optional, 16-byte UUID v7) — a per-router identifier generated once at `TransportRouter` construction and stamped on every outbound envelope. This is the identity that discriminates two running routers hosting the same document name — two processes each hosting one instance of the same machine generate distinct `routing_id`s at ctor time, so a peer that receives both can tell their echoes apart. A SOME/IP server pool on one process shares a single `routing_id` across its N sessions: self-echo filtering is a router-level property (one transport session per router on each backend), and every in-tree echo site (Zenoh server put-sub) is excluded from the pool shape (§14.4) so the single router-scoped identity is sufficient.

**Invariants**:

1. **Outbound stamp** — every envelope leaving a `TransportRouter` MUST carry `routing_id = self.routing_id_`. Generated code stamps this at every construction site (mesh-send-callback, machine-lifetime subscribe/unsubscribe, `error.communication` raises, mesh-rpc requests) so no outbound path bypasses the stamp.
2. **Self-filter axis** — echo-suppression sites (e.g. the Zenoh server put-subscriber that observes its own publish on a shared key, liveliness observers that see their own token) compare `env.routing_id` against the local **router's** `routing_id_` — not `env.source` against `machine_name`. Routing_id is router-scoped (see invariant 7): under §14.4 server pool, sibling sessions within the same router share this single value, so echo-suppression operates at router granularity rather than per-session. Same-document, different-router envelopes (the scenario a future multi-router topology would introduce) pass through the filter; same-router wire echoes are dropped. An absent `routing_id` (decoded from a peer backend that has not yet been migrated) compares unequal to the local value and therefore passes the filter — cross-backend rollout does not disturb echo semantics during the transition.
3. **SCXML invisibility** — authors do not see `routing_id`. It is not exposed in `sce:` attributes, `_event.data`, or any datamodel surface. `error.communication.target` carries `machine_name` (document identity); author rules `cond="_event.data.source == 'motor'"` reason over the stable axis.
4. **Liveliness axis stays on document identity** — Zenoh's `sce/live/<machine_name>` keyexpr, `CommunicationError.target` for `PEER_PARTITIONED`, and `peer_last_seen_` keying are document-level concepts: "is this machine reachable" is answered by the deploy.yaml roster, not by individual sessions. Under multi-instance (§14.4), a token remains alive while any session of the machine is up; partition is raised only when the last session drops.
5. **Sender-identity keying stays on `env.source`** — containers that key on sender identity (DedupRouter `windows_` §10.5, OrderingBuffer `state_` §10.6, `peer_last_seen_` §16.7 row 8) key on `env.source` (machine_name). This axis is correct under both single-session and §14.4 server pool topologies, by construction:
   - **R2 (DedupRouter)** — `env.id` is a fresh UUID v7 stamped per outbound emit at the mesh-send-callback site (unconditional). A pool router's N sessions emit through the same callback, so successive envelopes carry distinct ids regardless of which session raised them; cross-session id collision under one source is statistically impossible. Routing_id would not improve discrimination.
   - **R4 (OrderingBuffer)** — `seq_counter_{target}_` is a router-scoped member, incremented under one mutex per (router, target). A pool router's N sessions share a single monotonic axis per outbound target, so a peer's OrderingBuffer keyed on this source receives one well-ordered stream regardless of pool cardinality.
   - **T3 (`peer_last_seen_`)** — kept on document identity per invariant 4 (machine reachability is a document-level concept; aggregating across sessions is the intended semantic).

   The collision scenario this invariant was originally written to gate against — two routers contributing distinct sequence streams under one source — is **multi-router (cross-process) hosting of the same machine name**, not §14.4 single-router pool. That topology is not currently supported by deploy.yaml grammar (one machine declaration per ECU); if it is opened, a `(env.source, env.routing_id)` keying axis would become necessary and would be recorded as a new invariant under that gate.
6. **Transport subscription lifetime is target-scoped, not (target, event)-scoped** — pub/sub transports (today Zenoh; SOME/IP eventgroups follow the same shape) express subscription state at the target granularity: one Zenoh `declare_subscriber` per keyexpr, one SOME/IP `subscribe` per `(service, instance, eventgroup)`. The logical SCXML refcount on `(target, event)` records how many regions hold each event axis; the transport-native declare/undeclare fires on the 0↔1 transition of the **per-target sum** of those axes. Concretely: `target_sub_live_count_[target]` counts the number of `(target, *)` axes with refcount > 0, and only its 0↔1 crossings reach the transport. A single `(target, event)` unsubscribe that drives its per-event refcount to zero MUST NOT tear down the shared transport subscriber while sibling `(target, *)` axes are still live.
7. **Dispatch is session-indexed, `sessions_` is the sole SSoT** — the generated `TransportRouter` stores hosted SCXML documents as `std::array<SenderEngine*, N_SESSIONS> sessions_`; every inbound path reaches engines through `dispatchToSession(env, session_idx)` (and the layered `admitInbound` / `admitOrdered` / `admitZenohInbound` variants that thread the index unchanged). There is no parallel `sender_` alias, no machine-level sender reference, and no dual vocabulary for pool vs non-pool routers. SOME/IP server-pool handlers resolve `msg->get_instance()` to a slot via `session_index_for_instance()`; every other inbound call site passes `0` because its path is not pool-dispatched (client-side SOME/IP, Zenoh server, custom_tcp, local/linkTo, ordering driveOrderingTick). `raiseCommunicationError` fans out to every `sessions_[i]` because transport-layer conditions (ordering gap, missing sequence, PEER_PARTITIONED) are observed on the router, not on a specific session.
8. **Reply-correlation tables are router-scoped; pool coexistence with router-scoped RPC clients is rejected at codegen** — three correlation containers observed on reply paths live one-per-`TransportRouter`, not one-per-session:
   - **`invoke_correlation_`** (§9.5 mesh-rpc) — `unordered_map<invoke_id, DeliverCallback>`. The callback closure is captured at `<invoke>` entry with the parent engine's `invokeId` string, so firing it raises `done.invoke.<id>` / `error.invoke.<id>` on one specific session; hosting multiple sessions would alias `invoke_id` entries and cross-deliver their completions.
   - **`active_invokes_`** (§9.5 mesh-rpc cancel) — `unordered_map<(target, field_suffix), uuid>`. Two sessions entering the same `<invoke>` site concurrently would overwrite each other's UUID, leaving one state-exit `<cancel>` with no entry to cancel.
   - **`pending_rpcs_`** (SOME/IP `<send>` RpcRequest) — `unordered_map<correlation_id, reply-event-name>`. The generated client-side receive handler dispatches the matched reply to `sessions_[0]` because no session identity is threaded through the correlation key. Under a pool router this hard-codes cross-session reply misrouting for any `<send event="service.request.*">` pattern.
   
   The structural remedy for all three tables would key their state on a `(session_idx, key)` axis, but no in-tree product surface calls for pool + client-RPC coexistence today — §14.4 Phase 6 motivation scopes pool to server-side replication of the same SCXML document. `sce-build` rejects the combination at codegen via `MeshCodegenPoolWithRpcClientUnsupported` (two `kind` arms: `MeshRpc` for `<invoke>` sites, `SomeipRpcRequest` for SOME/IP `<send>` Request-Reply patterns) so the runtime never observes aliasing. When a concrete consumer arrives, the reject lifts alongside the correlation-key migration in one commit — spec-design and implementation move together.

**Wire compatibility**: `routing_id` is an optional CBOR key. Decoders MUST skip unknown keys (per §13 canonical CBOR contract), so a sender backend that has not yet been migrated emits envelopes without key 15 and peers decode correctly. The self-filter invariant above ensures that during per-backend rollout, cross-backend echo paths remain functional — unmigrated peers' envelopes are correctly passed through.

### 10.10 `OutboundBuffer` — readiness-gated outbound admit

Sibling of §10.5 `DedupRouter` and §10.6 `OrderingBuffer`. Both of those layers sit on the **inbound** path (duplicate suppression, sequence reorder); `OutboundBuffer` sits on the **outbound** path and addresses a distinct failure mode: transports whose peer may not yet be ready when `route_send` runs drop the payload silently.

**The silent-drop surface**:
- **SOME/IP**: vsomeip `app.send()` on a NOT_AVAILABLE service returns `true` and drops the payload. Pre-§10.10 the first `<send target="#peer">` on a service that has not yet been `offer_service`'d by the server is lost with **no `error.communication`** raised. The harness-level workaround (`test_mesh_someip_runtime.cpp` manually blocks on `register_availability_handler` before sending) does not generalise to production code that cannot predict peer boot order.
- **Zenoh (PUT-style only — FireForget, FieldWrite)**: `session.put` to a keyexpr with no matching subscriber is lost. Zenoh's default delivery model has no retention for publisher-first samples — a subscriber that declares after the put never observes it. GET-style patterns (RpcRequest, FieldRead) and subscribe patterns (EventSubscribe) are structurally resilient to this (GETs surface late peers via `on_drop` → §9.5 gap Z3 `RpcStatus::Unavailable`; subscribers get future samples by construction).

**The §10.10 primitive**: per-target `OutboundBuffer` instance (`sce/include/mesh/OutboundBuffer.h`), constructed by the generated `TransportRouter` with three inputs: the target identifier (for `BACKPRESSURE_DROP` event data), a dispatch closure bound to the transport-specific send function, and a capacity bound (`max_pending_per_target` from deploy.yaml). `admit(env)` is the single entry from `route_send`:

- **Fast path** (ready && queue empty): dispatch immediately.
- **Enqueue** (not ready, or queue non-empty mid-drain): push to FIFO queue up to `max_pending_per_target`.
- **Overflow** (queue at capacity, not ready): raise `error.communication` with reason `BACKPRESSURE_DROP` (§16.7 row 9) and drop the newest envelope.

Transport readiness primitives call `markReady()` / `markNotReady()`:
- SOME/IP: `app.register_availability_handler` — installed in `init()` before `start()` so the initial NOT_AVAILABLE→AVAILABLE edge is observed. Availability is service-level, so the buffer gates **all** outbound patterns on this target.
- Zenoh: `Publisher::declare_matching_listener` on a declared publisher — observes subscribers appearing and disappearing on the publisher's keyexpr. `get_matching_status()` seeds the initial state at `init()` time. Gates **FireForget and FieldWrite only**; GET / Subscribe paths stay on the existing `send_zenoh` branch.

**FIFO guarantees**: `admit` holds the buffer mutex for the fast-path dispatch so a concurrent `markReady` drain cannot interleave with a direct-dispatch envelope. Per-target `seq_counter_{target}_` (§10.6) is stamped in the mesh-send-callback **before** `route_send` is called, so sequence numbers reflect call order regardless of whether a given envelope takes the fast path, the enqueue path, or the drain path — all three preserve per-target call-order monotonicity.

**Scope (what `OutboundBuffer` is NOT)**:
- **Not a retry layer**. A dispatcher return of `false` (transport-native send failure after readiness) is not re-enqueued. Existing error surfaces (§16.7 row 2 `SEND_FAILED`, row 3 `DELIVERY_EXHAUSTED`) cover that axis and are raised by call-sites that already exist before admit.
- **Not an age-based drop policy**. Overflow policy is fixed at `BACKPRESSURE_DROP` + drop-newest. `max_age_ms` and `overflow: drop_oldest` are additive grammar extensions gated on a future consumer.
- **Not a retention store**. The buffer is router-scoped (destroyed with the `TransportRouter`); envelopes in the queue at `shutdown()` are discarded silently.
- **Not applicable to local / shm / custom_tcp targets**. Local is in-process (no readiness concern); shm pairs at ctor time; `CustomTcp::Client` has its own connect-retry semantics. The template only emits `OutboundBuffer` members for `target.state.kind in ("someip", "zenoh")`.

**Opt-in gate**: absent `outbound_buffer:` section on the machine ⇒ zero buffer code emitted, `route_send` arms keep the pre-§10.10 direct-dispatch shape. See §14 grammar below.

**Configuration** (per-machine, single knob):
```yaml
machines:
  brake:
    outbound_buffer:
      max_pending_per_target: 64
```

`max_pending_per_target` must be `>= MIN_OUTBOUND_BUFFER_MAX_PENDING` (1) per the `sce-build/src/mesh/deploy.rs` validation floor. Zero is rejected at parse time with `mesh/deploy-invalid-outbound-buffer` because a zero-capacity buffer is semantically indistinguishable from opting out.

---

## 11. Performance Characteristics

All performance numbers below are **estimates based on architectural analysis, not measured benchmarks**. Actual performance will be validated during implementation. Numbers are presented as ranges (best case — worst case) rather than single optimistic values.

### 11.1 AOT State Machine Cost (Local, No Transport)

These numbers represent the pure state machine transition cost, **excluding** transport, serialization, and discovery overhead.

| Operation | Best Case | Worst Case | Notes |
|-----------|-----------|------------|-------|
| Single transition (switch/case) | 1 ns | 20 ns | Worst: cache miss on cold path |
| Data model update | 10 ns | 200 ns | Worst: complex expression eval |
| Internal event emit | 20 ns | 500 ns | Worst: queue contention |
| **Total per transition (local)** | **30 ns** | **720 ns** | |

### 11.2 Transport Overhead (Added to Local Cost)

Since transport code is generated as direct API calls (no runtime abstraction layer), the overhead is the transport library's native cost only — no vtable dispatch, no routing map lookup, no type erasure.

| Transport | Best Case | Worst Case | Mesh Abstraction Overhead | Notes |
|-----------|-----------|------------|--------------------------|-------|
| Direct call (same process) | 0 ns | 0 ns | **0 ns** (inlined away) | Codegen emits direct function call |
| Shared memory | 100 ns | 5 us | **0 ns** | Generated code calls shm_write directly |
| SOME/IP (same network) | 50 us | 2 ms | **0 ns** | Generated code calls vsomeip::send directly |
| DDS (same network) | 50 us | 2 ms | **0 ns** | Generated code calls dds_write with native QoS |
| CAN bus | 100 us | 10 ms | **0 ns** | Generated code calls SocketCAN write directly |
| gRPC (WAN) | 1 ms | 100 ms | **0 ns** | Generated code calls gRPC stub directly |

The "Mesh Abstraction Overhead" column is always 0 because there is no runtime abstraction — sce-build generates code that calls each transport's native API as if hand-written. The only overhead compared to hand-written transport code is the routing dispatch (constexpr if-chain), which the compiler eliminates for single-target cases.

### 11.3 Throughput at 60Hz Game Tick (16.6ms)

Based on **local transitions only** (no transport). Real throughput depends on ratio of local vs remote events.

| Configuration | Transitions/tick (best) | Transitions/tick (worst) |
|--------------|------------------------|--------------------------|
| Single core | ~550,000 | ~23,000 |
| 8 workers | ~4,400,000 | ~184,000 |

### 11.4 Memory per Instance

| Component | Minimum | Typical | Maximum |
|-----------|---------|---------|---------|
| State enum | 1 byte | 2 bytes | 4 bytes |
| Data model | 0 bytes | 64 bytes | 4 KB (complex models) |
| Event queue pointer | 8 bytes | 8 bytes | 8 bytes |
| Dedup tracking (if enabled) | 0 bytes | 0 bytes | 16 bytes per source |
| **Total per entity** | **9 bytes** | **74 bytes** | **~4 KB** |

### 11.5 Interpreter vs AOT Comparison

| Metric | Interpreter | AOT |
|--------|-------------|-----|
| XML parsing | Every load | Build-time only (zero at runtime) |
| Transition cost | 1-10 us | 30-720 ns (10-100x faster) |
| Memory per instance | Several KB (tree structure) | 9 bytes - 4 KB (enum + data) |
| Cache efficiency | Low (pointer chasing) | High (contiguous memory) |
| SIMD potential | No | Yes (batch processing) |
| Branch prediction | Low (indirect calls) | High (static branches) |

---

## 12. Example: Same SCXML, Three Domains

```xml
<!-- door_controller.scxml -->
<scxml name="door">
  <state id="closed">
    <transition event="open.request" target="opening"/>
  </state>
  <state id="opening">
    <onentry>
      <send event="motor.run" target="#actuator"/>
    </onentry>
    <transition event="sensor.fully_open" target="open"/>
    <transition event="error.obstruction" target="closing"/>
  </state>
  <state id="open">
    <transition event="close.request" target="closing"/>
  </state>
  <state id="closing">
    <onentry>
      <send event="motor.reverse" target="#actuator"/>
    </onentry>
    <transition event="sensor.fully_closed" target="closed"/>
  </state>
</scxml>
```

The same SCXML generates different transport code depending on deploy.yaml:

| Domain | deploy.yaml scheduler | deploy.yaml transport | Generated `send_to_actuator()` calls |
|--------|----------------------|----------------------|--------------------------------------|
| Game (dungeon door) | `game_loop` (60Hz) | `udp` | `sendto(udp_socket_, ...)` |
| Vehicle (car door) | `real_time` (10ms) | `can` | `write(can_socket_, &frame, ...)` |
| Simulator | `event_driven` | `local` | `actuator_sm.processEvent(...)` (inlined) |

**The SCXML is identical across all three. Only deploy.yaml changes.**

---

## 13. Roadmap

**Scope commitment**: Phase 1-2 are complete. Phase 3-5 are directional plans refined after Phase 2 delivery.

### Phase 1: Codegen Infrastructure + Local Transport — COMPLETE

`--deploy` CLI option, deploy.yaml parser (serde_yaml), topology analyzer, `local_transport` template, `SchedulerConcepts.h` (TickScheduling/EventDrivenScheduling C++20 concepts with C++17 fallback), `EventQueueBridge.h` (lock-free MPSC, Vyukov algorithm). Build-time topology completeness and event coverage validation. Verification: single-process tests pass unchanged with `--deploy` codegen path.

### Phase 2: Shared Memory Transport — COMPLETE

`shm_transport` template (POSIX `shm_open`/`mmap`), `ShmChannel` with placement-new EventQueueBridge in shared memory, ready-flag handshake for any-order startup. Build-time event coverage enforcement (`UncoveredEvents` error — eliminates silently-broken-hooks pattern). Verification: two processes on same machine communicating via SCXML through generated SHM code, SCXML documents unchanged. QoS consistency check deferred to Phase 3.

### Phase 3: Vehicle Network Transport Templates + Communication Patterns — COMPLETE

Transport templates for real-world middleware. Each template generates code that calls the protocol's native API directly, preserving all protocol-specific features.

#### Status: All Patterns Realized (2026-04-16)

Section 8.1 defines 7 `CommunicationPattern` values and Section 8.2 defines a per-transport capability matrix. Build-time validation works. All patterns are realized in the wire format and transport templates for both SOME/IP and Zenoh.

| Pattern | Build-time validation | SOME/IP realization | Zenoh realization |
|---|---|---|---|
| `service.fire_forget` | OK | `app.send(request)` — method, no response | `session.put(key, envelope)` |
| `service.request` / `service.response` | OK | `app.send(request)` + `register_message_handler` + correlation table | `session.get` with on_reply closure |
| `event.subscribe` / `event.notification` | OK | `app.request_event` + `app.subscribe` / `app.unsubscribe` + refcount | `declare_subscriber` + RAII handle map |
| `field.get` / `field.set` | OK | `app.send(request)` to getter/setter method + response handler | `session.get` (read) / `session.put` (write) |

Compile-verified: `mesh_someip_multipattern_compile_verification` (Session C) and `mesh_zenoh_multipattern_compile_verification` / `mesh_zenoh_multipattern_runtime` (Session D) exercise all branches against real transport headers.

#### Architecture additions in Phase 3

- **Communication Pattern Semantics** (Section 8.1): Transport-agnostic event vocabulary (`service.request`, `event.subscribe`, `field.get`, etc.) enabling deploy.yaml-only middleware switching
- **Transport Capability Matrix** (Section 8.2): Build-time validation that SCXML communication patterns are supported by the bound transport
- **Pattern capability build check** (Section 7.7): sce-build emits build error when SCXML uses a pattern unsupported by its deploy.yaml transport

#### Transport templates

All transports share the unified `mesh_transport.h.jinja2` template via `{% elif %}` dispatch (Section 3.2):

- `someip` transport: SOME/IP via real vsomeip 3.7.x — service/instance/method IDs from deploy.yaml, TCP/UDP protocol selection, build-time pattern capability validation — **COMPLETE** (Session C: all 4 capabilities — FireForget via `app.send(request)`, RPC via method + `register_message_handler` + CBOR correlation table, PubSub via `app.request_event`/`app.subscribe`/`app.unsubscribe` + refcount, FieldAccess via getter/setter methods + response handlers)
- `zenoh` transport: Zenoh pub/sub via zenoh-cpp — device-shared session with `Config::insert_json5`, key expressions from deploy.yaml `extra` — **COMPLETE** (Session D: client-side realization of all four capabilities — `session.put` (FireForget/FieldWrite), `session.get` with on_reply closure (RpcRequest/FieldRead), `declare_subscriber` + RAII handle map (EventSubscribe/EventUnsubscribe). Server-side `declare_queryable` emission deferred to Session E alongside `<invoke type="sce:mesh-rpc">` lifecycle.)
- Additional transports as demand arises — each adds a `{% elif %}` block in the template following the established pattern (Section 6.4)

#### Infrastructure

- **deploy.yaml native QoS**: each transport section carries full transport-native QoS configuration, passed directly to template without abstraction
- **Remote `<invoke>` codegen**: invoke request/response/cancel as generated send/receive pairs over configured transport — *blocked on Phase 3.5*
- **SCE Forge procedure integration**: Forge `procedure` kinds work with Mesh remote `<invoke>` across device boundaries — *blocked on Phase 3.5*
- **Build-time verification**: interface match, cross-transport ordering warnings, pattern capability check

#### Entry sequence

1. Communication Pattern event semantics definition (Section 8.1 formalization) — COMPLETE
2. First transport template: `someip_transport` via real vsomeip 3.7.x — COMPLETE (all 4 capabilities: FireForget, RPC, PubSub, FieldAccess — Session C)
3. Second transport template: `zenoh_transport` via zenoh-cpp — validates that Communication Pattern Semantics abstraction is correct across middlewares — COMPLETE (all four categories: FireForget, RPC via queryable/get, PubSub via subscriber/put, FieldAccess — Session D)
4. Mesh-native event serialization (byte-stream group):
   a. Canonical CBOR `MeshEnvelope` wire format via `encodeEnvelope`/`decodeEnvelope` — COMPLETE (FireForget realized, others stubbed)
   b. SHM control-ring + payload-arena layout (Section 7.5) — replaces fixed-size `ShmEvent` — COMPLETE
   c. Receive path: transport drain → `raiseExternal` (no `step()` in drain) — scheduler-owned macrostep — COMPLETE for shm; COMPLETE for someip (Session C: `register_message_handler` + `receive_callback_` + correlation); COMPLETE for zenoh (Session D: `declare_subscriber` + `session.get` on_reply closures drive `receive_callback_` natively)
   d. End-to-end payload test: SCXML `<param>` → `_event.data` on receiver with type preservation — COMPLETE (commit `3a3e36df`)
5. Application-level test demonstrating deploy.yaml-only middleware switch — COMPLETE for FireForget (`mesh_middleware_switch_demo.sh`)

### Phase 3.5: Pattern Realization (next priority)

Realize the remaining 8 communication patterns at the wire and runtime level. Closes the gap between `pattern.rs` capability advertisements and what transports actually implement.

#### Design decisions (Session A, sign-off 2026-04-13; revised Session E1 path B/C, 2026-04-14)

| Axis | Decision | Rationale |
|---|---|---|
| Pattern enum | 9 base variants + immutable `wire_value: u16` + values 10-13 reserved for future Stream patterns + values 14-20 assigned to full remote invoke lifecycle (§9.6.2) + value 21 `ParallelRegionDone` for distributed parallel-final barrier (§16.5). Discriminator overloading (e.g., reusing a wire value with a subject suffix) is **forbidden** — every semantic pattern has its own wire value. | Protobuf field-number convention; prevents wire breakage on future additions. Explicit wire values keep envelope dispatch a pure pattern-switch. |
| Wire encoding | **2-layer**: envelope = CBOR (RFC 8949), payload = per-event codec (`json` / `cbor` / `typed` / `raw`) | CloudEvents v1.0 precedent; default safe & debuggable, opt-in zero-overhead path for game/MMORPG workloads |
| Envelope field naming | Aligned with CloudEvents v1.0 (`id`, `source`, `type`, `subject`, `datacontenttype`, `data`) | Familiar to external integrators; future CloudEvents binding compatibility |
| Correlation ID | UUID v7 (RFC 9562, 16 bytes); v4 fallback acceptable when language lib lacks v7 | Distributed uniqueness for dynamic peers (zenoh peer mode, MMORPG clients, federated meshes) + monotonic ms-prefix for log ordering |
| **SCXML purity (Session E1 revision)** | **SCXML remains 100% W3C-standard except the single type value `<invoke type="sce:mesh-rpc">`.** All distribution-specific concerns (pattern classification, reply correlation, subscription scope, QoS) are either *inferred from topology at build time* or *declared in deploy.yaml*. The same SCXML runs in single-process, vehicle, IntraECU, and MMORPG deployments without modification. | "Same SCXML, three domains" is a core principle (§1). `sce:pattern`, `sce:reply-event`, `sce:reply-timeout`, `sce:qos` shipped in Sessions C-D are **deprecated in Session E1** — they leaked deployment concerns into state specification. |
| RPC model | **`<invoke type="sce:mesh-rpc">` is the sole RPC API.** W3C §6.4 provides the full lifecycle (`done.invoke.ID`, `error.invoke.ID`, `<cancel>`); `invoke_id` is the correlation token. The `<send sce:reply-event=...>` shortcut from Session C is deprecated — equivalent topology is now inferred from request/response event-name pairing. Blocking RPC rejected (RTC violation). | One blessed path eliminates dual documentation and migration ambiguity. |
| Pattern classification | **Inferred at build time** from event-name convention (`service.request.*` / `service.response.*` / `event.notification.*` / `field.get.*` etc.) + optional deploy.yaml `patterns:` override map. No SCXML-level pattern annotation. | Codegen already analyzes topology (who sends, who receives, who handles). Pattern follows from structure; annotating is redundant. |
| Request/response pairing | **Inferred at build time** from structural analysis: machine M handles `service.request.X` and raises `service.response.X` → M is the RPC server for X; runtime captures inbound envelope and auto-attaches `correlation_id`/`reply_to` to paired outbound. | Mirrors single-process SCXML where "reply" doesn't exist as a concept — just events. |
| Transport-native IDs | **deploy.yaml references external infrastructure config files** (`someip_config: ./vsomeip.json`, `zenoh_config: ./zenoh.json5`). sce-build parses the external file at build time to resolve service/method/event names → IDs. Inline `service_id`/`method_id` in deploy.yaml are deprecated. | `vsomeip.json` in automotive is auto-generated from ARXML/Franca IDL by OEM tooling — deploy.yaml duplicating IDs creates a sync burden. Single source of truth wins. |
| Subscription lifecycle | Dual-path, **both topology-inferred**: (a) `<onentry><send event="event.subscribe.X"/></onentry>` — codegen auto-emits `EventUnsubscribe` at the matching `<onexit>` (no author boilerplate); (b) deploy.yaml `subscriptions:` list for always-on machine-lifetime bindings | Author writes standard SCXML; lifecycle symmetry comes from codegen. |

#### MeshEnvelope schema (final)

CloudEvents-aligned field names. Wire form is canonical CBOR (RFC 8949 §4.2.1) with integer keys for size; alternate JSON form available for diagnostics.

```cpp
namespace SCE::Mesh {

/// Wire-stable pattern discriminator. Values are immutable once shipped.
/// Range 1-9 base patterns; 10-13 reserved for future Stream patterns
/// (wire-layer snapshot + delta optimization on EventSubscribe /
/// EventNotification, §8.1); 14-20 full remote invoke lifecycle
/// (§9.6.2); 21 distributed parallel-final barrier (§16.5).
/// Adding a variant requires a new wire value, never reuse.
enum class PatternKind : uint16_t {
    FireForget         = 1,
    RpcRequest         = 2,
    RpcReply           = 3,   // success or error — see envelope.rpc_status
    EventSubscribe     = 4,
    EventUnsubscribe   = 5,
    EventNotify        = 6,
    FieldRead          = 7,
    FieldWrite         = 8,
    FieldNotify        = 9,
    // 10-13 RESERVED for Stream* (wire-layer snapshot+delta optimization, §8.1) — DO NOT REASSIGN
    // 14-20 RESERVED for full remote invoke lifecycle (§9.6.2, Session F).
    //       Enum variants are declared NOW (Session E1) so wire values are
    //       pinned before any Session E-or-later session can accidentally
    //       grab them for an unrelated control pattern. Variants parse as
    //       valid envelopes in E1/E2 but trigger error.execution with
    //       SESSION_F_NOT_IMPLEMENTED if processed at runtime until F lands.
    //       DO NOT REASSIGN any of 14-20 for any other purpose.
    InvokeStart        = 14,
    InvokeStarted      = 15,
    ChildEvent         = 16,
    ParentEvent        = 17,
    InvokeDone         = 18,
    InvokeCancel       = 19,
    InvokeError        = 20,
    // 21 Distributed parallel-final barrier (§16.5, Session E2).
    ParallelRegionDone = 21,
};

/// Payload codec discriminator. Wire-stable; do not reuse values.
enum class PayloadCodec : uint8_t {
    None    = 0,   // payload absent (FireForget control messages)
    Json    = 1,   // default; W3C SCXML 5.10 _event.data
    Cbor    = 2,   // structured, smaller than JSON
    Typed   = 3,   // codegen-emitted binary using event schema
    Raw     = 4,   // user-supplied encoder (escape hatch)
};

/// gRPC-style status for RpcReply. Success = Ok, all others are errors.
enum class RpcStatus : uint8_t {
    Ok                 = 0,
    Cancelled          = 1,
    InvalidArgument    = 3,
    NotFound           = 5,
    Unavailable        = 14,
    Unimplemented      = 12,
    DeadlineExceeded   = 4,
    Internal           = 13,
};

/// Cross-transport message envelope. CloudEvents v1.0 field naming where
/// applicable. Optional fields are CBOR-omitted when absent for size.
struct MeshEnvelope {
    // ── Required (CloudEvents core) ──
    std::array<uint8_t, 16> id;       // CE 'id' — UUID v7 (v4 fallback)
    std::string source;               // CE 'source' — sender machine name
    std::string type;                 // CE 'type' — SCXML event name
    PatternKind pattern;              // SCE extension; pattern discriminator
    PayloadCodec datacontenttype;     // CE 'datacontenttype' — payload codec
    std::vector<uint8_t> data;        // CE 'data' — payload bytes (codec-encoded)

    // ── Optional ──
    std::optional<std::string> subject;             // CE 'subject' — interaction key (someip method_id, zenoh key)
    std::optional<std::array<uint8_t, 16>> correlation_id;  // RPC req↔resp matching (UUID v7)
    std::optional<std::string> reply_to;            // CE-extension; response routing endpoint
    std::optional<std::array<uint8_t, 16>> invoke_id;  // RESERVED for <invoke type="sce:mesh-rpc"> lifecycle
    std::optional<RpcStatus> rpc_status;            // RpcReply only; absent = Ok
    std::optional<std::string> rpc_error_message;   // RpcReply non-Ok detail
    std::optional<uint64_t> deadline_unix_ms;       // RPC timeout absolute
    QosHints qos;                                   // delivery hints (best_effort, reliable, ordered)
};

}  // namespace SCE::Mesh
```

CBOR map integer keys (wire-stable):

| Key | Field | Required | Notes |
|---|---|---|---|
| 0 | id | yes | 16 bytes |
| 1 | source | yes | string |
| 2 | type | yes | string |
| 3 | pattern | yes | uint16 |
| 4 | datacontenttype | yes | uint8 |
| 5 | data | yes | bstr (may be empty) |
| 6 | subject | no | string |
| 7 | correlation_id | no | 16 bytes |
| 8 | reply_to | no | string |
| 9 | invoke_id | no | 16 bytes — reserved for `<invoke>` RPC |
| 10 | rpc_status | no | uint8 |
| 11 | rpc_error_message | no | string |
| 12 | deadline_unix_ms | no | uint64 |
| 13 | qos | no | nested map |
| 14 | sequence_no | no | uint64 — per-(source, target) monotonic; §10.6.3 |
| 15 | routing_id | no | 16 bytes — per-session routing UUID v7; §10.9 |
| 16 | parallel_id | no | string — §16.5 wire-21 region routing; set together with key 17 |
| 17 | region_id | no | string — §16.5 wire-21 region routing; set together with key 16 |
| 18 | child_session_id | no | string — §9.6.2 wire-15/16/18 child session URI (§9.6.1 L1410 `<D_P>:<P.machine>:<invoke_id>`) |

Unknown integer keys MUST be skipped by readers. New fields use unused integers; never reuse.

**Low-overhead path (game/MMORPG)**: a deploy.yaml binding may declare `codec: typed` + `correlation: none` + omit `subject`/`reply_to`. The resulting envelope serializes to ~12-18 bytes (id + source + type + pattern + codec + data length + payload), comparable to bespoke binary protocols while remaining within CBOR.

#### Pattern dispatcher

`TransportRouter` gains `wireReceiver(engine)` symmetric to existing `wireTo`/`wireSender`. Incoming envelopes dispatch on `pattern`:

```cpp
template<typename Engine>
class TransportRouter {
    void wireSender(Engine&);
    void wireReceiver(Engine&);

    void onIncoming(MeshEnvelope env, Engine& engine) {
        switch (env.pattern) {
            case PatternKind::FireForget:       handleFireForget(env, engine); break;
            case PatternKind::RpcRequest:       handleRpcRequest(env, engine); break;
            case PatternKind::RpcReply:         handleRpcReply(env, engine); break;
            case PatternKind::EventSubscribe:   handleSubscribe(env, engine); break;
            case PatternKind::EventUnsubscribe: handleUnsubscribe(env, engine); break;
            case PatternKind::EventNotify:      handleNotify(env, engine); break;
            case PatternKind::FieldRead:        handleFieldRead(env, engine); break;
            case PatternKind::FieldWrite:       handleFieldWrite(env, engine); break;
            case PatternKind::FieldNotify:      handleFieldNotify(env, engine); break;
        }
    }

    // RPC correlation table — keyed by invoke_id (<invoke type="sce:mesh-rpc">)
    // or correlation_id (topology-inferred response path).
    struct PendingRpc {
        std::string paired_response_event;          // SCXML event name to deliver on reply
        std::optional<std::array<uint8_t,16>> invoke_id;  // present for <invoke> path
        std::chrono::steady_clock::time_point deadline;
    };
    std::unordered_map<UuidKey, PendingRpc> pending_;

    // Subscription registry — keyed by (source, type)
    struct Subscription {
        SubscriptionScope scope;  // StateEntry | MachineLifetime
        std::string owning_state; // for StateEntry scope
    };
    std::unordered_map<SubscriptionKey, Subscription> subs_;
};
```

Per-transport native API mapping (codegen target):

| Pattern | SOME/IP (vsomeip) | Zenoh |
|---|---|---|
| RpcRequest (sender) | `create_request` + `send` | `session.get(key, query)` |
| RpcRequest (receiver) | `register_message_handler(REQUEST)` | `declare_queryable(key)` |
| RpcReply | `vsomeip::message::create_response` + `send` | `Query::reply(sample)` |
| EventNotify (publisher) | `offer_event` + `notify` | `declare_publisher` + `put` |
| EventSubscribe | `request_event` + `subscribe_eventgroup` | `declare_subscriber(key, callback)` |
| EventUnsubscribe | `unsubscribe_eventgroup` | drop subscriber handle |
| FieldRead | field getter via `register_message_handler` | `session.get(key)` |
| FieldWrite | field setter | `session.put(key, value)` |
| FieldNotify | field notifier (`notify` to subscribers) | `declare_publisher` on field key |

#### SCXML authoring model (path B: purity)

SCXML documents in SCE Mesh are **standard W3C SCXML 1.0** with exactly one extension point: the invoke type value `sce:mesh-rpc`. No `sce:*` attributes. No new elements. The document is portable to any conforming SCXML 1.0 processor (where it degrades gracefully — see compatibility analysis).

**RPC request (`<invoke type="sce:mesh-rpc">`)**

```xml
<state id="braking">
  <invoke type="sce:mesh-rpc" src="#brake_service" id="compute_force_inv">
    <param name="_mesh_event"       expr="'service.request.compute_force'"/>
    <param name="_mesh_deadline_ms" expr="100"/>
    <param name="velocity"          expr="v"/>
  </invoke>
  <transition event="done.invoke.compute_force_inv"  target="apply_brake"/>
  <transition event="error.invoke.compute_force_inv" target="brake_failed"/>
  <onexit>
    <cancel sendid="compute_force_inv"/>
  </onexit>
</state>
```

- **W3C §6.4.1 compliant**: `done.invoke.ID` / `error.invoke.ID` / `<cancel>` are all standard.
- Result delivered via `_event.data` per spec.
- `invoke_id` field in the envelope (CBOR key 9) carries the SCXML invoke ID for correlation across the wire.
- `<cancel>` emits a `RpcStatus::Cancelled` envelope to the remote peer.
- Reserved `<param>` names (stripped from payload, used as codegen metadata): `_mesh_event` (required, the SCXML event name), `_mesh_deadline_ms` (optional, request timeout). All other `<param>`s form the request payload. Shadowing a reserved name with a business payload is a build-time hard error (see §9.5 "Reserved-name conflict").

**RPC response (plain SCXML, no annotations)**

The server-side machine is standard SCXML:

```xml
<state id="serving">
  <transition event="service.request.compute_force" target="computing">
    <assign location="v" expr="_event.data.velocity"/>
  </transition>
</state>
<state id="computing">
  <onentry>
    <raise event="service.response.compute_force">
      <param name="force" expr="v * mass"/>
    </raise>
  </onentry>
</state>
```

Build-time inference (in `sce-build` topology analyzer):
1. Machine B has a `<transition>` on `service.request.X` → B is a candidate RPC server for X.
2. Within B's reachable control flow from that transition, B `<raise>`s `service.response.X` → confirmed server; `(request X, response X)` is a paired tuple.
3. Codegen emits the transport-native server primitive (`declare_queryable` for Zenoh, `register_message_handler` for SOME/IP).
4. Runtime captures the inbound envelope on receive (`correlation_id`, `reply_to` stashed in a per-transition slot). When the paired response `<raise>` fires, the transport layer auto-attaches the captured correlation and routes back via `reply_to`.

The inference is **static** — no SCXML annotation, no `#_reply` virtual target, no `sce:reply-to`. If the name convention is insufficient (e.g., response event name differs), deploy.yaml supplies an explicit pairing override (see `patterns:` in §14).

**Subscription lifecycle — state-entry path (plain SCXML)**

```xml
<state id="monitoring_brake">
  <onentry><send target="#bus" event="event.subscribe.brake_status"/></onentry>
  <transition event="event.notification.brake_status" target="..."/>
</state>
```

No explicit `<onexit>` unsubscribe is required in the simple case.

**Auto-symmetry eligibility rules** (build-time analyzer):

An `<onentry>` `<send event="event.subscribe.X">` qualifies for automatic `<onexit>` unsubscribe generation **iff all** of the following hold:

1. **Unconditional direct child**: the `<send>` is a direct child of `<onentry>` — not nested inside `<if>`, `<foreach>`, or any other conditional/iterative executable content.
2. **Non-iterative**: the `<onentry>` is not the body of a parallel-region fork that the same state can re-enter concurrently. (History re-entry is not a problem — see §3 below.)
3. **No manual unsubscribe present**: the state's `<onexit>` does not already contain a `<send event="event.unsubscribe.X">` for the same X. An explicit manual unsubscribe takes precedence and suppresses auto-generation.

If any condition fails, auto-generation is suppressed and the analyzer emits a **lint notice** directing the author to write an explicit `<onexit>` unsubscribe. The notice is not a build error — the document still compiles — but subscription lifecycle becomes the author's responsibility.

**Edge case semantics** (runtime, regardless of auto-generation):

- **Conditional subscribe**: `<if cond><send event="event.subscribe.X"/></if>` inside `<onentry>`. No auto-unsubscribe is generated. Author must match with an explicit conditional unsubscribe in `<onexit>`. The mesh runtime maintains a **per-(machine, event)** subscription refcount (§10.5 dedup layer is separate); an `unsubscribe` when refcount is zero is a silent no-op, not an error.
- **History re-entry**: when a state with an auto-symmetric subscribe is re-entered via `<history>`, standard W3C semantics re-run `<onentry>` on re-entry, which re-issues `event.subscribe.X`. The runtime refcount treats this as a **fresh subscription**; the prior exit had already issued `event.unsubscribe.X`, so refcount returns to 1.
- **Parallel region subscribe**: a subscribe inside one `<parallel>` region lives for that region's active lifetime. Auto-unsubscribe fires on region exit (which includes the enclosing parallel's exit). If two sibling regions both subscribe to the same X, each has an independent subscribe/unsubscribe pair; refcount reaches 2 while both are active and decrements to 0 when both exit.
- **Duplicate subscribe in the same `<onentry>`**: structurally redundant; the analyzer emits a warning. Runtime refcount still tracks each pair correctly.
- **Subscribe without matching unsubscribe**: if analysis determines that a subscribe's state may be exited via a transition path that skips the state's own `<onexit>` (impossible under W3C normal exit-set semantics, but trivially possible for cross-region transitions under permissive mode §16.4 after auto-merge), the analyzer emits a lint. Auto-generated unsubscribes always fire on normal exit-set computation; paths that skip `<onexit>` (there are none in standard SCXML) would leak subscriptions — but W3C guarantees this cannot happen, so the concern is theoretical.

Author opts into automatic symmetry by writing the subscribe in a qualifying position. Explicit manual unsubscribe is always accepted and takes precedence. Mixing is fine — an author may auto-symmetry one state and hand-write another in the same machine.

**Subscription lifecycle — machine-lifetime path (deploy.yaml only)**

```yaml
machines:
  ecu_brake:
    transport: someip
    subscriptions:
      - event: event.notification.vehicle_speed
        source: "#chassis"
```

Codegen emits subscribe on engine init, unsubscribe on engine shutdown. SCXML document is not touched.

**Choosing between the two paths**

| Concern | State-entry (plain SCXML) | Machine-lifetime (deploy.yaml) |
|---|---|---|
| Subscription scope | Per-state — auto-unsubscribe at exit | Per-router — subscribe at init, unsubscribe at shutdown |
| Events delivered during state re-entry cycle | **Dropped** during the exit → re-entry window (subscriber undeclared at exit, redeclared on next entry) | **Delivered** — subscription spans the router's entire lifetime. The build-time resolver synthesises an implicit target per `subscriptions.source`, so the subscribe envelope dispatched at `init()` reaches the transport through the same `route_send` path SCXML-driven sends use. |
| Author surface | Standard SCXML `<send event="event.subscribe.X"/>` in `<onentry>` | No SCXML change; deploy.yaml `machines.<name>.subscriptions:` |
| Bus cost | Subscription present only while the owning state is active | Subscription present for the router's entire lifetime |
| Use when | Interest is genuinely scoped to a state — e.g. monitoring state subscribes to a diagnostic event, closing it on exit frees peer fan-out | Event delivery must not drop across state changes — e.g. vehicle-speed telemetry feeding multiple states concurrently |

The state-entry path's re-entry window is intrinsic to its semantics: the synthetic `event.unsubscribe.X` on exit is a real transport-level undeclare, and the re-entry's `event.subscribe.X` is a real redeclare. Between the two, no subscriber is active on the transport. This is transport-behaviour-as-declared, not a transport defect — authors who need at-least-once delivery across state transitions should pick the machine-lifetime path.

Mixing is supported by design: one machine may declare both a machine-lifetime subscription (continuous telemetry) and a state-entry subscribe (scoped interest) for different events on the same binding. Both paths compose through the same subscription refcount (SCE_MESH.md §13 refcount) — a duplicate subscribe on the same `(target, event)` is a refcount no-op, and the state-entry unsubscribe refs down only its own contribution.

#### External infrastructure config integration

Industrial deployments already carry native transport configuration, often auto-generated from OEM tooling:

- **SOME/IP**: `vsomeip.json` — service/instance/event-group/method IDs, routing manager, security policies. Typically generated from ARXML (AUTOSAR XML) or Franca IDL.
- **Zenoh**: `zenoh.json5` — session mode, endpoints, scouting config, ACL.
- **DDS** (Phase 4): `rti_connext.xml` or Cyclone DDS XML profiles.

SCE Mesh **does not duplicate** these. deploy.yaml references the external file by path and resolves names → IDs at build time:

```yaml
topology:
  brake_ecu:
    transports:
      someip:
        config: ./config/vsomeip.json        # external file, single source of truth
        application_name: brake_app          # matches vsomeip.json applications[*].name
      zenoh:
        config: ./config/zenoh.json5         # external file
    machines:
      brake:
        source: brake.scxml
        bindings:
          "#motor":
            transport: someip
            service: motor_control           # resolved against vsomeip.json → service_id
            events:
              service.request.compute_force:
                method: compute_force         # resolved → method_id
              event.notification.status:
                event_group: status_group     # resolved → event_group_id + event_id
```

Build-time resolution:
1. `sce-build` parses `vsomeip.json` at generation time using a **minimal partial-schema serde model** — only the fields SCE Mesh consumes are declared (see table below); all other fields are tolerated (no `deny_unknown_fields`) because vsomeip.json is owned by the platform team / OEM tooling, not by SCE.
2. For each named entity in deploy.yaml (`service: motor_control`), it looks up the matching entry and embeds the numeric ID in generated code.
3. Unresolved names → hard build failure (never silent). This replaces the runtime misconfigurations that Session C/D's inline `service_id: 0x1234` style would only catch at bus contact.

Minimum vsomeip.json fields consumed by sce-build:

| deploy.yaml reference | vsomeip.json path | Used for |
|---|---|---|
| `service: <name>` | `services[*]` where `services[*].name == <name>` (vsomeip-standard name field) | Resolves `service_id`, `instance_id` |
| `method: <name>` | `services[*].methods[*]` where matching `name` | Resolves `method_id` |
| `event_group: <name>` | `services[*].eventgroups[*]` where matching `name` | Resolves `event_group_id` and the contained event id |
| `getter: <name>` / `setter: <name>` | `services[*].methods[*]` (field accessors are methods in SOME/IP) | Resolves the getter/setter `method_id` |
| `application_name: <name>` under `topology.<device>.transports.someip` | `applications[*]` with matching `name` | Binds the generated runtime to a vsomeip application identity |

Unresolved names produce a single consolidated error per build:
```
error: deploy.yaml references SOME/IP entities that do not exist in
       ./config/brake_ecu/vsomeip.json:
         - service "motor_control"     → no services[*] with name == "motor_control"
         - method "apply_force"        → no methods[*] with name == "apply_force"
                                         in service "brake_control"
```

The minimum-schema approach keeps SCE's dependency on vsomeip surface small: any vsomeip configuration feature not listed here (routing manager trace filters, security policies, service-group mappings, etc.) passes through untouched to vsomeip at runtime. SCE never rewrites the file.

**Per-event `service:` — not supported.** A binding's `service:` reference is binding-level only; the per-event `events:` entries carry `method` / `event_group` / `getter` / `setter` but **no** `service:`. A target identity (`#motor`) resolves to exactly one SOME/IP `{service_id, instance_id}` pair, so "this event on the same target actually routes to a different service" would require a second logical target — declare it as a separate binding (`#motor_ctrl` vs. `#motor_diag`) and `<send>` to the appropriate target, rather than overloading per-event `service:`. sce-build rejects `service:` inside an `events:` entry as an unknown field.

Inline numeric IDs in deploy.yaml remain supported for incremental migration of existing Session C/D test fixtures but are **deprecated** and will be removed after migration in Session E1.

**Build-time consistency check (3-way)**

Every named event traverses three declaration points. They must agree:

1. **SCXML**: `<send event="service.request.compute_force"/>` / `<transition event="...">` — defines what the state machine expects.
2. **deploy.yaml**: `events.service.request.compute_force.method: compute_force` — maps the SCXML event name to an external entity name.
3. **vsomeip.json**: `services[X].methods[].name == "compute_force"` — defines the numeric ID.

sce-build enforces all three alignments and fails the build on any gap:
- SCXML event used in `<send>` with no deploy.yaml binding → build failure.
- deploy.yaml name `compute_force` absent from `vsomeip.json` → build failure.
- Request/response pair inferred by topology analyzer but one side missing → build failure.

#### W3C SCXML 1.0 compatibility analysis

The only SCXML extension is the invoke `type` value `sce:mesh-rpc` (W3C §6.4 allows implementation-defined type URIs). No extension attributes, no extension elements.

| Extension point | Standard? | Behavior in conforming SCXML 1.0 processor |
|---|---|---|
| `<invoke type="sce:mesh-rpc">` | W3C §6.4 — type is implementation-defined URI | Unknown type → `error.execution` raised per §6.4.1. Document author can catch via standard `<transition event="error.execution">`. Document itself remains parseable. |
| `<send target="#machine_id">` | W3C §6.2.4 — IO processor target URIs are implementation-defined | Unknown target → `error.execution`. Behavior is domain-standard. |
| Event names with dot-delimited conventions (`service.request.*`, `event.notification.*`, `field.get.*`) | W3C §5.10 — event names are arbitrary dot-delimited tokens | Plain event names. Conventions are for SCE tooling only; the processor sees ordinary strings. |
| Target `#_reply`, `#_bus`, etc. | *Not used — rejected in Session E1* | — |
| `sce:*` attributes | *Not used — removed in Session E1* | — |

**Conclusion**: A foreign SCXML 1.0 processor parses the document cleanly (XML is valid; no foreign namespaces except `sce:mesh-rpc` as a type value), and executes every construct except `<invoke type="sce:mesh-rpc">`, which fails locally with `error.execution` — a fault the author already handles for any unsupported invoke type per §6.4.1.

Degradation is natural and does not require porting work. A user-facing compatibility guide (`docs/MESH_SCXML_COMPATIBILITY.md`, delivered in Session E1) walks through this with examples.

#### Session C/D attribute deprecation (Session E1 cleanup)

Sessions C and D shipped these `sce:*` attributes on `<send>`. They are **removed in Session E1**. The removal went through a staged deterministic migration so in-tree fixtures could be migrated one by one:

**Stage 1 (Session E1 start, transitional)**: parser accepted the attribute, emitted one warning per occurrence (with file + line), and ignored the attribute value. Build succeeded. This let Session C/D test fixtures continue to compile during migration.

**Stage 2 (Session E1 end, current state — `sce-build` HEAD)**: parser **rejects** the attribute as a hard error (`DiagnosticCode::ValidationRemovedAttribute`, `validation/removed-attribute`). Build fails with a diagnostic pointing to this deprecation table. Third-party documents carrying the attributes must migrate before building against Session E1 or later.

§5 QoS summary, §14 deploy.yaml schema, and this table all reference the same migration. The Stage 1 warning infrastructure has been removed from `sce-build`; only Stage 2 hard-error enforcement remains. Authors targeting Session E1 or later should read this table as authoritative.

| Shipped attribute | Removed in | Replacement |
|---|---|---|
| `sce:pattern="request"` | Session E1 | Topology analyzer infers pattern from event-name convention + optional deploy.yaml `patterns:` override. |
| `sce:reply-event="..."` | Session E1 | Topology analyzer pairs `service.request.X` ↔ `service.response.X` structurally. For cross-name pairing, deploy.yaml `patterns:` declares the mapping. Primary RPC path is `<invoke type="sce:mesh-rpc">`. |
| `sce:reply-timeout="500"` | Session E1 | `<invoke>` `<param name="_mesh_deadline_ms">` (textbook W3C path, see §9.5) or deploy.yaml per-binding deadline. |
| `sce:qos="reliable"` | Session E1 | deploy.yaml transport-native QoS (§5). |
| `sce:deadline`, `sce:priority` | Session E1 | deploy.yaml transport-native QoS. |

Session C/D test fixtures are migrated to the inferred/deploy.yaml-only form as part of Session E1.

#### Migration plan (existing FireForget E2E acid test)

Current FireForget tests that MUST continue to pass throughout Phase 3.5:

| Test | Current wire | Migration |
|---|---|---|
| `tests/mesh/test_mesh_local.cpp` | `MeshEnvelope` via `dispatchEnvelope` | DONE — `route_send` local uses shared dispatch helper |
| `tests/mesh/test_mesh_shm_runtime.cpp` | CBOR envelope over ShmChannel | DONE — `channel.send(env)` / `drain()` with internal encode/decode |
| `tests/mesh/test_mesh_shm_payload_runtime.cpp` | CBOR envelope + JSON payload | DONE — `datacontenttype=Json`, `_event.data.force == 42` guard passes |
| `tests/mesh/test_mesh_someip_runtime.cpp` (compile-only) | CBOR envelope in vsomeip payload | DONE — `encodeEnvelope(env)` in generated send function |

Migration acid test for Session B completion:
1. All 4 tests above pass with envelope-on-wire (FireForget path through dispatcher).
2. Non-FireForget patterns return `RpcStatus::Unimplemented` envelopes (stub).
3. W3C 404 conformance suite shows zero regressions.

#### Entry sequence (multi-session)

Sessions B-E execute the design above. Each session is a working unit; estimates are not commitments.

| Session | Scope | Acid test |
|---|---|---|
| **A (this)** | Design + sign-off (this section) | User approval of all 5 axes + envelope schema |
| **B (DONE)** | Replace `MeshSendRequest` with `MeshEnvelope`; CBOR codec (`MeshEnvelopeCodec`); shared dispatch helper (`MeshDispatch.h`); ShmChannel symmetric send/drain; UUID v7 (`sce::uuid`); `MeshSendRequest.h`/`MeshWireFormat.h` deleted | All 4 FireForget E2E tests pass on envelope wire; W3C zero regression; `grep -r MeshSendRequest` → 0 code matches |
| **C (DONE)** | SOME/IP multi-pattern: pattern-branching send (`send_to_X` switches on `PatternKind`), RPC correlation table (`pending_rpcs_` with mutex), `register_message_handler` for RPC response + event notify + field notify, `sce:reply-event` SCXML attribute parsed + threaded to codegen, `resolvePattern()` build-time event→PatternKind lookup in `wireTo()`, `MeshDispatch.h` handles RpcReply/EventNotify/FieldNotify inbound patterns, deploy.yaml extended with `event_group_id`/`event_id`/`getter_id`/`setter_id`. 32/32 tests GREEN. | Existing `mesh_someip_compile_test` + all 32 ctest pass |
| **D (DONE)** | Zenoh multi-pattern client-side realization: `send_zenoh` TransportRouter member dispatches on `PatternKind` (put for FireForget/FieldWrite; `session.get` with on_reply closure for RpcRequest/FieldRead; `declare_subscriber` + `zenoh_subscribers_` RAII handle map for EventSubscribe/EventUnsubscribe). Native correlation replaces `pending_rpcs_` — zenoh runtime delivers replies to the capturing closure. `find_package(zenohc)` precedes `find_package(zenohcxx)` to resolve the `zenohcxx::zenohc` target. ZENOH capability descriptor adds `RequestReply`. Multi-pattern compile-only + peer-mode runtime E2E (TCP locator discovery, no daemon) validate all five switch branches. **Post-review textbook refactor**: `wireTo()` removed entirely; `TransportRouter<SenderEngine, LocalEngines...>` takes the sender in its ctor, stores a `SenderEngine& sender_` const reference, installs `setMeshSendCallback` in the ctor body. Replaces type-erased mutable `receive_callback_` with direct `dispatchToSender()` — race on reassignment is structurally impossible. All call sites migrated; no legacy stub. Pattern-capability negative test restored (shm+RPC replaces the deleted zenoh case). | Multi-pattern peer-mode E2E without daemon: `mesh_zenoh_multipattern_runtime` 1/1 PASS (FireForget, RpcRequest round-trip with `sce:reply-event` rewrite, EventSubscribe → EventNotify, FieldRead, EventUnsubscribe handle drop). 35/35 ctest + 369/369 cargo test GREEN. |
| **E1 (path C, in progress)** | **SCXML purity + mesh-rpc correction**. Architectural correction: remove `sce:pattern`/`sce:reply-event`/`sce:reply-timeout`/`sce:qos`/`sce:deadline`/`sce:priority`; migrate Session C/D test fixtures. External config integration: deploy.yaml references `vsomeip.json` / `zenoh.json5`; `sce-build` parses them at build time. 3-way consistency check. Topology-inferred request↔response pairing. `<invoke type="sce:mesh-rpc">` full lifecycle (§9.5). Subscription dual-lifecycle (state-entry auto-symmetry + deploy.yaml machine-lifetime). **W3C compat doc**: `docs/MESH_SCXML_COMPATIBILITY.md`. No partition/distribution machinery yet. | (1) All Session C/D tests migrated, zero `sce:*` attributes remaining on `<send>`; (2) `<invoke type="sce:mesh-rpc">` integration test passes on zenoh or someip; (3) 3-way consistency check rejects a seeded misnamed method at build time; (4) reserved-name conflict rule rejects a seeded `<param name="_mesh_event">` collision at build time; (5) `docs/MESH_SCXML_COMPATIBILITY.md` landed; (6) 35+ ctest + 369+ cargo test green, zero regressions. |
| **E2 (path C, next)** | **Distributed conformance foundation**. Transport Contract (§10.4); `custom_tcp` reference transport; mesh runtime dedup layer (§10.5); `_event` field wiring (§10.7) including structured `error.*` data (§10.7.1); sender-hold delayed send + cancel (§10.8); `partitions:` schema in deploy.yaml with explicit coverage rule (§14); distributability analyzer + auto-merge (§16.3/16.4); parallel `<final>` barrier runtime + `ParallelRegionDone` wire pattern 21 (§16.5); `error.communication` catalog (§16.7); IRP distributed harness (§16.8) for the `<parallel>`-only distributable subset. | (1) `custom_tcp` transport passes a minimal FireForget E2E; (2) dedup layer suppresses seeded duplicate envelopes; (3) sender-hold cancel test: cancel before emit_at leaves zero wire traffic; (4) Distributed IRP harness runs the `<parallel>`-only `distributable: yes` subset with identical verdicts to single-process; (5) Distributability analyzer correctly flags R1/R2 violations in seeded test documents; (6) `merged_single_partition` label appears for seeded shared-write test; (7) Parallel `<final>` barrier test: N region partitions each reach `<final>` → root raises `done.state.PAR` at correct macrostep boundary; (8) zero regressions of E1 acid tests. |
| **F (path C, after E2)** | **Full remote `<invoke type="scxml">` + complete IRP distributed coverage**. Wire patterns InvokeStart/InvokeStarted/ChildEvent/ParentEvent/InvokeDone/InvokeCancel/InvokeError (§9.6.2). `_event.invokeid`/`origin` wiring for child events (§9.6.3). `<finalize>` at parent's macrostep on child events (§9.6.4). `autoforward="true"` parent→child forwarding (§9.6.5). Inline `<content>` precompilation — synthesize `<parent>__sce_synth_invoke__<id>` machines (§9.6.6), with collision detection. Extend IRP distributed manifest to cover `<invoke type="scxml">`-using tests. Foreign processor compatibility harness (graceful `error.execution` validation with an external SCXML 1.0 reference interpreter). Mesh Conformance Suite: distributed-only tests exercising §10.4/10.5/10.8/16.5/16.7 edge cases not covered by W3C IRP. | (1) All IRP tests classified `distributable: yes` pass in both single-process and distributed mode; (2) Remote `<invoke>` lifecycle test across processes (done/error/cancel) (**LANDED** — done: `mesh_session_f_wire_roundtrip` / tests/mesh/test_mesh_session_f_wire_roundtrip.cpp drives the wire-14 `InvokeStart` → wire-15 `InvokeStarted` → wire-18 `InvokeDone` round-trip, parent observes `done.invoke.remote_inv`; error: `mesh_session_f_not_implemented_verification` / tests/mesh/test_mesh_session_f_not_implemented.cpp exercises the §9.6 L1393 local-raise path — `performScxmlInvokeStart` returns false absent a peer transport binding and the parent raises `error.execution` with `SESSION_F_TRANSPORT_UNAVAILABLE` without wire traffic, wire-20 envelope round-trip itself is scoped to a future cross-device Session F continuation per the same spec paragraph; cancel: `mesh_session_f_cancel` / tests/mesh/test_mesh_session_f_cancel.cpp drives `<transition target="cancelled">` whose `onexit` emits wire-19 `InvokeCancel`, worker `WorkerSessionHost::onWire19` invokes `adapter->cancel()` and erases the session, parent reaches `cancelled` without observing `error.execution`; cross-device custom_tcp (Session 3 Stage B, 2026-04-24): `mesh_session_f_crossdev_lifecycle` / tests/mesh/test_mesh_session_f_crossdev_lifecycle_parent.cpp + _worker.cpp drives the same wire-14 → wire-15 → wire-18 path across a two-process two-ecu boundary — worker announces its kernel-ephemeral `Server::local_endpoint()` via the `LISTEN_ENDPOINT=` orchestrator handshake, parent threads it into `TransportRouter::init(PortOverride)` (first router-level consumer of the Session 2b plumbing), reaching `State::Pass` via `done.invoke.*`; single-process someip (Session 4b, 2026-04-25): `mesh_someip_scxml_invoke_roundtrip` / tests/mesh/test_mesh_someip_scxml_invoke_roundtrip.cpp drives wire-14 → wire-15 → wire-18 over vsomeip's internal routing (SD disabled, parent's `<machine>_scxml_invoke_app_` as the nominated routing manager per vsomeip_scxml_invoke.json) — each TransportRouter instantiates a dedicated `<machine>_scxml_invoke_app_` distinct from any per-`<send>`-target application so the SCE-reserved 0x8100..0x81FF service range never shares an OEM vsomeip.json application (§13 boundary); single-process zenoh (Session 5, 2026-04-25): `mesh_zenoh_scxml_invoke_roundtrip` / tests/mesh/test_mesh_zenoh_scxml_invoke_roundtrip.cpp drives the same wire-14 → wire-15 → wire-18 path over Zenoh peer-mesh routing — both routers share the device-wide `zenoh_session_` (Zenoh has no §13 OEM boundary equivalent, so the SCE-reserved §9.6 namespace is carved out via the `sce/scxml_invoke/` key-expression prefix instead of session identity), each ScxmlInvokeEndpoint declares its Publisher with `Z_CONGESTION_CONTROL_BLOCK + Z_PRIORITY_DATA` so wire-14/18 cannot silently drop on a slow subscriber, and the test fixture's relay session (anchoring peer-mesh convergence at `tcp/127.0.0.1:17448` per `RESOURCE_LOCK zenoh_invoke_port_17448`) mirrors the `mesh_zenoh_runtime` motor↔brake handshake shape; two-host cross-device someip + zenoh (Session 5b, 2026-04-25): `mesh_someip_scxml_invoke_crossdev` and `mesh_zenoh_scxml_invoke_crossdev` (tests/mesh/test_mesh_{someip,zenoh}_scxml_invoke_crossdev_{parent,worker}.cpp) drive the same wire-14 → wire-15 → wire-18 round-trip across distinct Linux netns wired by veth — parent in sce-mesh-parent (172.16.10.1), worker in sce-mesh-worker (172.16.10.2), reaching the peer through SOME/IP-SD multicast (vsomeip_scxml_invoke_crossdev_{parent,worker}.json with `service-discovery.enable: true`) or Zenoh peer-mesh listen/connect (parent listens at tcp/172.16.10.1:17449, worker connects), gated on the new `SCE_ENABLE_NETNS_TESTS` CMake option with `SKIP_RETURN_CODE 77` so a non-root checkout reports them Skipped not Failed; donedata payload evidence: `mesh_session_f_crossdev_donedata` / tests/mesh/test_mesh_session_f_crossdev_donedata_parent.cpp + _worker.cpp scales this to the three `<donedata>` shapes (`<param>`, primitive `<content expr>`, nested object/array/integer) each with its own ephemeral port, parent seeds a three-entry `PortOverride::peer_connect_endpoints` map over the multi-peer `LISTEN_ENDPOINT_<peer>=` handshake, reaching Pass proves decoded `_event.data` equivalence to the shm baseline at the semantic level — byte-identical wire is structurally impossible across transports due to CBOR + length-prefix framing divergence); (3) `autoforward="true"` forwarding test (**LANDED** — `mesh_session_f_autoforward` / tests/mesh/test_mesh_session_f_autoforward.cpp; parent `<invoke autoforward="true">` + test driver `raiseExternal(Event::Trigger)` → `forwardToAutoforwardChildren` routes through `performScxmlInvokeParentEvent` which publishes wire-17 `ParentEvent` on `/sce_p2c_<parent>_<worker>` → worker `WorkerSessionHost::onWire17` calls `adapter->raiseExternal("trigger", data, "")` → child `<transition event="trigger" target="done">` fires → host tick observes `isFinal()` and emits wire-18 `InvokeDone` → parent `done.invoke.remote_inv` → `State::Pass`, closing the full §9.6.5 autoforward round-trip); (4) Inline `<content>` synthesized child executes on a different partition (**LANDED** — `mesh_synth_invoke_override_e2e` / tests/mesh/test_mesh_synth_invoke_override_e2e.cpp; parent parses inline `<content>` → parser materialises `parent_synth_inline__sce_synth_invoke__remote_inv.scxml` → `deploy_synth_inline.yaml` places parent and synth in distinct partitions → wire-14 `InvokeStart` / wire-15 `InvokeStarted` / wire-18 `InvokeDone` round-trip fires `done.invoke.remote_inv`); (5) Synthesized-name collision test is rejected at build time (**LANDED** — `mesh_partition_rule5_rejection` / tests/mesh/partition_rule5_synth_infix_collision.yaml; author-declared machine id `motor_partition__sce_synth_invoke__ghost` with no matching parent stem fires `mesh/deploy-partition-synth-infix-collision` via `validate_synth_invoke_infix` — the §9.6.6 rule 3 explicit-override carve-out remains gated on a sibling parent entry); (6) Mesh Conformance Suite 100% pass; (7) Zero single-process regressions. |

#### Out of Phase 3.5 scope

- Additional transports (DDS, gRPC, MQTT, UDP) — Phase 4
- Stream patterns (wire-layer snapshot+delta optimization on EventSubscribe/EventNotification per §8.1; wire values 10-13 reserved)
- Performance optimizations (drain allocation, Lua compile bypass, double serialization) — re-evaluated after envelope settles
- Middleware-level service discovery (SCE-maintained peer tables, IDiscovery trait, `runtime_targets_` map) — rejected; transport-native routing is not reimplemented (§3.3 invariant)
- Cross-transport automatic bridging codegen — rejected; bridging is explicit SCXML responsibility (§14.5)
- Runtime target selection via binding placeholders — landed in §14.4 (Phase 5)
- True blocking RPC (`sce:blocking="true"`) — rejected; revisit only if a concrete use case forces it

### Phase 4: Game Scale (Directional — refined after Phase 3)

High-throughput batch processing for massive entity counts.

- `GameLoopScheduler` with SoA (Structure of Arrays) layout
- `udp_transport` + `grpc_transport` templates
- Batch event processing (group by event type)
- Zero-allocation event pool (arena allocator)
- ECS integration API
- Delta state synchronization (changed-state-only client updates)
- **Codegen trade-off**: SoA layout requires a second codegen mode alongside the existing per-instance AoS generation. Evaluated based on Phase 3 benchmarks

### Phase 5: Runtime Target Selection (landed)

Runtime selection of a remote target *instance* within an already-declared binding. Scope kept deliberately narrow; see §3.3 for the design invariant (SCE does not reimplement transport discovery).

**Foundation (landed)**:
- **Binding value-field placeholder grammar** (§14.4) — `{name}` tokens in Zenoh `key:` and the `instance_from: <param-name>` binding field for SOME/IP are parsed, capability-gated, and substituted at runtime. Zenoh emits `zenoh::KeyExpr(std::string(...) + ...)` at the send site; SOME/IP emits a `request_service(SERVICE, i)` loop over `instances:` at init and validates runtime values against the list before `set_instance`. Placeholder substitution failures (SOME/IP instance out-of-set, missing placeholder name, malformed payload) are pre-envelope setup faults and raise `error.execution` with `reason=INVOKE_SRC_NOT_FOUND` per §10.7.1 — no wire envelope is emitted, so no `RpcStatus` applies (§9.5 three-tier error table).
- **Transport capability gating** — `TransportDescriptor::supports_pool` registry field; custom_tcp / shm / local / can do not support placeholders and a binding on them carrying a placeholder is rejected at parse time (`mesh/pool-not-supported-by-transport`).
- **`<invoke type="sce:mesh-rpc" srcexpr>`** (§9.5) — parser accepts `srcexpr` with the exactly-one rule against `src`, typed as the `MeshRpcTarget` sum type so "both empty" and "both set" are structurally impossible. `srcexpr` is evaluated at `<invoke>` entry through the datamodel; the resolved `#<name>` is looked up in static topology. A miss is a pre-envelope setup fault — `error.execution` with `reason=INVOKE_SRC_NOT_FOUND` per §10.7.1 (§9.5 three-tier error table). No retry, no wait.

**Rejected (explicitly, not deferred)**:
- Middleware-level service discovery (IDiscovery trait / `runtime_targets_` map / SCE-maintained peer tables) — transport-native routing is the source of truth (§3.3)
- Cross-transport automatic bridging codegen — bridging is explicit SCXML responsibility (§14.5)
- `discovery.mode: static | dynamic` deploy-level switch — runtime target selection is a per-binding property, not a deployment mode
- `PEER_NAME_COLLISION` error.communication row — SCE does not maintain a peer table, so collisions are not observable at the SCE layer
- Server-side pool (`server.instances: [...]`) — moved into Phase 6 landings; see §14.4 for the SOME/IP-only shape, the pool+mesh-rpc-client codegen reject, and the session-indexed dispatch SSoT.

**Deferred (may land in later phase after concrete motivation)**:
- External registries (Consul, etcd, mDNS) — the binding-placeholder surface does not preclude them but requires a separate adapter
- Priority-based cross-transport resolution (see §4.3 dynamic mode) — requires multi-transport binding per target

### Phase 6: Multi-session (Server-side instance pool)

Runtime support for a single SCE-generated process hosting N independent SCXML sessions, each representing one SOME/IP instance of a service. Scope is SCE-runtime-wide, not per-transport: the capability surfaces through the transport registry (`supports_multi_instance_server` flag), but the lifecycle / isolation / dispatch machinery lives in the C++ SCE runtime. Per `ARCHITECTURE.md` Principle 8, mesh is a C++-only capability; per-language expansion is case-by-case and gated on concrete demand, not a standing parity obligation.

**Motivation**. vsomeip fully supports one process offering multiple service instances (`routing_manager_impl.cpp` tracks per-(service, instance) state independently, handlers can dispatch by `msg->get_instance()`). SCE's earlier "one document = one state machine = one identity" runtime model was the constraint, not the transport. Phase 6 lifts that constraint for SOME/IP (and transports whose native routing provides peer-distinguishable inbound delivery) while preserving SCXML authoring unchanged: the same brake.scxml runs as one of N instances without source edits.

**Landed**:
- Deploy-time `server.instances: [...]` parse-accept on `supports_multi_instance_server: true` transports; other transports reject with `mesh/deploy-server-pool-not-supported`
- Codegen emission of `SOMEIP_SERVER_INSTANCES` as a fixed-size array, `N_SESSIONS` pinned to the list cardinality, `sessions_` pointer array replacing the legacy single `sender_` member as the dispatch SSoT (§10.9 invariant 7)
- `init()` loop: per-instance `offer_service` / `register_message_handler` / `offer_event` from one traversal of `SOMEIP_SERVER_INSTANCES`
- Inbound server callbacks (`rpc_pairs`, `fire_forget_events`, `field_access_pairs`) capture `server_instance`, resolve it to a session slot via `session_index_for_instance()`, and dispatch to `sessions_[session_idx]` through the existing admit layers (dedup, ordering, direct)
- Outbound response preservation — `create_response(original)` path in `handleServerResponse` unchanged
- Spontaneous notification — `publishEventgroupNotify(env, session_idx)` emits on the raising session's own offered instance (`SOMEIP_SERVER_INSTANCES[session_idx]`)
- Pool + any router-scoped RPC client (`<invoke type="sce:mesh-rpc">` or SOME/IP `<send>` RpcRequest) coexistence rejected at codegen with `mesh/codegen-pool-with-rpc-client-unsupported` (router-scoped correlation tables `invoke_correlation_` / `active_invokes_` / `pending_rpcs_` cannot safely alias across sessions — §10.9 invariant 8)

**Not in Phase 6 scope**:
- Author-visible session identity (`_event.session_id` or equivalent) — session_idx is internal; SCXML authors address peers through `machine_name` alone
- Cross-session SCXML communication primitives — a Phase 6 session is isolated; inter-session messaging still goes through `mesh-rpc` (same as inter-process today)
- Session hot-swap / live reload — Beyond-Phase-6 per the Future Direction list
- Multi-instance server on transports without peer-distinguishable inbound delivery (Zenoh's KeyExpr is not a peer identity) — the `supports_multi_instance_server` flag stays `false` for such transports
- Multi-router (cross-process) hosting of the same machine name — would require `(env.source, env.routing_id)` keying for §10.5 dedup / §10.6 ordering. §10.9 invariant 5 explains why pool topology preserves R2/R4/T3 correctness on `env.source` alone (router-scoped `seq_counter` + per-emit fresh UUID v7 envelope id + document-level liveliness aggregation); multi-router introduces distinct `routing_id`s under one source, which the current single-router pool does not. Not opened by current deploy.yaml grammar.

### Future Direction (Beyond Phase 6)

The following capabilities are explicitly deferred. They will be evaluated after Phase 1-6 are validated in at least one production domain:

- **Saga pattern** — distributed compensation transactions as a separate orchestration layer above SCE Mesh
- **Consistency modes** — strong/eventual consistency guarantees for cross-instance state
- **Formal verification** — model checking integration (e.g., TLA+, UPPAAL) for safety-critical certification
- **Hot reload** — runtime replacement of AOT state machines without service interruption

---

## 14. deploy.yaml Schema

Revised in Session E (path B, 2026-04-14): deploy.yaml declares **topology and references to external infrastructure config**. Transport-native IDs (SOME/IP service/method IDs, Zenoh session parameters) are never duplicated in deploy.yaml — they live in `vsomeip.json`/`zenoh.json5`/etc. and are resolved by name at build time.

```yaml
# SCE Mesh Deployment Descriptor
version: "1.0"

scheduler:
  type: event_driven | real_time | game_loop | cooperative
  cycle_ms: <integer>               # real_time, cooperative
  tick_rate: <integer>              # game_loop

topology:
  <device_name>:
    platform: linux | qnx | autosar | windows
    target: x86_64 | aarch64 | arm32

    # Device-shared transport sessions. Each transport may reference an external
    # infrastructure config file (single source of truth for IDs).
    transports:
      someip:
        config: <path to vsomeip.json>   # OEM-supplied / ARXML-generated
        application_name: <name>         # matches vsomeip.json applications[*].name
      zenoh:
        config: <path to zenoh.json5>    # Zenoh-native session config
        # Any deploy-level overrides (mode, connect, listen) merge over the file.

    machines:
      <scxml_name>:
        source: <path.scxml>
        bindings:
          "<target_id>":
            transport: someip | can | zenoh | shm | dds | dbus | grpc | ...
            # For named-entity transports, use NAMES that resolve against the
            # external config. Never inline numeric IDs.
            service: <name>              # someip: resolves to service_id + instance_id
            key: <zenoh key expr>        # zenoh: literal key (no external resolution needed)
            codec: json | cbor | typed | raw  # optional; default 'json'. See "Codec schema
                                              # gate" below — `typed` requires per-event
                                              # schema in events.yaml.
            events:
              "<scxml event name>":
                method: <name>           # someip: resolves to method_id
                event_group: <name>      # someip event: resolves to event_group_id + event_id
                getter: <name>           # someip field getter
                setter: <name>           # someip field setter
            qos:
              <transport-native-key>: <value>  # DDS reliability, SOME/IP protocol, etc.

        # Machine-lifetime event subscriptions (always-on sensors).
        # Codegen emits subscribe at init, unsubscribe at shutdown.
        subscriptions:
          - event: <scxml event name>
            source: "<target_id>"

        # Optional explicit pattern override. Scope: user declaration only.
        # By default sce-build INFERS pattern from the event-name convention
        # (service.request.* → RpcRequest, service.response.* → RpcReply,
        #  event.notification.* → EventNotify, field.get.* → FieldRead,
        #  field.set.* → FieldWrite, field.notify.* → FieldNotify,
        #  event.subscribe.* → EventSubscribe, event.unsubscribe.* → EventUnsubscribe,
        #  everything else → FireForget).
        # Use this block ONLY when:
        #   - An event name falls outside the convention and must be classified.
        #   - The author wants RpcRequest↔RpcReply pairing for events whose
        #     names don't follow the "service.request.X ↔ service.response.X"
        #     mirror (e.g. service.request.compute_force ↔ force.computed).
        # Inferred classifications CANNOT be overridden: declaring a different
        # `kind:` for an event whose name matches a convention is a build error.
        patterns:
          "<scxml event name>":
            kind: RpcRequest | RpcReply | EventSubscribe | EventUnsubscribe |
                  EventNotify | FieldRead | FieldWrite | FieldNotify | FireForget
            paired_with: "<other event>"   # REQUIRED iff kind == RpcRequest and
                                           # the response event name does not
                                           # follow the "service.response.X" mirror.
                                           # REJECTED for other kinds.

# Aggressive distribution: partition a single machine across multiple
# processes along W3C-orthogonal axes (parallel regions and <invoke> children).
# Omit `partitions:` entirely for default single-process execution per machine.
# See §16 for the partition distributability rule and §14 schema details.
partitions:
  <partition_name>:                      # process identity (CI harness / deploy)
    device: <device_name>                # where to run; defaults to first device
    machines: [<machine_id>, ...]        # machines whose pieces this partition hosts
    contains:
      parallel_regions:                  # child <state> IDs directly under <parallel>
        - machine: <machine_id>
          region: <state_id>
      invokes:                           # <invoke> ids (including synthesized
        - machine: <machine_id>          # <parent>__sce_synth_invoke__<id> from §9.6.6
          invoke: <invoke_id>
    hosts_parallel_roots:                # <parallel>s this partition is the root for.
                                         # Required on exactly one partition per distributed
                                         # <parallel> (rule 12). Implicit for a <parallel>
                                         # whose regions all live in one partition; may be
                                         # omitted in that degenerate case.
      - machine: <machine_id>
        parallel: <parallel_state_id>    # id of <parallel> element in the named machine
    transport_binding: <transport_name>  # inter-partition traffic within same machine
                                         # (defaults to device-shared transport of kind tcp/shm)
    barrier_timeout_ms: <integer|null>   # per-partition parallel-final barrier timeout
                                         # (§16.5). null = infinity (W3C normative default).
                                         # Applies only to partitions that claim at least one
                                         # <parallel>'s root via hosts_parallel_roots: above;
                                         # a value on a partition with no claim is a rule 12
                                         # configuration error (see §16.5 for the runtime
                                         # consumer that this field gates).

events: <path to events.yaml>          # Event payload type definitions
```

### 14.4 Binding value-field placeholders

A binding value field may carry `{name}` tokens that are substituted at runtime from `<send>` / `<invoke>` `<param>` values. This is the only mechanism SCE Mesh adds above transport-native routing; see §3.3 for the design invariant ("SCE does not reimplement transport discovery"). The SCXML author writes a single `<param name="id" expr="...">`; deploy.yaml decides whether that param feeds a Zenoh KeyExpr substitution or a SOME/IP instance selector, without any SCXML change.

**Grammar.** A placeholder is a `{` followed by a non-empty identifier (`[A-Za-z_][A-Za-z0-9_]*`) followed by a `}`. `{` and `}` literals are not escapable; this may be revisited if a transport's value space ever requires a literal brace, but today no supported transport does.

**Placeholder carriers** (where substitution is legal today):

| Transport | Binding field | Substitution source | Substitution target |
|---|---|---|---|
| Zenoh | `key:` | `{name}` tokens in the value | The KeyExpr string at `session.put` / `session.get` call site |
| SOME/IP | `instance_from: <param-name>` | the named `<param>`'s runtime value (must be a member of `instances:`) | `message->set_instance(...)` argument |

Both mechanisms converge on "binding references a `<param>` name". Zenoh embeds the reference syntactically inside `key:`; SOME/IP names it via an explicit binding field (`instance_from:`) because the instance_id is a typed `uint16_t`, not a string. A referenced `<param>` name that no `<param>` supplies at the call site is a build-time error (`mesh/pool-param-name-missing`). Reserved `_mesh_*` param names (§9.5) never supply placeholders.

**Transport capability gating.** A binding may carry placeholders only if its transport declares `supports_pool: true` in the registry. Today:

- **Zenoh** — `supports_pool: true`. Open placeholder range: the KeyExpr substitution is validated only for placeholder grammar; the resulting string is passed to Zenoh's native routing which delivers to whichever peer has a matching subscriber.
- **SOME/IP** — `supports_pool: true` *with a bounded instance list*. vsomeip's `request_service(SERVICE, ANY_INSTANCE)` does not actually subscribe to every instance — it requests instance `0xFFFF` as a specific instance. A SOME/IP placeholder binding therefore requires an explicit `instances:` list in deploy.yaml. Codegen emits one `request_service(SERVICE, i)` per declared instance at `init()`; the runtime placeholder value is validated against the list and fails-closed with `error.execution` + `reason=INVOKE_SRC_NOT_FOUND` (§9.5 three-tier error table, §10.7.1) if out of range — no `vsomeip::message->send()` is called, so no wire `RpcStatus` is involved.
- **custom_tcp** / **shm** / **local** / **can** — `supports_pool: false`. These transports have no runtime routing layer; adding one would be middleware-SD reinvention (§3.3). Any binding on these transports carrying a placeholder is rejected at parse time (`mesh/pool-not-supported-by-transport`).

**SOME/IP client pool schema.** When a SOME/IP binding declares `instance_from:`, it must also declare `instances: [<int>, ...]` — the finite set of instance IDs to pre-request at `init()`. Missing list is `mesh/pool-missing-instance-list`; empty list is `mesh/pool-empty-instance-list`. The runtime send path validates the resolved placeholder is a member of the list before calling `message->set_instance`; out-of-range values raise `error.execution` with `reason=INVOKE_SRC_NOT_FOUND` per §10.7.1 (pre-envelope setup fault — see the §9.5 three-tier error table). This is a *client-side* pool — the sender selects which instance to invoke among the declared set.

**Server-side multi-instance — SOME/IP pool.** A machine acting as a SOME/IP server may declare `server.instances: [<int>, ...]` to host N independent offered instances of the same service on one router. Codegen emits a fixed-size `SOMEIP_SERVER_INSTANCES` array, `TransportRouter::N_SESSIONS` matches that cardinality, and the `sessions_` pointer array is sized accordingly (caller supplies N `SenderEngine*` pointers at construction). Non-pool deployments keep `N_SESSIONS == 1` and behave exactly as before. At `init()` every declared instance gets its own `offer_service` and per-method `register_message_handler`; inbound callbacks resolve `msg->get_instance()` to a session slot via `session_index_for_instance()` and dispatch there. Spontaneous notifications (`field.notify.*` / `event.notification.*`) emit on the raising session's own offered instance via `publishEventgroupNotify(env, session_idx)`, so client subscribers pinned to a specific instance see only their bound session's events.

- **Transport capability**. Only transports whose registry entry sets `supports_multi_instance_server: true` accept `server.instances:` at parse time — today SOME/IP alone. Other transports reject the list with `mesh/deploy-server-pool-not-supported` because their inbound delivery has no peer-identifying discriminator (Zenoh's KeyExpr is not a peer identity; custom_tcp / shm / local / can have no runtime routing layer at all). Dropping the list or switching to a SOME/IP transport opens the pool.
- **RPC client coexistence**. Pool + any router-scoped RPC client on the same router is rejected at codegen with `mesh/codegen-pool-with-rpc-client-unsupported`. Two rejection shapes share one diagnostic code (distinguished by the `kind` payload):
  - **`<invoke type="sce:mesh-rpc">`** — `invoke_correlation_` + `active_invokes_` are router-scoped (§9.5 "one live correlation entry per invoke_id" invariant).
  - **SOME/IP `<send>` RpcRequest** — `pending_rpcs_` is the router-scoped `correlation_id → reply-event-name` map; the generated client-side receive handler dispatches the matched reply to `sessions_[0]` unconditionally, so cross-session reply misrouting would be silent under a pool router. Zenoh RpcRequest is unaffected because `session.get()` on_reply closures correlate natively per query handle, not through a shared table.
  
  The rejection is mechanical in both shapes: either remove the RPC client site(s) from this machine or reduce `server.instances:` to a single entry. Split across deployments is fine — the rejection is per-router, not per-deployment. See §10.9 invariant 8 for the structural argument against aliasing.
- **Self-echo filtering remains router-scoped**. `routing_id` (§10.9) is generated once at ctor time and shared across the pool's N sessions. Every current self-echo site (Zenoh server put-sub) is already excluded from the pool shape (Zenoh's `supports_multi_instance_server = false`), so the single router-scoped identity is sufficient for every Phase 3 shape. If a future backend gains both pool support and a self-echo path, the echo gate would need to migrate to session granularity — but the codegen reject above keeps that extension from landing silently.

**Backward compatibility.** A binding without placeholders behaves exactly as before — the carrier field is a literal value resolved at build time. No `discovery:` deploy-level block is required or supported; runtime target selection is a *binding* property, not a deployment-wide mode.

### 14.5 Cross-transport auto-bridging — rejected

SCE Mesh does not automatically translate envelopes between transports. A machine that receives an envelope over one transport and wants to forward it over another does so through explicit SCXML transitions, with a `<send target="#other">` action whose target resolves to a binding on the second transport. This is application responsibility, not codegen.

Rationale:
1. Transport-native encoding matters (SOME/IP method_id, Zenoh KeyExpr structure, DDS topic QoS). Mechanical envelope translation erases semantics that the author may depend on.
2. Bridging logic is often application-specific (rate limit, filter, enrich). Pushing it into codegen forces a one-size-fits-all policy.
3. The explicit SCXML path makes the cross-transport hop visible in the state machine — essential for debugging and auditing.

Any future revisit would require explicit motivation in a new section; the default answer remains "write the bridge as an SCXML transition in the bridging machine".

**Principles**:

1. **No duplication with external infra**: `vsomeip.json` declares `{service: motor_control, id: 0x1001}` exactly once. deploy.yaml says `service: motor_control` and the build tool resolves.
2. **Names, not numbers**: IDs are implementation details of the transport. SCXML and deploy.yaml talk in stable logical names.
3. **Same SCXML, multiple deploy.yaml**: A single `brake.scxml` deploys to a vehicle (SOME/IP + vsomeip.json from OEM), an IntraECU bench (SHM, no external config), or an MMORPG server (Zenoh + peer config) just by swapping deploy.yaml.
4. **Strict validation**: `deny_unknown_fields` on every config struct. Missing names in external configs fail the build. Request/response pairing mismatch fails the build.

**Deprecated (Session E cleanup)**: inline `service_id: 0x1234` / `method_id: 0x0042` / `event_group_id` / `event_id` / `getter_id` / `setter_id` as shipped in Session C. Retained as a parse-tolerant migration path but emitting a warning; removed after all in-tree fixtures are migrated.

### Partition resolution rules

When `partitions:` is present:

1. **Coverage**: every `<parallel>` region and every `<invoke>` in every listed machine must be covered by exactly one partition, OR be explicitly assigned to a default partition (see rule 2). Uncovered orthogonal units are a build error — the analyzer never silently places them anywhere.
2. **Default partition** (must be explicit):
   - A machine **not mentioned in any `partitions:` entry** runs single-process on its device with no split. No warning; this is the normal monolithic case.
   - A machine that IS mentioned but leaves some orthogonal units unassigned — **build error**. The analyzer prints every unassigned orthogonal unit with its path and requires the author to either assign it to an existing partition or add a dedicated `<machine>_default:` partition. There is no silent inheritance. The error diagnostic reads:
     ```
     error: machine 'brake' has partitions declared, but the following orthogonal
            units are unassigned:
              - parallel_region: brake/monitoring
              - invoke: brake/compute_force_inv
            Either add them to an existing partition under machines: [brake],
            or declare a 'brake_default' partition with contains: entries for each.
     ```
   - This explicit-everywhere rule keeps distribution topology readable: any deploy.yaml reader can recover the full state → process map by reading only `partitions:`.
3. **Distributability check**: each partition boundary is validated against the Parallel Region Distributability rule (§16.3) and Cross-Region Transition rule (§16.4). Violations fail the build in strict mode or auto-merge in permissive mode.
4. **Transport binding**: inter-partition events (between pieces of the same machine) travel over `transport_binding`. Default is `shm` if all partitions run on the same device; otherwise `custom_tcp` (Session E reference).
5. **Synthesized machines**: `<invoke type="scxml">` inline `<content>` produces machine `<parent>__sce_synth_invoke__<id>` (§9.6.6). It follows the same partition rules as named peers and may be placed in any partition. The reserved `__sce_synth_invoke__` infix is checked at build time against author-declared machine ids; a collision is a hard error. A synthesized machine's parent's partition assignment **does not** auto-propagate — the synthesized machine must be either explicitly assigned or placed in a default partition per rule 2.

6. **Uniqueness of partition names**: Partition names are globally unique across the deployment. A duplicate `partitions.<name>:` is a deploy.yaml parse error (`deny_unknown_fields` catches it on re-insert). Partition names double as process identities at runtime; uniqueness is required for log correlation and the IRP harness (§16.8) to map partitions to OS processes.

7. **Single-device per partition**: A partition occupies exactly one device (its `device:` field). A single partition cannot span multiple devices — that would require cross-device transport for the partition's internal membership, which contradicts the partition abstraction (a partition is the unit of single-process execution). Cross-device splits are expressed as multiple partitions, one per device.

8. **Unit-to-partition uniqueness**: Every orthogonal unit (parallel region, invoke) appears in **at most one** partition's `contains:` block. A unit listed in two partitions is a hard build error, citing both partition names. The analyzer emits the unit's canonical path (`<machine>/<region>` or `<machine>/<invoke_id>`) so authors can locate the collision precisely.

9. **Machine-membership consistency**: A partition's `contains:` entries must reference only machines listed in its `machines:` field. Listing `contains.parallel_regions[*].machine: X` while `machines: [Y]` is a hard build error. This prevents one partition from reaching into another's address space.

10. **Empty partitions**: A partition with `contains:` omitted or fully empty is a hard build error. Empty partitions have no runtime purpose and usually indicate a copy-paste error. Authors who want a reserved partition with no initial units must add a placeholder comment and a dummy unit (which itself must exist).

11. **Nested parallel partitioning**: Inner-region units of a nested `<parallel>` (§16.3) follow the same partition rules independently. An outer region assigned to partition `P_outer` may contain an inner region assigned to `P_inner`; the inner region then runs in `P_inner`'s process, while `P_outer` retains the outer region's non-inner-parallel states. The two partitions communicate via `transport_binding`.

12. **Parallel root partition designation**: Every distributed `<parallel>` — a `<parallel>` whose regions span two or more partitions — must have exactly one partition claiming that `<parallel>`'s root via `partitions.<name>.hosts_parallel_roots: [{machine, parallel}]`. The root partition owns the `ParallelCompletionTracker` (§16.5) and is the site where `done.state.<parallel_id>` is raised into the local external queue. A `<parallel>` whose regions live entirely in a single partition has that partition as implicit root; the field may be omitted in that case. Rule 8's unit enumeration is unchanged — a `<parallel>` container id is **not** an orthogonal unit (the container runs wherever any of its regions run); rule 12 is a layered, orthogonal obligation on top of rules 1/8. A claimant partition must co-host at least one region of the claimed parallel: the tracker aggregates local region completions plus inter-partition `ParallelRegionDone` envelopes (§16.5 wire 21), and a root that co-hosts no region would force gratuitous inter-partition traffic for its own region updates. Violations:
    - Zero claimants on a distributed `<parallel>`: `mesh/partition-parallel-root-undesignated`.
    - Two or more claimants on the same `(machine, parallel)` pair: `mesh/partition-parallel-root-ambiguous`.
    - `hosts_parallel_roots[*].machine: X` while `machines: [Y]` (rule 9 shape applied to root entries): `mesh/partition-parallel-root-not-in-machines`.
    - A claimant that co-hosts no region of the claimed parallel: `mesh/partition-parallel-root-non-host`.
    - `barrier_timeout_ms:` set on a partition with no `hosts_parallel_roots:` entries: `mesh/partition-barrier-timeout-without-root` (the timeout has no tracker to gate; §14 grammar L2731+).

    **Status (2026-04-20, rule 12 atomic bundle + §16.5 transport landed)**: the `hosts_parallel_roots:` serde field, the rule 12 validator, the §16.5 `ParallelCompletionTracker` runtime, the `tools/codegen/templates/mesh/cpp/parallel_final.jinja2` partition-aware branch body, AND the inter-partition wire-21 transport delivery have landed together in two commits (atomic trio + transport closure). The five rejection diagnostics above (`undesignated`, `ambiguous`, `not-in-machines`, `non-host`, `barrier-timeout-without-root`) each have a unit test in `sce-build/src/mesh/partitions.rs` and a `tests/mesh/partition_rule12{a..e}_*.yaml` integration fixture registered in the ctest matrix. The tracker primitive lives at `sce/include/mesh/ParallelCompletionTracker.h` (header-only, per-`<parallel>` threshold firing + re-entry reset + single-shot duplicate absorption); `MeshDispatch::dispatchEnvelope` routes wire-21 `ParallelRegionDone` envelopes via SFINAE-detected `engine.onParallelRegionDone(env)` into the root partition's per-parallel tracker. The `--partition <name>` CLI flag on `sce-codegen` selects the partition identity for codegen; `model.partition_parallel_roles` maps each `<parallel>` id to one of `Root | NonRoot | SinglePartition`, and `parallel_final.jinja2` branches accordingly. **Inter-partition transport** is wired through the generated `TransportRouter`: NonRoot codegen emits one outbound shm channel per unique destination partition (`/sce_p21_<src>_<dst>`, `Mode::Create`); Root codegen emits one inbound channel per source partition (same name, `Mode::Open` with lazy reopen on `pumpWire21Inbound()`). The dispatch from `parallel_final.jinja2` goes through `engine.triggerParallelRegion{Local,Remote}*` base hooks (no derived-SM downcast) — the SM ctor installs closures that terminate in `tracker_<pid>_.onLocalRegionComplete(...)` (Root) or `sendParallelRegionDone(...)` (NonRoot, which routes through `parallel_region_done_callback_` set by the router). End-to-end verification lives in `tests/mesh/test_mesh_partition_rule12_e2e.cpp`, which fork+execs two binaries built from the same `motor_partition.scxml` with `--partition motor_left` (Root) and `--partition motor_right` (NonRoot) and asserts the Root SM enters `<final id="all_done">` after the wire-21 round-trip. **Scope carve-outs that intentionally do not land in this transport closure**: (a) `transport_binding: custom_tcp` codegen — the wire-21 channel emitter currently only materializes shm (`PartitionWire21Channel = ShmChannel<>`); a `custom_tcp` partition pair on a distributed `<parallel>` route is **rejected at deploy validation time** via `mesh/partition-wire21-custom-tcp-unimplemented` (`validate_parallel_root_designation` Pass 2b), so the configuration gap surfaces at build instead of runtime. The diagnostic message guides the author to switch to `shm` (same-device deployments) or wait for the custom_tcp wire-21 emitter. (b) §16.5 L3500 finite barrier-timeout runtime firing — `barrier_timeout_ms` is validated at deploy time but no scheduler callback fires `error.communication PARALLEL_BARRIER_TIMEOUT` yet; a stuck NonRoot leaves the Root tracker indefinitely incomplete (matches W3C normative default of infinite timeout). The original atomicity argument — any two-of-three landing alone creates the "built but unconsumed" or "silent selection" anti-patterns — is preserved by the single-commit landing of validator + tracker + template, and the transport closure is itself atomic (codegen + runtime + E2E in one commit). Rule 12 enforcement is now unconditional and end-to-end.

### Pattern override grammar

The `patterns:` block under each machine declares explicit pattern classification for events whose names fall outside the inference conventions (§8.1). Formally:

```
patterns ::= { event_name → pattern_override }*
pattern_override ::=
  { kind: PatternKind, paired_with?: event_name }

PatternKind ::= RpcRequest | RpcReply | FireForget
              | EventSubscribe | EventUnsubscribe | EventNotify
              | FieldRead | FieldWrite | FieldNotify
```

**Resolution**: An event's pattern is resolved in this precedence order:

1. **Inference (highest authority)**: If the event name matches a convention prefix (`service.request.*`, `service.response.*`, `event.notification.*`, `field.get.*`, `field.set.*`, `field.notify.*`, `event.subscribe.*`, `event.unsubscribe.*`), the pattern is fixed by inference. A `patterns:` entry that declares a **different** `kind:` for the same event is a hard build error (`kind override of inferred pattern`).
2. **Explicit override**: If the event name does not match any convention prefix, the `patterns:` entry's `kind:` applies. Absence of a `patterns:` entry for such an event means `kind: FireForget`.

**`paired_with` semantics**:
- **Required**: when `kind: RpcRequest` AND the event name is not `service.request.X` (i.e., the default mirror pairing cannot apply). Absence is a hard build error (`RpcRequest without paired_with outside convention`).
- **Rejected**: on any `kind:` other than `RpcRequest`. Presence is a hard build error.
- **Referent**: `paired_with` must name an event that either (a) appears in the same machine's SCXML as a `<transition event="...">` or (b) has a `patterns:` entry with `kind: RpcReply`. Unresolved referents are a hard build error (`paired_with target not found`).
- **Same-target constraint**: the RpcReply event must be received from the SAME transport target as the RpcRequest's `<send target>`. Cross-target pairing (request to `#A`, reply from `#B`) is not supported in E1 — it would require an RPC routing table the engine does not maintain. Cross-target RPC is a deferred capability (Phase 4).

**Overlap with inference**: if `paired_with` names an event whose own pattern is inferred as something other than `RpcReply` (e.g., `field.notify.X` which infers as `FieldNotify`), the build fails with `paired_with target has incompatible inferred kind`. The author must either rename the reply event to match the `service.response.*` mirror or declare both via `patterns:` entries that are consistent.

**Worked example**: an author models a request/response where the reply event carries a value-oriented name:

```yaml
patterns:
  "service.request.compute_force":
    # inferred: RpcRequest. paired_with required because mirror name doesn't match.
    paired_with: "force.computed"
  "force.computed":
    kind: RpcReply         # explicit: not inferrable from the name
```

### Codec schema gate

`codec:` selects the payload serialization (envelope framing is always CBOR per §13). Allowed values: `json` (default), `cbor`, `typed`, `raw`.

- `json`, `cbor`, `raw`: payload is opaque bytes to sce-build; no schema required.
- `typed`: payload is a compact binary encoding generated by sce-build from an event schema. **Schema declaration is REQUIRED.**

**Rule**: When a binding declares `codec: typed`, every event sent or received on that binding MUST have a schema declared under `events.yaml:<event_name>.schema`. Absence is a hard build error (`TopologyError::TypedCodecMissingSchema`). This prevents silent codec fallback — a missing schema would otherwise force sce-build to degrade to JSON, producing a codec tag mismatch between the two endpoints.

The rule is applied per-event, not per-binding: a binding may carry a mix of events where only some use the typed codec (via per-event override — future capability); the typed events must all have schemas, the others may omit them.

### Example: IRP distributed conformance harness

```yaml
version: "1.0"

topology:
  harness_host:
    platform: linux
    target: x86_64
    transports:
      custom_tcp:
        listen: "127.0.0.1:0"     # ephemeral port; harness assigns
    machines:
      test487:
        source: tests/w3c/test487.scxml
      test487__sce_synth_invoke__subtask:   # synthesized from inline <content>
        source: <auto>            # extracted at build time

partitions:
  test487_main:
    device: harness_host
    machines: [test487]
    contains:
      parallel_regions:
        - { machine: test487, region: watchdog }
  test487_worker:
    device: harness_host
    machines: [test487]
    contains:
      parallel_regions:
        - { machine: test487, region: executor }
      invokes:
        - { machine: test487, invoke: subtask }
```

The harness spawns `test487_main` and `test487_worker` as separate OS processes on `harness_host`, wires them via `custom_tcp`, and runs the standard W3C IRP verification against the combined system.

### Example: Automotive

```yaml
version: "1.0"

topology:
  brake_ecu:
    platform: qnx
    target: aarch64
    transports:
      someip:
        config: ./config/brake_ecu/vsomeip.json   # OEM-supplied
        application_name: brake_app
    machines:
      brake:
        source: brake.scxml
        bindings:
          "#motor":
            transport: someip
            service: motor_control                 # resolves to service_id/instance_id via vsomeip.json
            events:
              service.request.apply_force:
                method: apply_force                # resolves to method_id
              event.notification.motor_status:
                event_group: status_group          # resolves to event_group_id + event_id
            qos:
              protocol: TCP                        # SOME/IP native: TCP for reliable
          "#dashboard":
            transport: can
            address: "can0:0x100"
            signals: "vehicle.dbc"                 # CAN native: DBC signal layout
        subscriptions:
          - event: event.notification.vehicle_speed
            source: "#chassis"

  motor_ecu:
    platform: qnx
    target: aarch64
    transports:
      someip:
        config: ./config/motor_ecu/vsomeip.json
        application_name: motor_app
    machines:
      motor:
        source: motor.scxml
        bindings:
          "#brake":
            transport: someip
            service: brake_control

  dashboard_ecu:
    platform: linux
    target: aarch64
    machines:
      dashboard:
        source: dashboard.scxml
        bindings:
          "#brake": { transport: can, address: "can0:0x101" }
          "#cloud":
            transport: grpc
            address: "telemetry.oem.com:443"
            tls: true                              # gRPC native: TLS config

discovery:
  mode: static
```

SOME/IP application routing (`application_name`, routing manager, security) comes from the referenced `vsomeip.json`. `sce-build` never re-declares it.

### Example: MMORPG

```yaml
version: "1.0"

topology:
  zone_server_1:
    platform: linux
    target: x86_64
    machines: [npc_ai, quest_tracker, loot_system]

  zone_server_2:
    platform: linux
    target: x86_64
    machines: [boss_ai, raid_controller]

  gateway:
    platform: linux
    target: x86_64
    machines: [session_manager, matchmaker]

transport:
  default: udp
  overrides:
    "* -> gateway": grpc

discovery:
  mode: dynamic
  resolution:
    strategy: priority
    priority_order: [local, udp, grpc]

qos:
  defaults:
    qos: best-effort
    priority: normal
```

---

## 15. Zenoh Transport Template Specification

Zenoh is a primary Phase 3 transport target. This section defines the `zenoh_transport` codegen template — how deploy.yaml Zenoh configuration maps to generated code that calls zenoh-c/zenoh-pico APIs directly.

### 15.1 Key Expression Mapping

SCXML `#id` targets are mapped to Zenoh key expressions via `deploy.yaml`. SCXML documents never contain Zenoh key expressions — location transparency is preserved.

```yaml
# deploy.yaml
topology:
  brake_ecu:
    machines:
      brake:
        bindings:
          "#motor":
            transport: zenoh
            key: "vehicle/powertrain/motor/cmd"
          "#dashboard":
            transport: zenoh
            key: "vehicle/body/dashboard/status"
```

#### Key Expression Structure

```
Convention: {domain}/{subsystem}/{machine}/{direction}

Examples:
  vehicle/powertrain/motor/cmd        # commands to motor
  vehicle/powertrain/motor/status     # status from motor
  vehicle/body/dashboard/events       # events to dashboard
  vehicle/chassis/brake/cmd           # commands to brake
```

#### Subscribe Patterns

The receiving side uses Zenoh wildcard subscriptions generated at build time:

```
brake.scxml receives events from any source:
  subscribe("vehicle/chassis/brake/cmd")

motor.scxml receives from brake and dashboard:
  subscribe("vehicle/powertrain/motor/cmd")

monitoring tool subscribes to everything:
  subscribe("vehicle/**")
```

The build tool generates subscribe patterns from the topology map — if machine A sends to machine B, B's subscribe pattern includes A's publish key.

### 15.2 Session Management

A single Zenoh session is shared across all state machine instances on the same device. The `zenoh_transport` template generates an `init_zenoh_session()` function that creates one session, and all per-target send functions share it:

```
Generated code (per device):
    |
    +-- init_zenoh_session()  (one session per device)
        |
        +-- send_to_motor()   (uses shared session)
        +-- send_to_dashboard() (uses shared session)
        +-- subscribe_brake()  (uses shared session)
```

```cpp
// [generated] zenoh session initialization
static z_owned_session_t session_;

void init_zenoh_session() {
    z_config_t config = z_config_default();
    // deploy.yaml zenoh config applied here
    z_config_insert(config, Z_CONFIG_MODE_KEY, "peer");
    z_config_insert(config, Z_CONFIG_CONNECT_KEY, "tcp/192.168.1.1:7447");
    session_ = z_open(z_move(config));
}

// All send functions use the shared session
void send_to_motor(const EventDescriptor& event) {
    z_put(z_loan(session_), motor_key_, payload, len, &opts);
}
```

Rationale: Zenoh sessions manage peer discovery, connection pooling, and resource allocation. Multiple sessions on the same device waste resources and can cause discovery conflicts.

### 15.3 QoS Configuration (deploy.yaml → Generated Code)

Zenoh QoS is configured in deploy.yaml using Zenoh-native terminology. The `zenoh_transport` template generates code that applies these settings directly:

```yaml
# deploy.yaml — Zenoh-native QoS, not abstracted
bindings:
  "#motor":
    transport: zenoh
    key: "vehicle/powertrain/motor/cmd"
    qos:
      reliability: reliable           # Zenoh native: Reliable / BestEffort
      congestion_control: block       # Zenoh native: Block / Drop
      priority: real_time             # Zenoh native: 1-7 priority levels
      express: true                   # Zenoh native: skip batching
```

Generated code applies Zenoh QoS natively:

```cpp
// [generated] — Zenoh API called directly, all QoS preserved
void send_to_motor(const EventDescriptor& event) {
    z_put_options_t opts;
    z_put_options_default(&opts);
    opts.congestion_control = Z_CONGESTION_CONTROL_BLOCK;
    opts.priority = Z_PRIORITY_REAL_TIME;
    opts.express = true;
    z_put(session_, z_keyexpr("vehicle/powertrain/motor/cmd"),
          payload, payload_len, &opts);
}
```

**QoS is deploy.yaml-only** (see §5, Session E path B). SCXML carries no QoS annotation. The deploy.yaml binding's QoS block is the sole source for code generation.

#### Deadline Enforcement

Zenoh does not natively support delivery deadlines. For `<invoke type="sce:mesh-rpc">`, the deadline comes from `<param name="_mesh_deadline_ms">` and is enforced in generated code:

```cpp
// [generated] — deadline enforcement from <invoke> _mesh_deadline_ms param
void send_invoke_with_deadline(const Envelope& env, uint64_t deadline_ms) {
    auto timer = start_deadline_timer(std::chrono::milliseconds(deadline_ms));
    z_put(session_, motor_key_, env_bytes, len, &opts);
    if (timer.expired()) {
        inject_error_invoke(env.invoke_id, RpcStatus::DeadlineExceeded);
    }
}
```

For FireForget `<send>` without `<invoke>`, no deadline enforcement applies — the transport-native reliability setting in deploy.yaml governs retry/drop semantics.

### 15.4 Deployment Topology

Zenoh supports three modes with different trade-offs:

| Mode | Topology | Latency | Use Case |
|------|----------|---------|----------|
| `peer` | Direct peer-to-peer | Lowest | ECU-to-ECU on same network |
| `client` | Connect to router | Medium | ECU-to-cloud via gateway |
| `router` | Accept connections | N/A (relay) | Central gateway, bridge node |

#### Automotive Deployment

```
Typical vehicle network:

[Brake ECU]---+
  (peer)      |
[Motor ECU]---+--- Vehicle Ethernet ---+--- [Gateway ECU] --- Cloud
  (peer)      |                        |     (router+client)
[Body ECU]----+                        |
  (peer)                               |
                                       +--- [Diagnostic Tool]
                                              (client)

Intra-vehicle: peer mode (direct, lowest latency)
Vehicle-to-cloud: gateway acts as Zenoh router + cloud client
Diagnostics: client mode connecting to gateway router
```

#### deploy.yaml Zenoh Configuration

```yaml
zenoh:
  mode: peer | client | router
  connect: ["tcp/192.168.1.1:7447"]     # router endpoints (client/peer mode)
  listen: ["tcp/0.0.0.0:7447"]          # listen endpoints (router mode)
  shmem: true                           # enable Zenoh shared memory
  key_prefix: "vehicle"                 # prepended to all key expressions
  transport:
    unicast:
      max_links: 4                      # max concurrent links
    multicast:
      enabled: true                     # peer discovery via multicast
```

### 15.5 Zenoh SHM and `shm_transport` Template Relationship

Zenoh has built-in shared memory support for same-host communication. This overlaps with Phase 2's `shm_transport` template.

**Rule**: when deploy.yaml specifies `transport: zenoh` for a same-host binding, the `zenoh_transport` template generates code that uses Zenoh SHM automatically. Do not use `transport: shm` and `transport: zenoh` for the same target.

| Scenario | deploy.yaml transport | Generated code uses |
|----------|----------------------|---------------------|
| Same process | `local` | Direct function call (inlined) |
| Same ECU, no Zenoh | `shm` | POSIX shared memory ring buffer |
| Same ECU, Zenoh enabled | `zenoh` | Zenoh SHM (automatic, same API) |
| Cross ECU | `zenoh` | Zenoh network |

Zenoh SHM is transparent — the generated `z_put()` / `z_subscriber()` calls work identically whether the subscriber is on the same host (SHM) or remote (network). The `zenoh_transport` template generates the same code either way; only the deploy.yaml Zenoh session config enables SHM:

```yaml
# deploy.yaml — same ECU processes use Zenoh SHM automatically
zenoh:
  shmem: true    # Zenoh detects same-host subscribers and uses SHM
```

This eliminates the need for routing logic to choose between SHM and network paths — Zenoh handles it internally at the protocol level.

### 15.6 SCXML Concept → Zenoh Primitive Mapping

| SCXML Concept | Zenoh Primitive | Details |
|--------------|----------------|---------|
| `<send>` (fire-and-forget) | `session.put(key, payload)` | One-shot publish |
| `<send>` + response wait | `session.get(key, payload)` | Query/reply |
| `<invoke>` create child | `session.put(key/invoke/create, {session_id, params})` | Pub to invoke topic |
| `<invoke>` child events | `session.put(key/{session_id}/events, event)` | Child publishes on session key |
| `<invoke>` parent receives | `subscriber(key/{session_id}/events)` | Parent subscribes to session |
| `done.invoke` | `session.put(key/{session_id}/done, data)` | Child publishes final event |
| `<cancel>` invoke | `session.put(key/{session_id}/cancel)` | Parent sends cancel |
| Event receive | `subscriber(key/cmd)` | Persistent subscription |
| `error.communication` | Subscriber disconnect callback | Zenoh detects peer loss |

#### Invoke Session Key Pattern

```
Invoke lifecycle over Zenoh keys:

Parent (brake) invokes child (motor_control):

1. Parent publishes:
   vehicle/powertrain/motor/invoke/create
   payload: { session: "brake_ecu:brake:1", src: "motor_control.scxml", params: {...} }

2. Child subscribes (auto-generated):
   vehicle/powertrain/motor/invoke/brake_ecu:brake:1/cmd

3. Child publishes events:
   vehicle/chassis/brake/invoke/brake_ecu:brake:1/events

4. Parent subscribes (auto-generated):
   vehicle/chassis/brake/invoke/brake_ecu:brake:1/events

5. Child reaches <final>:
   vehicle/chassis/brake/invoke/brake_ecu:brake:1/done

6. Parent cancels:
   vehicle/powertrain/motor/invoke/brake_ecu:brake:1/cancel
```

### 15.7 Build Dependencies

| Target | Zenoh Library | Notes |
|--------|--------------|-------|
| Linux x86_64 | `zenoh-c` or `zenoh-cpp` | Full feature set |
| QNX aarch64 | `zenoh-c` | Cross-compile, no Rust runtime needed |
| AUTOSAR Classic | `zenoh-pico` | Pure C, no allocator required, no_std |
| AUTOSAR Adaptive | `zenoh-c` | POSIX-compatible |

CMake integration — generated code links directly against zenoh-c:

```cmake
# When deploy.yaml uses transport: zenoh, generated code requires zenoh-c
# User's CMakeLists.txt links the generated target against zenoh:
find_package(zenohc REQUIRED)
target_link_libraries(brake_app
    PRIVATE SCE::Generated::brake    # generated transport code
    PRIVATE zenohc::lib              # user provides zenoh library
)

# For resource-constrained targets:
# deploy.yaml: zenoh: { implementation: pico }
# Generated code calls zenoh-pico API instead of zenoh-c
```

### 15.8 Example: Complete Automotive deploy.yaml with Zenoh

```yaml
version: "1.0"

topology:
  brake_ecu:
    platform: qnx
    target: aarch64
    machines:
      brake:
        bindings:
          "#motor":     { transport: zenoh, key: "vehicle/powertrain/motor/cmd" }
          "#dashboard": { transport: zenoh, key: "vehicle/body/dashboard/cmd" }
          "#cloud":     { transport: zenoh, key: "cloud/telemetry/brake" }

  motor_ecu:
    platform: qnx
    target: aarch64
    machines:
      motor:
        bindings:
          "#brake":     { transport: zenoh, key: "vehicle/chassis/brake/cmd" }
          "#dashboard": { transport: zenoh, key: "vehicle/body/dashboard/cmd" }

  dashboard_ecu:
    platform: linux
    target: aarch64
    machines:
      dashboard:
        bindings:
          "#brake": { transport: zenoh, key: "vehicle/chassis/brake/cmd" }
          "#motor": { transport: zenoh, key: "vehicle/powertrain/motor/cmd" }

  gateway:
    platform: linux
    target: x86_64
    machines:
      telemetry_bridge:
        bindings:
          "#cloud_analytics": { transport: grpc, address: "analytics.oem.com:443" }

events: events.yaml

zenoh:
  key_prefix: "vehicle"
  shmem: true
  transport:
    unicast:
      max_links: 8

# Per-device Zenoh mode overrides
zenoh_overrides:
  brake_ecu:
    mode: peer
  motor_ecu:
    mode: peer
  dashboard_ecu:
    mode: peer
  gateway:
    mode: router
    listen: ["tcp/0.0.0.0:7447"]
    connect: ["tcp/cloud-gateway.oem.com:7447"]

discovery:
  mode: static

qos:
  defaults:
    qos: reliable
    deadline: 10ms
    priority: high
```

---

## 16. Distributed W3C SCXML Conformance

This section is the formal conformance statement for distributed execution of W3C SCXML 1.0 under SCE Mesh. It defines what "distributed conformance" means, which constructs are distributable, how partition boundaries are validated, and how the W3C IRP test suite is leveraged as empirical proof.

### 16.1 Conformance claim

**Claim**: SCE Mesh executes a W3C SCXML 1.0 document distributed across multiple OS processes (and optionally multiple devices) such that, for every document and every deployment satisfying the Distributability Rules (§16.3, §16.4, §16.5), the observable behavior is **distributed-equivalent** (§16.2) to single-process execution of the same document.

**Scope of conformance**:
- All W3C SCXML 1.0 normative constructs: `<state>`, `<parallel>`, `<final>`, `<history>`, `<transition>`, `<onentry>`/`<onexit>`, `<raise>`, `<send>`, `<invoke>` (type=`scxml`), `<cancel>`, `<if>`/`<elseif>`/`<else>`, `<foreach>`, `<log>`, `<assign>`, `<datamodel>`/`<data>`, `<donedata>`, `<script>`, `<finalize>`, `<content>`, `<param>`, `<namelist>`.
- All executable content semantics including `<finalize>`, `autoforward`, `<send delay>`, `<cancel>`, internal/external event queue ordering.
- `_event` standard fields including `origin`, `origintype`, `sendid`, `invokeid`, `data`, `type`.
- Error events: `error.execution`, `error.communication` per the W3C error-naming convention.

**Scope exclusions** (explicitly not claimed):
- Cross-machine macrostep atomicity. W3C §3.12 requires atomicity only within a single session; SCE Mesh respects this per-session. Cross-session observable ordering is defined by §10.1 (per-sender FIFO) and §16.2 (distributed equivalence), not by macrostep atomicity.
- Strong real-time ordering guarantees that a physical transport cannot supply (e.g., synchronized wall-clock delivery across continents). Conformance is observational, not chronometric.
- Execution under transports marked `conformance: degraded` (§10.4). Authors opting into degraded transports accept reduced conformance.

### 16.2 Distributed equivalence (weak)

The observational equivalence relation used for conformance is **weak equivalence**, defined below. Strong equivalence (all interleavings match single-process) is not claimed because it is provably unachievable without a global coordinator, which would eliminate the performance benefit of distribution.

**Weak distributed equivalence** between a distributed execution `E_d` and a single-process reference execution `E_s` of the same document D holds iff:

1. **Final configuration match**: if `E_s` reaches a stable configuration `C_s` (all active states; no more transitions enabled), then `E_d` reaches a configuration `C_d` whose set of active states (projected across all partitions) equals `C_s`.
2. **Done-state match**: if `E_s` terminates by reaching `<final>` at the session root with donedata `D_s`, then `E_d` terminates with the same final state and equivalent donedata.
3. **Per-sender causality**: for any two events `e1`, `e2` emitted by the same sender session in `E_s` with `e1` before `e2`, if both are observed in `E_d`, `e1` is observed before `e2` by every receiver.
4. **Invoke lifecycle match**: for every `<invoke id="X">` in `E_s` that reaches its child's `<final>`, the corresponding `done.invoke.X` is raised in `E_d` at the same invoking state (or later, but before invoking-state exit).
5. **Error event preservation**: every `error.execution` raised in `E_s` due to a document-level fault is raised in `E_d`. Additional `error.communication` events raised in `E_d` due to transport faults are permitted and not a conformance violation.
6. **External output equivalence**: for any external observer (`<log>` outputs, `<send target>` to non-mesh endpoints, `sce:output-to-file` or equivalent testing output), the multiset of outputs in `E_d` equals that in `E_s`. Total ordering of outputs across sessions is not required; per-session order is preserved.

These six properties are collectively sufficient to verify W3C IRP test outcomes: every IRP test verdict is a function of final configuration, donedata, and `<log>` outputs — all covered.

### 16.3 Parallel region distributability rule

W3C §3.4 defines `<parallel>` as concurrent orthogonal regions sharing the enclosing session's datamodel. Single-process RTC (§3.12) provides sequential consistency implicitly; distribution must reconstruct this contract without cross-process locks.

**Rule**: A `<parallel>` element's child regions may be placed in separate partitions iff **all** of the following hold. When a violation is detected, `sce-build` either fails the build with a specific diagnostic (strict mode) or auto-merges the offending regions into one partition (permissive mode, default).

**(R1) No shared writable data**. For every `<data>` declared in an ancestor scope of the `<parallel>`:
- at most one child region (including its descendants) may contain an `<assign location>`, a `<data expr=...>` initializer that depends on the data, or a `<script>` that assigns to it.
- If two or more regions perform writes to the same ancestor data, those regions must share a partition.

Exception: region-local `<datamodel>` declared inside a region is not ancestor scope from the sibling region's perspective — sibling regions cannot reach it syntactically, so no coordination is needed.

**(R2) No cross-region transitions**. A `<transition target>` must not resolve to a state inside a sibling region. A cross-region transition, when fired, exits and re-enters states across the parallel boundary; preserving the W3C exit-set/enter-set computation across processes requires macrostep atomicity, which distribution cannot supply.

Exception: a transition whose target is the `<parallel>` itself or any of its ancestors is NOT cross-region — it exits the parallel wholesale, which is local to each region (each region exits itself and the combined effect is well-defined).

**(R3) Shared reads are snapshot-scoped**. A region may read an ancestor-scope `<data>` that another region does not write (within the lifetime of the parallel activation). The value is **snapshot-captured** at parallel entry and frozen for each region partition's runtime. If the value is written by the parent outside the parallel scope, re-entries see the new snapshot.

**(R4) Script opacity**. Any `<script>` body that the static analyzer cannot prove side-effect-safe is treated as "writes every ancestor-scope data name observed in the script's lexical context". This is the conservative default. A `sce:script-safe="true"` attribute on `<script>` opts out of the conservative assumption (author promises no ancestor-scope writes); misuse is a documented risk.

**Analyzer implementation** (build-time, in `sce-build`):
```
for each <parallel> P in every machine M:
  for each child region R of P:
    writes_R = all locations written by R ∪ descendants
    reads_R  = all locations read by R ∪ descendants
    targets_R = all transition targets resolving outside R (but inside a sibling)
  for each data location L in ancestor-scope(P):
    writers = { R : L ∈ writes_R }
    if |writers| ≥ 2:
      emit constraint "regions {writers} must share a partition (R1)"
    elif |writers| = 1 and L ∈ reads_R' for R' ≠ writers[0]:
      emit constraint "region {R'} snapshot-reads L; entry-point sync required"
  for each R with targets_R ≠ ∅:
    target_regions = { sibling R'' containing a target in targets_R }
    emit constraint "regions {R, target_regions} must share a partition (R2)"
```

Emitted constraints are matched against `deploy.yaml partitions:`. If a constraint-group is split across partitions:
- **Strict mode** (`distributability: strict` in deploy.yaml): build fails with a diagnostic naming the regions and the offending data/target.
- **Permissive mode** (default): the named regions are silently merged into a single partition (the one with the lowest sort-ordered name). A build-log notice is emitted.

**Nested `<parallel>`**: R1-R4 apply at every `<parallel>` element independently. An inner parallel's regions are analysed against inner `<data>` scope AND all ancestor scopes including the outer parallel's regions. The analyzer walks `<parallel>` elements depth-first; inner violations can force outer-region merges (e.g., an inner region writes ancestor data → its enclosing outer region must share a partition with any sibling outer region that writes the same data).

**`<invoke>` as a distribution axis**: Each `<invoke>` in a machine is a distinct orthogonal unit under §14 — it may be assigned to its own partition. `<invoke>` inside a `<parallel>` region is analysed twice: (1) as an invoke unit in §14 partition coverage, (2) under R1/R2 only for the writes the invoke performs on the parent's datamodel via `<finalize>` (§9.6.4). An invoke whose `<finalize>` writes an ancestor-scope data location counts as a writer for R1 purposes, contributed to the partition hosting the invoking state.

**`<data>` initialization ordering**: R1 applies to the **lifetime** of the parallel activation — initializer writes at parallel entry count as writes for R1 purposes. A `<data>` initialized from an ancestor-scope expression inside one region (e.g., `<data id="local" expr="shared + 1"/>`) is treated as a **read** of `shared`, subject to R3 snapshot semantics.

**Status**: §14 partition schema (rules 6-10, parse-time) and §14 cross-reference rules 1/2/5/11 (coverage / default-discipline / synth-infix / nested-parallel) landed in `sce-build`. The R1/R2/R3/R4 distributability analyzer described above (shared-write detection, cross-region transition detection, snapshot-scope reads, script opacity) is **implemented** in `sce-build/src/mesh/distributability.rs` and runs automatically inside [`mesh::resolve_deploy_config`] (the shared entry point for `inject_partition_context_for` and `compile_mesh_transport`). Behaviour: R1 (shared ancestor-data writes) and R2 (cross-region transition targets) produce merge constraints; R3 snapshot-reads emit advisory notices without blocking the build; R4 script opacity is conservative — any ancestor-scope data identifier appearing as a word-bounded token in a `<script>` body is treated as a write. The `sce:script-safe="true"` opt-out from §16.3 R4 is not parsed yet; its absence means authors who hit a false positive must restructure the script until a consumer opens that opt-out. The `distributability: strict | permissive` knob is parsed on [`deploy::DeployConfig`] with `permissive` as the default; strict converts any R1/R2 constraint into a [`DeployError::DistributabilityR1SharedWrite`] / [`DistributabilityR2CrossRegionTransition`] and halts the build, while permissive feeds the §16.4 auto-merge fixed-point.

### 16.4 Cross-region transition auto-merge

When (R2) is violated under permissive mode (the default), the two or more regions transitively connected by cross-region transitions are merged into one partition:

```
violation: regionA has <transition target="regionB.sub">
  → merge(regionA, regionB)
  → if regionB was in partition P_B but regionA in P_A:
      P_A absorbs regionB; P_B loses regionB. If P_B becomes empty,
      P_B is dropped and its device allocation returned to pool.
```

Repeat until fixed point. Result is a **minimum-merge partition plan** that satisfies distributability while honoring as much of the author's partition request as possible.

**Status**: implemented in `sce-build/src/mesh/distributability.rs` as a fixed-point loop. Each R1/R2 constraint lists the regions that must share a partition; the resolver selects the lowest sort-ordered partition among the involved ones as the canonical survivor, absorbs the rest, and repeats until a full pass produces no merges. BTreeMap iteration order over partition names yields a deterministic result regardless of constraint discovery order. Each merge event records a [`distributability::MergeNotice`] on the returned plan; CLI tooling surfaces them via the `merge_notices()` accessor on [`ResolvedDeployConfig`].

**Region-partition liveness scope**. `error.communication` with `reason="PEER_PARTITIONED"` (§16.7 row 8) carries a `target: string` that is a **deploy.yaml machine identity** — the same axis the `sce/live/<machine_name>` liveliness keyexpr keys on (§10.9 invariant 4). Under multi-instance (§14.4 server pool), the machine is reported alive while any session is up. **Region partition** — an intra-machine split of `<parallel>` regions across separate OS processes — is an orthogonal axis: a region's hosting process dying does not surface as `PEER_PARTITIONED` (the machine-identity axis), so a dedicated §16.7 row 13 reason code `REGION_PARTITIONED` raises it with `_event.data = {machine, partition, last_seen_ms_ago?}`. The `PARALLEL_BARRIER_TIMEOUT` fallback (§16.5) remains available for authors who set `barrier_timeout_ms:` but fires only after the author-configured timer expires; row 13 raises as soon as the transport layer observes the partition's liveliness token drop. Transport wiring: **Zenoh per-partition liveliness tokens** under the `sce/live/<machine>/<partition>` key subspace — emitted by every partition binary whose machine declares `liveliness:` in deploy.yaml, subscribed by every sibling partition. The machine-level `sce/live/<machine>` token (row 8 axis) and the partition-level tokens coexist on the same subscriber; segment-count parsing disambiguates which reason code to raise. A machine declaring `liveliness:` must carry a `<transition event="error.communication">` handler in its SCXML; codegen otherwise rejects the build (`feedback_silently_broken_hooks` — see [`sce-build/src/generator.rs::reject_liveliness_without_handler`]). The gate is symmetric for row 8 (`PEER_PARTITIONED`, machine-level) and row 13 (`REGION_PARTITIONED`, partition-level) since both raises flow through `error.communication`.

**Transport-compat gate (Zenoh-only in current SCE)**. The codegen template emits liveliness primitives (token + subscriber + callback-driven `peer_last_seen_` table) only when `"zenoh"` appears in the machine's transport set. Declaring `liveliness:` on a machine that has no Zenoh binding or server would pass the handler-presence gate yet produce zero transport-layer observer code — the required `error.communication` handler would compile but never fire, matching the `feedback_silently_broken_hooks` anti-pattern exactly. [`sce-build/src/mesh/deploy.rs::validate_liveliness`] therefore rejects any machine that declares `liveliness:` without at least one `transport: zenoh` binding or server ([`DeployError::InvalidLiveliness`] with a reason naming Zenoh specifically). Authors hitting this rejection either add a Zenoh binding/server or drop the `liveliness:` section until a non-Zenoh liveness path lands. The **SOME/IP `register_availability_handler` per-partition app** variant named in §16.9's Session E2 list is a separate landing — it requires per-partition application-name allocation, OEM `vsomeip.json` coordination (§13 — vsomeip.json is OEM-owned, SCE-build must not rewrite it), a Service-Discovery enablement decision (current `mesh_someip_*` fixtures set `service-discovery.enable=false` because they are point-to-point static-configured), and a §10.4 transport-contract micro-revision that the Zenoh path did not need. Until that design round lands (§16.9 F candidate), the validator gate above keeps the silent-broken window closed.

### 16.5 Parallel `<final>` barrier

W3C §3.7: when every child region of a `<parallel>` reaches `<final>`, the enclosing session raises `done.state.<PARALLEL_ID>` with optional donedata. Distribution requires a **convergence barrier** across partitions:

1. The **root partition** for each `<parallel>` is designated at deploy time via `partitions.<name>.hosts_parallel_roots: [{machine, parallel}]` (§14 rule 12) and owns a `ParallelCompletionTracker` for that `<parallel>`. A `<parallel>` whose regions live entirely in a single partition has that partition as implicit root; a `<parallel>` whose regions span two or more partitions requires exactly one explicit claimant. One partition may claim multiple parallels' roots (collapsing tracker ownership into a single per-machine process) or different partitions may each claim a subset of parallels (distributing tracker ownership) — rule 12 constrains per-`<parallel>` uniqueness of claimants, not per-machine cardinality of root-hosting partitions.
2. **Emission timing**: when the region's `<final>` `<onentry>` executable content completes — i.e., after the final microstep of the macrostep that transitioned into `<final>`, and **before** the region's scheduler yields control — the region partition emits a `ParallelRegionDone` control envelope (wire value 21, dedicated pattern — **not** an overload of another wire value) with:
   - `parallel_id` (CBOR key 16) and `region_id` (CBOR key 17) — typed string fields; senders MUST set BOTH on every wire-21 envelope, receivers silently drop any envelope missing either field (sender contract violation). Dispatcher routes on `pattern` + typed `parallel_id`, not on string-concat parsing of `subject`.
   - `data` = donedata payload computed from the region's `<final>` `<donedata>` (absent if none declared)
   Emission is single-shot per region activation: once a region has emitted, further microsteps within that activation do not re-emit. Re-entry of the parallel (via history or new enter-set computation) resets the tracker and starts a fresh activation.
3. The root partition's tracker records the completion. When all regions of `<parallel_id>` have reported, the root partition raises `done.state.<PARALLEL_ID>` into its **own external queue** at its current macrostep boundary, so W3C §3.7 ordering (done.state raised at the next stable configuration after the final region completes) is preserved from the root's perspective.
4. **Barrier timeout**: a configurable timer arms at the **first region completion** (local or remote) and re-arms on every subsequent completion before threshold — each re-arm captures a fresh `missing_regions` set so the eventual `_event.data` matches the tracker's state at fire time. If not all regions report before the timer expires, the root raises `error.communication` with `reason="PARALLEL_BARRIER_TIMEOUT"` and `_event.data` carrying `{parallel_id, missing_regions: [string], timeout_ms}` (§16.7 row 6). Timeout defaults to **infinity** (W3C normative — parallel final waits indefinitely); finite values are configured per-partition via deploy.yaml `partitions.<name>.barrier_timeout_ms` (§14).

**Runtime plumbing** (C++-only, §16.5 L3500):
- `deploy.yaml → PartitionDecl.barrier_timeout_ms → SCXMLModel.partition_barrier_timeouts[parallel_id] → state_machine.jinja2 Root branch → ParallelCompletionTracker::TimerHooks`. A Root partition with `barrier_timeout_ms:` set emits an `arm` / `cancel` / `on_timeout` closure triple that routes through the base engine's `PullScheduler`: `arm` calls `this->scheduleEvent(Event::Error_communication, …, "__sce_barrier_timeout_<pid>", json_bytes)`; `cancel` calls `this->cancelEvent("__sce_barrier_timeout_<pid>")`. The deterministic per-parallel send-id keeps cancel idempotent.
- **Observability gate**: a Root partition with `barrier_timeout_ms:` set whose SCXML has no `<transition event="error.communication">` is rejected at codegen (`sce-build/src/generator.rs::reject_barrier_timeout_without_handler`) — a set knob with no author-visible raise path is the silent-broken anti-pattern `feedback_silently_broken_hooks` forbids. The repair is to add the handler (optionally guarded on `_event.data.reason == 'PARALLEL_BARRIER_TIMEOUT'`) or to drop `barrier_timeout_ms:` from the partition declaration.
- **Scheduler pump**: the fire path relies on the base engine's `PullScheduler`, which is pulled by `tick()` — **not** `step()` alone. The `tick()` short-circuit is gated on `isGlobalFinalState()` (top-level `<final>` only; parent-presence check on `currentState_`), so a Root partition holding a `<parallel>` whose local region has reached a regional `<final>` while sibling regions are still pending continues to pump — which is exactly the window the §16.5 barrier timer needs. The companion `isInFinalState()` predicate keeps leaf semantics for SCXML-level queries and is not used as a "machine is done" guard. `pumpScheduledEvents()` remains public as a fine-grained drain helper (harnesses that interleave scheduler-only pulses with explicit `step()` sequencing), but normal polling loops should call `tick()` alone.

**Cancel propagation on barrier timeout — author responsibility**. Barrier timeout raises `error.communication` on the root partition's queue and does **not** propagate a cancel signal to region partitions. Slow-but-alive regions continue executing their microsteps; cleanup of in-flight region work is the author's responsibility via standard SCXML state-leave semantics — typically `<transition event="error.communication" target="parallel_abort"/>` on an ancestor state that exits the `<parallel>`. This is load-bearing for §16.2 weak equivalence: a distributed-only cancel that interrupted a region mid-macrostep would prevent that region's `<final>` `<onentry>` from executing in distributed mode while single-process execution would (eventually) run it, producing an observable state-sequence divergence §16.2 explicitly forbids. A future wire pattern explicitly abortting regions on timeout is therefore an explicit E2 non-goal (§16.9) — the minimal docs behaviour above is the architecturally correct move pending a consumer who accepts the weak-equivalence cost.

### 16.6 `<history>` in distributed parallel

Each region partition maintains its own `<history>` states locally. On parallel re-entry, each partition restores its own history without cross-partition coordination. Because (R1) forbids cross-region shared writable data, history states are region-local by construction.

### 16.7 `error.communication` raise policy

Runtime conditions that raise `error.communication`. Each condition pins a machine-readable `reason` code and the `_event.data` shape that carries it (extends §10.7.1 base schema).

| # | Condition | `reason` | Extra `_event.data` fields |
|---|---|---|---|
| 1 | Transport connect or reconnect failure | `TRANSPORT_UNAVAILABLE` | `transport: string`, `target: string` |
| 2 | Envelope `send()` returns error from transport API | `SEND_FAILED` | `transport: string`, `target: string`, `transport_error: string` |
| 3 | Reliable transport unable to deliver after configured retries | `DELIVERY_EXHAUSTED` | `transport: string`, `target: string`, `attempts: int` |
| 4 | Inbound envelope deserialization / schema validation fails | `ENVELOPE_CORRUPT` | `transport: string`, `codec: "cbor" \| "json" \| "typed" \| "raw"`, `position?: int` |
| 5 | Invoke child device unreachable (transport-level) | `INVOKE_CHILD_LOST` | `invoke_id: string`, `target: string` |
| 6 | Parallel barrier timeout (§16.5) | `PARALLEL_BARRIER_TIMEOUT` | `parallel_id: string`, `missing_regions: [string]`, `timeout_ms: int` |
| 7 | Envelope dedup window overflow (sustained rate exceeds window capacity) | `DEDUP_WINDOW_OVERFLOW` | `source: string`, `window_size: int` |
| 8 | Network partition detected (peer heartbeat or liveness probe fail) | `PEER_PARTITIONED` | `target: string`, `last_seen_ms_ago: int` |
| 9 | Transport backpressure queue full, outbound envelope dropped (§10.10 `OutboundBuffer` at `max_pending_per_target`) | `BACKPRESSURE_DROP` | `transport: string`, `target: string`, `queue_depth: int` |
| 10 | Peer rejected envelope due to authorization failure | `UNAUTHORIZED` | `target: string`, `transport_status?: string` |
| 11 | Inbound envelope reached an active `OrderingBuffer` without `sequence_no` (§10.6.3) | `MISSING_SEQUENCE` | *(baseline only — `source`, `envelope_id` carry the diagnosis)* |
| 12 | `OrderingBuffer` fast-forwarded past a missing sequence range after `gap_timeout` expired (§10.6.4) | `ORDERING_GAP` | `lost_seq_lo: uint64`, `lost_seq_hi: uint64` (inclusive range of skipped sequence numbers) |
| 13 | Peer region-partition's liveliness token transitioned to DELETE (§16.4 per-partition liveness). Orthogonal axis from row 8 — raises intra-machine when one OS process of a `<parallel>`-split machine exits while siblings remain. | `REGION_PARTITIONED` | `machine: string`, `partition: string`, `last_seen_ms_ago: int?` |

**Common fields (§10.7.1 baseline)**: `errorName: "communication"`, `reason: <one of above>`, `detail?: string`, `source?: string` (envelope `source` field for inbound conditions), `sendid?: string`, `envelope_id?: string`, `invoke_id?: string`. These are always available when relevant; the table above lists condition-specific additional fields.

**Delivery semantics**: `error.communication` is raised into the affected machine's external queue (W3C §5.10) and delivered at the next macrostep boundary. Multiple conditions observed within a single microstep produce multiple events (one per condition); coalescing is not permitted because authors rely on one-to-one condition-to-event mapping.

**Scope of synthesis**: `error.execution` events arising from invoke setup (§9.3) or document-level faults retain their W3C-standard semantics. Distribution does NOT synthesize new `error.execution` occurrences beyond what a single-process engine would raise — the only new event class distribution introduces is `error.communication` from the catalogue above.

**Out of scope — mesh-rpc author-level deadline expiry**: `<invoke type="sce:mesh-rpc">` with a `_mesh_deadline_ms` (§9.5 L1318) that elapses before the reply arrives is **not** a §16.7 condition. Per §9.5 L1347 it surfaces as `error.invoke.<id>` with `rpc_status = DeadlineExceeded`, matching the W3C invoke lifecycle. The catalogue above lists transport-layer faults the runtime synthesizes on the peer-observing side; a scheduler-fired local deadline on the caller side is an author-level RPC lifecycle event, not a transport fault, and reaches the document through the invoke-lifecycle channel the author already wires for non-`Ok` `rpc_status`.

**Unknown transport errors**: A transport impl MUST map native errors to one of the catalogue reasons. Unclassifiable errors map to `SEND_FAILED` with `transport_error` carrying the raw transport-native string; `detail` carries a human-readable description. The catalogue is closed at this section — new reasons require a spec revision.

### 16.8 Conformance test harness

**Status (2026-04-18)**: Session E2 scope per §16.9 — not yet implemented. None of the artifacts named in §16.8.1–16.8.4 exist in the tree: no `tests/w3c_distributed_manifest.yaml`, no `tests/w3c/dist/` tree, no `run_distributed.py` driver, no `w3c_distributed_conformance` ctest label, and none of the 44 mesh ctest fixtures spawn per-partition OS processes or cross-compare a distributed run against a single-process reference. The sub-sections below describe the Session E2 target shape; [`docs/SCE_MESH_CONFORMANCE_MATRIX.md`](../docs/SCE_MESH_CONFORMANCE_MATRIX.md) is the day-0 map of transport primitives and mesh-runtime invariants the harness will consume once built.

SCE Mesh's conformance harness runs the full W3C IRP suite twice per test: once single-process, once distributed. Identical verdicts in both modes is the pass criterion.

#### 16.8.1 Harness architecture

```
irp_runner
  |
  |-- for each test t in tests/w3c/*.scxml:
  |    |
  |    |-- mode = single_process:
  |    |      load t, run via standard AOT engine,
  |    |      collect { final_config, log_output, donedata, raised_errors }
  |    |
  |    |-- mode = distributed:
  |    |      resolve partition plan via tests/w3c_distributed_manifest.yaml
  |    |      for each partition p:
  |    |         spawn ./build/tests/w3c/dist/test_<t>_<p> as OS process
  |    |      wait for all partitions with global deadline (default: 10x local time)
  |    |      collect outputs from each partition, merge via §16.2 equivalence
  |    |
  |    |-- compare single_process vs distributed under §16.2 equivalence
  |    |-- emit PASS / FAIL with diagnostic on mismatch
```

#### 16.8.2 IRP distributable subset

Not every IRP test has distributable structure. Tests without `<parallel>` or `<invoke>` have no W3C-orthogonal split axis; their "distributed" execution is trivially N=1 and adds no signal. These tests remain in the **single-process conformance set**.

Classification in `tests/w3c_distributed_manifest.yaml`. The `distributable` field takes one of four literal values — no "conditional" — so the CI report never conflates an analyzer-merged test with a genuinely distributed one:

| Label | Meaning | Counted toward distributed acid test? |
|---|---|---|
| `yes` | Author-declared partition plan passes analyzer with 2+ effective partitions; test runs N ≥ 2 OS processes. | **Yes** |
| `merged_single_partition` | Analyzer took the path (applied R1/R2), but the result is 1 effective partition. Runs as single process; distributed mode has no new signal. | **No** (reported separately as "analyzer-exercised") |
| `no` | No `<parallel>`, no `<invoke>` — no orthogonal split axis exists. Single-process only. | **No** |
| `forbidden` | Author's partition plan violates R1/R2 in strict mode. Test does not compile with distribution. | **No** (CI fails if a `yes` test regresses to `forbidden`) |

```yaml
# W3C IRP 202 distributed conformance manifest

tests:
  test144:
    distributable: no
    reason: "no <parallel>, no <invoke> — no orthogonal axis"

  test187:
    distributable: yes
    partitions:
      main:    { contains: { parallel_regions: [{ region: p1 }] } }
      worker:  { contains: { parallel_regions: [{ region: p2 }] } }
    inferred_constraints: []

  test230:
    distributable: merged_single_partition
    reason: "writes shared ancestor 'var1' from both regions (R1 violated)"
    effective_partitions:
      merged:
        contains: { parallel_regions: [{ region: p1 }, { region: p2 }] }

  test216:
    distributable: yes
    partitions:
      parent:        { contains: { invokes: [] } }
      child_process: { contains: { invokes: [{ invoke: sub1 }] } }
    notes: "Session F: validates remote invoke lifecycle across processes."
```

CI output example:

```
Distributed conformance report
  yes                        : 42 tests PASS / 42
  merged_single_partition    : 7 tests (analyzer-exercised, no N≥2 signal)
  no                         : 148 tests (single-process only, out of scope)
  forbidden                  : 0 tests
  REGRESSIONS                : 0
```

The acid-test claim is measured against the `yes` bucket only. `merged_single_partition` is a reported-but-not-counted category — useful to verify the analyzer fires on known-shared-data fixtures, but not evidence of distribution.

#### 16.8.3 Transport selection for harness

The harness defaults to `custom_tcp` for inter-partition traffic. `custom_tcp` is a minimal conformance-complete transport (§10.4) built specifically for the harness — zero external dependencies, UUID-v7 envelope framing over TCP stream, local-only (IPv4 loopback). Implementation lives in `sce/mesh/transports/custom_tcp/`.

Alternate transports may be selected per-test via manifest `transport_override: someip | zenoh | shm` for cross-transport regression coverage, but do not change the conformance claim — only `custom_tcp` is the reference path for the CI claim.

**Two-process harness (cross-device §9.6 fixtures, §9.6 Session 3 Stage A4 foundation, 2026-04-24).** Cross-device fixtures exercise a real OS-process boundary rather than a two-router-single-process hybrid. The orchestrator is a bash script (`tests/mesh/run_two_process_fixture.sh`) registered via the CMake helper `sce_register_two_process_mesh_test` (`tests/cmake/two_process_test.cmake`). Handshake protocol: (1) the worker binds `Server` to `"127.0.0.1:0"` (ephemeral) and reads back its actual port via `Server::local_endpoint()` (A2); (2) the worker writes one `LISTEN_ENDPOINT=host:port` line to stderr and blocks until SIGTERM; (3) the orchestrator greps the line out of worker stderr (handshake timeout default 5000 ms, overridable via `SCE_TWO_PROCESS_HANDSHAKE_MS`), exports it as `MESH_PEER_ENDPOINT`, and launches the parent; (4) the parent reads `MESH_PEER_ENDPOINT` and feeds it into `TransportRouter::init(PortOverride{ peer_connect_endpoints = {{"worker", endpoint}} })` (A3) so its `Client` dials the kernel-assigned port instead of the deploy.yaml `"host:0"` placeholder; (5) after the parent exits the orchestrator waits 300 ms for the kernel buffer to drain, then SIGTERMs the worker (2 s grace before SIGKILL) and propagates the parent's exit code. `mesh_two_process_smoke` (`tests/mesh/test_two_process_smoke_{worker,parent}.cpp`) exercises the orchestration path using `CustomTcp::Server`/`Client` directly — Stage B cross-device fixtures swap the plain sockets for generated `TransportRouter` pairs without reworking the handshake.

**Two-host harness (cross-device §9.6 fixtures, §9.6 Session 5b foundation, 2026-04-25).** Sister to the two-process harness for transports whose discovery mechanism cannot run over `127.0.0.1` alone. SOME/IP service discovery routes via UDP multicast (default 224.244.224.245); Zenoh peer-mesh discovery convergence behaves differently when both peers share one routing table — both demand a real network stack with distinct IPs, distinct ARP caches, distinct routing tables, and a wire that carries multicast end-to-end. The harness mirrors `tc8-harness/mock_dut/env/setup-netns.sh`: `tests/mesh/setup_crossdev_netns.sh` creates `sce-mesh-parent` (172.16.10.1) and `sce-mesh-worker` (172.16.10.2) netns linked by a veth pair, adds the `224.0.0.0/4` multicast route on both sides (vsomeip's default SD destination falls inside it; without the route the routing manager flips into a "no peers" state on first sendto), and runs a `parent → worker` ping as the reachability sanity check; `tests/mesh/cleanup_crossdev_netns.sh` is the idempotent teardown. `tests/mesh/run_two_host_fixture.sh` (registered via `tests/cmake/two_host_test.cmake`'s `sce_register_two_host_mesh_test()`) is a sister of `run_two_process_fixture.sh` — same worker-first stderr `LISTEN_READY` barrier, but each binary is launched via `ip netns exec <ns>` and a configurable post-`READY` settle (`SCE_TWO_HOST_SETTLE_MS`, default 500 ms) covers SOME/IP-SD / Zenoh peer-mesh convergence on the worker→parent direction before the parent process starts. **Privilege handling**: the orchestrator self-elevates via `exec sudo -E "$0" "$@"` when `sudo -n true` succeeds (passwordless sudo is configured) so the whole script runs under one root UID and SIGTERM teardown of root children works without further escalation; it skips with `exit 77` (the ctest `SKIP_RETURN_CODE`) when neither root nor passwordless sudo is available, so a fresh `cmake .. && ctest` in a non-root checkout stays Skipped not Failed. Recommended dev-box config is `<user> ALL=(ALL) NOPASSWD: ALL` in sudoers (one-time `sudo visudo`); after that plain `ctest -R mesh_.*_scxml_invoke_crossdev` runs as the regular user. The alternative without sudoers config is `sudo ctest -R ...`. The fixtures are gated behind the `SCE_ENABLE_NETNS_TESTS` opt-in option (default OFF) and document the one-time `sudo tests/mesh/setup_crossdev_netns.sh` step in the option help text. The two-tier shape — loopback `custom_tcp` (Session 3 Stage B) + netns `someip`/`zenoh` (Session 5b) — reflects each transport's discovery requirements rather than arbitrary fragmentation: TCP-only transports work over loopback; multicast / peer-mesh transports do not.

#### 16.8.4 Harness build integration

Per test `tests/w3c/testXXX.scxml`:
1. `sce-build` with the distributed manifest entry produces N binaries `testXXX_<partition>` in `build/tests/w3c/dist/`.
2. A harness driver `tests/w3c/dist/run_distributed.py` (or C++ test) invokes each binary, sets up inter-partition transport endpoints, and collects outputs.
3. Outputs are merged via §16.2 equivalence and compared to the single-process run.
4. CTest invokes the harness driver as a single test label `w3c_distributed_conformance`, which expands into one ctest per IRP test.

### 16.9 Incremental delivery: Sessions E1, E2, F

The spec above describes the target state. Implementation is split into three sessions to keep each session's scope tractable; no single session attempts to land all of distribution, purity correction, and full remote invoke at once. Splitting is purely an implementation-sequencing choice — the normative content of this §16 does not change between sessions.

**Session E1 (SCXML purity + mesh-rpc correction)** — no distribution machinery yet:
- Remove `sce:pattern`/`sce:reply-event`/`sce:reply-timeout`/`sce:qos`/`sce:deadline`/`sce:priority` from parser and model; migrate Session C/D tests.
- §14 deploy.yaml external-config integration (`vsomeip.json`, `zenoh.json5`) + 3-way consistency check.
- §13 topology-inferred request/response pairing.
- §9.5 `<invoke type="sce:mesh-rpc">` full lifecycle with `_mesh_*` reserved-name enforcement.
- §13 subscription dual-lifecycle (state-entry auto-symmetry with the eligibility rules above, and deploy.yaml `subscriptions:` for machine-lifetime).
- `docs/MESH_SCXML_COMPATIBILITY.md` published.

**Session E2 (distributed conformance foundation)** — adds the partition machinery and `<parallel>`-only IRP coverage:
- §10.4 Transport Contract and `custom_tcp` reference transport.
- §10.5 mesh runtime dedup layer.
- §10.7 `_event` field wiring for distributed events, including the structured `error.*` convention (§10.7.1).
- §10.8 sender-hold delayed send + cancel.
- §14 `partitions:` schema + deploy.yaml partition resolver with explicit coverage rule.
- §16.3/16.4 distributability analyzer (R1–R4) + cross-region transition auto-merge.
- §16.5 parallel `<final>` barrier runtime, using the dedicated `ParallelRegionDone` wire value 21.
- §16.4 region-partition liveness signalling — **Zenoh path landed** via the §16.7 row 13 `REGION_PARTITIONED` reason code + per-partition token at `sce/live/<machine>/<partition>` + segment-count-discriminating subscriber (§16.4 reference implementation). Codegen rejects any machine that declares `liveliness:` without an `error.communication` handler in the SCXML (`reject_liveliness_without_handler`), covering both row 8 and row 13 under a single symmetric gate. Deploy-validator rejects any machine that declares `liveliness:` without a Zenoh binding or server ([`validate_liveliness`]'s transport-compat check) so SomeIP-only or binding-less machines cannot silently opt into a signal the template emits zero code for. The **SOME/IP `register_availability_handler` per-partition app** variant remains deferred (§16.9 Session F candidate) — it needs a per-partition application-name scheme, OEM `vsomeip.json` coordination (§13), a Service-Discovery enablement decision, and a §10.4 transport-contract micro-revision that the Zenoh path did not need. The validator gate keeps the silent-broken window closed until that landing.
- §16.5 barrier-timeout cancel propagation remains an **explicit non-goal** for E2. The root's `error.communication` raise is the sole signal; region-partition cleanup stays author-driven via standard state-leave semantics. Introducing a distributed cancel wire pattern would diverge distributed state sequences from single-process ones (§16.2 weak-equivalence violation) and is deferred until a consumer accepts that trade-off.
- §16.7 `error.communication` raise policy and catalog.
- §16.8.1–16.8.3 harness architecture and `custom_tcp` harness transport.
- IRP distributable manifest covering tests that use only `<parallel>` (no remote `<invoke type="scxml">`). Expected ~20 tests in the `yes` bucket, a handful in `merged_single_partition`, remainder `no`.

**Session F (full remote `<invoke type="scxml">` + complete IRP coverage)**:
- §9.6 full remote session establishment (InvokeStart / InvokeStarted / ChildEvent / ParentEvent / InvokeDone / InvokeCancel / InvokeError wire patterns 14–20).
- §9.6.3 `_event.invokeid`/`origin` wiring for child events.
- §9.6.4 `<finalize>` execution on child events at parent's macrostep.
- §9.6.5 `autoforward="true"` parent → child forwarding.
- §9.6.6 inline `<content>` precompilation in `sce-build`, with the `__sce_synth_invoke__` collision check.
- Extension of the IRP distributable manifest to cover tests using `<invoke type="scxml">`.
- Foreign processor compatibility harness (exercise `error.execution` graceful degrade against an external SCXML 1.0 reference interpreter).
- Mesh Conformance Suite: distributed-only tests exercising §10.4/10.5/10.8/16.5/16.7 edge cases not covered by W3C IRP.

**Session-scoped acid tests** are in the §13 roadmap table (one row per session). The overall conformance claim of §16.1 is satisfied only when Session F lands; E1 and E2 deliver subsets of the claim against documented subsets of the IRP suite.

### 16.10 Relationship to W3C SCXML 1.0 Normative Text

The distributed execution model in this section is compatible with W3C SCXML 1.0 as follows:

- All normative constructs are supported with identical observable behavior, subject to §16.2 weak equivalence.
- No normative behavior is altered; distribution only refines the physical execution model, not the language.
- Extensions (`<invoke type="sce:mesh-rpc">`, `sce:mesh-rpc` as an invoke type URI, and `partitions:` in deploy.yaml) are confined to SCE-specific namespaces and the deploy.yaml file — they do not introduce new SCXML syntax beyond a single implementation-defined invoke type URI (permitted by W3C §6.4).
- A foreign W3C SCXML 1.0 processor loading the same SCXML file executes it as a single-process monolith with identical observable behavior, except for constructs using `<invoke type="sce:mesh-rpc">` (which raise `error.execution` per §6.4.1, handleable by the author).

This completes the conformance claim.

---

## 17. Distributed-Friendly SCXML Design Principles

This section is a design guide for SCXML authors who want their documents to be distributable while preserving AOT performance. It is normative for SCE Mesh tooling defaults (the analyzer encodes these rules) but advisory to authors.

### 17.1 Why design matters for AOT + distributed

AOT compilation reduces state transition cost to ~1-720 ns (§11.1). Transport overhead for a remote event is 50 μs – 2 ms (§11.2) — roughly **100–1,000,000× more expensive**. The arithmetic of distributed AOT performance is therefore dominated by a single question: **what fraction of event traffic crosses partition boundaries?**

A well-designed SCXML document keeps its **hot path** inside one partition and places only **rare, semantically-important boundaries** at partition edges. A poorly-designed document creates shared writable state between regions or frequent cross-region transitions, forcing the analyzer to merge partitions back or (in strict mode) fail the build.

**The same AOT engine that powers single-process sub-microsecond transitions powers each partition in distributed mode.** Distribution does not slow down intra-partition execution; it adds cost only at boundaries. Every good SCXML design principle for single-process performance (locality, minimal shared state, event-driven composition) is **identical** to the principle for distributed performance.

### 17.2 Five principles

**P1 — Actor identity per region/machine.** Treat each `<parallel>` region and each `<invoke>` child as an actor: encapsulated state, message-only interface. Do not reach into a sibling region's `<data>`.

**P2 — Local-state rule: declare `<datamodel>` at the narrowest scope.** The `<data>` that a region mutates should live in that region's own `<datamodel>`. The ancestor scope holds only immutable configuration and values written by exactly one region.

**P3 — Event-driven composition.** Regions and machines coordinate through events, not through shared variables. Where single-process code might read a sibling's counter, distributed-friendly code receives a `counter.increment` event.

**P4 — `<invoke>` as service boundary.** When a subsystem has its own long-lived state and lifecycle — a sensor driver, a worker pool, a subordinate controller — model it with `<invoke>`. `<invoke>` is the natural process-boundary marker: it maps cleanly to a child partition, preserves W3C `<finalize>`/`done.invoke`/`<cancel>` semantics across processes, and keeps the parent's RTC uncoupled from the subsystem's.

**P5 — Respect region boundaries with transition targets.** Do not `<transition target>` to a state inside a sibling region. If control flow needs to coordinate two regions, use an event. If you find yourself wanting a cross-region transition, that is a signal the two states belong to the same region — the parallel decomposition is wrong.

### 17.3 Good vs bad patterns

#### Shared counter — bad

```xml
<parallel>
  <datamodel><data id="shared" expr="0"/></datamodel>
  <state id="producer">
    <transition event="tick"><assign location="shared" expr="shared + 1"/></transition>
  </state>
  <state id="consumer">
    <transition cond="shared > 10"><assign location="shared" expr="0"/></transition>
  </state>
</parallel>
```

Violates R1 (both regions write `shared`). Analyzer auto-merges into a single partition — no distribution benefit. Forcing distribution would require a coordinator, destroying the AOT performance advantage.

#### Shared counter — good

```xml
<parallel>
  <state id="producer">
    <datamodel><data id="count" expr="0"/></datamodel>
    <transition event="tick">
      <assign location="count" expr="count + 1"/>
      <send event="counter.inc"/>
    </transition>
  </state>
  <state id="consumer">
    <datamodel><data id="received" expr="0"/></datamodel>
    <transition event="counter.inc"><assign location="received" expr="received + 1"/></transition>
    <transition cond="received > 10" target="done"/>
  </state>
</parallel>
```

Region-local data, event-only coordination. Distributable. Producer's hot path is `<assign>` + single envelope emit (~1 ns + ~50 μs on remote transport). Consumer's hot path is `<assign>` (pure local). The 10:1 ratio of locally-observed ticks to remote increments means ~90% of work stays AOT-fast.

#### Subsystem lifecycle — bad

```xml
<state id="brake_control">
  <datamodel>
    <data id="motor_torque" expr="0"/>
    <data id="motor_temperature" expr="0"/>
    <data id="motor_duty_cycle" expr="0"/>
    <data id="motor_fault_code" expr="0"/>
  </datamodel>
  <transition event="motor.telemetry">
    <assign location="motor_torque" expr="_event.data.torque"/>
    <assign location="motor_temperature" expr="_event.data.temp"/>
    <!-- ... -->
  </transition>
</state>
```

Parent's datamodel grows with every attribute of the subsystem. Every telemetry event is an external event for the parent — cannot be locally-scoped.

#### Subsystem lifecycle — good

```xml
<state id="brake_control">
  <invoke type="scxml" src="#motor_subsystem" id="motor">
    <finalize>
      <assign location="motor_status_summary" expr="_event.data.summary"/>
    </finalize>
  </invoke>
  <transition event="motor.fault" target="fault_recovery"/>
</state>
```

Motor internals live in the invoked child. Parent sees only high-level events (`motor.fault`) and summary snapshots via `<finalize>`. In distribution, `motor_subsystem` becomes a separate partition — parent/child boundary is natural. Single-process execution is unchanged.

### 17.4 Data locality rules of thumb

- **Hot path locality**: hot event streams (sensor at 1 kHz, game tick at 60 Hz, trading feed) must be serviced by a region that owns all the data they touch. If a hot event triggers a cross-partition read, AOT performance is lost on every tick.
- **Cold path tolerance**: rare events (fault escalation, configuration change, human command) may cross partitions freely. 50 μs once per second is invisible; 50 μs per tick is catastrophic.
- **Snapshot everything immutable**: configuration and sensor calibration at startup, not in the hot path. Region partitions receive snapshots at parallel entry (§16.3 R3); deployments provide constant configs once at process start.
- **One writer per datum**: if two pieces of logic update the same value, combine them into one region or one machine. Multi-writer discipline is harder than event-driven composition and throws away W3C's sequential-consistency guarantee.

### 17.5 When to pick `<parallel>` vs `<invoke>`

Both split state machine logic into concurrent units. Use this heuristic:

| Aspect | `<parallel>` region | `<invoke>` child |
|---|---|---|
| Lifecycle | Active whenever the enclosing parallel is active | Independently started/ended via `<invoke>`/`<cancel>` or child `<final>` |
| Shared datamodel | Ancestor scope (limited writes per R1) | None (own datamodel) |
| Best for | Concurrent aspects of a single entity (e.g., a car's braking + steering monitors simultaneously) | Subsystem with its own lifecycle (e.g., a worker job, a long-running sensor, a remote service) |
| Distribution fit | Natural for symmetric concurrent aspects | Natural for asymmetric client-subsystem relationships |
| Termination | All regions run until the parallel itself exits | Child exits on own `<final>`, on parent-issued `<cancel>`, or on invoking-state exit |

Concretely: **if the two things have the same lifetime, use `<parallel>`. If one thing's existence is conditional on the other, use `<invoke>`.**

### 17.6 Design checklist before distribution

Before adding `partitions:` to a deploy.yaml:

1. Run `sce-build --analyze-partitions <machine.scxml>`. It prints the dependency graph, shared-data violations, and cross-region transitions.
2. For every violation, choose: move `<data>` to a narrower scope, replace shared writes with events, or collapse the regions (they were never truly orthogonal).
3. Once the analyzer reports zero violations, the document is partition-ready. Any partition plan that respects the analyzer's constraint groups will compile.
4. Benchmark both modes. If distributed mode is slower by more than expected transport cost × cross-partition event rate, the hot path is crossing boundaries — revisit regions' data ownership.

### 17.7 Heterogeneous deployment as a first-class use case

A well-designed SCXML document supports radically heterogeneous deployments from the same source:

- **Single-process**: entire document in one binary. IRP tests, unit tests, monoliths.
- **Multi-process same host**: partitions on one machine via SHM or `custom_tcp`. Load distribution across cores; process-level fault isolation.
- **Multi-host same LAN**: partitions across devices via SOME/IP, DDS, or Zenoh. Automotive ECUs, factory-floor PLCs.
- **Hybrid edge/cloud**: hot-path partitions on the edge, rare/bulky partitions in the cloud via gRPC. IoT, telemetry.
- **Security-isolated**: sensitive partition (authentication, crypto) in a TEE/SGX enclave; rest in normal-world. Zero-trust architectures.

Each deployment is a different `deploy.yaml` against the **same SCXML source** and the same AOT-generated binaries (per target). This is the payoff of distributed-conformance discipline: one logical design, many physical realizations, no runtime interpretation.

