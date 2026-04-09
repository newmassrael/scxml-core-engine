# SCE Forge: Extended SCXML Kind System for Multi-Pattern Code Generation

## 1. Vision

### Problem

W3C SCXML is a state machine language. The scxml-core-engine generates multi-language code from SCXML statecharts. But real-world systems require more than state machines:

- Value conversion formulas (sensor raw → physical value)
- Byte-level encode/decode (CAN frames, UDS messages)
- Lookup tables, interpolation maps, signal filters
- Sequential procedures with retry logic
- Threshold monitoring with hysteresis
- Periodic task scheduling

Today, these patterns are hand-coded per language. There is no single source of truth, no codegen, and no way to guarantee identical behavior across C++/Kotlin/Go/Python/Rust.

### Solution

SCE Forge extends SCXML with a **kind system** — the `sce:kind` attribute on the `<scxml>` root element declares what pattern the document represents. The codegen reads this attribute and selects the appropriate generation template.

```
Author writes:               SCE Forge generates:
  SCXML with sce:kind          C++ header
                               Kotlin file
                               Python module
                               Rust module
                               All from one source of truth
```

### Core Principle

**One SCXML, all languages.** Extended SCXML is the single source of truth for any codegen-able pattern — not just state machines. The kind system makes SCXML a **universal intermediate representation** for multi-language code generation.

### Positioning

SCE Forge does not change what SCXML is. It extends what SCXML can represent.

```
W3C SCXML         = state machines only
SCE Forge          = state machines + transforms + codecs + procedures + ...
scxml-core-engine  = codegen for all of the above
```

Value SCE Forge adds:

- **Single source of truth** — one SCXML generates all target languages
- **Beyond state machines** — 10 additional kinds covering common embedded/automotive patterns
- **Multi-language codegen** — deterministic, identical logic across C++/Kotlin/Rust/Go/Python
- **W3C compatible** — Extended SCXML is valid W3C SCXML with `sce:` namespace extensions
- **Human-readable** — every kind is editable, diffable, version-controllable

---

## 2. Architecture

### Layer Diagram

```
+-------------------------------------------------------------------+
|                    Extended SCXML (input)                          |
|        hand-authored, tool-generated, or any other source         |
+===================================================================+
|                                                                   |
|  +-------------------------------------------------------------+ |
|  |            sce:kind System                                   | |
|  |                                                              | |
|  |  +------------+  +----------+  +----------------------+     | |
|  |  | Statechart |  | Data     |  | sce: extension       |     | |
|  |  | (W3C std)  |  | Model    |  | namespace            |     | |
|  |  +------------+  +----------+  +----------------------+     | |
|  |                                                              | |
|  |  Document kinds:                                             | |
|  |    statechart | transform | lookup | condition               | |
|  |    procedure  | codec | validator | filter                   | |
|  |    interpolation | timer | observer                         | |
|  |  Inline kinds (within statechart):                           | |
|  |    condition | lookup | codec | transform                    | |
|  +-------------------------------+------------------------------+ |
|                                  |                                |
|  +-------------------------------v------------------------------+ |
|  |              sce-build (codegen)                              | |
|  |                                                               | |
|  |  +------+ +--------+ +------+ +--------+ +------+            | |
|  |  | C++  | | Kotlin | |  Go  | | Python | | Rust |            | |
|  |  +------+ +--------+ +------+ +--------+ +------+            | |
|  +-------------------------------------------------------------+ |
|                                                                   |
+-------------------------------------------------------------------+
```

### Dependency Rule

- Extended SCXML is valid W3C SCXML + custom `sce:` namespace extensions
- Standard SCXML parsers can read the files (ignoring `sce:` attributes)
- Codegen reads `sce:kind` to select the appropriate generation template
- Target code depends only on sce_runtime interfaces
- How SCXML files are authored (manually, by tools, etc.) is outside this specification

### Relationship to Existing SCE Architecture

SCE Forge extends the existing codegen pipeline within sce-build:

```
scxml-core-engine  (existing — SCXML → C++/Kotlin/Go/Python/Rust)
   |
sce-build          (existing — Rust codegen CLI)
   |
sce-build + kinds  (NEW — kind-specific templates alongside statechart)
```

No new binary or project is introduced. `sce-build` gains new templates for each `sce:kind`. The existing statechart codegen is unchanged.

---

## 3. Extended SCXML: The `sce:` Namespace

### 3.1 W3C Compliance

W3C SCXML Section 3.1 explicitly allows elements and attributes from foreign namespaces. The `sce:` extension namespace is a standard XML extension mechanism — not a spec violation.

```xml
<scxml xmlns="http://www.w3.org/2005/07/scxml"
       xmlns:sce="http://sce.dev/ext"
       sce:kind="statechart"
       initial="defaultSession">
  <!-- Standard W3C SCXML content -->
  <!-- sce: attributes are ignored by standard parsers -->
</scxml>
```

### 3.2 The `sce:kind` Attribute

`sce:kind` operates at two levels: **document-level** (on `<scxml>` root) and **inline** (on `<data>` elements within a statechart). The distinction drives codegen architecture.

#### Document-Level Kind

Declared on the `<scxml>` root element. The entire file is a single kind. Produces an independent codegen unit.

```xml
<scxml sce:kind="statechart">    <!-- State machine (default, existing) -->
<scxml sce:kind="transform">     <!-- Mathematical formula / conversion -->
<scxml sce:kind="lookup">        <!-- Discrete value mapping -->
<scxml sce:kind="condition">     <!-- Boolean decision (also usable inline) -->
<scxml sce:kind="procedure">     <!-- Sequential procedure with branching -->
<scxml sce:kind="codec">         <!-- Byte-level encode/decode -->
<scxml sce:kind="validator">     <!-- Range/plausibility/integrity check -->
<scxml sce:kind="filter">        <!-- Signal filtering (moving avg, debounce) -->
<scxml sce:kind="interpolation"> <!-- 1D/2D table interpolation -->
<scxml sce:kind="timer">         <!-- Periodic/delayed task timing -->
<scxml sce:kind="observer">      <!-- Threshold monitoring with hysteresis -->
```

#### Inline Kind

Declared on `<data>` elements inside a `sce:kind="statechart"` document. Generates helper functions/types co-located with the statechart code. **Only stateless kinds** may be inlined — kinds with runtime dependencies or persistent state must be standalone files.

```xml
<!-- Inline-eligible (stateless): -->
<data id="engineStatus" sce:kind="lookup" .../>
<data id="canProgram" sce:kind="condition" .../>
<data id="response" sce:kind="codec" .../>
<data id="temperature" sce:kind="transform" .../>

<!-- NOT inline-eligible (stateful or runtime-dependent): -->
<!-- procedure, filter, validator, timer, observer → must be standalone files -->
```

**Inline transform example** — generates a helper function within the parent statechart:

```xml
<scxml sce:kind="statechart" initial="idle">
  <datamodel>
    <!-- Inline transform: raw sensor value → physical temperature -->
    <data id="temperature" sce:kind="transform"
          sce:input="rawTemp" sce:input-type="uint16"
          sce:output-type="float64" expr="rawTemp * 0.1 - 40.0"/>

    <!-- Used in guard expressions -->
    <data id="rawTemp" sce:type="uint16" sce:direction="in"/>
  </datamodel>

  <state id="idle">
    <transition cond="temperature &gt; 95.0" target="overheating"/>
  </state>
  <state id="overheating">...</state>
</scxml>
```

```cpp
// Generated inline helper within statechart namespace
inline double computeTemperature(uint16_t rawTemp) {
    return rawTemp * 0.1 - 40.0;
}
```

#### Codegen Discovery Order

```
1. Read <scxml sce:kind="...">  → select document-level template
2. If kind == "statechart":
   a. Scan <data sce:kind="..."> elements → generate inline helpers
   b. Scan <invoke> elements → resolve references to standalone kind files
   c. Generate statechart class that uses inline helpers and standalone references
3. If kind != "statechart":
   a. Generate standalone codegen unit (function, struct, or class)
```

### 3.3 Common Extension Attributes

```xml
sce:type="uint8 | uint16 | uint32 | uint64 | int8 | int16 | int32 | int64 | float32 | float64 | bool | string | bytes"
sce:direction="in | out | internal"
sce:unit="celsius | rpm | ms | percent | ..."   <!-- documentation only, no codegen effect -->
sce:default-endian="big | little | native"  <!-- document-level default, big if omitted -->
sce:input="<signal-name>"       <!-- inline kind: names the external signal mapped to this kind's input -->
sce:input-type="<sce:type>"    <!-- inline kind: type of the input signal (uses same types as sce:type) -->
sce:length="<integer>"          <!-- codec input: expected frame length in bytes (validation hint) -->
sce:bit-offset="<integer>"      <!-- codec field: bit offset within the byte, default 0 if omitted -->
```

#### Cross-Language Type Mapping

All kind templates use this canonical mapping. It is defined once; templates reference it, never define their own.

| sce:type | C++ | Kotlin | Python | Rust | Go |
|----------|-----|--------|--------|------|----|
| `uint8` | `uint8_t` | `UByte` | `int` | `u8` | `uint8` |
| `uint16` | `uint16_t` | `UShort` | `int` | `u16` | `uint16` |
| `uint32` | `uint32_t` | `UInt` | `int` | `u32` | `uint32` |
| `uint64` | `uint64_t` | `ULong` | `int` | `u64` | `uint64` |
| `int8` | `int8_t` | `Byte` | `int` | `i8` | `int8` |
| `int16` | `int16_t` | `Short` | `int` | `i16` | `int16` |
| `int32` | `int32_t` | `Int` | `int` | `i32` | `int32` |
| `int64` | `int64_t` | `Long` | `int` | `i64` | `int64` |
| `float32` | `float` | `Float` | `float` | `f32` | `float32` |
| `float64` | `double` | `Double` | `float` | `f64` | `float64` |
| `bool` | `bool` | `Boolean` | `bool` | `bool` | `bool` |
| `string` | `std::string` | `String` | `str` | `String` | `string` |
| `bytes` | `std::vector<uint8_t>` | `ByteArray` | `bytes` | `Vec<u8>` | `[]byte` |

### 3.4 Expression Language

In standard W3C SCXML, `expr` attributes are evaluated at runtime by a datamodel processor. In Extended SCXML for SCE Forge, **`expr` is an AOT codegen input, not a runtime evaluation target**. The codegen parses `expr` and generates equivalent target-language code.

#### Grammar: ECMAScript Expression Subset

W3C SCXML's normative datamodel is ECMAScript (ECMA-262). SCE Forge reuses the same grammar — expressions in `expr` attributes follow **ECMAScript expression syntax** as defined in ECMA-262. The codegen parses these expressions into an **Abstract Syntax Tree (AST)** and emits equivalent target-language code via per-language emitters.

**Supported** — any ECMAScript expression that has a direct, stateless mapping to all target languages (C++/Kotlin/Rust/Go/Python):

| ECMA-262 Production | Syntax | Examples |
|---------------------|--------|---------|
| ArithmeticExpression | `+`, `-`, `*`, `/`, `%` | `raw * 0.1 - 40.0` |
| ComparisonExpression | `===`, `!==`, `<`, `>`, `<=`, `>=` | `rpm > 8000` |
| LogicalExpression | `&&`, `\|\|`, `!` | `engineStop && ignOn` |
| BitwiseExpression | `&`, `\|`, `^`, `~` | `raw & 0x0F` |
| ShiftExpression | `<<`, `>>`, `>>>` | `(raw[1] >> 4) & 0x0F` |
| ConditionalExpression | `? :` | `status === 'OK' ? 1 : 0` |
| MemberExpression | `.field`, `[index]` | `writeResult.result`, `data[0]` |
| CallExpression | `func(args)`, `obj.method(args)` | `computeKey(seed)` |
| Literal | number, string, boolean, null | `0x03`, `'STOP'`, `true` |
| GroupingExpression | `(expr)` | `(a + b) * c` |
| UnaryExpression | `-`, `+`, `!`, `~` | `-offset`, `!valid` |

**Not supported** — ECMAScript constructs that require runtime semantics and cannot be statically transpiled:

| Rejected Construct | Reason |
|-------------------|--------|
| `new`, `delete`, `typeof`, `instanceof` | Object system / runtime type info |
| Arrow functions, closures `() => {}` | Capture semantics not transpilable |
| `this`, prototype chain | Runtime object model |
| `eval`, `Function()` | Dynamic code execution |
| `async`/`await`, generators, `yield` | Concurrency model |
| Regular expression literals `/pattern/` | Engine-specific implementation |
| Template literals `` `${x}` `` | Runtime string interpolation |
| Destructuring `{a, b} = obj` | Runtime pattern matching |
| Spread/rest `...args` | Runtime collection operations |
| Optional chaining `?.`, nullish coalescing `??` | Runtime null semantics |

Rejected constructs cause a **build-time error** with the specific unsupported syntax highlighted.

**Equality convention**: `===` (strict equality) is used per ECMAScript convention. Codegen maps it to language-appropriate equality (`==` in C++/Kotlin/Go/Python/Rust). `==` (loose equality) is not permitted in Extended SCXML to avoid type coercion ambiguity. If the parser encounters `==`, it must emit a **build-time error**: `"Loose equality '==' is not permitted in Extended SCXML. Use '===' (strict equality) instead."` This avoids silent type coercion bugs and guides authors toward the correct syntax.

#### Transpiler Architecture

The expression transpiler uses a proper **AST-based pipeline**, not regex string replacement. This is necessary because target languages have different operator precedence rules — naive text substitution produces incorrect code for complex expressions.

```
Source text ("(raw >> 4) & 0x0F")
  │
  ├── Tokenize    → [Ident("raw"), Shr, Number("4"), Amp, Number("0x0F")]
  ├── Parse       → Binary(BitAnd, Binary(Shr, Ident("raw"), 4), 0x0F)
  │                  (recursive descent, ECMAScript precedence)
  └── Emit        → per-language code with correct parenthesization
        ├── C++:    raw >> 4 & 0x0F     (>> > & in C++, no parens needed)
        ├── Kotlin: raw shr 4 and 0x0F  (shr = and in Kotlin, left-assoc OK)
        ├── Rust:   raw >> 4 & 0x0F
        └── Python: raw >> 4 & 0x0F
```

**Operator precedence** (highest to lowest, matching ECMA-262):

| Level | Operators | ECMAScript |
|-------|-----------|------------|
| 12 | `- + ! ~` (prefix) | Unary |
| 11 | `* / %` | Multiplicative |
| 10 | `+ -` | Additive |
| 9 | `<< >> >>>` | Shift |
| 8 | `< > <= >=` | Relational |
| 7 | `=== !==` | Equality |
| 6 | `&` | Bitwise AND |
| 5 | `^` | Bitwise XOR |
| 4 | `\|` | Bitwise OR |
| 3 | `&&` | Logical AND |
| 2 | `\|\|` | Logical OR |
| 1 | `? :` | Conditional |

**Per-language precedence handling**: Each target language has its own precedence function. The emitter adds parentheses wherever the target language would parse the expression differently from the AST's intended semantics.

| Language | Precedence difference from ECMAScript | Emitter behavior |
|----------|---------------------------------------|------------------|
| C++, Rust, Go | None — same relative ordering | Uses ECMAScript precedence directly |
| Kotlin | Bitwise ops (`and`, `or`, `xor`, `shl`, `shr`) are infix functions sharing **one** precedence level, higher than comparison | Inserts parens for right-child infix with equal precedence |
| Python | Bitwise ops have **higher** precedence than comparison (opposite of ECMAScript for `&` vs `==`) | Inserts parens around comparison children of bitwise parents |

This ensures that `a & (b === c)` (ECMAScript semantics: equality before bitwise AND) generates correct code in all languages, even where the target would otherwise parse it differently.

#### Kind Reference Resolution

Method calls on `<data>` ids are resolved by matching the id against inline kind declarations:

```xml
<!-- SCXML source -->
<data id="securityResponse" sce:kind="codec">...</data>
<assign location="writeResult" expr="securityResponse.decode(_event.data)"/>

<!-- Codegen resolves "securityResponse" as inline codec,
     generates a call to the codec's decode method -->
```

```cpp
// Generated C++
auto writeResult = SecurityResponse::decode(event.data(), event.dataLen());
```

#### External Function References

Function calls that do not match any `<data>` id (e.g., `computeKey(seed)`) are treated as **user-provided functions**. Codegen generates a forward declaration; the user supplies the implementation.

```cpp
// Generated: forward declaration
extern std::vector<uint8_t> computeKey(const std::vector<uint8_t>& seed);

// User provides: implementation
std::vector<uint8_t> computeKey(const std::vector<uint8_t>& seed) {
    // application-specific key derivation
}
```

#### AOT-Only Constraint

**Extended SCXML kinds (`sce:kind` != "statechart") are AOT-only** — they are not supported by the Interpreter engine. Only the standard statechart kind runs on both Interpreter and AOT.

### 3.5 Namespace URI

The namespace URI `http://sce.dev/ext` is the unified SCE extension namespace, shared with SCE Mesh. Both SCE Forge (kind system) and SCE Mesh (distributed runtime) use the same `sce:` prefix and URI. A permanent URI will be assigned if the extension is proposed as a formal standard. Implementations must match on the URI string, not resolve it as a URL.

### 3.6 Extension Attribute Classification

The `sce:` namespace contains attributes from two distinct subsystems. Each attribute has a clear owner that determines when and how it is processed:

| Attribute | Owner | Processing Time | Purpose |
|-----------|-------|----------------|---------|
| `sce:kind` | SCE Forge | Build-time (codegen) | Selects generation template |
| `sce:type`, `sce:direction` | SCE Forge | Build-time (codegen) | Type mapping, I/O direction |
| `sce:byte`, `sce:bit-offset`, `sce:bit-size` | SCE Forge | Build-time (codegen) | Codec field layout |
| `sce:service`, `sce:subfunc`, `sce:addr`, `sce:payload` | SCE Forge | Build-time (codegen) | Procedure `<send>` hints |
| `sce:unit`, `sce:default-endian` | SCE Forge | Build-time (codegen) | Documentation, endianness |
| `sce:range-min`, `sce:range-max` | SCE Forge | Build-time (codegen) | Validator range bounds |
| `sce:max-delta`, `sce:sample-interval` | SCE Forge | Build-time (codegen) | Validator rate-of-change |
| `sce:plausibility` | SCE Forge | Build-time (codegen) | Validator cross-field check |
| `sce:filter`, `sce:window`, `sce:alpha` | SCE Forge | Build-time (codegen) | Filter type and parameters |
| `sce:interpolation`, `sce:out-of-bounds` | SCE Forge | Build-time (codegen) | Interpolation method |
| `sce:axis-{id}` | SCE Forge | Build-time (codegen) | Interpolation axis points |
| `sce:timer`, `sce:interval`, `sce:duration`, `sce:delay` | SCE Forge | Build-time (codegen) | Timer scheduling |
| `sce:monitor`, `sce:enter`, `sce:leave` | SCE Forge | Build-time (codegen) | Observer threshold |
| `sce:on-enter`, `sce:on-leave`, `sce:on-timeout` | SCE Forge | Build-time (codegen) | Observer/timer event names |
| `sce:qos` | SCE Mesh | Runtime (transport) | Delivery guarantee |
| `sce:deadline` | SCE Mesh | Runtime (transport) | Maximum delivery latency |
| `sce:priority` | SCE Mesh | Runtime (transport) | Scheduling priority |

**Rule**: A single `<send>` element may carry attributes from both owners. SCE Forge processes its attributes at build time to generate the send logic. SCE Mesh processes its attributes at runtime to select transport QoS. Neither subsystem reads the other's attributes.

```xml
<!-- Both Forge and Mesh attributes on the same <send> -->
<send sce:service="SecurityAccess" sce:subfunc="0x01"
      sce:qos="reliable" sce:deadline="5ms"/>
<!--   ^^^^^^^ Forge (codegen)  ^^^^^^^ Mesh (runtime) -->
```

---

## 4. Kind Specifications

### 4.1 statechart (Existing)

The existing W3C SCXML statechart. No changes to current codegen.

```xml
<scxml sce:kind="statechart" initial="defaultSession">
  <!-- datamodel omitted for brevity; engineStop and ignOn are <data> elements of type bool -->
  <state id="defaultSession">
    <transition event="SID_0x10_0x02"
                cond="engineStop &amp;&amp; ignOn"
                target="programmingSession"/>
    <transition event="SID_0x10_0x03"
                target="extendedSession"/>
  </state>
  <state id="programmingSession">...</state>
  <state id="extendedSession">...</state>
</scxml>
```

**Codegen**: Existing state machine class generation (unchanged).

### 4.2 transform

Pure mathematical formula. No state. Input → computation → output.

```xml
<scxml sce:kind="transform">
  <datamodel>
    <data id="raw" sce:type="uint16" sce:direction="in"/>
    <data id="temperature" sce:type="float64" sce:direction="out"
          expr="raw * 0.1 - 40.0" sce:unit="celsius"/>
  </datamodel>
</scxml>
```

**Codegen** (C++):
```cpp
inline double computeTemperature(uint16_t raw) {
    return raw * 0.1 - 40.0;
}
```

**Codegen** (Rust):
```rust
pub fn compute_temperature(raw: u16) -> f64 {
    raw as f64 * 0.1 - 40.0
}
```

### 4.3 lookup

Discrete value mapping. Enumerated input → enumerated output. A `sce:default` attribute specifies the fallback when input matches no entry. If `sce:default` is omitted, the first `<sce:entry>`'s value is used as the default (in the example below, `"STOP"` from key `0x00`). Codegen must emit a comment documenting the implicit default to make the behavior visible in generated code.

```xml
<scxml sce:kind="lookup">
  <datamodel>
    <data id="engSta" sce:type="uint8" sce:direction="in"/>
    <data id="status" sce:type="string" sce:direction="out"/>
    <data id="mapping" sce:default="STOP">
      <sce:entry key="0x00" value="STOP"/>
      <sce:entry key="0x01" value="STOP"/>
      <sce:entry key="0x02" value="STOP"/>
      <sce:entry key="0x03" value="RUNNING"/>
      <sce:entry key="0x07" value="FAULT"/>
    </data>
  </datamodel>
</scxml>
```

**Codegen** (C++):
```cpp
enum class EngineStatus { STOP, RUNNING, FAULT };

inline EngineStatus lookupEngineStatus(uint8_t engSta) {
    switch (engSta) {
        case 0x00: case 0x01: case 0x02: return EngineStatus::STOP;
        case 0x03: return EngineStatus::RUNNING;
        case 0x07: return EngineStatus::FAULT;
        default:   return EngineStatus::STOP;
    }
}
```

**Codegen** (Kotlin):
```kotlin
enum class EngineStatus { STOP, RUNNING, FAULT }

fun lookupEngineStatus(engSta: UByte): EngineStatus = when (engSta.toInt()) {
    0x00, 0x01, 0x02 -> EngineStatus.STOP
    0x03 -> EngineStatus.RUNNING
    0x07 -> EngineStatus.FAULT
    else -> EngineStatus.STOP
}
```

**Codegen** (Rust):
```rust
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum EngineStatus { Stop, Running, Fault }

pub fn lookup_engine_status(eng_sta: u8) -> EngineStatus {
    match eng_sta {
        0x00..=0x02 => EngineStatus::Stop,
        0x03 => EngineStatus::Running,
        0x07 => EngineStatus::Fault,
        _ => EngineStatus::Stop,
    }
}
```

Language mapping conventions: C++ uses PascalCase enums + camelCase functions, Kotlin uses PascalCase enums + camelCase functions, Rust uses PascalCase enums + snake_case functions. These conventions are consistent across all kinds.

**Range pattern optimization**: Codegen may collapse consecutive keys with the same value into range patterns (e.g., Rust `0x00..=0x02`). This optimization is only applied when keys are numerically consecutive with no gaps. Non-consecutive keys (e.g., `0x00`, `0x02` without `0x01`) must remain as individual match arms to avoid unintended matching.

### 4.4 condition

Named boolean guard expression, reusable across multiple transitions. Supports both **standalone** (shared across statecharts) and **inline** (within a single statechart) usage. For simple single-use guards, inline is preferred; for guards shared by multiple statecharts, standalone avoids duplication.

**Standalone** (shared across statecharts):
```xml
<scxml sce:kind="condition">
  <datamodel>
    <data id="engineStatus" sce:type="string" sce:direction="in"/>
    <data id="ignition" sce:type="bool" sce:direction="in"/>
    <data id="canEnterProgramming" sce:type="bool" sce:direction="out"
          expr="engineStatus === 'STOP' &amp;&amp; ignition === true"/>
  </datamodel>
</scxml>
```

**Inline** (within a statechart's `<datamodel>`):
```xml
<data id="canEnterProgramming" sce:kind="condition"
      expr="engineStatus === 'STOP' &amp;&amp; ignition === true"/>

<!-- Referenced in multiple transitions -->
<transition cond="canEnterProgramming" target="programmingSession"/>
<transition cond="canEnterProgramming &amp;&amp; securityUnlocked" target="flashMode"/>
```

**Codegen** (C++):
```cpp
inline bool canEnterProgramming(const std::string& engineStatus, bool ignition) {
    return engineStatus == "STOP" && ignition;
}
```

### 4.5 procedure

**Domain attributes**: `sce:service` and `sce:subfunc` on `<send>` are codegen hints — sce-build maps them to method calls on the runtime's `DiagClient` interface (e.g., `client.send(ecuAddr, SecurityAccess{0x01})`). They are opaque strings to the SCXML parser; their interpretation is defined by the codegen template.

**Note**: procedure uses `<state>` elements internally but is semantically **run-to-completion** — it always reaches a `<final>` state and holds no persistent state across invocations. This distinguishes it from statechart, which is long-lived and driven by external events.

Sequential procedure with branching, retry, and error handling.

```xml
<scxml sce:kind="procedure" initial="sendTesterPresent">
  <datamodel>
    <data id="ecuAddr" sce:type="uint32" sce:direction="in"/>
    <data id="seed" sce:type="bytes" sce:direction="internal"/>
    <data id="maxRetries" expr="3" sce:type="int32" sce:direction="internal"/>
    <data id="retryCount" expr="0" sce:type="int32" sce:direction="internal"/>
  </datamodel>

  <state id="sendTesterPresent">
    <onentry>
      <send sce:service="TesterPresent" sce:addr="ecuAddr"/>
    </onentry>
    <transition event="ok" target="requestSeed"/>
    <transition event="fail" target="error"/>
  </state>

  <state id="requestSeed">
    <onentry>
      <send sce:service="SecurityAccess" sce:subfunc="0x01"/>
    </onentry>
    <transition event="ok" target="sendKey">
      <assign location="seed" expr="_event.data"/>
    </transition>
    <transition event="fail" target="retry"/>
  </state>

  <state id="sendKey">
    <onentry>
      <send sce:service="SecurityAccess" sce:subfunc="0x02"
            sce:payload="computeKey(seed)"/>
    </onentry>
    <transition event="ok" target="done"/>
    <transition event="fail" target="retry"/>
  </state>

  <state id="retry">
    <transition cond="retryCount &lt; maxRetries" target="requestSeed">
      <assign location="retryCount" expr="retryCount + 1"/>
    </transition>
    <transition cond="retryCount &gt;= maxRetries" target="error"/>
  </state>

  <final id="done">
    <donedata><param name="result" expr="'success'"/></donedata>
  </final>
  <final id="error">
    <donedata><param name="result" expr="'failure'"/></donedata>
  </final>
</scxml>
```

**Codegen**: procedure produces **two outputs** to support both `<invoke>` and direct invocation:

**Primary output** — state machine class (compatible with W3C `<invoke>`):
```cpp
// securityAccess_sm.h — invocable as child state machine
class SecurityAccessSM : public sce::StateMachine {
    uint32_t ecuAddr_;
    std::vector<uint8_t> seed_;
    int retryCount_ = 0;
    static constexpr int MAX_RETRIES = 3;

    void onEvent(const Event& e) override {
        switch (state_) {
        case State::SendTesterPresent:
            // ... state machine transitions matching SCXML ...
        }
    }
public:
    enum class FinalResult { SUCCESS, FAILURE };
    // Result accessible after reaching <final>
};
```

**Convenience wrapper** — synchronous run-to-completion function:
```cpp
// securityAccess.h — for direct invocation without <invoke>
inline SecurityAccessSM::FinalResult
executeSecurityAccess(DiagClient& client, uint32_t ecuAddr) {
    SecurityAccessSM sm{ecuAddr};
    return sm.runToCompletion(client);  // blocks until <final> reached
}
```

The state machine class is the canonical output. The wrapper is a thin convenience layer. When invoked via `<invoke type="scxml">`, the runtime uses the state machine class directly.

`runToCompletion()` is a method on `ProcedureStateMachine`, a standalone base class/trait provided in each language's forge runtime package. It creates an internal event loop, drives the state machine until a `<final>` state is reached, and returns the result. This is a **blocking call** intended for test harnesses, CLI tools, and non-real-time contexts. In RTOS/embedded environments, use `<invoke>` instead — the procedure runs as an async child state machine with no blocking.

`ProcedureStateMachine` is separate from the W3C statechart engine (`StaticExecutionEngine` / `StateMachineEngine`). It is a lightweight execution loop for run-to-completion procedures only. Generated code extends this base class (Kotlin/Python) or implements the `ProcedurePolicy` trait/interface (Rust/Go), providing the state-specific logic as abstract method implementations.

### 4.6 codec

Byte-level encode/decode. Bit position, size, endianness.

**Endianness**: default is `big` (automotive CAN/UDS convention). Override per-field with `sce:endian="little"` or per-document with `sce:default-endian="little"` on the `<scxml>` root.

**`sce:bit-size` values**: a fixed integer (`8`, `16`, `24`, `32`, `64`) or one of:

| Value | Meaning | Required attributes |
|-------|---------|-------------------|
| `tail` | Remaining bytes from `sce:byte` to end of frame | `sce:max-size` (codegen buffer limit) |
| `length-ref` | Size determined by another field's value | `sce:length-field="<field-id>"` |

`tail` example:
```xml
<sce:field id="payload" sce:byte="2" sce:bit-size="tail" sce:max-size="255"/>
<!-- bytes 2 through end of frame, max 255 bytes -->
```

`length-ref` example:
```xml
<sce:field id="dataLen" sce:byte="1" sce:bit-size="8"/>
<sce:field id="data" sce:byte="2" sce:bit-size="length-ref"
           sce:length-field="dataLen" sce:max-size="255"/>
<!-- data length is determined by the value of dataLen field (in bytes) -->
```

`length-ref` rules:
- Referenced field must be declared **before** the variable-length field in the same codec
- Length value is interpreted as **byte count**
- `sce:max-size` is still required for codegen buffer allocation

Codegen generates bounds checking for all variable-length fields: frame length is validated against `sce:byte` offset + `sce:max-size` before access. Decode returns an error/empty result for truncated frames.

```xml
<scxml sce:kind="codec" sce:default-endian="big">
  <datamodel>
    <data id="raw" sce:type="bytes" sce:direction="in" sce:length="8"/>
    <data id="serviceId" sce:type="uint8" sce:direction="out"
          sce:byte="0" sce:bit-offset="0" sce:bit-size="8"/>
    <data id="dtcSeverity" sce:type="uint8" sce:direction="out"
          sce:byte="1" sce:bit-offset="4" sce:bit-size="4"/>
    <data id="dtcStatus" sce:type="uint8" sce:direction="out"
          sce:byte="1" sce:bit-offset="0" sce:bit-size="4"/>
    <data id="dtcCode" sce:type="uint32" sce:direction="out"
          sce:byte="2" sce:bit-size="24"/>  <!-- sce:bit-offset defaults to 0; inherits big endian from document default -->
  </datamodel>
</scxml>
```

**Codegen** (C++):
```cpp
struct DtcResponse {
    uint8_t  serviceId;
    uint8_t  dtcSeverity;
    uint8_t  dtcStatus;
    uint32_t dtcCode;

    // decode: returns nullopt on truncated frame
    static std::optional<DtcResponse> decode(const uint8_t* raw, size_t len) {
        if (len < 5) return std::nullopt;
        return DtcResponse{
            .serviceId   = raw[0],
            .dtcSeverity = (raw[1] >> 4) & 0x0F,
            .dtcStatus   = raw[1] & 0x0F,
            .dtcCode     = (raw[2] << 16) | (raw[3] << 8) | raw[4]
        };
    }

    // encode: returns serialized bytes (value semantics, consistent with composition usage)
    std::vector<uint8_t> encode() const {
        return {
            serviceId,
            static_cast<uint8_t>((dtcSeverity << 4) | (dtcStatus & 0x0F)),
            static_cast<uint8_t>((dtcCode >> 16) & 0xFF),
            static_cast<uint8_t>((dtcCode >> 8) & 0xFF),
            static_cast<uint8_t>(dtcCode & 0xFF)
        };
    }
};
```

### 4.7 validator

Range check, rate-of-change detection, plausibility verification. Validator has minimal internal state (previous values for rate-of-change).

Validation rules are expressed as `sce:` attributes on `<data>` elements — each rule is co-located with the field it validates:

```xml
<scxml sce:kind="validator">
  <datamodel>
    <data id="rpm" sce:type="uint16" sce:direction="in"
          sce:range-min="0" sce:range-max="8000"
          sce:max-delta="500" sce:sample-interval="100ms"/>
    <data id="engineState" sce:type="string" sce:direction="in"/>
    <data id="valid" sce:type="bool" sce:direction="out"
          sce:plausibility="rpm === 0 || engineState !== 'STOP'"/>
  </datamodel>
</scxml>
```

- `sce:range-min`, `sce:range-max` — bounds check on the input field
- `sce:max-delta` — rate-of-change threshold per call
- `sce:sample-interval` — documents expected call frequency (codegen does not time-scale)
- `sce:plausibility` — cross-field boolean expression on the output field

> **Migration note**: The current implementation uses the legacy `<sce:rules>` custom element syntax (see tests). The `<data>` attribute syntax above is the target format; parser support will be added alongside `<sce:rules>` deprecation in a future release.

**Codegen** (C++):
```cpp
struct RpmValidator {
    uint16_t prevRpm_ = 0;

    ValidationResult validate(uint16_t rpm, const std::string& engineState) {
        if (rpm > 8000)
            return {false, "range_exceeded"};
        uint16_t delta = (rpm > prevRpm_) ? (rpm - prevRpm_) : (prevRpm_ - rpm);
        if (delta > 500)
            return {false, "rate_of_change_exceeded"};
        if (!(rpm == 0 || engineState != "STOP"))
            return {false, "plausibility_failed"};
        prevRpm_ = rpm;
        return {true, ""};
    }
};
```

### 4.8 filter

Signal filtering — moving average, low-pass, debounce. Filter configuration is expressed as `sce:` attributes on the output `<data>` element.

**Filter types**: `moving-average`, `low-pass`, `debounce`

**Moving average** — sliding window average:
```xml
<scxml sce:kind="filter">
  <datamodel>
    <data id="rawTemp" sce:type="float64" sce:direction="in"/>
    <data id="filtered" sce:type="float64" sce:direction="out"
          sce:filter="moving-average" sce:window="5"/>
  </datamodel>
</scxml>
```

**Low-pass** — exponential smoothing (alpha = smoothing factor, 0..1):
```xml
<scxml sce:kind="filter">
  <datamodel>
    <data id="rawSignal" sce:type="float64" sce:direction="in"/>
    <data id="smoothed" sce:type="float64" sce:direction="out"
          sce:filter="low-pass" sce:alpha="0.1"/>
  </datamodel>
</scxml>
```

**Debounce** — value must be stable for N consecutive samples:
```xml
<scxml sce:kind="filter">
  <datamodel>
    <data id="rawButton" sce:type="bool" sce:direction="in"/>
    <data id="stable" sce:type="bool" sce:direction="out"
          sce:filter="debounce" sce:window="3"/>
  </datamodel>
</scxml>
```

**Codegen** (C++, moving-average):
```cpp
struct TempFilter {
    std::array<double, 5> buffer_{};
    size_t index_ = 0;
    bool filled_ = false;

    double update(double rawTemp) {
        buffer_[index_] = rawTemp;
        index_ = (index_ + 1) % 5;
        if (!filled_ && index_ == 0) filled_ = true;
        size_t count = filled_ ? 5 : index_;
        double sum = 0;
        for (size_t i = 0; i < count; i++) sum += buffer_[i];
        return sum / count;
    }

    void reset() {
        buffer_ = {};
        index_ = 0;
        filled_ = false;
    }
};
```

### 4.9 interpolation

1D/2D table interpolation with axis definitions. Axis breakpoints are `sce:axis-{input_id}` attributes on the output `<data>` element; the table values are the element's text content (row-major for 2D).

**Interpolation methods**: `linear` (1D), `bilinear` (2D)
**Out-of-bounds**: `clamp` (default) | `extrapolate` | `error`

**1D example** (single axis):
```xml
<scxml sce:kind="interpolation">
  <datamodel>
    <data id="rpm" sce:type="uint16" sce:direction="in"/>
    <data id="torqueLimit" sce:type="float64" sce:direction="out"
          sce:interpolation="linear" sce:out-of-bounds="clamp"
          sce:axis-rpm="800 1200 2000 3000 4000 6000">
      120.0 145.0 200.0 230.0 210.0 180.0
    </data>
  </datamodel>
</scxml>
```

**2D example** (two axes, row-major values):
```xml
<scxml sce:kind="interpolation">
  <datamodel>
    <data id="rpm" sce:type="uint16" sce:direction="in"/>
    <data id="load" sce:type="uint8" sce:direction="in"/>
    <!-- Values are row-major: first axis (rpm) = rows, second axis (load) = columns -->
    <data id="injectionTime" sce:type="float64" sce:direction="out"
          sce:interpolation="bilinear" sce:out-of-bounds="clamp"
          sce:axis-rpm="800 1200 2000 3000 4000 6000"
          sce:axis-load="10 25 50 75 100">
      2.1 3.0 4.5 5.8 7.0
      2.5 3.5 5.0 6.5 8.0
      3.0 4.2 6.0 7.8 9.5
      3.5 5.0 7.0 9.0 11.0
      3.8 5.5 7.5 9.5 12.0
      4.0 5.8 8.0 10.0 12.5
    </data>
  </datamodel>
</scxml>
```

- `sce:axis-{id}` — axis breakpoints (space-separated), where `{id}` matches an input `<data>` id
- Text content — interpolation values (space/newline separated)
- Axis order determines row-major interpretation: first `sce:axis-*` = rows, second = columns

**Codegen** (C++):
```cpp
struct InjectionMap {
    static constexpr double AXIS_RPM[] = {800, 1200, 2000, 3000, 4000, 6000};
    static constexpr double AXIS_LOAD[] = {10, 25, 50, 75, 100};
    static constexpr double VALUES[6][5] = { /* ... */ };

    static double lookup(uint16_t rpm, uint8_t load) {
        return bilinearInterpolate(AXIS_RPM, 6, AXIS_LOAD, 5, VALUES, rpm, load);
    }

    // bilinearInterpolate is generated inline by codegen (no runtime dependency)
    static double bilinearInterpolate(/* ... */) { /* ... */ }
};
```

### 4.10 timer

Periodic, delayed, and timeout task timing. Each timer is a `<data>` element with `sce:timer` attributes specifying the scheduling type and parameters.

> **Naming note**: This kind is named `timer`, not `scheduler`, to avoid collision with SCE Mesh's `IScheduler` interface. `IScheduler` controls *how and when state machines process events* (tick-based vs event-driven execution model). The `timer` kind generates *periodic/delayed task timing logic* — a different abstraction level.

**Timer types**: `periodic`, `timeout`, `delayed`

```xml
<scxml sce:kind="timer">
  <datamodel>
    <data id="testerPresent" sce:timer="periodic" sce:interval="2000"
          sce:event="TesterPresent"/>
    <data id="responseTimeout" sce:timer="timeout" sce:duration="5000"
          sce:on-timeout="handleTimeout"/>
    <data id="retryDelay" sce:timer="delayed" sce:delay="10000"
          sce:event="retrySecurityAccess"/>
  </datamodel>
</scxml>
```

- `sce:timer` — timer type: `periodic` (repeating), `timeout` (one-shot, fires on expiry), `delayed` (one-shot, fires after delay)
- `sce:interval`, `sce:duration`, `sce:delay` — time in milliseconds
- `sce:event` — event name to emit when timer fires
- `sce:on-timeout` — callback name for timeout expiry (alternative to event)

**Codegen** (C++):
```cpp
struct DiagScheduler {
    Timer testerPresentTimer_{2000ms, [this]{ sendTesterPresent(); }};
    Timer responseTimeout_{5000ms, [this]{ handleTimeout(); }};

    void start() { testerPresentTimer_.startPeriodic(); }
    void waitResponse() { responseTimeout_.startOneShot(); }
    void onResponse() { responseTimeout_.cancel(); }
};
```

> **Runtime dependency**: Generated code requires a platform-provided `Timer` interface. The codegen emits calls to `startPeriodic()`, `startOneShot()`, `cancel()`. The platform implementation (POSIX, FreeRTOS, etc.) is injected at link time.

### 4.11 observer

Threshold monitoring with hysteresis. Each threshold monitor is a `<data>` element with `sce:monitor` attributes defining enter/leave conditions and event names.

```xml
<scxml sce:kind="observer">
  <datamodel>
    <data id="coolantTemp" sce:type="float64" sce:direction="in"/>
    <data id="warning" sce:monitor="threshold"
          sce:enter="coolantTemp &gt; 110.0"
          sce:leave="coolantTemp &lt; 100.0"
          sce:on-enter="emitWarning" sce:on-leave="clearWarning"/>
    <data id="critical" sce:monitor="threshold"
          sce:enter="coolantTemp &gt; 120.0"
          sce:leave="coolantTemp &lt; 105.0"
          sce:on-enter="emergencyShutdown"/>
  </datamodel>
</scxml>
```

- `sce:monitor` — monitor type (currently `threshold`; future: `rate`, `pattern`)
- `sce:enter` — condition expression for entering the active state (hysteresis high)
- `sce:leave` — condition expression for leaving the active state (hysteresis low)
- `sce:on-enter` — event name emitted when entering active state
- `sce:on-leave` — event name emitted when leaving active state (optional)

Event names are auto-derived from the `sce:on-enter`/`sce:on-leave` attribute values. The generated `Events` enum contains entries like `Event::WARNING`, `Event::WARNING_CLEARED`, `Event::EMERGENCY_SHUTDOWN`.

**Codegen** (C++):
```cpp
struct CoolantMonitor {
    bool warningActive_ = false;
    bool criticalActive_ = false;

    Events update(double coolantTemp) {
        Events events;
        if (!warningActive_ && coolantTemp > 110.0) {
            warningActive_ = true;
            events.push(Event::EMIT_WARNING);
        } else if (warningActive_ && coolantTemp < 100.0) {
            warningActive_ = false;
            events.push(Event::CLEAR_WARNING);
        }
        if (!criticalActive_ && coolantTemp > 120.0) {
            criticalActive_ = true;
            events.push(Event::EMERGENCY_SHUTDOWN);
        } else if (criticalActive_ && coolantTemp < 105.0) {
            criticalActive_ = false;
        }
        return events;
    }
};
```

---

## 5. Kind Composition

Kinds are not isolated. In real specifications, state machines reference lookups, procedures invoke codecs, and guards depend on conditions. The composition model uses SCXML's existing `<datamodel>` and `<cond>` mechanisms.

### 5.1 Composition Rules

```
statechart ──→ references all other kinds (guard, action, onentry)
    ├──→ lookup     (inline: guard references lookup result)
    ├──→ condition  (inline: guard uses named condition)
    ├──→ codec      (inline: onentry encodes, transition decodes)
    ├──→ transform  (inline: guard/action uses computed value)
    ├──→ procedure  (standalone: invoked via W3C <invoke>)
    ├──→ validator  (standalone: called in guard)
    ├──→ observer   (standalone: events feed into statechart)
    └──→ filter     (standalone: filtered values used in guards)

Other compositions:
    procedure ──→ codec (encode request, decode response)
    observer  ──→ filter (monitor filtered values)
    validator ──→ transform (validate after conversion)
```

### 5.2 Procedure Invocation via W3C `<invoke>`

Procedures are invoked using the standard W3C `<invoke>` element, not a custom `sce:invoke`. This preserves W3C compatibility and reuses the existing invoke codegen infrastructure. The procedure SCXML is a standalone file; `<invoke>` runs it asynchronously as a child state machine.

```xml
<!-- Standard W3C invoke — procedure runs as child, reports done.invoke when finished -->
<state id="unlocking">
  <invoke type="scxml" src="securityAccess.scxml" id="securityProc">
    <param name="ecuAddr" expr="ecuAddr"/>
    <finalize>
      <assign location="unlockResult" expr="_event.data.result"/>
    </finalize>
  </invoke>
  <transition event="done.invoke.securityProc"
              cond="unlockResult === 'success'"
              target="securityUnlocked"/>
  <transition event="done.invoke.securityProc"
              cond="unlockResult === 'failure'"
              target="securityLocked"/>
</state>
```

The procedure SCXML reaches a `<final>` state with `<donedata>` containing the result, which generates the standard W3C `done.invoke` event with the result accessible via `_event.data`.

### 5.3 Composition Example

A diagnostic session manager that combines statechart (document-level), lookup + condition + codec (inline), and procedure (standalone via invoke):

```xml
<scxml xmlns:sce="http://sce.dev/ext"
       sce:kind="statechart" initial="defaultSession">

  <datamodel>
    <!-- Inline lookup: external signal ENG_EngSta → engine status -->
    <data id="engineStatus" sce:kind="lookup"
          sce:input="ENG_EngSta" sce:input-type="uint8" sce:default="STOP">
      <sce:entry key="0x00" value="STOP"/>
      <sce:entry key="0x03" value="RUNNING"/>
      <sce:entry key="0x07" value="FAULT"/>
    </data>

    <!-- Inline condition: named composite guard -->
    <data id="canEnterProgramming" sce:kind="condition"
          expr="engineStatus === 'STOP' &amp;&amp; ignition === true"/>

    <!-- Inline codec: response parser -->
    <data id="securityResponse" sce:kind="codec">
      <sce:field id="result" sce:byte="0" sce:bit-size="8"/>
      <sce:field id="seed" sce:byte="1" sce:bit-size="32" sce:endian="big"/>
    </data>

    <!-- Inline codec: write request encoder -->
    <data id="writeDataRequest" sce:kind="codec">
      <sce:field id="did" sce:byte="0" sce:bit-size="16" sce:endian="big"/>
      <sce:field id="payload" sce:byte="2" sce:bit-size="tail" sce:max-size="255"/>
    </data>

    <data id="writeData" sce:type="bytes" sce:direction="in"/>
  </datamodel>

  <!-- Statechart uses inline kinds in guards and actions -->
  <state id="defaultSession">
    <transition event="SID_0x10_0x02"
                cond="canEnterProgramming"
                target="programmingSession"/>
  </state>

  <state id="extendedSession" initial="securityLocked">
    <state id="securityLocked">
      <transition event="requestUnlock" target="unlocking"/>
    </state>

    <!-- Procedure invoked via standard W3C <invoke> -->
    <state id="unlocking">
      <invoke type="scxml" src="securityAccess.scxml" id="securityProc">
        <param name="ecuAddr" expr="ecuAddr"/>
        <finalize>
          <assign location="unlockResult" expr="_event.data.result"/>
        </finalize>
      </invoke>
      <transition event="done.invoke.securityProc"
                  cond="unlockResult === 'success'"
                  target="securityUnlocked"/>
      <transition event="done.invoke.securityProc"
                  cond="unlockResult === 'failure'"
                  target="securityLocked"/>
    </state>

    <state id="securityUnlocked">
      <transition event="SID_0x2E"
                  cond="engineStatus === 'STOP' &amp;&amp; ignition"
                  target="writing"/>
    </state>

    <state id="writing">
      <onentry>
        <send event="SID_0x2E">
          <param name="payload" expr="writeDataRequest.encode(0xF190, writeData)"/>
        </send>
      </onentry>
      <transition event="response">
        <!-- expr uses data id directly; codegen maps to generated type -->
        <assign location="writeResult"
                expr="securityResponse.decode(_event.data)"/>
      </transition>
      <transition cond="writeResult.result === 0x00" target="writeSuccess"/>
      <transition cond="writeResult.result !== 0x00" target="writeFailed"/>
    </state>
  </state>
</scxml>
```

### 5.4 Resolution Rules

**Shared references**: When two statecharts reference the same standalone lookup, the lookup is generated once as an independent header. Both statecharts `#include` it. No duplication.

**Name isolation**: Standalone kinds generate code in a namespace derived from their filename (`SCE::Generated::<filename>`). Inline kinds generate code in the parent statechart's namespace. No collision possible.

**Circular composition**: Prohibited at the static reference level. Build-time analysis detects cycles and reports an error. Note: event-based communication is not a cycle — observer emits events that a statechart receives via its event queue. This is asynchronous decoupling, not a reference cycle.

### 5.5 Generated Code (Composition)

The codegen produces inline helpers from inline kinds and references standalone kinds. The statechart orchestrates both.

```cpp
namespace SCE::Generated::DiagSession {

// From inline lookup kind
enum class EngineStatus { STOP, RUNNING, FAULT };
inline EngineStatus lookupEngineStatus(uint8_t engSta) { /* ... */ }

// From inline codec kind
struct SecurityResponse {
    uint8_t result;
    uint32_t seed;
    static SecurityResponse decode(const uint8_t* raw) { /* ... */ }
};

// From inline condition kind
inline bool canEnterProgramming(EngineStatus es, bool ign) {
    return es == EngineStatus::STOP && ign;
}

// Standalone procedure is in its own header:
// #include "securityAccess_sm.h" (generated from securityAccess.scxml)

// From statechart kind — orchestrates all above
class DiagSession : public sce::StateMachine {
    EngineStatus engineStatus_ = EngineStatus::STOP;
    bool ignition_ = false;

    void onEvent(const Event& e) override {
        switch (state_) {
        case State::DefaultSession:
            if (e.id == SID_0x10_0x02 &&
                canEnterProgramming(engineStatus_, ignition_))
                transition(State::ProgrammingSession);
            break;
        case State::Unlocking:
            // W3C <invoke> — procedure runs as child state machine
            // done.invoke event arrives when procedure reaches <final>
            if (e.name == "done.invoke.securityProc") {
                auto result = e.data<std::string>("result");
                transition(result == "success"
                    ? State::SecurityUnlocked : State::SecurityLocked);
            }
            break;
        case State::Writing: {
            auto resp = SecurityResponse::decode(e.data());
            transition(resp.result == 0x00
                ? State::WriteSuccess : State::WriteFailed);
            break;
        }
        }
    }

    void updateInputs(uint8_t engSta, bool ign) {
        engineStatus_ = lookupEngineStatus(engSta);
        ignition_ = ign;
    }
};

}
```

---

## 6. Code Generation Architecture

### 6.1 Self-Contained vs Runtime-Dependent

| Kind | Dependencies | C++ Header-Only | Needs sce_runtime |
|------|-------------|-----------------|-------------------|
| transform | None | Yes (pure inline) | No |
| lookup | None | Yes (pure inline) | No |
| condition | None | Yes (pure inline) | No |
| codec | None | Yes (pure inline) | No |
| filter | None (internal buffer) | Yes (class with state) | No |
| interpolation | None (internal table) | Yes (constexpr data) | No |
| validator | None (internal prev state) | Yes (class with state) | No |
| procedure | ProcedureServiceHandler, ProcedureStateMachine | Yes (requires runtime headers) | Yes |
| timer | Timer | Yes (requires runtime headers) | Yes |
| observer | EventBus | Yes (requires runtime headers) | Yes |
| statechart | EventQueue, ActionHandler | Yes (requires runtime headers) | Yes (existing) |

Kinds marked "requires runtime headers" are header-only in that they have no `.cpp` files, but they `#include` sce_runtime interface headers (`sce/DiagClient.h`, `sce/Timer.h`, etc.) and require linking against a platform-specific runtime implementation.

### 6.2 Expression Transpiler

The expression transpiler (`sce-build/src/forge/expr.rs`) is an AST-based pipeline shared by all kind generators. It is the single point responsible for converting ECMAScript expressions to target-language code.

```
sce-build/src/forge/
  expr.rs          — tokenizer, parser, AST, per-language emitters
  model.rs         — ForgeKind data structures (Transform, Lookup, Condition, Codec)
  parser.rs        — SCXML parsing, sce:kind attribute extraction
  generator.rs     — language-specific code generation, calls expr::transpile()
```

**Public API** (used by `generator.rs`):

```rust
// Transpile an ECMAScript expression to the target language.
pub fn transpile(expr: &str, target: ExprTarget) -> Result<String, String>;

// Strip string literal contents (for expression analysis in generator).
pub fn strip_string_literals_pub(expr: &str) -> String;
```

The generator calls `expr::transpile()` for every `expr` and `cond` attribute in the SCXML source. The transpiler handles all language-specific differences (operator syntax, precedence, string quoting) so generators only deal with structural code emission.

### 6.3 Template Structure

New kind templates are added to `sce-build` (Rust + minijinja), which is the single source of truth for code generation. Templates in `tools/codegen/templates/` are shared with the `sce-build` binary.

```
tools/codegen/templates/forge/
  cpp/
    transform.h.jinja2
    lookup.h.jinja2
    condition.h.jinja2
    procedure.h.jinja2        (L1: guard-only)
    procedure_l2.h.jinja2     (L2: event-driven, StaticExecutionEngine)
    codec.h.jinja2
    validator.h.jinja2
  kotlin/
    ...                       (same kinds as cpp, .kt.jinja2)
  rust/
    ...                       (same kinds as cpp, .rs.jinja2)
  go/
    ...                       (same kinds as cpp, .go.jinja2)
  python/
    ...                       (same kinds as cpp, .py.jinja2)
```

### 6.4 Codegen Dispatch

```
SCXML input
  │
  ├─ Parse XML (existing parser)
  ├─ Read sce:kind attribute (absent → default "statechart")
  │
  ├─ kind == "statechart"    → existing codegen pipeline (unchanged)
  │   └─ Scan <data sce:kind="..."> → generate inline helpers
  │
  ├─ kind == "transform"     → transform template
  ├─ kind == "lookup"        → lookup template
  ├─ kind == "condition"     → condition template (standalone bool function)
  ├─ kind == "procedure"     → procedure template (SM class + convenience wrapper)
  ├─ kind == "codec"         → codec template
  ├─ kind == "validator"     → validator template
  ├─ kind == "filter"        → filter template
  ├─ kind == "interpolation" → interpolation template
  ├─ kind == "timer"         → timer template
  └─ kind == "observer"      → observer template

```

### 6.5 Error Model

Error handling varies by kind — this is intentional, as each kind has a different failure domain:

| Kind | Error Model | Rationale |
|------|-------------|-----------|
| transform | No error — pure computation | Division by zero etc. is caller's input validation responsibility |
| lookup | Default value on miss | `sce:default` attribute; no key match is expected, not exceptional |
| condition | No error — pure boolean | Always returns true or false |
| codec | `std::optional` / nullable return | Truncated frames are common in automotive; caller must check |
| validator | Result struct with reason | Validation is expected to fail; reason is part of the value |
| filter | No error — always produces output | Partial window returns partial average |
| interpolation | Depends on `sce:out-of-bounds` | `clamp`: no error, `error`: optional return |
| procedure | Result enum (success/failure) | Communicated via `<donedata>` on `<final>` |
| timer | No error — timer management | Invalid timing is a codegen-time error, not runtime |
| observer | No error — threshold evaluation | Always produces (possibly empty) event list |

---

## 7. Roadmap

### Phase 1: Foundation (Complete)

Core kind support and codegen templates for the most common patterns.

- `sce:kind` attribute parsing in sce-build
- Document-level codegen templates for: `transform`, `lookup`, `condition`, `codec`
- Inline kind support in statechart template: `condition`, `lookup`, `codec`, `transform`
- Schema validation (XSD) for Extended SCXML
- C++, Kotlin, Rust codegen output (header-only / single-file generation)
- Kind conformance test suite: reference SCXML + expected codegen output per kind
- Architecture decision for cross-file kind references (Phase 2 dependency: how sce-build resolves `<invoke src="other.scxml">` and standalone kind imports — build manifest, CMake integration, or sce-build dependency scanner)

### Phase 2: Procedural + Multi-Language

Sequential logic support and multi-language code generation.

- ~~Codegen templates for: `procedure`, `validator`~~ **Done**: procedure (L1 guard-only + L2 event-driven) and validator for all 5 languages
- ~~`sce::ProcedureStateMachine` base class in sce_runtime~~ **Done**: Per-language runtime packages (Kotlin abstract class, Rust trait, Go interface, Python ABC) with shared event loop and service types
- Cross-file kind composition: standalone kinds referencing other standalone kinds (e.g., procedure → codec, validator → transform). Phase 1 inline kinds are within a single statechart; Phase 2 enables references across separate SCXML files.
- ~~Go, Python codegen templates for all Phase 1+2 kinds~~ **Done**: All Phase 1+2 kinds generate for all 5 languages — 101 conformance tests

### Phase 3: Signal Processing + Advanced Kinds

Embedded/automotive signal processing patterns.

- Codegen templates for: `filter`, `interpolation`, `timer`, `observer`
- sce_runtime extensions (Timer, EventBus interfaces)
- Platform-specific runtime implementations (POSIX, FreeRTOS)

**Phase 3 exit criteria**: All 11 kinds generate compilable code for all 5 target languages (C++/Kotlin/Rust/Go/Python). Kind conformance test suite passes for all kinds.

### Phase 4: SCE Mesh Integration + Ecosystem

Integration with SCE Mesh distributed runtime and tooling.

- **codec → ISerializer bridge**: Forge `codec` kinds generate encode/decode structs. SCE Mesh's `ISerializer` wraps these generated structs to provide event serialization for transport. When a `codec` kind exists for an event payload type, the build tool generates an `ISerializer` adapter instead of requiring a separate `events.yaml` declaration. This makes the codec SCXML the single source of truth for both local parsing and remote serialization.
- **procedure → remote invoke**: Forge `procedure` kinds generate `sce::StateMachine`-compatible classes. SCE Mesh's remote `<invoke>` (Section 9) executes these across device boundaries unchanged. No additional codegen required — the procedure class is the same whether invoked locally or remotely.
- **observer → EventRouter**: Forge `observer` kinds generate threshold events. SCE Mesh's `EventRouter` routes these events to remote state machines via the configured `ITransport`. The observer kind remains transport-agnostic; routing is determined by `deploy.yaml`.
- VS Code extension for Extended SCXML editing with kind-aware autocomplete
- Unified XSD schema covering both Forge and Mesh `sce:` attributes

---

## 8. Kind Summary

| Kind | State | Runtime Dep | Scope | Complexity | Priority |
|------|-------|-------------|-------|------------|----------|
| statechart | Persistent (N states) | Yes | Document | Existing | Existing |
| transform | None | No | Document or Inline | Low | Phase 1 |
| lookup | None | No | Document or Inline | Low | Phase 1 |
| condition | None | No | Document or Inline | Low | Phase 1 |
| codec | None | No | Document or Inline | Medium | Phase 1 |
| procedure | Transient (run-to-completion) | Yes | Document | Medium | Phase 2 |
| validator | Minimal (prev value) | No | Document | Medium | Phase 2 |
| filter | Internal (buffer) | No | Document | Medium | Phase 3 |
| interpolation | None | No | Document | Medium | Phase 3 |
| timer | Internal (timers) | Yes | Document | Medium | Phase 3 |
| observer | Internal (flags) | Yes | Document | Medium | Phase 3 |

---

## 9. Success Criteria

### Phase 1 Exit Criteria

- [x] `sce:kind` attribute parsed and dispatched by sce-build
- [x] Codegen templates for `transform`, `lookup`, `condition`, `codec` produce compilable C++/Kotlin/Rust/Go/Python output
- [x] Inline kind support generates helper functions/types within statechart
- [ ] XSD schema validates Extended SCXML documents
- [x] Kind conformance test suite: 101 tests across 7 kinds and 5 languages

### Overall Success Definition

**An engineer writes an Extended SCXML file with `sce:kind`. sce-build generates working C++/Kotlin/Rust/Go/Python code (Phase 1 kinds complete for all 5 languages). All languages produce identical behavior from the same SCXML source.**

The measure is: **one SCXML source generates correct, compilable code for all target languages**, with behavior equivalence verified by kind conformance tests.

**Cross-language equivalence verification**: each conformance test provides a reference SCXML, a set of input values, and expected output values. The generated code for each language is compiled, executed with the same inputs, and outputs are compared against the golden expected values. All languages must produce identical results.
