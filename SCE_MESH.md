# SCE Mesh: Distributed SCXML State Machine Framework

## 1. Vision

### Problem

Distributed systems require state machines that span multiple devices, processes, and networks. Today, developers hand-code state logic on top of communication middleware (Zenoh, SOME/IP, gRPC), resulting in:

- Fragile if/else state management scattered across codebases
- No formal verification of cross-device state interactions
- Tight coupling between application logic and transport protocols
- Impossible to visualize or reason about system-wide behavior

### Solution

SCE Mesh extends the SCXML Core Engine with **location-transparent state machine communication**. The same SCXML that runs locally runs across devices, processes, and clouds — unchanged.

```
SCXML Author sees:           sce-build generates:
  <send target="#motor"/>      Local? → direct call (inlined)
                               Remote? → transport-native API call
                               Same ECU? → shared memory write
                               Different ECU? → SOME/IP, Zenoh, CAN native call
                               Cloud? → gRPC stub call
```

### Core Principle

**SCXML authors write business logic. Platform engineers configure deploy.yaml. Neither needs to know the other's domain.** SCXML declares behavioral intent; deploy.yaml declares platform-specific realization; sce-build generates transport-native code that directly calls each middleware's API — no runtime abstraction layer, no feature loss.

### Design Principle: Build-Time Resolution

SCE's core philosophy is: **resolve at build time what can be resolved at build time.** The AOT engine compiles state machines into switch/case at build time. The expression transpiler compiles ECMAScript expressions into target-language code at build time. Transport dispatch follows the same principle — `deploy.yaml` determines routing at build time, and sce-build generates code that calls transport APIs directly. No runtime indirection, no vtable overhead, and no loss of transport-native features (DDS QoS policies, SOME/IP service model, D-Bus object paths, etc.).

### Positioning

SCE Mesh does not compete with communication middleware. It sits above them.

```
Zenoh/SOME/IP/gRPC = roads     (how data moves)
SCE Mesh           = navigation (what to do when data arrives)
```

Value SCE Mesh adds on top of existing middleware:

- **Formal verification** of cross-device state interactions at build time
- **Visual design** of distributed behavior via statecharts
- **AOT code generation** eliminating runtime overhead
- **Transport independence** — same SCXML works over any protocol
- **Build-time topology analysis** — routing tables, serialization, proxies generated automatically

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
    cpp/shm_transport.h.jinja2    # Mesh: transport × language templates
    cpp/someip_transport.h.jinja2
    cpp/dds_transport.h.jinja2
    cpp/zenoh_transport.h.jinja2
    cpp/dbus_transport.h.jinja2
    ...
```

### Relationship to SCE Forge

SCE Mesh and SCE Forge are orthogonal extensions that compose naturally:

```
SCE Forge  = extends WHAT scxml-core-engine can generate (kinds: codec, transform, procedure, ...)
SCE Mesh   = extends WHERE generated code can execute (transports: SOME/IP, Zenoh, SHM, ...)
```

Key integration points:
- **Forge `codec` kind → Mesh serialization**: Codec-generated `encode()`/`decode()` are called directly in transport-generated serialization code — single source of truth for wire format (see Section 7.5)
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

Transport dispatch is resolved at **build time**, not runtime. sce-build reads `deploy.yaml` bindings and generates code that directly calls each transport's native API. There is no `ITransport` runtime interface — each transport has a dedicated **Jinja2 codegen template** that emits transport-native code.

#### Template Architecture

```
tools/codegen/templates/mesh/
  cpp/
    shm_transport.h.jinja2       # POSIX shared memory ring buffer
    someip_transport.h.jinja2    # vsomeip native API calls
    dds_transport.h.jinja2       # Cyclone DDS / RTI Connext native API
    zenoh_transport.h.jinja2     # zenoh-c / zenoh-pico native API
    can_transport.h.jinja2       # SocketCAN / AUTOSAR CAN native API
    dbus_transport.h.jinja2      # GDBus / sd-bus native API
    grpc_transport.h.jinja2      # gRPC stub calls
    local_transport.h.jinja2     # same-process direct call (inlined away)
```

Each template receives the full transport-native configuration from deploy.yaml and generates code that uses 100% of the target transport's features — no abstraction loss.

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
| `sce:qos="reliable"` send fails | Transport-native retry (DDS: reliability QoS, SOME/IP: method retry), then `error.communication` |
| `sce:deadline` exceeded | Timer-based enforcement in generated code, `error.communication` with `reason: "DEADLINE_EXCEEDED"` |
| Transport disconnected | Transport-native disconnect detection, `error.communication`, instance lifecycle → DRAINING |
| `sce:qos="best-effort"` send fails | Silent drop, no error event (fire-and-forget semantics) |

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

**Dynamic Discovery (runtime):** For environments where targets appear/disappear at runtime (cloud auto-scaling, game zone migration), a minimal `IDiscovery` runtime concept is provided in `sce_mesh_common`. This is Phase 5 scope.

```
Phase 1-3: Static (build-time) — constexpr routing tables
Phase 5:   Dynamic (runtime)   — IDiscovery concept for runtime resolution
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

#### Dynamic Discovery (Phase 5, Deferred)

For dynamic environments, `sce_mesh_common` provides a minimal discovery concept:

| Discovery Strategy | Mechanism | Domain |
|--------------------|-----------|--------|
| Transport-native SD | SOME/IP-SD, Zenoh scouting, mDNS | Each transport's built-in discovery |
| External registry | Consul, etcd | Datacenter |

Dynamic discovery generates code that calls the transport's native discovery API (e.g., `vsomeip::request_service()`, `zenoh::scout()`), preserving transport-specific discovery features. The codegen template emits callbacks that update the routing table at runtime when services appear/disappear.

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

**Best for**: Safety-critical automotive (ASIL-B/D), embedded systems.

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

### 4.4 Event Deduplication

Event deduplication applies **only when the discovery mode or transport can produce duplicates** (Dynamic mode with failover, multi-path delivery). It is not required for Static or Scoped modes with single-path routing.

**When enabled** (Dynamic mode, or explicit `sce:qos="reliable"` with failover):

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

## 5. QoS Model: Intent vs Realization

SCE Mesh separates QoS into two layers:

1. **SCXML (`sce:qos`, `sce:deadline`, `sce:priority`)** — behavioral intent. "This message needs reliable delivery." Stays transport-agnostic. Used for build-time validation.
2. **deploy.yaml (transport-native QoS)** — platform realization. "Reliable on DDS means Reliability::Reliable + Durability::TransientLocal + History::KeepLast(5)." Full transport-native feature access.

### SCXML QoS Attributes (Intent Layer)

```xml
<scxml xmlns:sce="http://sce.dev/ext">

  <!-- Intent: must deliver, within 1ms, highest priority -->
  <send event="brake.activate" target="#brake_ecu"
        sce:qos="reliable"
        sce:deadline="1ms"
        sce:priority="critical"/>

  <!-- Intent: loss acceptable, fast delivery -->
  <send event="npc.move" target="zone://forest"
        sce:qos="best-effort"
        sce:priority="normal"/>

</scxml>
```

| Attribute | Values | Role |
|-----------|--------|------|
| `sce:qos` | `reliable`, `best-effort` | Build-time validation hint |
| `sce:deadline` | Duration (e.g. `1ms`, `16ms`) | Build-time validation hint |
| `sce:priority` | `critical`, `high`, `normal`, `low` | Build-time validation hint |

These attributes **do not directly control generated code**. They are validation hints — sce-build checks that the deploy.yaml QoS configuration for each binding is consistent with the SCXML intent. For example, if SCXML declares `sce:qos="reliable"` but deploy.yaml configures `reliability: BEST_EFFORT`, sce-build emits a **build-time warning**.

If `sce:qos` attributes are omitted, no validation occurs — deploy.yaml QoS is used as-is.

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

sce-build cross-references SCXML intent with deploy.yaml realization:

| SCXML Intent | deploy.yaml Check | Result |
|-------------|-------------------|--------|
| `sce:qos="reliable"` | DDS `reliability: BEST_EFFORT` | **Warning**: intent/config mismatch |
| `sce:deadline="1ms"` | DDS `deadline: 100ms` | **Warning**: deadline exceeds intent |
| `sce:qos="reliable"` | SOME/IP `protocol: TCP` | OK — TCP provides reliability |
| `sce:qos="best-effort"` | CAN (inherently best-effort) | OK — matches |
| No `sce:qos` attribute | Any config | OK — no validation, user takes responsibility |

### Shared `sce:` Namespace with SCE Forge

SCE Mesh and SCE Forge share the unified `sce:` extension namespace (`http://sce.dev/ext`). A single `<send>` element may carry attributes from both subsystems:

```xml
<send sce:service="SecurityAccess" sce:subfunc="0x01"
      sce:qos="reliable" sce:deadline="5ms"/>
<!--   ^^^^^^^ Forge (codegen)  ^^^^^^^ Mesh (codegen) -->
```

**Ownership rule**: All `sce:` attributes are now processed at **build time**. SCE Forge attributes (`sce:kind`, `sce:type`, `sce:service`, etc.) control Forge codegen output. SCE Mesh attributes (`sce:qos`, `sce:deadline`, `sce:priority`) serve as build-time validation hints cross-referenced against deploy.yaml. Neither subsystem reads the other's attributes. See SCE_FORGE.md Section 3.6 for the full attribute classification table.

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

To add a new transport, create a Jinja2 codegen template:

```
tools/codegen/templates/mesh/cpp/my_transport.h.jinja2
```

The template receives:
- `bindings`: list of target bindings from deploy.yaml
- `qos`: transport-native QoS configuration
- `transport_config`: any transport-specific settings from deploy.yaml

And emits:
- `init_transports()`: transport initialization code
- `send_to_<target>()`: per-target send functions calling native API
- `subscribe_<pattern>()`: subscription setup calling native API
- `on_error_<target>()`: error handler generating `error.communication` events

```
// Compile error if template is missing:
// "No codegen template found for transport 'my_transport'. 
//  Expected: tools/codegen/templates/mesh/cpp/my_transport.h.jinja2"
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
  - sce:qos intent matches deploy.yaml QoS config
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

Serialization is generated per-transport. Different transports have fundamentally different data encoding requirements:

- **Buffer-based** (gRPC, SOME/IP, SharedMem): Variable-length byte streams
- **Signal-based** (CAN, LIN): Bit-level packing into fixed-size frames

Each transport template generates serialization code appropriate to the wire format — no runtime `ISerializer` interface:

#### Type Information Source

W3C SCXML is typeless — `<param name="brake_force" expr="brakeForce"/>` carries no type information. The build tool requires an external type source to generate serialization code. Three sources are supported, in priority order:

1. **SCE Forge `codec` kind** (preferred when available): If an event payload has a corresponding `sce:kind="codec"` SCXML file, the transport template generates serialization code that directly calls the codec's `encode()`/`decode()` methods. The codec SCXML becomes the single source of truth for both local byte parsing and remote event serialization. No `events.yaml` entry is needed for codec-backed payloads.
2. **Buffer-based transports**: Types are declared in an **event schema file** (`events.yaml`) alongside `deploy.yaml`. This is the fallback for payloads without a codec kind definition.
3. **Signal-based transports (CAN, LIN)**: Types, scaling, offsets, and bit layouts are imported from standard automotive database files (`.dbc`, `.arxml`, AUTOSAR system description). This is Phase 3 scope.

```
Type resolution priority:
  1. SCE Forge codec kind (sce:kind="codec")  → generated struct with encode/decode
  2. events.yaml                                → explicit type declarations
  3. .dbc / .arxml                              → automotive signal database
```

**SCE Forge codec integration**: When the build tool detects that a `<send>` event payload matches a Forge codec kind (by matching the codec's `<data id>` against the event's `<param>` structure), the transport template generates serialization code that calls the Forge-generated codec directly:

```cpp
// [generated] — transport template calls Forge-generated codec directly
void send_to_motor(const EventDescriptor& event) {
    // Forge-generated encode — single source of truth for wire format
    auto payload = MotorCutPowerCodec::encode(event.params());
    // Transport-native send — no serialization abstraction layer
    vsomeip_send(motor_service_, payload.data(), payload.size());
}
```

This eliminates type duplication between `events.yaml` and codec SCXML files. When both exist for the same event, the codec kind takes precedence and the build tool emits a warning about the redundant `events.yaml` entry.

**Data model type mismatch**: `events.yaml` declares static types (e.g., `float32`), but SCXML data models (particularly Lua) are dynamically typed. A Lua variable `brakeForce` could be integer or float at runtime. The mismatch strategy will be defined in Phase 2 implementation. Likely approach: runtime type coercion at the serialization boundary with build-time warnings when `events.yaml` types cannot be statically verified against the data model.

```yaml
# events.yaml — event payload type definitions (Phase 2+)
events:
  motor.cut_power:
    params:
      brake_force: { type: float32 }

  sensor.frame:
    params:
      timestamp: { type: uint64 }
      data:      { type: bytes, max_size: 4096 }

  brake.indicator.on: {}    # no payload
```

```yaml
# deploy.yaml — CAN signal import (Phase 3)
topology:
  brake_ecu:
    machines:
      brake:
        bindings:
          "#motor":
            transport: can
            address: "can0:0x100"
            signals: "vehicle.dbc"     # DBC file provides type/layout info
```

The build tool merges type information from `events.yaml` (buffer-based) or `.dbc`/`.arxml` (signal-based) with the SCXML `<send>` analysis to generate the correct serialization code.

Generated code example (buffer-based transport, types from `events.yaml`):

```cpp
// [generated] brake_events.h — types derived from events.yaml
namespace SCE::Generated::brake::events {

struct MotorCutPower {
    static constexpr auto NAME = "motor.cut_power";
    float brake_force;    // from events.yaml: float32

    void serialize(SCE::Mesh::Buffer& buf) const;
    static MotorCutPower deserialize(SCE::Mesh::BufferView buf);
};

}  // namespace SCE::Generated::brake::events
```

Generated code example (CAN signal-based transport, types from `.dbc`):

```cpp
// [generated] brake_can_signals.h — layout derived from vehicle.dbc
namespace SCE::Generated::brake::signals {

struct MotorCutPower {
    static constexpr auto NAME = "motor.cut_power";
    static constexpr uint32_t CAN_ID = 0x100;   // from DBC: message ID
    static constexpr uint8_t START_BIT = 0;      // from DBC: signal layout
    static constexpr uint8_t LENGTH = 16;        // from DBC: 16 bits
    static constexpr float SCALE = 0.1f;         // from DBC: scaling
    static constexpr float OFFSET = 0.0f;        // from DBC: offset

    void pack(uint8_t frame[8]) const;
    static MotorCutPower unpack(const uint8_t frame[8]);
};

}  // namespace SCE::Generated::brake::signals
```

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
- `done.invoke` delivery is not guaranteed without `sce:qos="reliable"`
- `<cancel>` is best-effort over unreliable transports
- Child cannot access parent's data model (no shared memory across devices)

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

**Scope commitment**: Phase 1-2 are the concrete implementation target. Phase 3-5 are directional plans that will be refined after Phase 2 delivers an end-to-end working demo. The first milestone is: **two processes on the same machine communicating via SCXML through shared memory, with no changes to the SCXML documents themselves.**

### Phase 1: Codegen Infrastructure + Local Transport

Extend sce-codegen with `--deploy` option and generate the trivial case (same-process direct call).

- **`--deploy` option for sce-codegen**: when `--deploy deploy.yaml` is provided, sce-codegen parses topology + bindings alongside SCXML and generates transport code in addition to SM code. Without `--deploy`, existing behavior is unchanged
- **deploy.yaml parser** (serde_yaml in Rust): topology, bindings, scheduler, QoS schema
- **Topology analyzer**: collect `<send>` targets from SCXML, match against deploy.yaml bindings, detect same-device vs cross-device boundaries
- **`local_transport` template**: same-process targets generate inlined direct calls (zero overhead, existing behavior preserved)
- **Scheduler concepts** in `sce_mesh_common`: `TickScheduling`, `EventDrivenScheduling` (C++20 concepts, C++17 fallback — same pattern as existing `StatePolicyConcepts.h`)
- **MPSC EventQueue bridge** in `sce_mesh_common`: thread-safe event queue for cross-thread event injection
- **Build-time validation**: topology completeness (all targets resolve), event coverage (all sent events have receivers), QoS intent/config consistency check
- **Verification**: existing single-process tests pass with `--deploy` codegen path, generated code is functionally identical to non-deploy AOT output

### Phase 2: Shared Memory Transport (first cross-process)

First transport template that crosses a process boundary.

- **`shm_transport` Jinja2 template**: generates ring-buffer shared memory code (POSIX `shm_open` / `mmap`)
- **Event serialization codegen**: per-transport serialization — SHM gets binary event encoding (not JSON)
- **SCE Forge codec integration**: if event payload matches a Forge `codec` kind, generated serialization wraps the codec's `encode()`/`decode()`. Single source of truth for both local byte parsing and remote event serialization
- **Receive-side codegen**: generated subscriber code that polls/waits on SHM ring buffer, deserializes events, injects into instance's EventQueue via MPSC bridge
- **Instance lifecycle**: REGISTERED → READY → ACTIVE → DRAINING → GONE (state machine for each deployed instance)
- **Error propagation codegen**: SHM failures (segment not found, peer crash) generate `error.communication` events
- **Verification**: two processes on same machine communicating via SCXML through generated shared memory code. SCXML documents are unchanged

### Phase 3: Vehicle Network Transport Templates (Directional — refined after Phase 2)

Transport templates for automotive protocols. Each template generates code that calls the protocol's native API directly, preserving all protocol-specific features.

- **`someip_transport` template**: generates vsomeip API calls — service offer/find, method call/fire-and-forget, event group subscription. Full SOME/IP service model preserved
- **`can_transport` template**: generates SocketCAN API calls — frame TX/RX, DBC signal packing. CAN FD support
- **`zenoh_transport` template**: generates zenoh-c/zenoh-pico API calls — put/subscribe, key expressions, SHM mode. Reusable for IoT/robotics
- **`dds_transport` template**: generates Cyclone DDS API calls — **all 22 QoS policies** from deploy.yaml passed through natively. Typed topics, content filters, partitions
- **deploy.yaml native QoS**: each transport section in deploy.yaml carries the full transport-native QoS configuration. sce-build passes it directly to the template without abstraction
- **Remote `<invoke>` codegen**: invoke request/response/cancel as generated send/receive pairs over configured transport
- **SCE Forge procedure integration**: Forge `procedure` kinds generate SM-compatible classes. Mesh remote `<invoke>` codegen executes these across device boundaries
- **Build-time verification**: interface match (sender event names == receiver transition triggers), cross-transport ordering warnings

### Phase 4: Game Scale (Directional — refined after Phase 3)

High-throughput batch processing for massive entity counts.

- `GameLoopScheduler` with SoA (Structure of Arrays) layout
- `udp_transport` + `grpc_transport` templates
- Batch event processing (group by event type)
- Zero-allocation event pool (arena allocator)
- ECS integration API
- Delta state synchronization (changed-state-only client updates)
- **Codegen trade-off**: SoA layout requires a second codegen mode alongside the existing per-instance AoS generation. Evaluated based on Phase 3 benchmarks

### Phase 5: Dynamic Discovery + Cloud (Directional — refined after Phase 4)

Dynamic environments where targets appear/disappear at runtime.

- **`IDiscovery` runtime concept** in `sce_mesh_common` — minimal runtime discovery for dynamic environments
- Dynamic discovery codegen: templates generate code that calls transport-native discovery (SOME/IP-SD, Zenoh scouting, Consul) and updates routing table at runtime
- Cross-transport bridging codegen: sce-build generates bridge functions that convert events between wire formats (e.g., SOME/IP payload → gRPC protobuf)
- Build-time verification: circular dependency detection, reachability analysis

### Future Direction (Beyond Phase 5)

The following capabilities are explicitly deferred. They will be evaluated after Phase 1-5 are validated in at least one production domain:

- **Saga pattern** — distributed compensation transactions as a separate orchestration layer above SCE Mesh
- **Consistency modes** — strong/eventual consistency guarantees for cross-instance state
- **Formal verification** — model checking integration (e.g., TLA+, UPPAAL) for safety-critical certification
- **Hot reload** — runtime replacement of AOT state machines without service interruption

---

## 14. deploy.yaml Schema

```yaml
# SCE Mesh Deployment Descriptor
version: "1.0"

scheduler:
  type: event_driven | real_time | game_loop | cooperative
  # Scheduler-specific settings (passed to scheduler constructor)
  cycle_ms: <integer>              # real_time, cooperative
  tick_rate: <integer>             # game_loop

topology:
  <device_name>:
    platform: linux | qnx | autosar | windows
    target: x86_64 | aarch64 | arm32
    machines:
      <scxml_name>:
        bindings:
          "<target_id>":
            transport: someip | can | zenoh | shm | dds | dbus | grpc | ...
            # Transport-specific settings — passed directly to codegen template
            # Each transport defines its own schema (see examples below)
            qos:
              # Transport-NATIVE QoS — full feature access
              # Schema depends on transport type (DDS: 22 policies, SOME/IP: protocol, etc.)
              <transport-native-key>: <value>

events: <path to events.yaml>              # Event payload type definitions

discovery:
  mode: static | dynamic                   # static = build-time, dynamic = Phase 5

# Transport-global configuration (optional)
<transport_name>:                          # e.g., zenoh:, someip:
  <transport-global-settings>              # Shared across all bindings of this type
```

### Example: Automotive

```yaml
version: "1.0"

topology:
  brake_ecu:
    platform: qnx
    target: aarch64
    machines:
      brake:
        bindings:
          "#motor":
            transport: someip
            service: 0x1001
            instance: 0x01
            method: 0x01
            protocol: TCP                    # SOME/IP native: TCP for reliable
          "#dashboard":
            transport: can
            address: "can0:0x100"
            signals: "vehicle.dbc"           # CAN native: DBC signal layout

  motor_ecu:
    platform: qnx
    target: aarch64
    machines:
      motor:
        bindings:
          "#brake":
            transport: someip
            service: 0x1002
            instance: 0x01

  dashboard_ecu:
    platform: linux
    target: aarch64
    machines:
      dashboard:
        bindings:
          "#brake": { transport: can, address: "can0:0x101" }
          "#cloud":
            transport: grpc
            address: "telemetry.oem.com:443"
            tls: true                        # gRPC native: TLS config

discovery:
  mode: static

someip:
  application_name: "sce_vehicle"
  routing_manager: "brake_ecu"               # SOME/IP native: routing config
```

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

**sce:qos validation**: If the SCXML declares `sce:qos="reliable"` but deploy.yaml sets `reliability: best_effort`, sce-build emits a build-time warning. The deploy.yaml value takes precedence for code generation.

#### Deadline Enforcement

Zenoh does not natively support delivery deadlines. The codegen template generates timer-based enforcement:

```cpp
// [generated] — deadline enforcement for sce:deadline="1ms"
void send_to_motor_with_deadline(const EventDescriptor& event) {
    auto timer = start_deadline_timer(std::chrono::milliseconds(1));
    z_put(session_, motor_key_, payload, len, &opts);
    if (timer.expired()) {
        inject_error_communication(event, "DEADLINE_EXCEEDED");
    }
}
```

For `sce:qos="best-effort"`, deadline violations are silently ignored (consistent with fire-and-forget semantics).

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
