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
SCXML Author sees:           Runtime handles:
  <send target="#motor"/>      Local? → direct call
                               Remote? → serialize → transport → deserialize
                               Same ECU? → shared memory
                               Different ECU? → SOME/IP, Zenoh, CAN
                               Cloud? → gRPC
```

### Core Principle

**SCXML authors write business logic. Platform engineers write transport plugins. Neither needs to know the other's domain.** The contract between them is three interfaces: `IScheduler`, `ITransport`, `IDiscovery`.

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
|                      AOT Code Generator                          |
|                (C++ / Rust / C per target)                       |
+=================================================================+
|                                                                  |
|   +-------------------------------------------------------+     |
|   |                SCE Core (invariant layer)              |     |
|   |                                                        |     |
|   |  +----------+  +----------+  +--------------------+   |     |
|   |  | SM Engine |  | Event    |  | Data Model         |   |     |
|   |  | (AOT)    |  | Router   |  | (Lua/ECMA/C)       |   |     |
|   |  +----------+  +----------+  +--------------------+   |     |
|   +------------------------+------------------------------+     |
|                            |                                     |
|   +------------------------v------------------------------+     |
|   |           Platform Abstraction Layer                   |     |
|   |                                                        |     |
|   |  +------------+  +------------+  +-----------------+  |     |
|   |  | IScheduler |  | ITransport |  | IDiscovery      |  |     |
|   |  | (when)     |  | (how)      |  | (where)         |  |     |
|   |  +------------+  +------------+  +-----------------+  |     |
|   +-------------------------------------------------------+     |
|                            |                                     |
|   +--------+--------+--------+--------+--------+---------+      |
|   | Game   |Vehicle |IntraECU| Cloud  | Robot  | Custom  |      |
|   |Profile |Profile |Profile |Profile |Profile | Profile |      |
|   +--------+--------+--------+--------+--------+---------+      |
|                                                                  |
+-----------------------------------------------------------------+
```

### Dependency Rule

- Upper layers depend only on interfaces, never on implementations
- Implementations are injected through Profiles
- SCXML documents reference no platform code
- Transport implementations are unaware of each other
- Cross-transport bridging (e.g., SOME/IP <-> gRPC) is handled by a dedicated `ITransportBridge` — an `ITransport` implementation that wraps two `ITransport` instances and translates events between them. Bridged transports remain unaware of each other; only the bridge knows both

### Relationship to Existing SCE Architecture

SCE Mesh extends the existing 4-tier library architecture:

```
sce_core          (existing — AOT engine, W3C algorithms)
   |
sce_base          (existing — utilities, logging)
   |
sce_scripting     (existing — Lua/JS engines)
   |
sce_runtime       (existing — interpreter)
   |
sce_mesh          (NEW — ITransport, IScheduler, IDiscovery, Profiles)
   |
sce_mesh_plugins  (NEW — protocol implementations)
```

`sce_mesh` depends on `sce_core` (for AOT) and optionally on `sce_runtime` (for interpreter mode). It does not modify existing tiers.

**CMake integration** follows the existing feature flag pattern (`SCE_ENABLE_QUICKJS`, `SCE_ENABLE_LUA`):

```cmake
option(SCE_ENABLE_MESH "Build sce_mesh distributed runtime" OFF)
option(SCE_MESH_WITH_INTERPRETER "Include interpreter support in sce_mesh" OFF)

# sce_mesh always links sce_core (header-only, no cost)
# sce_mesh optionally links sce_runtime when SCE_MESH_WITH_INTERPRETER=ON
# sce_mesh_plugins are individually toggleable:
option(SCE_MESH_PLUGIN_SHM "Shared memory transport" ON)
option(SCE_MESH_PLUGIN_SOMEIP "SOME/IP transport" OFF)
option(SCE_MESH_PLUGIN_ZENOH "Zenoh transport" OFF)
option(SCE_MESH_PLUGIN_CAN "CAN bus transport" OFF)
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

The Runtime is templated on the scheduler type, enabling compile-time dispatch with zero overhead:

```cpp
template<typename Scheduler, typename Transport, typename Discovery>
class Runtime {
    void run() {
        if constexpr (TickScheduling<Scheduler>) {
            while (running_) {
                auto events = transport_.collect();
                scheduler_.tick(instances_, events);
            }
        } else if constexpr (EventDrivenScheduling<Scheduler>) {
            transport_.subscribe("*", [this](Event e) {
                auto& inst = discovery_.resolve(e.target);
                scheduler_.onEvent(inst, e);
            });
            transport_.run();
        }
    }
};

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

### 3.2 ITransport — How to Deliver

Delivers events between state machine instances.

```
+--------------------------------------------------------------+
|  ITransport                                                   |
+--------------------------------------------------------------+
|  name()                        -> "someip" | "zenoh" | ...   |
|  send(event, target, qos)      -> deliver event              |
|  subscribe(pattern, callback)  -> register receiver           |
|  capabilities()                -> supported QoS set          |
|  onError(callback)             -> register error handler     |
+--------------------------------------------------------------+
```

#### Error Propagation Contract

Transport errors must be surfaced to the SCXML state machine as W3C-compliant `error.communication` events. The propagation path is:

```
Protocol error (e.g., SOME/IP TIMEOUT, CAN bus-off, gRPC UNAVAILABLE)
    |
    v
ITransport.onError callback fires
    |
    v
EventRouter creates SCXML event:
    name:   "error.communication"        (W3C SCXML 4.9.1)
    data:   { transport: "someip",
              target: "#motor",
              reason: "TIMEOUT",
              original_event: "brake.activate" }
    |
    v
Event enters external event queue of the sending state machine
    |
    v
SCXML <transition event="error.communication"> handles it
```

QoS violation behavior:

| Situation | Behavior |
|-----------|----------|
| `sce:qos="reliable"` send fails | Retry according to transport policy, then `error.communication` |
| `sce:deadline` exceeded | `error.communication` with `reason: "DEADLINE_EXCEEDED"` |
| Transport disconnected | `error.communication` immediately, instance lifecycle -> DRAINING |
| `sce:qos="best-effort"` send fails | Silent drop, no error event (fire-and-forget semantics) |

Implementations by domain:

**Intra-ECU (same device, different processes)**

| Transport | Mechanism | Latency |
|-----------|-----------|---------|
| `SharedMemTransport` | Zero-copy shared memory | < 1 us |
| `PosixMqTransport` | POSIX message queue | < 10 us |
| `PipeTransport` | Unix pipe / named pipe | < 10 us |
| `DBusTransport` | D-Bus session/system bus | < 100 us |

**Vehicle Network (ECU to ECU)**

| Transport | Mechanism | Latency |
|-----------|-----------|---------|
| `SomeIpTransport` | SOME/IP over Ethernet | < 1 ms |
| `ZenohTransport` | Zenoh pub/sub + query | < 1 ms |
| `CanTransport` | CAN bus frames | < 1 ms |
| `LinTransport` | LIN bus (low-speed sensors) | < 10 ms |

**Game / Cloud**

| Transport | Mechanism | Latency |
|-----------|-----------|---------|
| `UdpTransport` | Raw UDP (game servers) | < 1 ms LAN |
| `GrpcTransport` | gRPC (service-to-service) | < 10 ms |
| `NatsTransport` | NATS pub/sub | < 1 ms |
| `MqttTransport` | MQTT (lightweight IoT) | variable |

**IoT / Robotics**

| Transport | Mechanism | Latency |
|-----------|-----------|---------|
| `ZenohTransport` | Zenoh (reusable across domains) | < 1 ms |
| `DdsTransport` | DDS (real-time pub/sub) | < 1 ms |
| `Ros2Transport` | ROS2 topics/services | < 10 ms |

### 3.3 IDiscovery — Where to Find

Resolves logical instance IDs to physical addresses.

```
+--------------------------------------------------------------+
|  IDiscovery                                                   |
+--------------------------------------------------------------+
|  resolve(target_id)            -> physical address            |
|  announce(instance_id, meta)   -> advertise presence          |
|  watch(pattern, callback)      -> observe changes             |
+--------------------------------------------------------------+
```

Implementations:

| Discovery | Mechanism | Domain |
|-----------|-----------|--------|
| `StaticRegistry` | Compile-time routing table | Safety-critical automotive |
| `LocalRegistry` | In-process HashMap | Same process |
| `SharedMemRegistry` | Shared memory segment | Same ECU, cross-process |
| `SomeIpSd` | SOME/IP Service Discovery | Vehicle network |
| `ZenohDiscovery` | Zenoh scouting | Vehicle + IoT |
| `ZoneRouter` | Game zone routing table | MMORPG |
| `MdnsDiscovery` | mDNS/DNS-SD | LAN auto-discovery |
| `ConsulDiscovery` | Consul/etcd | Datacenter |

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
SCE Event Header (optional, added by transport layer):
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

## 5. QoS Annotations

SCXML extension namespace `sce:` for transport-level quality-of-service:

```xml
<scxml xmlns:sce="http://sce-mesh/1.0">

  <!-- Hard real-time, must deliver, 1ms deadline -->
  <send event="brake.activate" target="#brake_ecu"
        sce:qos="reliable"
        sce:deadline="1ms"
        sce:priority="critical"/>

  <!-- Best-effort, loss acceptable, fast delivery -->
  <send event="npc.move" target="zone://forest"
        sce:qos="best-effort"
        sce:priority="normal"/>

  <!-- Zero-copy, immediate delivery -->
  <send event="sensor.frame" target="proc://perception"
        sce:qos="zero-copy"/>


</scxml>
```

### Core QoS Attributes (Phase 1-3)

Transport-level hints that map naturally to existing protocol QoS mechanisms:

| Attribute | Values | Description |
|-----------|--------|-------------|
| `sce:qos` | `reliable`, `best-effort`, `zero-copy` | Delivery guarantee |
| `sce:deadline` | Duration (e.g. `1ms`, `16ms`) | Maximum delivery latency |
| `sce:priority` | `critical`, `high`, `normal`, `low` | Scheduling priority |

These are the only QoS attributes in the initial specification. They are simple transport hints — the runtime maps them to protocol-native QoS (e.g., `sce:qos="reliable"` -> SOME/IP reliable method call, Zenoh reliable publication).

### Future Direction: Distributed Transaction Patterns

The following patterns address distributed consistency but are **out of scope for the initial specification**. They may be defined in a separate extension specification after Phase 1-3 are validated:

- **Saga pattern** (`sce:saga`, `sce:compensate`) — orchestrated compensation transactions
- **Consistency modes** (`sce:consistency="strong|eventual"`) — cross-instance consistency guarantees

These are orchestration-layer concerns, not state machine concerns. Mixing them into the SCXML namespace risks violating W3C compatibility and the core simplicity of the state machine model. If needed, they should be implemented as a higher-level orchestration layer that uses SCE Mesh as its execution substrate.

---

## 6. Profiles

A Profile is a pre-configured combination of Scheduler + Transport(s) + Discovery + default QoS. Profiles use `sce::make_runtime()` which deduces template parameters from the provided types — scheduler concept satisfaction is checked at compile time.

`TransportSet` is a variadic template (`TransportSet<Ts...>`) holding a `std::tuple` of transport instances, where each `Ts` must satisfy the transport concept. The EventRouter iterates over the tuple at compile time to register subscriptions and route outgoing events based on the routing table. Exact implementation is a Phase 1 deliverable.

### 6.1 Vehicle Profile

```
Scheduler:   RealTimeScheduler (TickScheduling concept)
Transport:   SOME/IP + CAN + Zenoh (configurable)
Discovery:   Static (build-time) or SOME/IP-SD
QoS default: reliable, deadline=10ms, priority=high
Safety:      ASIL-B/D aware
```

```cpp
auto runtime = sce::make_runtime(
    RealTimeScheduler{.cycle_ms = 1, .safety_level = sce::ASIL_D},
    TransportSet{SomeIpTransport{cfg}, CanTransport{"can0"}},
    StaticRegistry{deploy_yaml}
);
```

### 6.2 Game Profile

```
Scheduler:   GameLoopScheduler (TickScheduling concept)
Transport:   UDP (zone-to-zone) + gRPC (gateway)
Discovery:   ZoneRouter + dynamic scaling
QoS default: best-effort, priority=normal
```

```cpp
auto runtime = sce::make_runtime(
    GameLoopScheduler{.tick_rate = 60, .max_entities = 500'000},
    TransportSet{UdpTransport{7777}, GrpcTransport{}},
    ZoneRouter{.zones = 16}
);
```

### 6.3 IntraECU Profile

```
Scheduler:   EventDrivenScheduler (EventDrivenScheduling concept)
Transport:   SharedMemory + POSIX MQ
Discovery:   LocalRegistry or SharedMemRegistry
QoS default: zero-copy, priority=high
```

```cpp
auto runtime = sce::make_runtime(
    sce::EventDrivenScheduler{},
    TransportSet{SharedMemTransport{"/sce_events", 4_MB}},
    LocalRegistry{}
);
```

### 6.4 Cloud Profile

```
Scheduler:   EventDrivenScheduler (EventDrivenScheduling concept)
Transport:   gRPC + NATS
Discovery:   Consul / etcd
QoS default: reliable, priority=normal
```

```cpp
auto runtime = sce::make_runtime(
    sce::EventDrivenScheduler{},
    TransportSet{GrpcTransport{}, NatsTransport{}},
    ConsulDiscovery{"consul.local:8500"}
);
```

### 6.5 Custom Profile

```cpp
// Any types satisfying the correct concepts work
auto runtime = sce::make_runtime(
    MyScheduler{},           // must satisfy TickScheduling or EventDrivenScheduling
    TransportSet{MyTransport{}},  // must satisfy ITransport concept
    MyDiscovery{}            // must satisfy IDiscovery concept
);

// Compile error if concept not satisfied:
// "MyScheduler satisfies neither TickScheduling nor EventDrivenScheduling"
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
sce-build --mesh deploy.yaml scxml/*.scxml

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
  brake->motor:     cross-device -> generate proxy + serialization
  brake->dashboard: cross-device -> generate proxy + serialization
  (same-device targets: direct call, no proxy needed)

Step 5: Generate per-device artifacts
```

### 7.3 Outputs

```
generated/
  device_a/
    brake_sm.h              # AOT state machine (existing codegen)
    brake_routing.h         # compile-time routing table
    brake_events.h          # event serialization/deserialization
    brake_mesh_init.h       # transport + discovery auto-config

  device_b/
    motor_sm.h
    motor_routing.h
    motor_events.h
    motor_mesh_init.h

  device_c/
    dashboard_sm.h
    dashboard_routing.h
    dashboard_events.h
    dashboard_mesh_init.h
```

### 7.4 Generated Routing Table

```cpp
// [generated] brake_routing.h
namespace sce::generated::brake {

constexpr RoutingEntry ROUTES[] = {
    { "#motor",     Transport::SOMEIP, someip::Address{0x1001, 0x01} },
    { "#dashboard", Transport::CAN,    can::Address{"can0", 0x100} },
};

}  // namespace sce::generated::brake
```

### 7.5 Generated Event Serialization

Serialization is not one-size-fits-all. Different transports have fundamentally different data encoding requirements:

- **Buffer-based** (gRPC, SOME/IP, UDP, SharedMem): Variable-length byte streams
- **Signal-based** (CAN, LIN): Bit-level packing into fixed-size frames (8 bytes CAN classic, 64 bytes CAN FD)

The build tool generates transport-appropriate serialization via the `ISerializer` interface:

```
+--------------------------------------------------------------+
|  ISerializer                                                  |
+--------------------------------------------------------------+
|  serialize(event, target_transport) -> bytes                  |
|  deserialize(bytes, source_transport) -> event                |
+--------------------------------------------------------------+
        |
        +-- BufferSerializer      (gRPC, SOME/IP, UDP, SHM)
        |     Variable-length, self-describing format
        |
        +-- SignalSerializer      (CAN, LIN)
              Bit-packed, DBC/ARXML-compatible layout
```

#### Type Information Source

W3C SCXML is typeless — `<param name="brake_force" expr="brakeForce"/>` carries no type information. The build tool requires an external type source to generate serialization code:

- **Buffer-based transports**: Types are declared in an **event schema file** (`events.yaml`) alongside `deploy.yaml`. This is the minimal approach for Phase 2.
- **Signal-based transports (CAN, LIN)**: Types, scaling, offsets, and bit layouts are imported from standard automotive database files (`.dbc`, `.arxml`, AUTOSAR system description). This is Phase 3 scope.

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
namespace sce::generated::brake::events {

struct MotorCutPower {
    static constexpr auto NAME = "motor.cut_power";
    float brake_force;    // from events.yaml: float32

    void serialize(sce::Buffer& buf) const;
    static MotorCutPower deserialize(sce::BufferView buf);
};

}  // namespace sce::generated::brake::events
```

Generated code example (CAN signal-based transport, types from `.dbc`):

```cpp
// [generated] brake_can_signals.h — layout derived from vehicle.dbc
namespace sce::generated::brake::signals {

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

}  // namespace sce::generated::brake::signals
```

### 7.6 What Developers Write

```cpp
int main() {
    auto runtime = sce::make_runtime(
        RealTimeScheduler{.cycle_ms = 1},
        TransportSet{SomeIpTransport{config}, CanTransport{"can0"}},
        StaticRegistry{generated::brake::ROUTES}
    );
    runtime.load<generated::brake::BrakeSM>();
    runtime.run();
}
```

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

How SCXML concepts map to each transport protocol:

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
| `<param>` passing | Direct memory reference | Serialized via ISerializer |
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
Parent sends INVOKE_REQUEST via ITransport
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
        +-- Child SM runs normally, sends events back to parent via ITransport
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
    .overflow_policy = sce::OverflowPolicy::ERROR,  // signal to sender
    .max_queue_depth = 64
};

GameLoopScheduler{
    .tick_rate = 60,
    .overflow_policy = sce::OverflowPolicy::DROP_OLDEST,
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

| Transport | Best Case | Worst Case | Notes |
|-----------|-----------|------------|-------|
| Direct call (same process) | 0 ns | 0 ns | No transport layer |
| Shared memory | 100 ns | 5 us | Worst: futex contention |
| SOME/IP (same network) | 50 us | 2 ms | Worst: network congestion |
| CAN bus | 100 us | 10 ms | Worst: bus arbitration, low priority |
| UDP (LAN) | 50 us | 1 ms | Worst: packet loss + retransmit |
| gRPC (WAN) | 1 ms | 100 ms | Worst: cross-region, TLS handshake |

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

The same SCXML runs in all three domains:

| Domain | Scheduler | Transport | `#actuator` resolves to |
|--------|-----------|-----------|------------------------|
| Game (dungeon door) | GameLoop 60Hz | UDP | Object in same zone |
| Vehicle (car door) | RealTime 10ms | CAN | Motor ECU on CAN bus |
| Simulator | EventDriven | SharedMem | Motor model in same process |

---

## 13. Roadmap

**Scope commitment**: Phase 1-2 are the concrete implementation target. Phase 3-5 are directional plans that will be refined after Phase 2 delivers an end-to-end working demo. The first milestone is: **two processes on the same machine communicating via SCXML through shared memory, with no changes to the SCXML documents themselves.**

### Phase 1: Core Interfaces

Define the three abstraction interfaces and Profile system. Refactor existing LocalBus into ITransport.

- `IScheduler`, `ITransport`, `IDiscovery` interface definitions
- `Profile` configuration structure
- `EventRouter` with routing table support
- `LocalTransport` as reference implementation (existing behavior, unchanged)

### Phase 2: Intra-ECU (simplest remote case)

First cross-process communication using shared memory.

- `SharedMemTransport` implementation
- `SharedMemRegistry` for cross-process discovery
- Event serialization framework
- Verification: two processes on same machine communicating via SCXML

### Phase 3: Vehicle Network (Directional — refined after Phase 2)

Automotive transport plugins with static discovery.

- `SomeIpTransport` + `SomeIpSd`
- `CanTransport`
- `ZenohTransport` (reusable for IoT/robotics)
- Static discovery mode (build-time routing tables)
- `deploy.yaml` schema and build tool integration
- QoS annotation support (`sce:deadline`, `sce:priority`)

### Phase 4: Game Scale (Directional — refined after Phase 3)

High-throughput batch processing for massive entity counts.

- `GameLoopScheduler` with SoA (Structure of Arrays) layout
- `UdpTransport` + `ZoneRouter`
- Batch event processing (group by event type)
- Zero-allocation event pool (arena allocator)
- ECS integration API
- Delta state synchronization (changed-state-only client updates)
- **Codegen trade-off**: SoA layout requires a second codegen mode alongside the existing per-instance AoS generation. This is significant codegen complexity — the decision to implement SoA vs. optimize AoS (cache-line alignment, prefetching) will be evaluated based on Phase 3 benchmarks

### Phase 5: Cloud + Hybrid (Directional — refined after Phase 4)

Multi-transport environments and dynamic scaling.

- `GrpcTransport` + `NatsTransport`
- `ConsulDiscovery`
- Dynamic discovery mode with priority-based resolution
- `ITransportBridge` for cross-protocol bridging (e.g., vehicle SOME/IP <-> cloud gRPC), keeping transports mutually unaware
- Build-time verification: remaining checks — circular dependency detection, reachability analysis (see Section 7.7 for phase mapping of all checks)

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

topology:
  <device_name>:
    platform: linux | qnx | autosar | windows
    target: x86_64 | aarch64 | arm32
    machines:
      <scxml_name>:
        bindings:                          # Static mode
          "<target_id>":
            transport: someip | can | zenoh | shm | udp | grpc | ...
            address: <transport-specific address>
            signals: <path to .dbc/.arxml>  # Signal-based transports only

events: <path to events.yaml>              # Event payload type definitions

transport:
  default: <transport_name>                # Default for unspecified bindings
  overrides:
    "<source> -> <target>": <transport>    # Per-path override

discovery:
  mode: static | scoped | dynamic
  scopes:                                  # Scoped mode
    <transport>: "<uri_pattern>"
  resolution:                              # Dynamic mode
    strategy: priority | first
    priority_order: [local, shm, someip, zenoh, udp, grpc]
  dedup:
    key: instance_id
    ttl: <duration>

qos:
  defaults:
    qos: reliable | best-effort
    deadline: <duration>
    priority: critical | high | normal | low
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
          "#motor":     { transport: someip, address: "service:0x1001" }
          "#dashboard": { transport: can,    address: "can0:0x100" }

  motor_ecu:
    platform: qnx
    target: aarch64
    machines:
      motor:
        bindings:
          "#brake": { transport: someip, address: "service:0x1002" }

  dashboard_ecu:
    platform: linux
    target: aarch64
    machines:
      dashboard:
        bindings:
          "#brake": { transport: can, address: "can0:0x101" }
          "#cloud": { transport: grpc, address: "telemetry.oem.com:443" }

discovery:
  mode: static

qos:
  defaults:
    qos: reliable
    deadline: 10ms
    priority: high
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

## 15. Zenoh Transport Specification

Zenoh is the primary Phase 3 transport target. This section defines the complete Zenoh integration, covering key mapping, session management, QoS translation, deployment topology, and the relationship with existing SCE Mesh transports.

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

A single Zenoh session is shared across all state machine instances on the same device. The Runtime owns the session; transports hold a reference.

```
Runtime (per device)
    |
    +-- Zenoh Session (one per Runtime)
        |
        +-- ZenohTransport for brake.scxml  (shared session reference)
        +-- ZenohTransport for abs.scxml    (shared session reference)
        +-- ZenohTransport for esc.scxml    (shared session reference)
```

```cpp
// Runtime creates session once
auto session = zenoh::Session::open(std::move(config));

// Each transport receives a shared reference
ZenohTransport brake_transport{session, generated::brake::ROUTES};
ZenohTransport abs_transport{session, generated::abs::ROUTES};
```

Rationale: Zenoh sessions manage peer discovery, connection pooling, and resource allocation. Multiple sessions on the same device waste resources and can cause discovery conflicts.

### 15.3 QoS Mapping

#### sce:qos → Zenoh Reliability + Congestion Control

| `sce:qos` | Zenoh Reliability | Zenoh CongestionControl | Notes |
|-----------|-------------------|------------------------|-------|
| `reliable` | `Reliability::Reliable` | `CongestionControl::Block` | Sender blocks if subscriber is slow |
| `best-effort` | `Reliability::BestEffort` | `CongestionControl::Drop` | Sender drops if subscriber is slow |
| `zero-copy` | `Reliability::Reliable` | `CongestionControl::Block` | + Zenoh SHM enabled (see 15.5) |

#### sce:priority → Zenoh Priority

| `sce:priority` | Zenoh Priority | Value |
|----------------|---------------|-------|
| `critical` | `Priority::RealTime` | 1 |
| `high` | `Priority::InteractiveHigh` | 2 |
| `normal` | `Priority::Data` | 5 |
| `low` | `Priority::Background` | 7 |

#### sce:deadline → Timer-Based Enforcement

Zenoh does not natively support delivery deadlines. SCE Mesh implements deadline enforcement at the transport layer:

```
send(event, target, qos={deadline: 1ms})
    |
    +-- start timer(1ms)
    +-- zenoh.put(key, payload)
    |
    +-- [ack received before timer] -> success, cancel timer
    +-- [timer fires] -> onError(DEADLINE_EXCEEDED)
```

For `sce:qos="best-effort"`, deadline violations are silently ignored (consistent with fire-and-forget semantics). For `sce:qos="reliable"`, deadline violations trigger `error.communication` with `reason: "DEADLINE_EXCEEDED"`.

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

### 15.5 Zenoh SHM and SharedMemTransport Relationship

Zenoh has built-in shared memory support for same-host communication. This overlaps with Phase 2's `SharedMemTransport`.

**Rule**: when Zenoh is the transport, use Zenoh SHM. Do not run both mechanisms.

| Scenario | Transport | SHM Provider |
|----------|-----------|-------------|
| Same process | Direct call | None (no transport) |
| Same ECU, no Zenoh | `SharedMemTransport` | SCE Phase 2 SHM |
| Same ECU, Zenoh enabled | `ZenohTransport` | Zenoh SHM (`shmem: true`) |
| Cross ECU | `ZenohTransport` | None (network) |
| Cross ECU to cloud | `ZenohTransport` | None (network) |

Zenoh SHM is transparent — the same `zenoh.put()` / `zenoh.subscribe()` API works regardless of whether the subscriber is on the same host (SHM) or remote (network). This means `ZenohTransport` code does not change; only the Zenoh session config enables SHM.

```yaml
# deploy.yaml — same ECU processes use Zenoh SHM automatically
zenoh:
  shmem: true    # Zenoh detects same-host subscribers and uses SHM
```

This eliminates the need for Discovery to choose between `SharedMemTransport` and `ZenohTransport` on the same host — Zenoh handles it internally.

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

CMake integration:

```cmake
# SCE_MESH_PLUGIN_ZENOH=ON triggers:
if(SCE_MESH_PLUGIN_ZENOH)
    find_package(zenohc REQUIRED)            # or FetchContent
    target_link_libraries(sce_mesh_zenoh
        PUBLIC sce_mesh
        PRIVATE zenohc::lib
    )
endif()

# For resource-constrained targets:
option(SCE_MESH_ZENOH_PICO "Use zenoh-pico instead of zenoh-c" OFF)
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
