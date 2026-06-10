# RFC — SCE Forge Extensions for Wire Protocol Synthesis

**Status:** **Draft (2026-04-24)**. Originates from the `watching-zenoh`
project, which aims to synthesize a Zenoh-protocol-compatible stack
(Rust for AP, C for MCU) entirely from SCXML + SCE Forge sources. This
RFC enumerates the SCE capabilities that project requires but SCE does
not yet provide, with motivation, proposed shape, non-goals, and phased
rollout.

**Counterparts on acceptance:**
- `ARCHITECTURE.md` → "Scope & Composition" charter admission
- `SCE_FORGE.md` → new kinds (`algorithm`, `link`, `buffer-pool`, `timer`, `worker`)
  and codec DSL extensions documented
- `SCE_ERROR_CONTRACT.md` §5 → new diagnostic codes
- `docs/SCE_ACCEPTED_SUBSET.md` § new subset appendix entries
- `schemas/sce-forge-ext.xsd` → new element declarations
- `sce-build/src/forge/{model,parser,generator,type_ctx}.rs` → IR and codegen
- `tools/codegen/templates/forge/{rust,c}/` → new C backend tree, no_std Rust variant

---

## §0 Reading guide

This RFC is structured for partial adoption. §1–§3 are the motivation
and end-state that every reader should understand. §4 is a capability
survey of current SCE (what already exists — respect prior work). §5
proposes the additions, grouped A through O, each individually
reviewable. §6 covers cross-cutting concerns (diagnostics, testing,
tooling). §7 is a phased rollout plan. §8 lists open questions. The
appendices contain worked examples and an honest scope statement for
the downstream project.

---

## §1 Motivation

### 1.1 Downstream project

`watching-zenoh` synthesizes a Zenoh-protocol-compatible networking
stack from SCXML + SCE Forge as the single source of truth. Two
artifacts are produced from the same sources:

- **AP backend** — Rust (with `std`, tokio runtime) targeting Linux
  x86_64 / aarch64
- **MCU backend** — C11 (no_std-equivalent, no_alloc). Target
  architecture and SoC are TBD; selection is deferred until deploy
  authoring begins. The extensions proposed in this RFC must not
  presume a specific CPU family — the target descriptor
  (deploy.yaml `platform.soc` / `platform.core`) is the single
  point where target identity enters codegen.

Both backends must interoperate on the wire with upstream `zenohd` and
upstream `zenoh-pico` clients for a bounded protocol subset (see
Appendix C).

### 1.2 Why SCE, not a direct rewrite

Using SCE as the synthesis framework yields three properties that a
hand-written reimplementation does not structurally guarantee:

1. **AP/MCU byte-equivalence by construction.** Both backends descend
   from the same SCXML source through the same Forge type system and
   the same expression transpiler. Cross-backend divergence reduces to
   codegen template bugs, not algorithm drift.
2. **Audit surface reduction.** Cert environments (ISO 26262, IEC
   61508) review the SCXML + kind definitions, not two independent
   language-specific implementations. The generated code is a derived
   artifact with a bounded generator.
3. **Reusable investment.** The extensions proposed here are not
   Zenoh-specific; they generalize to SOME/IP-SD, DDS-RTPS, OPC UA,
   and similar wire protocols. Once SCE can synthesize one, it can
   synthesize all of that class.

### 1.3 Why SCE cannot currently do this

The discovery pass (§4) found that SCE has a strong foundation —
bitwise operators are complete, an event-driven `procedure` kind
exists, the expression transpiler is production quality. What is
missing is the set of primitives needed to express:

- **Pure algorithmic code** (tight loops, mutable accumulators, const
  lookup tables) — no existing kind has this shape.
- **Binary wire formats** beyond fixed-width struct layouts — Zenoh
  uses VLE ZInt, discriminated unions, optional fields driven by flag
  bits, TLV extension chains.
- **Byte-level I/O endpoints** — SCE Mesh assumes a transport
  *library* exists below; here the project *is* the transport.
- **Static memory placement** — MCU zero-copy requires pool slots
  placed in named SRAM sections aligned to DMA constraints.
- **Multi-language backend coverage** — no C backend exists today;
  the Rust backend assumes `std`.

---

## §2 Non-goals

This RFC separates **permanent non-goals** from **MVP deferrals**
(items kept out of MVP scope for bounded review, but structurally
supportable as follow-up work). Each MVP deferral has a documented
migration path so the current design does not close the door on it.

### §2.1 Permanent non-goals

These are architecturally ruled out by this RFC and any future
extension of it:

- **A general-purpose embedded framework.** The extensions are
  wire-protocol-synthesis-shaped, not RTOS-shaped.
- **A replacement for `sce_runtime`.** The interpreter path remains
  untouched. This RFC only adds new AOT-generatable kinds.
- **Turing-completeness in `algorithm` kind.** `<sce:while>` accepts
  a `max-iter` attribute enforced at build time when feasible and at
  runtime otherwise; unbounded recursion is permanently forbidden.
  (§5.H adds bounded recursive data traversal in Phase 2; this is
  not the same as general recursion.)
- **Dynamic memory allocation on MCU.** The no-heap invariant is a
  physical constraint of the MCU target class, not a preference.
  Bounded-dynamic via declared `bounded-collection` (§5.L) is
  permitted; heap is not.
- **Script engines on MCU targets.** `sce_scripting` remains
  forbidden when `platform.class = mcu`. AP may use `sce_scripting`
  orthogonally.
- **Router-mode synthesis on MCU.** Global topology state is
  unbounded across the network; forwarding and aggregation tables
  cannot be bounded at build time without falsifying correctness.

### §2.2 MVP deferrals (with migration paths)

These are **not** permanently excluded; they are out of the Phase
A–C MVP but land in later phases. The RFC's kinds and deploy model
are designed so these can extend the design, not replace it.

| Item | MVP reason | Migration path |
|---|---|---|
| Router mode on AP | Unbounded dynamic state; new graph-algorithm kind | Phase D+. Builds on §5.H (recursive/tree types) + §5.G (parametric kinds) + a new dynamic aggregation kind. Uses MVP-synthesized session/codec/link as dependencies |
| Subscription aggregation on AP | Router-mode feature | Same path as router |
| Shared-memory transport | Advanced transport, not in zenoh-pico baseline | Phase E. New pool variant with cross-process slot identity |
| Additional link drivers (BLE, Raweth, QUIC, custom) | Platform-specific; not required for zenoh-pico parity | Already enabled by §5.I target-plugin mechanism; add plugin entries without patching SCE core |
| API shims (`zenoh-c`, `zenoh-pico` drop-in) | Not a synthesis concern | Host-side Rust/C crates that call synthesized library; mechanical wrapper, no new kinds needed |
| Plugin ecosystem on AP (storages, REST, etc.) | Host-side services; not synthesis | Synthesized AP code is a library crate; plugins consume it |
| **AP on Linux** | Phase D — first AP target after MCU zenoh-pico parity (Phase C14). `sce_link_runtime_tokio` runtime crate (epoll/kqueue via mio) + optional `io_uring` opt-in for high-fanout deploys | Phase D.1. Schema accommodates this from review #13 (§5.K `platform.os: linux`, §5.J OS-axis backend convention); implementation is Phase D first deliverable |
| **AP on QNX** | Phase D+ — second AP target. QNX SDP 7.1+ with io-sock POSIX sockets baseline; QNX-native `dispatch_*` reactor (mio has no QNX backend). QNX-native typed-message and shared-memory IPC link-classes (and any future OS-specific classes) land additively at Phase D.2 entry; not pre-declared in the §5.C enum | Phase D.2. Schema accommodates the OS dimension from review #13 (§5.K `platform.os: qnx`, §5.J OS-axis backend convention); `sce_link_runtime_qnx` crate is Phase D second deliverable. **The design is QNX-aware (OS axis is first-class in `platform.os`) so adding QNX does not require schema rework** |
| AP on macOS / FreeBSD / Windows | Phase E — additional AP targets after Linux + QNX baselines stabilize. Each follows the same OS-axis pattern (`sce_link_runtime_kqueue` / `sce_link_runtime_iocp`); these enum values land in `platform.os` when the corresponding phase opens, not preemptively | Phase E |

**MVP criterion reference.** The downstream watching-zenoh project
uses **full zenoh-pico feature parity** (in peer + client modes) as
its MVP gate. The kind set and codec DSL in §5 are sized accordingly:
`bounded-collection`, runtime wildcard KeyExpr matching, fragment
reassembly, and multi-link concurrency are all MVP items, not
deferrals.

---

## §3 Target end-state

After all phases in §7 land, a user authoring Zenoh-protocol-synthesis
sources experiences:

```
$ ls sources/
  zenoh_session.scxml            # session FSM, sce:kind="statechart"
  zenoh_codec_session_msg.scxml  # wire codec, sce:kind="codec"
  zenoh_codec_network_msg.scxml
  zenoh_vle.scxml                # sce:kind="algorithm"
  zenoh_crc.scxml                # sce:kind="algorithm"
  zenoh_keyexpr_match.scxml      # sce:kind="algorithm"
  zenoh_link_udp.scxml           # sce:kind="link"
  zenoh_timers.scxml             # sce:kind="timer"
  deploy.yaml                    # machines, platforms, QoS, memory

$ sce-codegen build deploy.yaml
  → Generated out/ap/*.rs    (Rust, std; runtime crate per platform.os:
                              linux  → sce_link_runtime_tokio,
                              qnx    → sce_link_runtime_qnx,
                              macos  → sce_link_runtime_kqueue,
                              ...)
  → Generated out/mcu/*.c + *.h (C11, no_std, DMA descriptors;
                              runtime crate sce_link_runtime_lwip
                              for bare_metal, sibling crates for
                              FreeRTOS / Zephyr / NuttX target plugins)

$ cargo build -p zenoh-ap
$ cmake --build build_mcu

$ zenohd --listen udp/224.0.0.224:7446 &
$ ./out/ap/zenoh-ap-node
  → SCOUT sent, HELLO received, OPEN handshake, Established
```

**Phase scope today.** The MCU path (`out/mcu/`) lands in Phase A–C
and is the priority track — full zenoh-pico parity (§7 Phase C14)
is the project gate. The AP path (`out/ap/`) lands in Phase D, with
Linux as the first AP target (Phase D.1) and QNX as the second
(Phase D.2). The runtime-crate-per-OS dispatch shown above is
schema-stable from review #13 onward so authoring `deploy.yaml`
during MCU phases does not foreclose AP target choices later.

QoS selection is per-binding in `deploy.yaml`, lowered to typed Rust
config (AP, runtime) or `static const` struct (MCU, compile-time).

Zero-copy holds for control-path and bounded-payload cases:
single-frame messages up to the declared pool slot size are parsed
in-place from the DMA buffer and published onward without staging.
Fragmented or oversized payloads go through a declared stage buffer
(see Appendix C for honest scope).

---

## §4 Current SCE capability survey

Verified by reading `sce-build/src/forge/` on 2026-04-24. Items are
listed to prevent duplicate work and to anchor later sections.

### Already present (do NOT re-propose)

| Capability | Location | Notes |
|---|---|---|
| Bitwise operators `& \| ^ << >> ~` | `forge/expr.rs:304` and 5 backend emitters | Full precedence tree; `join_int` type rule; `>>>` (UShr) included |
| Hex/bin/oct literals typed correctly | `forge/expr.rs:26` | Untyped-int inference with promotion rules |
| Expression transpiler to 5 targets | `forge/expr.rs` | C++, Rust, Kotlin, Go, Python emitters |
| Forge kind emitters (all 11 kinds × 5 languages) | `tools/codegen/templates/forge/{rust,cpp,kotlin,go,python}/*.jinja2` | Per-kind `.jinja2` templates; dispatched via `Language::{Rust,Cpp,Kotlin,Go,Python}` in `sce-build/src/generator.rs:30` (note: no `C` / `C11` variant — see §5.J) |
| Typed parameters, inputs, outputs | `forge/model.rs::ForgeField`, `SceType` | u8–u64, i8–i64, f32/f64, bool, string, bytes |
| Cross-file imports with alias | `forge/manifest.rs` | Stateless imports as functions; stateful as members |
| XSD validation at parse entry | `forge/xsd_validator.rs` | libxml2 FFI, two-file W3C+sce:ext |
| `sce:template` + `sce:param` (RFC accepted) | `claudedocs/rfc-sce-template-sce-param.md` | Lexical substitution primitive |
| Eleven kinds, all `is_supported() == true` | `ForgeKind::{Statechart, Transform, Lookup, Condition, Codec, Procedure, Validator, Filter, Interpolation, Timer, Observer}` (`model.rs:70`) | Runtime tier per `max_runtime_dep()` — `None` (Transform, Condition, Codec, Validator), `ForgeRuntime` (Lookup, Procedure, Filter, Interpolation, Observer), `ForgeRuntimeHal` (Timer), `SceRuntime` (Statechart). Kind characterization methods (`is_inline_eligible`, `needs_instance`) already encode cross-kind rules that new kinds in §5.A/C/D/E/L must fit |
| Rust Forge runtime is `no_std` + no-alloc baseline | `sce-forge-runtime/rust/src/lib.rs:16` (`#![no_std]`), `Cargo.toml` | Std/alloc are opt-in features; `Transform`/`Codec`/`Procedure`/`Validator` are thus already MCU-capable on Rust. The **statechart** runtime (`sce-rust-runtime/`) remains `std`-only — see §5.J.2 |

### Shape of existing `Procedure` kind

The existing `Procedure` kind is **event-driven** — `ProcedureState`
with `ProcedureTransition { event, cond, target, assigns }`,
`on_entry_sends`, `done_params`. It is a state machine whose
transitions are driven by asynchronous events, not a pure function
with synchronous control flow. It does not fit tight inner loops
(CRC byte fold, VLE shift loop, KeyExpr segment walk) without absurd
contortions. This RFC adds a separate kind rather than overloading
`Procedure`.

### Shape of existing `Codec` kind

`CodecModel` holds a list of `CodecField { bit_size, endian, ... }`
— fixed-width struct layout with explicit endianness. It does not
handle variable-length integers, discriminated unions, conditional
presence, repeated fields, or extension chains. §5.B extends it.

### Shape of existing `Timer` kind

`TimerModel` exists with `TimerEntry` and `TimerType` enum. The shape
and capabilities need to be reviewed against §5.D requirements; if
they already cover the protocol-synthesis need, §5.D collapses to a
no-op. Reviewer input requested.

---

## §5 Proposed extensions

Each sub-section is individually reviewable. Ordering is the
recommended review order, not a hard dependency (B depends on F; J
depends on all prior).

### 5.A New kind: `algorithm`

**Problem.** No existing kind expresses pure synchronous functions
with bounded loops and mutable locals. Attempting to use `Procedure`
for CRC or VLE forces per-byte event dispatch (absurd performance
and grotesque modeling). `Transform` is a single expression.

**Proposal.** Add `ForgeKind::Algorithm` with shape:

```xml
<scxml xmlns="http://www.w3.org/2005/07/scxml"
       xmlns:sce="http://scxml-core-engine.org/sce/forge"
       sce:kind="algorithm" name="crc16_ccitt" version="1.0">
  <sce:signature>
    <sce:param name="data" type="bytes"/>
    <sce:return type="u16"/>
  </sce:signature>

  <sce:const name="TABLE" type="array<u16, 256>" sce:compute-at="build">
    <!-- build-time fold, see §5.F -->
  </sce:const>

  <sce:body>
    <sce:var name="crc" type="u16" init="0xFFFF"/>
    <sce:foreach item="b" in="data">
      <sce:assign target="crc"
                  expr="TABLE[((crc >> 8) ^ b) &amp; 0xFF] ^ (crc &lt;&lt; 8)"/>
    </sce:foreach>
    <sce:return expr="crc"/>
  </sce:body>
</scxml>
```

**IR additions** (`forge/model.rs`):

```rust
pub struct AlgorithmModel {
    pub signature: AlgorithmSignature,
    pub consts: Vec<AlgorithmConst>,
    pub body: Vec<AlgorithmStmt>,
}
pub enum AlgorithmStmt {
    Var   { name: String, ty: SceType, init: Expr },
    Assign{ target: LValue, expr: Expr },
    If    { cond: Expr, then_body: Vec<AlgorithmStmt>,
            else_body: Option<Vec<AlgorithmStmt>> },
    While { cond: Expr, body: Vec<AlgorithmStmt>, max_iter: Option<u32> },
    Foreach{ item: String, source: Expr, body: Vec<AlgorithmStmt> },
    Return{ expr: Option<Expr> },
    Call  { target: String, args: Vec<Expr> },  // to other algorithm kinds
}
```

**Codegen contract.** Each of the six backends emits the same
algorithm body shape, lowered to its language's idiomatic loop
constructs. The §5.J.5 emitter table lists per-language signatures;
shared invariants:

- All emit a free function (no class/object state). Parameters are
  by-value scalars or by-reference (`&[u8]`/`span`/`memoryview`/
  `[]byte`/`bytes`/`const uint8_t*+size_t`) for byte slices.
- `for` lowers to the language's bounded counted loop; `while` lowers
  to a counter-checked loop when `max-iter` is build-time-known and
  to a runtime-counter-guarded loop otherwise (`algorithm/while-iter-
  exceeded` runtime diagnostic on overrun).
- `let mut`/local-var lowers to the language's mutable local;
  no heap allocation, no closures, no exceptions/panics.
- Rust `#![no_std]` compatible when no `bytes` params (or using
  `&[u8]` instead of `Vec<u8>`); C11 emits with `<stdint.h>` types
  and `_Static_assert` on bound constants. Cpp emits free functions
  in `namespace sce::generated`, no STL containers, no exceptions.
  Kotlin emits an `object` singleton with a `call(...)` method.
  Go emits a top-level `func`. Python emits a module-level `def`.

This makes algorithm authoring decoupled from `platform.os` and
`class`: the same algorithm SCXML emits in all six backends, and
the §6.2.6 cross-backend parity test verifies semantic equivalence
on shared test vectors.

**LValue scope v1:** identifier, member access (`x.field`), index
(`x[e]`). Nested/chained forms same as existing expression grammar.

**Max-iter on `<sce:while>`:** build-time checkable when the loop
variable bound is a const; runtime-checked otherwise (counter,
diagnostic `algorithm/while-iter-exceeded`). Required to preserve the
"bounded" guarantee that makes this kind MCU-viable.

**Diagnostics** (add to SCE_ERROR_CONTRACT.md):
- `algorithm/local-shadows-param`
- `algorithm/return-type-mismatch`
- `algorithm/return-missing` — non-void without return on all paths
- `algorithm/while-unbounded` — on MCU platform without `max-iter`
- `algorithm/lvalue-unsupported` — e.g. write to param (params are read-only v1)
- `algorithm/call-cycle` — recursion forbidden v1

**WCET annotation (cooperative-scheduler targets).** The bounded-
loops + no-recursion contract guarantees *termination*; it does not
guarantee that an algorithm fits inside a cooperative scheduler
slot. Algorithms that run inside a worker slot on a target with
`scheduler.kind: cooperative` MUST carry an explicit WCET bound:

```xml
<sce:wcet-bound mode="static"   estimate_us="120"/>   <!-- preferred -->
<sce:wcet-bound mode="measured" estimate_us="180" target="cortex_m7_400mhz"/>
<sce:wcet-bound mode="opaque"/>  <!-- forbidden under cooperative; build error -->
```

- `mode="static"` — derived from `max-iter` × per-iteration cost
  model. Required when all loops have build-time-known bounds.
- `mode="measured"` — recorded from a host-mode benchmark on the
  declared target. Required when an iteration bound is data-driven
  (e.g. KeyExpr matching over a runtime-sized bounded-collection,
  where the bound is `local_sub_table.capacity × max_segments`).
- The build refuses to emit when `estimate_us > scheduler.
  worker_slot_budget_us` (RFC §5.K).

KeyExpr matching, fragment reassembly, and CRC over a full pool
slot are the canonical cases requiring `measured` annotations. AP
targets (`scheduler.kind: tokio` / `rt`) do not require this
annotation because the runtime is preemptive.

**Measurement workflow.** `mode="measured"` annotations carry two
binding fields beyond `estimate_us`:

- `target="<soc>_<core>_<freq>"` — pins the measurement to a
  specific platform descriptor. Codegen matches this string against
  `deploy.yaml` `platform.{soc, core}` + `scheduler.tick_period_us`
  derived frequency; mismatch is a build error (see diagnostics).
- `source-hash="<sha256>"` — sha256 of the algorithm body's
  canonical IR (post-XInclude, pre-emission). When the body is
  edited, the hash diverges from the recorded value and the
  measurement is flagged stale — the author must re-run the
  host-mode benchmark and update the annotation, or the build fails.

The recommended workflow is: (1) author writes the algorithm with
`<sce:wcet-bound mode="static" .../>` if loops are bounded by
build-time constants, otherwise an initial `mode="measured"`
placeholder; (2) `sce-bench --target <descriptor>` runs the
algorithm against worst-case inputs derived from `bounded-collection`
capacities, emitting an `estimate_us` and `source-hash`; (3) the
author commits the updated annotation. The benchmark harness is part
of the SCE testing tooling (§6.2), not authored per project.

**Measurement environment classes.** Native execution on an x86_64
host of cross-compiled MCU C produces cycle counts that diverge from
real Cortex-M behavior by up to 1–2 orders of magnitude (deeper OoO
pipeline, multi-MB caches, branch predictor breadth). `mode="measured"`
must therefore declare which environment produced the number, via a
new attribute `measured_on=`:

| Class | `measured_on=` value | Accuracy | Cost | Required margin |
|---|---|---|---|---|
| HIL — actual target + cycle counter | `hil:<board_id>` | best | hardware fixture | none (raw value) |
| Cycle-accurate simulator (renode, QEMU `-icount`, GEM5) | `sim:<sim_id>:<core_model>` | good for the modeled core | simulator setup | × 1.2 safety factor |
| Cross-build host execution + calibration | `host:<host_arch>:calibration_<id>` | poor | cheapest | × 3.0 safety factor + diagnostic warning |

Codegen multiplies the recorded `estimate_us` by the class margin
before comparing against `scheduler.worker_slot_budget_us`. The
multiplied value is the one stored in the IR; the original raw
measurement stays in the SCXML for traceability. A `host:`
measurement therefore consumes 3× its measured budget against the
scheduler — authors are pushed toward HIL or simulator measurement
for tight slot budgets.

The `<sce:wcet-bound mode="opaque"/>` shape is forbidden for
cooperative scheduling targets (existing
`algorithm/wcet-mode-opaque-under-cooperative` diagnostic). For AP
preemptive targets, all environment classes are advisory.

**WCET diagnostics:**
- `algorithm/wcet-bound-missing` — algorithm called from a worker
  slot under cooperative scheduling without `<sce:wcet-bound>`
- `algorithm/wcet-exceeds-slot-budget` — `estimate_us` >
  `scheduler.worker_slot_budget_us`
- `algorithm/wcet-mode-opaque-under-cooperative` — `mode="opaque"`
  rejected when target uses cooperative scheduler
- `algorithm/wcet-measured-target-mismatch` — `mode="measured"`
  `target=` does not match the deploy.yaml platform descriptor
  (e.g. measurement was on `cortex_m7_400mhz` but deploy targets
  `cortex_m4_168mhz`)
- `algorithm/wcet-measured-stale-against-source-hash` —
  `mode="measured"` `source-hash=` does not match the canonical IR
  of the algorithm body; the body was edited after the measurement
  and must be re-benchmarked
- `algorithm/wcet-measurement-class-missing` — `mode="measured"`
  without `measured_on=` attribute; codegen cannot pick a safety
  factor and refuses to emit
- `algorithm/wcet-measurement-class-untrusted-without-margin` —
  `measured_on="host:..."` measurement consumes the slot budget at
  raw value (× 3.0 margin not applied); informational warning when
  the author has overridden via `<sce:override-measurement-margin
  factor="X"/>` with justification reference. Hard error if the
  override is present without the justification reference

**Codec aggregation cross-reference.** Algorithms invoked from
within a codec (e.g. `crc16_ccitt(payload)` over a length-prefixed
field) have their `<sce:wcet-bound estimate_us=...>` aggregated
into the enclosing codec's static WCET (§5.B "Codec aggregate
WCET"). Algorithms that ship with `mode="opaque"` therefore make
any codec that calls them un-aggregable, surfacing
`codec/wcet-aggregate-undeclared-on-rx-codec` even though §5.A
already rejected them. This is intentional defense-in-depth — the
codec-level diagnostic catches the case where an algorithm's
`mode="opaque"` was accepted on a non-RX-path codec but the codec
later got bound to an RX path through an FSM edit.

**Worked example:** Appendix A (CRC16-CCITT) and Appendix B
(VLE ZInt u64).

### 5.B Codec DSL extensions

**Problem.** Zenoh wire format requires constructs not expressible in
the current fixed-width `Codec` kind.

**Proposal.** Additive extensions, backward compatible with existing
codec fields.

| Extension | XML shape | Semantic |
|---|---|---|
| VLE integer | `<sce:field type="vle_u64" name="id"/>` | Variable-length encoding; emit decode loop |
| Discriminated union | `<sce:variant tag="header.id"> <sce:arm value="0x01" type="SessionOpen"/> ... </sce:variant>` | Tag field selects arm; decode picks branch |
| Conditional field | `<sce:field name="key" type="str" sce:present-if="flags.has_key"/>` | Decoded only if predicate true |
| Bit-flag field | `<sce:flags name="header"> <sce:flag name="reliable" bit="7"/> ... </sce:flags>` | Named bits within a byte or word |
| Length-prefixed | `<sce:field name="payload" type="bytes" sce:len-prefix="size"/>` | Field size = value of `size` (earlier VLE or int) |
| Repeated | `<sce:repeat count="n"> <sce:field ... /> </sce:repeat>` | Array of sub-structures; count from field `n` |
| Until-EOF | `<sce:field name="tail" type="bytes" sce:until-eof="true"/>` | Greedy consume-remaining |
| TLV chain | `<sce:kind="tlv-chain" max-depth="N" on-overflow="reject\|truncate\|diagnostic-event">` as a codec subtype | Bounded extension list, each has id+len+body. `max-depth` MUST be specified for MCU targets; enforced via `max-iter` on the parse loop (§5.A) — iterative, never recursive |
| DMA alignment (field) | `<sce:field ... sce:dma-burst-align="32" sce:pad-before="auto"/>` | Field start address constrained to DMA burst boundary; codegen inserts padding bytes to honor it |
| DMA alignment (codec) | `<sce:dma-constraint><sce:burst-align>32</sce:burst-align><sce:header-stride>4</sce:header-stride></sce:dma-constraint>` | Codec-level defaults applied to every field unless overridden |
| Test vector | `<sce:test-vector hex="a1b2c3..." value="..."/>` in codec file | Golden input/output pair for regression |
| Parse mode | `<sce:parse-mode>borrow</sce:parse-mode>` or `own` | Borrow = zero-copy slice into input buffer |

**Codegen contract.** Each codec emits two functions (encode +
decode) on each of the six language backends per the §5.J.5 emitter
table. Per-language cursor / error shapes:

- Rust: `cursor` is `bytes::BytesMut` (AP) or `&mut SceCursor` (MCU
  no_std); `decode` returns `Result<T, CodecError>` with a
  `NeedMoreBytes` variant for streaming parse.
- C11: `cursor` is `sce_cursor_t*` (pointer + remaining length);
  `decode` returns `int` (`SCE_OK` / `SCE_NEED_MORE_BYTES` /
  `SCE_<error>`) and writes through a `T* out` parameter.
- Cpp: `cursor` is `std::span<uint8_t>` + position; `decode` returns
  `std::optional<T>` and signals `NeedMoreBytes` via a separate
  `cursor.need_more_bytes()` flag (no exceptions).
- Kotlin: `cursor` is `java.nio.ByteBuffer`; `decode` returns `T?`
  (null when not enough bytes are available).
- Go: `cursor` is `*Cursor` (slice + position); `decode` returns
  `(T, error)` with a sentinel `ErrNeedMoreBytes`.
- Python: `cursor` is a `memoryview` slice; `decode` returns a
  `(T, consumed_bytes)` tuple and raises `NeedMoreBytes` only at
  the streaming boundary (parsers used in tight loops are non-
  raising and return `None` on truncation).

`NeedMoreBytes` semantics are uniform across backends — a truncated
input never aborts; it returns the typed need-more signal so the
caller can resume after additional bytes arrive (DMA boundary,
fragmented network read).

The MCU-only codec sub-features (`dma-burst-align`, codec-aggregate
WCET gate, `<sce:dma-constraint>`) emit only on `(rust, *)` and
`(c11, bare_metal)` per the §5.J.4 matrix. Authoring them with a
target backend in `{cpp, kotlin, go, python}` raises
`codegen/mcu-class-kind-on-non-mcu-language`.

**Test-vector integration.** `<sce:test-vector>` values generate
compile-time `_Static_assert` (C) and `#[test]` (Rust) that round-trip
the hex through the codec and compare to the declared value.
Cross-backend parity is guaranteed because both backends consume the
same test vector from the same SCXML.

**TLV chain bound enforcement.** Iterative parse only; `max-depth`
lowers to a `max-iter` on the chain traversal loop (§5.A). No
runtime recursion. On MCU the decoder carries a fixed-size working
set sized at `max-depth`; overflow is handled by `on-overflow`.

**DMA alignment semantics.** `sce:dma-burst-align` on a field
constrains the field's offset within the encoded buffer AND, on
MCU, within the pool slot the buffer is materialized into. Codegen
inserts explicit padding (emitted as `<sce:field name="_pad_n"
type="bytes"/>` of computed length) and emits
`_Static_assert`/`const _: () = assert!(...)` checks on the final
struct layout to guarantee the invariant at compile time.

**Scope: wire layout, not host allocator.** This is a wire-format
contract — both backends emit the same padded byte sequence, so
peers stay byte-compatible regardless of where the bytes were
allocated. On AP (`platform.class != mcu`) the encoded buffer is a
`bytes::BytesMut` and the AP allocator has **no obligation to
align it** to the burst boundary; the codec's own padding alone
satisfies the field-offset invariant. On MCU the same offsets
additionally land inside a DMA-coherent pool slot whose base is
aligned at link time (§5.E). AP and MCU share no memory; they
share only the wire. The constraint never propagates into AP host
memory placement.

**Scope: fixed-offset positions only (no VLE-following alignment).**
Static padding can only honor `dma-burst-align` at positions whose
offset is **build-time-known**. A field whose offset depends on a
preceding variable-length field (VLE integer, length-prefixed
bytes, repeated structure with runtime count, until-EOF body)
cannot be statically padded — its absolute offset within the
encoded buffer is determined at runtime, so codegen has no value
to pad against. Such configurations are **rejected** at build
time via `codec/dma-alignment-unsatisfiable`.

Legal positions for `dma-burst-align`:
- The first field of a codec (offset 0; trivially known)
- A field whose every preceding field is fixed-width
  (`u8/u16/u32/u64`, fixed-size byte arrays, struct of fixed-width
  fields)
- A field immediately following a `<sce:padding-to-boundary
  align="X"/>` directive (which itself must satisfy the rules
  above)

Illegal positions (build error):
- A field after a `vle_*` field (variable 1–10 bytes)
- A field after `sce:len-prefix=` byte string of runtime length
- A field after `<sce:repeat count="n">` where `n` is not
  build-time-known
- A field after `<sce:until-eof>` content

The intended use case is therefore narrow: align **headers**,
**fixed-prefix payloads**, or **DMA descriptor records** — not
arbitrary fields buried inside a TLV chain. The Zenoh wire format
already places fixed-shape headers up front (z-flag byte, message
ID byte, fixed extension flags), so the constraint accommodates
the common DMA-aware shapes (Ethernet frame headers, fixed-size
PDU prologues) without conflicting with VLE-heavy bodies.

**Codec aggregate WCET (cooperative-scheduler targets).** §5.A
`<sce:wcet-bound>` covers algorithm kinds in isolation, but a frame
parsed in a worker slot is not one algorithm — it is a *graph* of
codec field decodes (fixed reads, VLE loops, length-prefixed copies,
TLV chain traversals, repeats) plus the algorithms each field
invokes (`crc16_ccitt` over a length-prefixed payload, `keyexpr_match`
on a string field). An adversarial peer can construct a frame where
every field that can be variable-length is variable-length:
maximum-byte-count VLEs, maximum-depth TLV chains where every entry
itself contains maximum-byte VLEs, maximum-length length-prefixed
strings. The per-field WCETs multiply — and if the codec parse total
exceeds `worker_slot_budget_us`, the keepalive tick slips, the
scheduler misses, and the symptom is keepalive jitter under attack
(not a crash, which makes it harder to debug). The §11.6 fuzz
harness's *"slow input → fail (>1ms decode wall time)"* gate catches
this on F1 but not at build time, and it does not catch sub-1ms
inputs that still exceed slot budget on slower MCU targets.

The codec compiler therefore computes a static aggregate WCET for
every codec kind and compares it to the slot budget at build time.
The per-field model:

| Field shape | Static WCET contribution |
|---|---|
| Fixed-width (`u8`/`u16`/`u32`/`u64`, fixed-size byte arrays, struct of fixed-width fields) | constant (1–2 cycles per access; folded into a single cost coefficient) |
| `vle_uK` | `ceil(K / 7) × platform.vle_decode_cycles_per_byte / platform.clock_freq_mhz` (3 bytes for `u16`, 5 for `u32`, 10 for `u64`) |
| `len-prefix` byte string with declared `max-bytes` | `max_bytes × platform.memcpy_cycles_per_byte / platform.clock_freq_mhz` (treated as a copy when `parse-mode="own"`; bounded slice construction only when `parse-mode="borrow"`) |
| TLV chain with `max-depth=N`, `max-payload-per-entry=B` | `N × (platform.tlv_chain_per_entry_overhead_us + per-entry-body-WCET(B))` |
| `<sce:repeat count="n">` with build-time-known `max(n)` | `max(n) × per-element-aggregate-WCET` |
| `<sce:repeat count="n">` with runtime `n` whose source is `vle_*` or length-prefixed | `vle-or-len-prefix-max-value × per-element-aggregate-WCET` |
| Algorithm invocation (e.g. `crc16_ccitt(payload)`) | the algorithm's own `<sce:wcet-bound estimate_us=...>` × the worst-case payload size factor declared via the field that drives it |
| `<sce:variant>` | `max(arm WCETs)` (worst-case branch); attacker chooses the slowest arm |

The aggregate is `Σ` of these contributions plus a constant codec
overhead (cursor management, error-return path). The resulting
number lands as a derived attribute on the codec IR:

```xml
<sce:codec-wcet-bound mode="derived" estimate_us="180"/>
<!-- emitted by the build; not authored. -->
<!-- Authors may override with mode="measured" + target= + source-hash=, -->
<!-- mirroring §5.A measurement workflow, when the derived bound -->
<!-- is too pessimistic to ship. -->
```

The build refuses when `estimate_us > scheduler.worker_slot_budget_us`
(`codec/wcet-aggregate-exceeds-slot-budget`). When the codec is
bound to an RX path (an FSM state's `<sce:on-frame>` handler) on a
cooperative-scheduler target, the aggregate MUST be derivable —
otherwise the same attacker scenario remains undetected
(`codec/wcet-aggregate-undeclared-on-rx-codec` warning, promoted to
hard error when `pool.stage_copy_policy: error` or `forbid`, see
§5.K).

When a codec contains a TLV chain, the build also enforces:
- `tlv_chain.max-depth × per-entry-aggregate-WCET ≤
  worker_slot_budget_us × (1 - reserved_keepalive_headroom)` —
  the TLV chain alone cannot eat a slot. Hard error if violated
  (`codec/tlv-chain-aggregate-wcet-exceeds-slot-budget`).
- A codec containing both a TLV chain AND any other variable-length
  field (length-prefixed payload, additional VLE) must aggregate
  *both* costs against the same slot — the chain is not the only
  variable-length contribution. The aggregate computation does this
  automatically; this is documented for review clarity.

Author resolution paths when the gate fails:
1. Lower the bound that drives the worst case (`max-depth`, `max-bytes`,
   bounded-collection capacity referenced by a `len-prefix`).
2. Split the codec across slots — emit an FSM event after parsing the
   header, parse the body in the next slot. Mechanically expressible
   via §5.M reassembly but at the FSM granularity rather than the
   network fragment granularity.
3. Move the codec to a non-RX path (TX-only codecs are exempt because
   the application controls input size).
4. Override with `<sce:codec-wcet-bound mode="measured"
   target="..." source-hash="..."/>` — the derived bound was
   pessimistic; the measured bound is tighter. Same workflow as §5.A.

AP targets (`scheduler.kind: tokio` / `rt`) do not require codec
aggregate WCET; the runtime is preemptive and a slow parse blocks
only one task.

**Diagnostics:**
- `codec/vle-width-overflow` — `vle_u32` field receiving value >2^32
- `codec/variant-arm-unreachable` — tag value not in any arm and no `<sce:default>`
- `codec/present-if-refs-later-field` — forward reference
- `codec/len-prefix-refs-non-int` — length must be integer
- `codec/borrow-mode-with-owned-field` — mixed parse modes
- `codec/test-vector-drift` — regression on golden vector
- `codec/tlv-chain-depth-unspecified` — MCU target without `max-depth`
- `codec/tlv-chain-depth-exceeds-stack-budget` — `max-depth` × per-level working set > deploy-declared worker stack budget
- `codec/dma-alignment-unsatisfiable` — burst-align requirement cannot be honored given preceding variable-length fields
- `codec/dma-alignment-pool-mismatch` — field `dma-burst-align` > bound pool `alignment`
- `codec/wcet-aggregate-exceeds-slot-budget` — derived (or measured-override) codec aggregate WCET exceeds `scheduler.worker_slot_budget_us`. Hard error
- `codec/wcet-aggregate-undeclared-on-rx-codec` — codec is bound to an RX path (`<sce:on-frame>`) on a cooperative-scheduler target but the aggregate cannot be derived (e.g. `vle_*` field present without `platform.vle_decode_cycles_per_byte`, or a `<sce:repeat>` with unbounded runtime count). Warning by default; promoted to hard error when `pool.stage_copy_policy: error` or `forbid` (§5.K)
- `codec/wcet-aggregate-vle-cycles-missing` — codec contains `vle_*` field, target has `scheduler.kind: cooperative`, but `platform.vle_decode_cycles_per_byte` not declared. Hard error — the aggregate cannot be computed
- `codec/wcet-aggregate-tlv-overhead-missing` — codec contains `tlv-chain`, target has `scheduler.kind: cooperative`, but `platform.tlv_chain_per_entry_overhead_us` not declared. Hard error
- `codec/wcet-aggregate-repeat-unbounded` — `<sce:repeat count="n">` where `n` derives from a runtime field with no declared max (e.g. a `len-prefix` byte string with no `max-bytes`). Hard error — every repeat must have a build-time-known upper bound for aggregation
- `codec/tlv-chain-aggregate-wcet-exceeds-slot-budget` — TLV chain alone (`max-depth × per-entry-aggregate-WCET`) exceeds the slot budget, even before counting other fields. Hard error; the chain bound must be lowered before any other gate can pass
- `codec/wcet-measured-override-stale` — author-supplied `<sce:codec-wcet-bound mode="measured">` `source-hash=` does not match the current codec IR. Same shape as `algorithm/wcet-measured-stale-against-source-hash`

### 5.C Byte-stream link model — `sce:kind="link"`

**Backend coverage (MCU-class kind).** Per the §5.J.4 matrix this
kind emits only on `(rust, *)` and `(c11, bare_metal)` — its
substrate (DMA-aligned slot acquisition, ISR-driven RX, link-time
section placement, OS-native reactor binding) has no equivalent
on Cpp/Kotlin/Go/Python backends. Authoring a `link` kind whose
target backend is in `{cpp, kotlin, go, python}` raises
`codegen/mcu-class-kind-on-non-mcu-language` (hard error). The
runtime crate is selected by `platform.os` per §5.J.3
(`sce_link_runtime_lwip` for `bare_metal`, `sce_link_runtime_tokio`
for `linux`, `sce_link_runtime_qnx` for `qnx`).

**Problem.** SCE Mesh assumes a transport library (zenoh-c, vsomeip,
etc.) lives below and application events dispatch to it. In protocol
synthesis, the project *is* the transport; events originate from raw
bytes and terminate as raw bytes.

**Proposal.** New kind `link` declaring a byte-level endpoint pair.

```xml
<scxml sce:kind="link" name="udp_scout" version="1.0">
  <sce:link-class>udp</sce:link-class>
  <sce:framer ref="scout_frame_codec"/>      <!-- §5.B codec -->
  <sce:rx-pool ref="scout_rx_pool"/>         <!-- §5.E buffer-pool -->
  <sce:tx-pool ref="scout_tx_pool"/>
  <sce:backpressure>drop</sce:backpressure>  <!-- drop | block | signal-event -->
  <sce:events>
    <sce:inbound event="scout.hello.received" when="decoded.msg_id == 0x02"/>
    <sce:outbound event="scout.query.send" encode="scout_frame_codec"/>
  </sce:events>
</scxml>
```

**deploy.yaml binding:**

```yaml
machines:
  ap_node:
    links:
      udp_scout:
        bind: "224.0.0.224:7446"
        driver: tokio_udp
  mcu_node:
    links:
      udp_scout:
        bind: "224.0.0.224:7446"
        driver: lwip_udp
        rx_pool_slot_section: sram1
```

**Runtime contract.** A `sce_link_runtime_<os>` crate per target OS
provides the same trait surface; the OS suffix is part of the
naming convention formalized in §5.J:
- `trait Link { fn rx(&mut self) -> Option<RxFrame>; fn tx(&mut self, frame: TxFrame) -> Result<()>; }`
- Per-driver adapter selects the OS-native I/O primitive. Current
  crate plan:
  - `sce_link_runtime_lwip` (MCU bare_metal — Phase B baseline):
    `lwip_udp`, `lwip_tcp`, `serial_uart`, `websocket_tcp`.
  - `sce_link_runtime_tokio` (AP linux — Phase D.1): `tokio_udp`,
    `tokio_tcp`. Optional `tokio_uring` driver for `io_uring` opt-in
    on kernels ≥ 5.10.
  - `sce_link_runtime_qnx` (AP qnx — Phase D.2): `qnx_io_sock_udp`,
    `qnx_io_sock_tcp` over QNX-native `dispatch_create()` reactor
    (mio has no QNX backend; see OQ-W20). QNX-native typed-message
    and shared-memory IPC link-classes are out of scope until the
    Phase D.2 RFC that lands them (kept out of the enum until then —
    see "Link-class enumeration" below).

**Link-class enumeration.** The `<sce:link-class>` value lives in a
shared namespace whose phase availability follows the OS-axis:

| Class | Semantics | Available phase |
|---|---|---|
| `udp` | datagram, byte-stream framer | A (MCU lwIP), D.1 (AP linux), D.2 (AP qnx) |
| `tcp` | stream, byte-stream framer | B (MCU), D.1 (AP linux), D.2 (AP qnx) |
| `serial` | UART | C (MCU) |
| `websocket` | TCP + WebSocket framing | C (MCU) |
| `raw_eth` | L2 frames, target-plugin only | C (MCU plugin) |

OS-specific link classes (e.g. `unix_socket`, `unix_seqpacket`
for AP linux IPC, `qnx_msg` / `qnx_shm` for AP QNX inter-process
typed messaging and shared memory) are NOT pre-declared in this
enum. They land as additive `<sce:link-class>` values together
with the corresponding `sce_link_runtime_<os>` crate when the
relevant phase opens (D.1 for UNIX classes, D.2 for QNX classes).
This avoids declaring strings the schema accepts but the codegen
rejects, which is the exact "built-but-unconsumed parser surface"
shape §2.4 invariant 3 ("kinds are additive") forbids:
*additive* means added when wired, not pre-reserved as a
namespace placeholder. Authoring a not-yet-declared link-class
fails with `link/link-class-unknown` (the existing unknown-value
diagnostic), not a special "deferred" flavor.

**Codegen contract.** Each link emits:
- RX path: `driver.poll() -> bytes -> framer.decode() -> event inject`
- TX path: `event extract -> framer.encode() -> pool slot -> driver.send()`
- On MCU, the pool slot is DMA-aligned, and the codec's borrow-mode
  path (§5.B) references it directly — no intermediate copy.
- On AP linux with `tokio_uring` driver, the pool slot is an
  `io_uring`-registered fixed buffer; the same `sce:rx-pool` /
  `sce:tx-pool` model from §5.E applies (Phase D.1 elaboration).
- On AP qnx, the pool slot is io-sock allocated. Pulse-driven
  shared-memory IPC link-classes are not part of this RFC's scope;
  they land with the Phase D.2 RFC if and when needed.

**Listener-link sibling emission.** §5.M's "Listener-link trust-class
lifecycle" specifies that codegen models every listener link as two
logical link-instances — one `session_arming` (pre-handshake) and
one `established_session` sibling (post-handshake) — that share a
single physical socket. Mechanically:

- A `<sce:link>` whose deploy-resolved `domain_attrs.trust_class`
  is `session_arming` and which has SCXML `Accepting.*` attached
  (i.e. it is *acting as* a listener per [`docs/session-fsm.md`: Trust-class interaction](session-fsm.md#26-trust-class-interaction-cross-ref-to-5m))
  emits **both** instance variants. The `session_arming` instance
  receives all RX bytes prior to the per-peer handshake completion;
  the `established_session` sibling receives them after. The peer-
  level RX dispatch on the shared socket is driven by the unicast
  session FSM's per-peer state (`Established` flag), as established
  in `docs/session-fsm.md` §2.3.
- The sibling is **synthesized**, not author-declared. It inherits
  `bind`, `driver`, `mtu_bytes`, `expected_p99_bytes`, `burst_pps`,
  and `rx_dispatch` from the listener entry; it does NOT inherit
  the §5.K accept-side hardening fields (`session_arming_quota`,
  `accept_rate_*`, `accepting_inactivity_timeout_ms`,
  `stateless_accept`), which are meaningful only on the
  `session_arming` half.
- Reassembly-pool bindings (`<sce:rx-pool ref="...reassembly...">`)
  authored against the listener link kind resolve to the sibling
  `established_session` instance at codegen time and pass the
  §5.M `reassembly/untrusted-link-binding` gate. Authors do not
  spell the sibling explicitly.
- Runtime crate header / trait surfaces (`docs/runtime-crate-lwip.md`
  §4 / `docs/runtime-crate-tokio.md` §2.4) emit one variant per
  instance; the listener simply contributes two variants instead of
  one. Compile-time elision of trust-class-incompatible methods
  remains per-instance.

This is the §5.C resolution of OQ-W22; the trust-class semantics
that justify the split are in §5.M.

**Diagnostics:**
- `link/backpressure-undeclared`
- `link/framer-missing`
- `link/class-unsupported-on-target` (e.g. TCP on MCU without network stack)
- `link/pool-slot-smaller-than-framer-max`
- `link/link-class-unknown` — declared `<sce:link-class>` value
  is not in the enum table above. Hard error. (When future phases
  add OS-specific classes such as `unix_socket` or `qnx_msg`, they
  land as new enum rows; using them on a target whose
  `platform.os` cannot host them resolves to
  `link/link-class-incompatible-with-os` below.)
- `link/link-class-incompatible-with-os` — declared
  `<sce:link-class>` cannot run on `platform.os` (e.g. a
  hypothetical `qnx_msg` with `os: linux`). Hard error
- `link/listener-link-not-paired-with-established-sibling` —
  codegen self-check that every `session_arming` listener instance
  has emitted its `established_session` sibling per the
  "Listener-link sibling emission" contract above. Hard error.
  This is a template regression guard, unreachable in well-formed
  codegen; it exists to ensure the listener emission template
  cannot silently regress to single-instance shape (which would
  re-introduce the OQ-W22 contradiction)

### 5.D Timer and worker primitives

**Backend coverage (MCU-class kind for the worker primitive).** The
existing `Timer` kind retains its full six-backend coverage from
the §5.J.4 baseline (it emits a per-language deadline-tracker on
top of a generic event queue). The `worker` primitive added by this
section is MCU-class — it bottoms out on cooperative-scheduler
slot accounting on MCU and on a dedicated runtime task on AP, neither
of which has a defined emitter shape on Cpp/Kotlin/Go/Python.
Authoring `<sce:kind="worker">` against a target backend in
`{cpp, kotlin, go, python}` raises
`codegen/mcu-class-kind-on-non-mcu-language` (hard error).

**Problem.** `<send delay="...">` is a scalar with no reset/cancel
ergonomics, and the existing `Timer` kind may not cover the shape
needed (reviewer to confirm). Workers are needed for concurrent
read/write loops — `<parallel>` shares a macrostep, but workers need
independent execution contexts.

**Proposal (timer).** Extend or confirm `Timer` kind supports:

```xml
<scxml sce:kind="timer" name="keepalive" version="1.0">
  <sce:period>5s</sce:period>
  <sce:reset-on event="session.msg.received"/>
  <sce:cancel-on state-exit="established"/>
  <sce:fire-event>keepalive.tick</sce:fire-event>
</scxml>
```

**Proposal (worker).** New `sce:kind="worker"` (or annotation on
existing `<parallel>` regions):

```xml
<scxml sce:kind="worker" name="rx_loop" version="1.0">
  <sce:link-rx ref="udp_scout"/>       <!-- drives this worker -->
  <sce:inbox depth="16"/>              <!-- typed event queue -->
  <sce:outbox ref="session_fsm.inbox"/>
  <sce:body>                           <!-- algorithm-kind-style body -->
    <!-- usually empty; link-rx drives event injection automatically -->
  </sce:body>
</scxml>
```

**Codegen contract.**
- AP: timer = `tokio::time::interval`; worker = `tokio::spawn`
- MCU: timer = compile-time slot in a static timer wheel; worker =
  one tick slot in a cooperative scheduler; inbox = `heapless::spsc`
  (Rust) or fixed ring buffer (C)

**Diagnostics:**
- `timer/period-below-tick-rate` (MCU cooperative scheduler)
- `timer/slot-overflow` (static wheel depth exceeded)
- `worker/shared-mutable-state` — any non-inbox access to another worker's state
- `worker/scheduler-unsupported` — worker count exceeds scheduler slot count

### 5.E Buffer pool and memory placement

**Backend coverage (MCU-class kind).** Per the §5.J.4 matrix this
kind emits only on `(rust, *)` (heapless slot table under no_std,
optionally backed by `bytes::BytesMut` arena under std on AP) and
`(c11, bare_metal)` (linker-fragment-backed pool sections,
DMA-aligned slots). The lifecycle FSM, cache maintenance pinning,
linker fragment emission, and the phantom-typed `Slot<state>` API
defined below all bottom out on those two language backends.
Authoring `<sce:kind="buffer-pool">` against a target backend in
`{cpp, kotlin, go, python}` raises
`codegen/mcu-class-kind-on-non-mcu-language` (hard error). AP
linux/qnx Rust deploys consume the same `buffer-pool` declarations
through their `sce_link_runtime_<os>` crate (with `tokio_uring`
fixed-buffer registration on linux per §5.C codegen contract).

**Problem.** MCU zero-copy requires pool slots placed in specific SRAM
sections aligned to DMA constraints. No such primitive exists today.

**Proposal.** New `sce:kind="buffer-pool"` and field-level placement
attributes.

```xml
<scxml sce:kind="buffer-pool" name="rx_pool_sram1" version="1.0">
  <sce:slot-count>8</sce:slot-count>
  <sce:slot-size>256</sce:slot-size>
  <sce:section>sram1</sce:section>
  <sce:alignment>32</sce:alignment>
  <sce:dma-channel>DW0_CH3</sce:dma-channel>
  <sce:cache-policy>maintain</sce:cache-policy>
  <!-- maintain | non-cacheable | none -->
</scxml>
```

**Cache policy semantics.**
- `maintain`: codegen inserts `cache_clean_by_addr(slot, len)` before
  DMA TX and `cache_invalidate_by_addr(slot, len)` after DMA RX, via
  intrinsics from §5.I. CPU access is unrestricted; correctness comes
  from explicit maintenance around DMA boundaries.
- `non-cacheable`: pool memory declared in a non-cacheable MPU region
  (deploy.yaml `attr: [non_cacheable]`). No maintenance ops emitted.
  CPU access pays the uncached-load penalty.
- `none`: target has no data cache (e.g. Cortex-M0/M3/M4); no
  maintenance ops emitted, no MPU setup required.

The chosen policy MUST be consistent with the target descriptor
(§5.K `platform.has_dcache`). Codegen rejects `none` when the target
has a D-cache, and rejects `maintain` when the target has no D-cache.

**Cache-line invariants under `maintain`.** ARM (and most other
caching MCU architectures) operate cache maintenance by virtual
address at **cache-line granularity**: an `invalidate` of `(addr,
size)` invalidates every line that intersects the range, including
partial lines at the boundaries. CMSIS `SCB_InvalidateDCache_by_Addr`
explicitly states that if the size is not a multiple of cache line
size, the line containing the last byte is invalidated in full.

This means a pool under `cache-policy: maintain` MUST satisfy two
invariants — pool start AND every slot end falling on cache line
boundaries — or invalidate-after-RX will silently corrupt adjacent
slots:

1. `pool.alignment ≥ platform.dcache_line_size` (existing
   `mem/cache-line-alignment` diagnostic) — ensures pool start.
2. `slot_size % platform.dcache_line_size == 0` (new
   `mem/slot-size-not-cache-line-multiple` diagnostic) — ensures
   each slot occupies a whole number of cache lines, so no slot's
   maintenance touches an adjacent slot's line.

Without invariant 2, a 32-byte-line target with `slot_size=250`
puts slot 1 at offset 250 (not 32-aligned), and `cache_invalidate
(slot_0, 250)` operates on the line covering bytes 224–255, which
overlaps slot 1's bytes 250–255 — pending CPU writes to that range
get discarded. Authors who need a 250-byte logical payload declare
`slot_size: 256` (next cache-line multiple) and use only 250 of it;
codegen never silently pads, so RAM cost stays explicit.

**Codec field placement:**

```xml
<sce:field name="payload" type="bytes"
           sce:len-prefix="size"
           sce:section="sram1"
           sce:alignment="32"/>
```

**deploy.yaml** declares the sections, channels, and cache properties:

```yaml
machines:
  mcu_node:
    platform:
      class: mcu
      soc: <soc_id>          # target-specific, TBD
      core: <core_id>
      has_dcache: true       # drives cache-policy validation in §5.E
      dcache_line_size: 32   # used for alignment/maintenance granularity
    memory:
      sram_regions:
        dtcm:  { base: 0x20000000, size: 64K, attr: [fast, nocache] }
        sram1: { base: 0x08000000, size: 512K,
                 attr: [dma_coherent, cacheable] }
        sram2: { base: 0x08080000, size: 128K,
                 attr: [dma_coherent, non_cacheable] }
      dma_channels: [DW0_CH0, DW0_CH1, DW0_CH2, DW0_CH3]
```

A pool with `cache-policy: maintain` must land in a `cacheable`
region; `cache-policy: non-cacheable` must land in a `non_cacheable`
region. Mismatches are diagnosed at build time (see below).

**Codegen contract.**
- C: `__attribute__((section(".sram1"), aligned(32))) static uint8_t
  rx_pool_sram1_storage[8][256];` + bitmap freelist + paired DMA
  descriptor table in `.sram1_desc` section.
- Rust: `heapless::pool` for AP; MCU Rust variant uses
  `#[link_section = ".sram1"]` static arrays with a custom pool API.
- Linker script fragment generated alongside the sources as a
  sidecar. Each declared pool produces a `SECTIONS` entry with an
  explicit `ALIGN(x)` directive matching the pool's
  `<sce:alignment>`, so the section base is on the required
  boundary even if a toolchain quirk drops the storage variable's
  `aligned(x)` attribute. Paired DMA descriptor sections (e.g.
  `.sram1_desc`) carry their own `ALIGN()` matching the
  descriptor's burst alignment. **Between adjacent pool sections in
  the same `MEMORY` region, codegen emits an explicit
  `. = ALIGN(<line_size>);` sentinel** so the post-pool boundary is
  audibly aligned even if a downstream master linker script splices
  another section in via `INCLUDE`. Example shape:

  ```
  .sram1_pool_a (NOLOAD) : ALIGN(32) {
    KEEP(*(.sram1_pool_a*))
  } > SRAM1
  . = ALIGN(32);                  /* explicit inter-pool sentinel */
  .sram1_pool_b (NOLOAD) : ALIGN(32) {
    KEEP(*(.sram1_pool_b*))
  } > SRAM1
  . = ALIGN(32);
  .sram1_desc (NOLOAD) : ALIGN(32) {
    KEEP(*(.sram1_desc*))
  } > SRAM1
  ```

  The sentinel is redundant *given* that the next pool also carries
  `ALIGN(32)` — but ALIGN on a section only constrains its own
  base, not the byte distance from the previous section's tail.
  Explicit sentinels make the inter-pool boundary diff-visible (any
  PR that drops one shows up in the linker fragment), survive a
  master script that re-orders the INCLUDE, and are the artifact
  the `mem/inter-pool-padding-not-emitted` self-check inspects.

  This is defense-in-depth: `aligned(32)` on the storage variable,
  `ALIGN(32)` on the section, the inter-pool `. = ALIGN(32);`
  sentinel, and `_Static_assert` on the field layout (§5.B) all
  enforce the same invariant at four layers, so a single failure
  mode (toolchain bug, custom linker script override, manual edit
  to the storage struct, a master script that splices another
  section between two pools) cannot silently corrupt DMA or cause
  cross-pool cache-maintenance contamination.

  **Scope: line-level cross-contamination only.** The sentinels
  prevent `dcache_invalidate_by_addr(pool_a, pool_a_size)` from
  touching pool B's first cache line *under cache maintenance by
  VA*. They do **not** address cache **set associativity
  contention** between pools (e.g. a high-priority RX pool and a
  low-priority log pool sharing the same N-way set, where heavy RX
  evicts log entries). Set contention is a separate problem with a
  separate answer: split the pools across `cache-policy: maintain`
  vs `non-cacheable` regions, or place them in distinct memory
  banks via `memory.sram_regions`. Padding does nothing for set
  contention — the sentinels exist for the line-level concern only.
  Cross-ref ARCHITECTURE §3.4 cache coherency note.

**Burst absorption analysis (RX pools).** A pool bound to a link
that declares `burst_pps` (§5.K) is checked at build time for
worst-case inbound rate vs cooperative scheduler drain rate. The
invariant: across one tick window, the pool must absorb the burst
without depleting.

Two dispatch modes determine the analysis shape:

- **`rx_dispatch: isr_to_pool`** — RX-complete IRQ chains the next
  slot from a descriptor ring directly, decoupled from cooperative
  ticks. Wire-rate absorption is bounded only by descriptor ring
  size, not tick period. Required check:
  `slot_count ≥ burst_pps × max_handler_latency_us / 1_000_000`
  where `max_handler_latency_us = max(tick_period_us,
  worker_slot_budget_us)` is the worst-case time before a slot is
  drained back to `free`. Safety factor × 2.0 applied.
- **`rx_dispatch: worker_tick`** — RX progresses only when the RX
  worker slot runs, once per tick. Required check:
  `slot_count ≥ burst_pps × tick_period_us / 1_000_000` with
  safety factor × 2.0. This mode caps the wire-rate ceiling at
  `slot_count × ticks_per_second × slot_size × 8 bps`.

When neither check is satisfied, the build emits the appropriate
`deploy/link-...` diagnostic from §5.K. Authors resolve by raising
`slot_count`, lowering `tick_period_us`, or switching dispatch mode.
The build report includes the computed wire-rate ceiling so
operators see the bandwidth headroom at deploy time.

**Slot lifecycle FSM (ownership tracking).** Each pool slot's
ownership state is modeled as a fixed FSM, declared canonically by
the `buffer-pool` kind itself (not authored per pool). The FSM
expresses the legal transitions between CPU and DMA ownership of a
slot and pins cache-maintenance call sites to specific transition
edges. Codegen tracks each slot reference at the IR level
(handle-to-state binding) and refuses to emit code that performs an
operation forbidden by the current state — borrow-check-style
verification at the IR level, not at runtime.

States:

```
free          — on freelist, no holder
cpu-mut       — exclusive CPU write (encode TX, build reassembly)
dma-armed-tx  — TX descriptor queued, DMA not started
dma-busy-tx   — DMA actively reading slot for TX
dma-armed-rx  — RX descriptor armed, peripheral not yet writing
dma-busy-rx   — DMA actively writing slot from peripheral
cpu-ref       — shared CPU read (parse decoded slot, dispatch handler)
```

Transitions (allowed; everything else is a violation):

```
free          → cpu-mut          : pool_acquire_for_encode()
cpu-mut       → dma-armed-tx     : link_arm_tx(slot)
                                   [+cache_clean if maintain]
dma-armed-tx  → dma-busy-tx      : DMA controller signal
dma-busy-tx   → free             : TX-complete IRQ; pool_return(slot)
free          → dma-armed-rx     : link_arm_rx(slot)
                                   [+cache_invalidate if maintain
                                     && has_speculative_prefetch]
dma-armed-rx  → dma-busy-rx      : peripheral start
dma-busy-rx   → cpu-ref          : RX-complete IRQ
                                   [+cache_invalidate if maintain]
cpu-ref       → free             : handler complete; pool_return(slot)
cpu-ref       → cpu-mut          : in-place mutate
                                   [+cache_clean on next hand-off if maintain]
cpu-mut       → free             : abort encode (error path)
dma-armed-tx  → cpu-mut          : un-arm before DMA start (error path)
```

The FSM is total: every emitted operation that touches a slot maps
to a transition. The IR carries each slot handle's current state;
codegen rejects operations that don't match. Errors and abort paths
have explicit return-to-`free` / return-to-`cpu-mut` transitions —
silently dropping a `cpu-mut` or `cpu-ref` reference is a leak
diagnostic.

**FSM extension policy (forward namespace not preemptively reserved).**
Future bus masters (hardware crypto engines like STM32 CRYP / NXP
CAAM, compression IP, GPU/NPU DMA) will need additional states on
this FSM when they become MVP-relevant. Per §2.4 invariant 3
("kinds are additive") the extension shape is *adding states to
the existing FSM, not a parallel FSM competing with it* — but the
states themselves are NOT pre-emptively declared in the IR until
the corresponding capability lands. Until then, generated code
recognizes only the seven states above; an attempt to bind a pool
to a non-existent third-master extern symbol fails with a clear
"unknown extern" error, not a silent placeholder transition.
Adding hardware crypto in a later phase therefore (a) introduces
new states on this FSM at that time, (b) does not require renaming
existing states or transitions, (c) does not alter the public
`Slot<state>` phantom-type API for the seven-state subset.

**Cache maintenance pinning.** When `cache-policy: maintain`, cache
maintenance calls are emitted **exclusively** on the FSM transition
edges marked `[+cache_clean]` / `[+cache_invalidate]` above:

- `cpu-mut → dma-armed-tx` emits `cache_clean_by_addr(slot, len)`
  before the descriptor is published — flushes pending CPU writes to
  memory so the DMA reads them
- `free → dma-armed-rx` emits `cache_invalidate_by_addr(slot, len)`
  **only when `platform.has_speculative_prefetch == true`** —
  evicts any line the CPU's prefetcher / speculative load may have
  pulled into the slot region while it was on the freelist; without
  this pre-arm invalidate, those lines persist and shadow DMA writes
  even after the post-RX invalidate
- `dma-busy-rx → cpu-ref` emits `cache_invalidate_by_addr(slot, len)`
  in the RX-complete IRQ before the slot becomes CPU-readable —
  evicts any line speculatively loaded during the DMA window

**Why two-sided RX invalidate on speculative cores.** Cortex-M7,
Cortex-A series, and other cores with speculative execution and
hardware prefetching can populate D-Cache lines covering a slot
even when the slot is `free` and the CPU has not explicitly
touched it (stack proximity, prefetch on sequential access
pattern). When DMA later writes to that slot, the cache line
holds **stale data** that the CPU sees on next read — the post-RX
invalidate alone cannot prevent this, because between the post-RX
invalidate and the first CPU read, the prefetcher can re-fetch.

ARM CMSIS guidance and ARM Application Note 321 ("Cortex-M7 Cache
Maintenance") explicitly recommend pre-DMA invalidate for this
class. STM32H7 (Cortex-M7) has documented packet-corruption
incidents traced to single-sided invalidate.

The pre-arm invalidate is **gated on `has_speculative_prefetch`**
because:
- Cortex-M0/M0+/M3/M4 (no speculative load, no prefetcher) → not
  needed; emitting it just costs cycles
- Cortex-M7+ / Cortex-A → required; missing it silently corrupts data
- Cores without D-Cache (`cache-policy: none`) → both edges are
  no-op, regardless of speculative flag

Author code (link bodies, worker bodies, statechart actions)
**MUST NOT** call `cache_clean` / `cache_invalidate` directly via
`<sce:call>` to the §5.I intrinsics — those calls are FSM-driven, not
author-controlled. This eliminates the class of bugs where the
maintenance call sits in the wrong place (after hand-off, before DMA
completes, or duplicated across worker hand-offs).

**Author-visible API.** The `buffer-pool` kind exposes only the
FSM-aware operations to authors:

```
pool_acquire_for_encode(pool) -> Slot<cpu-mut>     // free → cpu-mut
pool_return(slot)             -> ()                // cpu-mut|cpu-ref → free
link_arm_tx(slot)             -> ()                // cpu-mut → dma-armed-tx
link_arm_rx(pool)             -> Slot<dma-armed-rx>
```

The state parameter (`Slot<cpu-mut>` / `Slot<dma-armed-rx>` / etc.)
is a phantom type on the Rust side and a tag-checked handle on the
C side; passing the wrong state to an operation is rejected at IR
build time before either backend's compiler ever sees the code.

**Application-facing API contract.** The lifecycle FSM above governs
**SCE-internal** authoring (link bodies, worker bodies, statechart
actions). The boundary where the synthesized library hands a decoded
message out to user application code is a separate API surface that
extends the same ownership discipline to the public façade.

**Rust (AP, generated in `out/ap/src/lib.rs`):**

```rust
pub struct Sample<'pool> {
    pub key_expr: KeyExprRef<'pool>,
    pub payload:  &'pool [u8],
    pub timestamp: Option<Timestamp>,
    /* attachments, encoding info — all borrowing from the slot */
    _slot: SlotGuard<'pool>,   // RAII; returns slot on Drop
}

impl Sample<'_> {
    /// Promote to an owned copy via stage-copy. Releases the
    /// underlying slot immediately. The only legal way to escape
    /// the borrow lifetime; the caller pays the copy explicitly.
    pub fn take(self) -> OwnedSample;
}
```

The subscriber callback receives `&Sample<'_>`; the slot returns to
the pool when the callback returns (`SlotGuard::drop`). Holding the
borrow past callback return is a lifetime error (caught by `rustc`,
not at runtime). Deferred processing must `.take()` — the same
stage-copy path as oversized RX (ARCHITECTURE §9.3), reusing the
same accounting (`link/stage-copy-invoked` increments).

**C (MCU, generated in `out/mcu/inc/sce_sample.h`):**

C lacks Rust's lifetime + Drop machinery, so a single mechanism
cannot cover all cases. The contract is therefore **layered**, with
each layer catching what earlier layers miss. Layers 1–2 are static
(release-build effective); layer 3 is runtime debug-only; release
builds without an analyzer rely on the API contract being
documented like any C library.

**Layer 1 — Clang typestate + capability attributes (compile-time,
Clang only).** When the build is Clang, codegen emits Clang's
`consumable` typestate and Thread-Safety attributes; `-Wconsumed`
and `-Wthread-safety` then catch ownership violations at compile
time, **including in release builds**:

```c
/* Generated header excerpt; SCE_OWNERSHIP_ATTRS_AVAILABLE is set
 * when __has_attribute(consumable) && __has_attribute(callable_when)
 */
#if SCE_OWNERSHIP_ATTRS_AVAILABLE
#  define SCE_CONSUMABLE         __attribute__((consumable(unconsumed)))
#  define SCE_CALLABLE_WHEN(s)   __attribute__((callable_when(s)))
#  define SCE_SET_TYPESTATE(s)   __attribute__((set_typestate(s)))
#  define SCE_PARAM_TYPESTATE(s) __attribute__((param_typestate(s)))
#  define SCE_WARN_UNUSED        __attribute__((warn_unused_result))
#else
#  define SCE_CONSUMABLE
#  define SCE_CALLABLE_WHEN(s)
#  define SCE_SET_TYPESTATE(s)
#  define SCE_PARAM_TYPESTATE(s)
#  define SCE_WARN_UNUSED
#endif

typedef struct SCE_CONSUMABLE {
    const sce_keyexpr_t* key_expr;
    const uint8_t*       payload;
    size_t               payload_len;
    sce_timestamp_t      timestamp;
    sce_slot_handle_t    _slot;          /* opaque */
} sce_sample_t;

/* Accessor — only valid while sample is unconsumed */
SCE_CALLABLE_WHEN("unconsumed")
const uint8_t* sce_sample_payload(const sce_sample_t* sample
                                  SCE_PARAM_TYPESTATE("unconsumed"));

/* Take — transitions sample to consumed; subsequent access is a
 * compile error under -Wconsumed */
SCE_WARN_UNUSED
SCE_CALLABLE_WHEN("unconsumed")
SCE_SET_TYPESTATE("consumed")
sce_result_t sce_sample_take(const sce_sample_t* sample
                             SCE_PARAM_TYPESTATE("unconsumed"),
                             uint8_t* dst, size_t dst_cap,
                             size_t* out_len);

typedef void (*sce_sub_callback_t)(const sce_sample_t* sample
                                   SCE_PARAM_TYPESTATE("unconsumed"),
                                   void* ctx);
```

Under Clang ≥ 9 with `-Wthread-safety -Wconsumed`, the following
are diagnosed at compile time:
- Calling `sce_sample_payload` after `sce_sample_take` (use after
  consume)
- Double `sce_sample_take` (consume of already-consumed)
- Ignoring `sce_sample_take`'s result (`-Wunused-result`)
- Subscriber callback that escapes `sample` into a global without
  `take` (the `param_typestate("unconsumed")` flows out of scope
  without being consumed → `-Wconsumed` flags the leak)

This is the closest C gets to Rust's compile-time ownership.

**Layer 2 — Lint / static-analyzer comments (compile-time,
analyzer-driven).** For builds that aren't Clang, codegen also
emits PC-Lint / Coverity / Polyspace recognizable annotations:

```c
/*lint -sem(sce_sample_take, custodial(1)) */
/*lint -function(sce_sample_take, sce_sample_payload) */
/* coverity[+free : arg-0] */
sce_result_t sce_sample_take(const sce_sample_t* sample, ...);
```

`custodial(1)` tells PC-Lint that argument 1 (the sample) becomes
invalid after the call; subsequent `sce_sample_payload(sample)` in
the same scope is flagged. Coverity's `+free` annotation behaves
similarly. These are advisory — analyzer absence means analyzer
silence — but they cost nothing at runtime and catch the same class
of bugs as Layer 1 in toolchains that aren't Clang.

**Layer 3 — Debug-build runtime poisoning (test/QA builds only).**
When `-DSCE_DEBUG_OWNERSHIP=1` is defined (recommended for test
and QA builds, NOT release), the runtime maintains a slot-state
shadow table and:
- Poisons `sample->payload` to `0xDE` on callback return — any
  use-after-callback dereference lands on the poison byte
- Traps on double `sce_sample_take`, take-after-callback-return
- Traps on `dst_cap < payload_len`

This catches what Layers 1–2 miss (e.g. dynamic indirection through
function pointers that confuses static analysis). Release builds
elide the shadow table for zero overhead.

**GCC ecosystem fallback (Clang-Tidy mandatory + auto-on Layer 3.5).**
The ARM embedded ecosystem default toolchain is `arm-none-eabi-gcc`,
which does **not** support Clang's `consumable` typestate or
Thread-Safety attributes. On a bare GCC build the Layer 1
attributes silently no-op via `__has_attribute()`, which by itself
would leave Layer 2 (commercial analyzers) and Layer 3 (debug-only
poisoning) as the only static / runtime defenses. That outcome
is **not an accepted release configuration** — silently-inert hooks
are exactly the failure mode §2.4 invariant 1 ("static-first") and
the kind contracts forbid.

Codegen therefore emits a `out/mcu/CMakeLists.txt` fragment that
**mandates Clang-Tidy as a parallel verification stage** for every
GCC build:

```cmake
# Generated by sce-codegen
find_program(CLANG_TIDY_EXE clang-tidy REQUIRED)
# REQUIRED makes CMake fail at configure time if clang-tidy is
# absent. Diagnostic 'pool/clang-tidy-not-configured' fires as a
# hard error when the build compiler is GCC and the configure step
# cannot resolve clang-tidy or the CXX_CLANG_TIDY property does
# not get set.
set_target_properties(${SCE_MCU_LIB} PROPERTIES
    CXX_CLANG_TIDY "${CLANG_TIDY_EXE};-warnings-as-errors=*")
```

Clang-Tidy runs against the same source as the GCC build, parses
the Clang `consumable` / `callable_when` annotations (which are
preserved in the headers via `__has_attribute()` macros even when
the build compiler ignores them), and reports use-after-take /
double-take violations as build errors. The build compiler remains
GCC; Clang-Tidy is a side-channel verification.

The accepted release configurations are therefore:
1. Clang ≥ 9 build (Layer 1 active natively).
2. GCC build **plus** Clang-Tidy parallel verification (Layer 1
   restored via side-channel).
3. GCC or Clang build **plus** a recognized commercial analyzer
   integrated in CI (PC-Lint, Coverity, Polyspace — Layer 2).

GCC alone, without Clang-Tidy and without a recognized analyzer,
is **not an accepted release configuration**. It fails at build
time with `pool/clang-tidy-not-configured` (hard error) unless
the deploy descriptor explicitly opts into Layer 2 by declaring
`build.static_analyzer: pc_lint | coverity | polyspace` (which
SCE then treats as Layer 1's substitute and lets the build pass
with Clang-Tidy unconfigured). Treating "GCC alone" as a typo of
"GCC + Clang-Tidy" rather than a valid configuration is the
defense against the silently-inert hook class.

To further hedge against bugs that elude static analysis even on
Clang or analyzer-equipped builds, **Layer 3.5 (defensive runtime
check) is default-on for GCC builds and default-off for Clang
builds** — Clang users have Layer 1 statically, GCC users have
Clang-Tidy statically PLUS Layer 3.5 dynamically (the parallel
pass and the runtime check are complementary; Clang-Tidy catches
intra-procedural escapes, Layer 3.5 catches indirect-call
callback escapes). Authors override per build via:

```c
/* GCC build defaults: SCE_DEFENSIVE_OWNERSHIP=1 (Layer 3.5 active)
   Clang build defaults: SCE_DEFENSIVE_OWNERSHIP=0 (Layer 1 covers it)
   Override either default by passing the macro explicitly. */
```

The recommended posture by toolchain:

| Build compiler | Static (Layer 1+2) | Runtime (Layer 3.5) | Cost |
|---|---|---|---|
| Clang ≥ 9 | active (consumable + Wthread-safety) | off by default | compile-time only |
| GCC + Clang-Tidy in CI | Clang-Tidy active (parallel) | on by default | compile-time + ≈10 cyc/callback |
| GCC + recognized commercial analyzer (PC-Lint / Coverity / Polyspace) | active (Layer 2) | on by default | analyzer pass + ≈10 cyc/callback |
| GCC, no Clang-Tidy, no commercial analyzer | **not accepted** — `pool/clang-tidy-not-configured` is a hard error at configure time | n/a | n/a |

This matrix matches the reality that most embedded teams ship
GCC binaries; the build refusal on the bare-GCC row is what closes
the silently-inert hook class. Adopting Clang-Tidy is the
zero-cost path (open-source, no build-compiler swap, no commercial
license) that lets a GCC team stay on GCC for binary distribution
while still getting Layer 1 coverage.

**Layer 3.5 — Release-mode opt-in defensive checks
(`SCE_DEFENSIVE_OWNERSHIP=1`).** Clang typestate (Layer 1) is
intra-procedural and loses context across function pointers — a
subscriber callback that stores `sample` into a global cannot
reliably be diagnosed at compile time. For deployments where
this escape class matters (safety-critical, untrusted callback
authors, third-party plugins), defining
`-DSCE_DEFENSIVE_OWNERSHIP=1` enables a reduced subset of the
Layer 3 shadow table in release builds:
- Slot-state shadow update on callback enter/exit (verifies state
  is `cpu-ref` on entry, `consumed` or back-to-`free` on exit)
- Trap on use-after-callback-return (the global-escape case)
- Skips the heavier checks (poison fill, full state diff) that
  Layer 3 enables in debug

The cost is a per-callback compare-and-branch (≈10 cycles on
Cortex-M7) and one `uint8_t` per slot in the shadow table. This
is the recommended setting for deployments that link
third-party subscriber code, host scripted callbacks, or operate
in adversarial environments where Layer 4's "trust the API
contract" stance is insufficient. Disable
(`-DSCE_DEFENSIVE_OWNERSHIP=0`, default) for tightest-loop
hot-path deployments where the per-callback overhead matters.

**Layer 4 — Release runtime (no checking).** Release builds without
defensive opt-in have no runtime ownership enforcement; the API
contract is documented and trusted, same as any other C library.
Layers 1–2 do the heavy lifting in release builds — provided the
build is Clang or a recognized analyzer is in use.

**Layer trade-off summary:**

| Layer | Build mode | Toolchain | Catches | Cost |
|---|---|---|---|---|
| 1 Clang typestate | release + debug | Clang ≥ 9 | use-after-take, double-take, intra-procedural leak | compile-time only |
| 2 Lint annotations | release + debug | PC-Lint, Coverity, Polyspace, etc. | use-after-take, similar to Layer 1 | analyzer pass only |
| 3 Debug poisoning | debug only (`-DSCE_DEBUG_OWNERSHIP=1`) | any | runtime use-after-callback, double-take, undersized dst, full poison-fill | small RAM + per-callback shadow update + memory writes |
| 3.5 Release defensive opt-in | release (`-DSCE_DEFENSIVE_OWNERSHIP=1`) | any | callback-escape (sample stored to global, deref after callback returns) | ≈10 cycles per callback + `uint8_t` per slot |
| 4 Release runtime | release (default) | any | nothing | zero |

Authors ship release builds with **Clang + `-Wconsumed
-Wthread-safety` as a hard error**, OR with a GCC build and
Clang-Tidy parallel verification active, OR with a recognized
analyzer (PC-Lint / Coverity / Polyspace) in CI. The bare-GCC,
no-analyzer configuration is rejected at configure time
(`pool/clang-tidy-not-configured`, hard error) — Layer 4 is the
*runtime* row of the table only, not a release configuration on
its own. RFC §6.2.6 (drift detection) preserves the requirement
that emitted attributes/comments stay in sync with the kind source.

**Application-layer ownership diagnostics:**
- `pool/sample-take-without-stage-pool` — generated `sce_sample_take`
  has no stage-copy pool to draw from (deploy.yaml missing a
  `stage_pool` declaration on the link)
- `pool/sample-callback-signature-non-borrow` — author-declared
  callback signature in SCXML does not match the borrow-mode
  contract (e.g. takes `sample` by value, attempting to move
  ownership across a callback that has none)
- `pool/sample-typestate-attributes-disabled` — Clang build detected
  but `__has_attribute(consumable)` evaluates false (Clang too old,
  or `-fno-thread-safety` set). Layer 1 protection is silently
  inert; build succeeds with a warning so the operator can decide
  whether Layer 2 / Layer 3 coverage is sufficient or whether to
  upgrade the compiler
- `pool/clang-tidy-not-configured` — GCC build detected (no Clang
  typestate at compile time) and the generated CMakeLists.txt's
  Clang-Tidy parallel verification stage cannot find a `clang-tidy`
  executable, AND the deploy descriptor has not declared
  `build.static_analyzer: pc_lint | coverity | polyspace` as a
  Layer 1 substitute. **Hard error at configure time.** Layer 1
  coverage cannot be silently absent in a release configuration;
  the operator must install Clang-Tidy, switch build compiler to
  Clang ≥ 9, or declare a recognized commercial analyzer

**Diagnostics:**
- `mem/pool-section-conflict` — section declared but not in deploy memory map
- `mem/dma-channel-collision` — same channel bound to two pools
- `mem/alignment-violation` — codec field alignment > pool alignment
- `mem/pool-too-large` — sum of pools exceeds declared section size
- `mem/cache-policy-missing-on-dcache-core` — D-cache target has a pool without explicit `cache-policy`
- `mem/cache-policy-mismatch-region` — pool `cache-policy` inconsistent with its SRAM region `attr`
- `mem/cache-policy-unsupported-on-no-dcache-core` — `maintain`/`non-cacheable` declared on a core without D-cache (use `none`)
- `mem/cache-line-alignment` — pool `alignment` < target `dcache_line_size` with `cache-policy: maintain` (partial-line invalidate corrupts adjacent data on the start side)
- `mem/slot-size-not-cache-line-multiple` — `cache-policy: maintain` with `slot_size % platform.dcache_line_size != 0`. Each slot must occupy a whole number of cache lines, else `cache_invalidate_by_addr` after RX corrupts the adjacent slot's bytes that share the boundary line. Author resolution: round `slot_size` up to the next cache-line multiple and use the original logical size from within the slot
- `mem/inter-pool-padding-not-emitted` — codegen self-check. Two adjacent pool sections in the same `MEMORY` region under `cache-policy: maintain` without an intervening `. = ALIGN(<line_size>);` sentinel in the emitted linker fragment. Internal invariant violation (should never reach authors); guards against template regression that would let `dcache_invalidate_by_addr(pool_a, pool_a_size)` touch pool B's first cache line when a master linker script splices content between them
- `pool/ownership-violation` — emitted operation maps to a forbidden FSM transition (e.g. CPU write on a slot in `dma-busy-rx`)
- `pool/cache-maintenance-misplaced` — author code attempts manual `cache_clean` / `cache_invalidate` via `<sce:call>`; cache maintenance is FSM-driven only
- `pool/slot-leak-on-error-path` — error branch in encode/parse drops a `cpu-mut` or `cpu-ref` reference without an explicit transition back to `free`
- `pool/double-arm` — `link_arm_tx` / `link_arm_rx` invoked on a slot already in `dma-armed-*` or `dma-busy-*`
- `pool/return-on-dma-state` — `pool_return` invoked while slot is `dma-armed-*` or `dma-busy-*` (must un-arm first)
- `pool/cache-pre-arm-invalidate-missing-on-speculative-core` — `cache-policy: maintain` + `platform.has_speculative_prefetch: true` but codegen failed to emit pre-DMA-RX invalidate on `free → dma-armed-rx`. Internal codegen invariant violation (should never reach authors); guards against template regression
- `pool/speculative-prefetch-flag-missing` — `has_dcache: true` declared but `has_speculative_prefetch` not set; codegen cannot decide whether pre-arm invalidate is needed. Author resolution: declare `has_speculative_prefetch` per the SoC datasheet (M7+/A-class = true, M3/M4 = false)

### 5.F Build-time evaluation

**Problem.** CRC tables, KeyExpr tries, and other derived constants
must be computed at build time so generated code carries a
`static const` table, not a runtime initializer.

**Proposal.** Two primitives: `sce:compute-at="build"` attribute on
`<sce:const>`, and a `<sce:fold>` statement shape.

```xml
<sce:const name="TABLE" type="array<u16, 256>" sce:compute-at="build">
  <sce:fold range="0..256" as="i" elem-type="u16">
    <sce:var name="c" type="u16" init="i &lt;&lt; 8"/>
    <sce:foreach times="8">
      <sce:if cond="(c &amp; 0x8000) != 0">
        <sce:assign target="c" expr="(c &lt;&lt; 1) ^ 0x1021"/>
        <sce:else/>
        <sce:assign target="c" expr="c &lt;&lt; 1"/>
      </sce:if>
    </sce:foreach>
    <sce:yield expr="c"/>
  </sce:fold>
</sce:const>
```

**Execution model.** `sce-build` interprets the fold body on the host
at generation time, using the same expression semantics as runtime
execution. The computed array literal is emitted into each of the
six target backends per the §5.J.5 emitter table:

- Rust: `pub static <NAME>: [T; N] = [ ... ];` (or `&'static [T]`).
- C11: `static const T <NAME>[N] = { ... };` plus header `extern const`.
- Cpp: `inline constexpr std::array<T, N> <NAME> = { ... };`.
- Kotlin: top-level `val <NAME>: <Array> = ...`.
- Go: package-level `var <NAME> = [N]T{ ... }`.
- Python: module-level `<NAME>: tuple = ( ... )`.

The host interpreter is single-source (one Rust crate inside
`sce-build`); the per-language emitters only differ on how the
resulting array is serialized into the target syntax. Cross-backend
parity is therefore byte-equivalent on the underlying numeric data
(verified by §6.2.6 const-fold parity test) regardless of language
syntax differences.

**Host interpreter bounds.** To avoid the build turning into a
general-purpose compute platform, the host interpreter enforces:
- Total iterations across all folds ≤ configurable budget (default 1M)
- No allocations beyond the fold's output array size
- No external I/O (the interpreter is pure)

Exceeding the budget is diagnostic `algorithm/const-fold-budget-exceeded`,
with explicit opt-in via `sce-build --const-fold-budget=N`.

**Worked examples.** CRC16-CCITT and VLE constant tables are
covered in Appendix A/B (§5.A). KeyExpr trie construction is the
third canonical target for §5.F:

- **Flat (offset-based) trie — buildable today.** A `<sce:fold>`
  multi-pass over the deploy-declared subscription set emits a
  `const` array of `(segment_hash, first_child_offset, sibling_offset,
  terminal_sub_mask)` records placed in `.flash` (ARCHITECTURE §8.4
  "linker-placed static KeyExpr lookup"). Lookup is an §5.A
  algorithm walking offsets — bounded loops, `mode="static"` WCET
  derivable from trie depth × segment count.
- **Recursive node trie — Phase 2 (§5.H).** The natural representation
  `KeyTrieNode { segment, children: List<KeyTrieNode> }` requires
  the recursive-tree kind and lands with §5.H. Until then, authors
  use the flat representation above.

**Choosing static trie vs runtime bounded-collection.** Both are
supported on MCU (ARCHITECTURE §2.4 invariant 1):

| Use case | Recommended path |
|---|---|
| Subscription set fixed at deploy, never changes | Build-time flat trie (this section) — zero RAM, lookup is read-only Flash access |
| Subscription set varies at runtime (zenoh-pico parity baseline) | Runtime linear/indexed scan over `bounded-collection` (§5.L) with `mode="measured"` WCET annotation (§5.A) |
| Mixed (permanent system topics + user-declared) | Hybrid: static trie tried first, fall through to bounded-collection on miss |

The MVP baseline is **runtime bounded-collection** because zenoh-pico
parity (ARCHITECTURE §2.1) requires runtime
`declare_subscriber`/`undeclare_subscriber`. Static trie is an
optimization opt-in for deployments that can pin their subscription
set, not a default.

**Diagnostics:**
- `algorithm/const-not-foldable` — body references runtime-only value
- `algorithm/const-fold-budget-exceeded`
- `algorithm/const-yield-type-mismatch`

### 5.G Parametric kinds (Phase 2)

**Problem.** VLE encoding logic is identical for u16, u32, u64 modulo
the max-shift bound. Duplicating three copies is a classic SSoT
violation.

**Proposal.** Kind-level type parameters.

```xml
<scxml sce:kind="algorithm" name="vle" version="1.0">
  <sce:type-param name="W" constraint="unsigned-integer"/>
  <sce:signature>
    <sce:param name="v" type="$W"/>
    <sce:param name="cursor" type="cursor"/>
    <sce:return type="Result"/>
  </sce:signature>
  <!-- body uses $W as the element type -->
</scxml>
```

Instantiation happens at import site:

```xml
<sce:import ref="vle.scxml" alias="vle_u64">
  <sce:type-arg name="W" value="u64"/>
</sce:import>
```

**Deferred to Phase 2** because v1 (non-parametric) suffices for the
crc/vle/keyexpr MVP (write three near-identical files; clean up once
generics land). Worth pre-planning the XSD and IR shape so §5.A's body
does not need rework.

### 5.H Recursive and tree data kinds (Phase 2)

**Problem.** TLV extension chains are recursive (`SessionMessage` has
a `next: Option<Extension>`). KeyExpr tries are n-ary trees. Neither
is expressible in the current `Codec`/`ForgeField` model.

**Proposal sketch.** A new kind `data-tree` or an attribute on codec
fields allowing `type="self"` or `type="ref<OtherKind>"` for explicit
recursion, with cycle detection at parse time.

**Deferred** — details to be worked out after §5.A/B/C/F land. The
MVP can use fixed-depth flattening (chain of up to N extensions) as a
stopgap.

### 5.I `sce:extern` — bounded escape hatch

**Problem.** Two narrow concerns resist declarative expression:
platform-specific intrinsics (atomic CAS, memory fences) and hardware
register access (MMIO, interrupt enable/disable). These legitimately
live outside SSoT because they are target-specific by nature.

**Proposal.** A whitelisted extern symbol reference.

```xml
<sce:extern name="sce_atomic_cas_u32"
            sig="(*mut u32, u32, u32) -> bool"
            abi="c"
            crate="sce_intrinsics_runtime"/>
```

**Whitelist.** A single `sce_intrinsics_runtime` crate (shared,
published alongside `sce_forge_runtime`) lists all permitted symbols.
SCE build rejects `sce:extern` references not present in the
whitelist. Maintains the SSoT claim for anything algorithmic while
acknowledging a bounded set of intrinsic bridges.

**Concrete whitelist (v1).** Every symbol below is declared with
exact signature, ABI, and memory-ordering semantics. Ordering
matches C11 `<stdatomic.h>` / Rust `core::sync::atomic::Ordering`.

*Atomics (per-width: u8, u16, u32, u64, usize):*
```
sce_atomic_load_{acquire,relaxed}
sce_atomic_store_{release,relaxed}
sce_atomic_cas_weak_{acq_rel,release,relaxed}    # returns old value
sce_atomic_cas_strong_{acq_rel,release,relaxed}
sce_atomic_fetch_add_{acq_rel,relaxed}
sce_atomic_fetch_sub_{acq_rel,relaxed}
sce_atomic_fetch_or_{acq_rel,relaxed}
sce_atomic_fetch_and_{acq_rel,relaxed}
```

*Fences:*
```
sce_atomic_fence_{acquire,release,acq_rel,seq_cst}
sce_compiler_barrier                   # reorders nothing at runtime; blocks compiler reordering
sce_dma_fence                          # maps to DSB on ARMv7-M+ where needed
```

*Cache maintenance (required by §5.E `cache-policy: maintain`):*
```
sce_dcache_clean_by_addr(const void* start, size_t len)
sce_dcache_invalidate_by_addr(void* start, size_t len)
sce_dcache_clean_invalidate_by_addr(void* start, size_t len)
```
All three MUST round start/len to `platform.dcache_line_size`
granularity; out-of-line spans trigger `mem/cache-line-alignment`
at build time via pool `alignment` check.

*Interrupt control (optional — only for workers that need critical sections):*
```
sce_irq_save() -> irq_state_t
sce_irq_restore(irq_state_t)
```

**SPSC/MPSC inbox ordering contract.** Worker inboxes (§5.D) MUST
use acquire/release pairs for head/tail indices. Use of `relaxed`
on cross-worker shared state is a diagnostic:
- `worker/inbox-ordering-relaxed-across-cores` — inbox producer and
  consumer on different cores; relaxed ordering insufficient
- `worker/inbox-ordering-unspecified` — no ordering chosen, codegen
  defaults to acquire/release with a warning

**Target-level extension.** Architectures may extend the whitelist
through a **target plugin** declared in deploy.yaml
(`extern_symbols.target_plugin: <path>`); the plugin file is a YAML
document listing additional symbols with signatures. Plugins are
part of deploy.yaml review scope, not an unbounded escape hatch.

The canonical use case is **hardware sync primitives on multi-core
MCUs**, where plain atomics + cache maintenance are insufficient
because cores may share state through hardware mailbox / semaphore
IPs (e.g. STM32H7 HSEM, ESP32 cross-core spinlock, NXP MU mailbox).
A target plugin shape:

```yaml
# configs/target_extensions_stm32h7.yaml
symbols:
  - name: sce_hw_sem_take
    sig: "(u32) -> bool"            # semaphore_id -> taken
    abi: c
    purpose: cross-core-mutex
  - name: sce_hw_sem_release
    sig: "(u32)"
    abi: c
    purpose: cross-core-mutex
  - name: sce_hw_mbox_send
    sig: "(u32, *const u8, usize) -> bool"
    abi: c
    purpose: cross-core-notify
```

When `platform.core_count > 1`, deploy.yaml MUST declare which
primitive each cross-core inbox uses. Codegen wires the matching
extern in place of the default atomic-only path:

```yaml
machines:
  mcu_node:
    cross_core_sync:
      default: hw_semaphore           # atomics_only | hw_semaphore | mailbox
      hw_semaphore_unit: stm32h7_hsem
      inbox_overrides:
        rx_to_session_inbox: hw_semaphore
        tx_to_link_inbox:    atomics_only   # cores share D-Cache region
```

Atomics-only (the default for `core_count: 1`) remains a valid
choice on cores that share a coherent D-Cache; the diagnostic
`worker/inbox-ordering-relaxed-across-cores` (above) catches the
unsafe combinations, and `deploy/multicore-without-target-plugin`
(§5.K) catches the case where a cross-core inbox is declared but
no plugin is loaded.

**What does NOT go here.** CRC, VLE, keyexpr matching — these are all
expressible via §5.A and MUST be authored as algorithm kinds, not as
extern references. This was established in the watching-zenoh RFC
review of 2026-04-24.

**Linker flavor declaration.** Vendor toolchains differ in linker
script syntax: GNU `ld` `MEMORY`/`SECTIONS` (STM32, ESP-IDF, NXP
MCUXpresso, Zephyr), ARM Compiler `scatter` files (`.sct`, used by
Keil µVision), and IAR ILINK `.icf`. Generated `linker_fragment`
(§5.E + ARCHITECTURE §6.2) defaults to GNU LD. Targets that use a
different flavor declare it via the target plugin:

```yaml
# configs/target_extensions_keil_stm32f4.yaml
linker_flavor: scatter_arm           # gnu_ld | scatter_arm | icf_iar | os_managed
linker_fragment_path: linker_fragment.sct  # output filename override
symbols:
  - ...
```

Supported flavors:

| Flavor | Toolchain | Phase | Notes |
|---|---|---|---|
| `gnu_ld` | GCC, Clang, esp-idf, MCUXpresso, Zephyr default | A (default) | The reference shape; all §5.E examples assume this |
| `scatter_arm` | ARM Compiler 5/6, Keil µVision | C+ | `*.sct` generation; section attributes mapped to scatter regions |
| `icf_iar` | IAR Embedded Workbench | D+ | `*.icf` generation; planned but not in MVP |
| `os_managed` | Zephyr / NuttX | A (passthrough) | OS owns the linker stage; SCE emits a CMake target the OS imports, no `.ld` written |

When `linker_flavor` is unspecified, the build defaults to `gnu_ld`
and emits an informational note. `scatter_arm` and `icf_iar` arrive
through target_plugin Phase C+ work; until then, Keil/IAR users
must hand-translate the `gnu_ld` fragment (one-time per project,
documented in the target_plugin guide).

**Diagnostics:**
- `extern/symbol-not-in-whitelist`
- `extern/abi-mismatch`
- `extern/signature-mismatch`
- `extern/ordering-unspecified` — atomic intrinsic invoked without explicit ordering suffix
- `extern/ordering-insufficient-for-cross-core` — relaxed ordering used on state shared across cores (paired with `worker/inbox-ordering-*`)
- `extern/target-plugin-symbol-conflict` — target plugin redefines a core whitelist symbol
- `extern/linker-flavor-unsupported` — `linker_flavor` declared as `scatter_arm`/`icf_iar` before the corresponding generator lands (Phase C+)
- `extern/linker-flavor-os-managed-without-cmake-import` — `os_managed` declared but the deploy does not produce a CMake target the host OS can import

**Fuzz coverage transport (Phase D+).** ARCHITECTURE §11.6 F4 tier
requires an on-device transport for two primitives: `deliver_input`
(host → target byte sequence) and `read_coverage_bitmap` (host ←
target edge-counter region). The transport is target-specific and
is therefore a target_plugin field rather than an SCE core concern.
For Phase A/B/C the field is absent (F4 has no implementation yet);
for Phase D and beyond, plugins SHOULD declare the transport on
safety-critical-class targets, and MUST declare it on any deploy
that opts F4 into `fuzz_tiers:`.

```yaml
# configs/target_extensions_stm32h7.yaml (Phase D+ excerpt)
fuzz_coverage_transport:
  kind: segger_rtt              # renode_sysbus | segger_rtt |
                                # openocd_memmap | dma_uart |
                                # semihosting
  bitmap_section: ".fuzz_cov"   # linker section holding
                                # __start/__stop___sancov_guards
  bitmap_max_bytes: 65536
  iteration_timeout_us: 10000
  # transport-specific:
  rtt_channel_input: 1          # host -> target (deliver_input)
  rtt_channel_coverage: 2       # target -> host (read_coverage_bitmap)
```

Supported transports:

| Kind | Adapter | Throughput | Vendor lock | Phase |
|---|---|---|---|---|
| `renode_sysbus` | Renode VM monitor protocol; reads bitmap memory directly via `sysbus.ReadDoubleWord`, delivers input via virtual UART | very high (native memory) | none | D |
| `segger_rtt` | SEGGER J-Link RTT channels (one input, one coverage) | ~MB/s, non-blocking | SEGGER J-Link probe | D |
| `openocd_memmap` | OpenOCD `mdw` polling against the bitmap symbol address; input via secondary UART or RTT | ~50–200 execs/s | none (any SWD/JTAG) | D |
| `dma_uart` | Host UART ↔ target high-speed UART, target uses DMA for non-blocking transmit | ~Mb/s, board-dependent | none (board-specific pinout) | D+ |
| `semihosting` | OpenOCD semihosting BKPT 0xAB | low (BKPT blocks target) | none | fallback |

Plugins MAY add transports beyond this set; the contract is the two
primitive shapes plus the bitmap section convention. SCE codegen
emits the on-target agent stub conditionally on
`BUILD_FUZZ_HARNESS` / `[features] fuzz`; production builds carry no
fuzz code (cross-ref §5.E pool ownership, ARCHITECTURE §11.4
no-alloc guard layer 4 stays armed under fuzz).

The Renode/HIL responsibility split inside F4 (Renode owns
FSM-timing + cache/MPU correctness; HIL owns DMA/ISR/peripheral/
vendor-libc) is documented in ARCHITECTURE §11.6; the transport
choice in this field selects the *executor* without changing that
split — `renode_sysbus` activates Renode-as-F4, the other four
kinds activate HIL-as-F4.

**Diagnostics (fuzz coverage transport):**
- `fuzz/coverage-transport-on-pre-D-tier` — `fuzz_coverage_transport`
  declared on a deploy whose phase target is < D; F4 has no
  implementation pre-D and the field would be silently ignored
- `fuzz/coverage-transport-not-declared-on-f4-target` — deploy
  declares F4 in `fuzz_tiers:` but `target_plugin.fuzz_coverage_transport`
  is missing; F4 cannot wire the runner
- `fuzz/coverage-instrumentation-mismatch-across-tiers` — generated
  fuzz harness was built for one tier with `trace-pc-guard` and
  another with `inline-8bit-counters`; corpora become non-
  interchangeable, breaking the §11.6 byte-sequence-portability
  invariant
- `fuzz/coverage-bitmap-section-symbol-missing` — the linked ELF
  exposes neither `__start___sancov_guards` / `__stop___sancov_guards`
  nor the configured `bitmap_section` equivalents; the F4 runner
  has nothing to read
- `fuzz/coverage-transport-kind-unsupported-by-plugin` — the declared
  transport `kind` is not in the SCE core list and the loaded plugin
  does not register it

### 5.J Codegen backend coverage

**Problem.** SCE shipped `Language::C11` in `generator::Language`
and closed byte-golden parity for the existing eleven Forge kinds
across all six backends (Rust/Cpp/Kotlin/Go/Python/C11) at commit
`758aea3f` ("close C11 byte-golden parity with 5 backends"). The
gaps this RFC has to close are therefore not the *enum membership*
or the *existing-kind* matrix — they are (a) emitter coverage for
the **new kinds** introduced by this RFC (§5.A algorithm, §5.B
codec DSL extensions, §5.F const-fold, §5.L bounded-collection,
§5.O sourcemap), which must extend to all six backends to preserve
the parity baseline (§5.J.1 / §5.J.4 / §5.J.5), and (b) the
**statechart** runtime `no_std` variant (§5.J.2). The Rust Forge
runtime (`sce-forge-runtime/rust/`) is already `no_std` + no-alloc,
so emitted Rust for pure-function kinds (`Transform`, `Codec`,
`Validator`, `Procedure` L1) is already MCU-usable; the remaining
runtime gap is `sce-rust-runtime/` (the statechart runtime backing
emitted session/declare/query/fragment FSMs), which is currently
`std`-only.

**Proposal.**

**5.J.1 New-kind emitter coverage on the existing C11 backend.**
The C11 template tree at `tools/codegen/templates/forge/c/` ships
eleven existing-kind templates as of `758aea3f`. Phase A5 closes
the §5.J.4 matrix on C11 for §5.A `algorithm` (Phase A3 had landed
algorithm on Rust + Cpp only). This RFC extends that tree with
the remaining new kinds defined in §5.B/§5.F/§5.L/§5.O (MCU-target
kinds §5.C/§5.D/§5.E/§5.M land here too — see §5.J.5 for the
`(language, os)` activation matrix that distinguishes generic kinds
from MCU-only kinds). Generated artifact: a single `<snake>.h` per
forge document (the existing C11 forge tree ships header-only —
the SCE-side decision pinned at `758aea3f` was that pure-function
forge kinds emit one self-contained header rather than `.h` + `.c`
pairs; the §5.J.2 statechart-runtime split is unaffected). Uses
`<stdint.h>` integer types, `_Static_assert` for invariants, no
varargs in generated code, no `malloc`. Uses a small
`sce_forge_runtime.h` for shared helpers (cursor, Result enum,
pool API). Algorithms with a `bytes` parameter pull
`sce/forge/procedure.h` for the runtime's stack-bounded
`sce_forge_bytes_t` value type (RFC §5.J.2 F1 fixed-cap copy
semantics).

SCE Mesh C11 is explicitly out of this RFC — the downstream project
does not use Mesh codegen (see §5.K).

**5.J.2 Statechart Rust `no_std` variant.** Scope is the **statechart
emitter** and the `sce-rust-runtime` crate it depends on — the Forge
runtime is already `no_std` (§4, `sce-forge-runtime/rust/`).

- Flag on the existing Rust backend: `sce-codegen generate ...
  -l rust --no-std`. Emits `#![no_std]` at the generated crate root
  and uses `heapless::String` / `heapless::Vec` for any collection
  previously emitted as `String` / `Vec`.
- `sce-rust-runtime` grows a `no_std` feature gate: default remains
  `std` (unchanged for existing consumers), `--no-default-features
  --features=no_std,script-engine-...` enables the firmware profile.
  The scheduler, event queue, and external-event hooks are the
  primary `std`-touching surfaces and must be re-expressed against
  `core::` + a small HAL trait (ticks, wake, irq-save) provided by
  `sce_intrinsics_runtime` (§5.I).
- Pure-function kinds (Transform/Codec/Validator/Procedure L1) flow
  through unchanged because their runtime is already `no_std`.

**No-alloc collection plan (cross-reference).** The no_std
statechart variant has zero `alloc` dependency. All event- or
data-bearing structures emitted by the statechart codegen bottom
out on `heapless`, with capacity sourced from `deploy.yaml` or
`<sce:capacity>` (§5.L resolution rule). There is no path from
generated no_std code into `alloc::*`.

| Structure | Backing | Defined in |
|---|---|---|
| Statechart event queue (per-instance inbox) | `heapless::spsc::Queue<E, N>` | §5.D |
| Declared subscriber / queryable / pending-query tables | `heapless::Vec<T, N>` via `bounded-collection` | §5.L |
| In-flight reassembly table | `heapless::Vec<T, N>` via `bounded-collection` | §5.L, §5.M |
| Per-link TX worker queue | `heapless::spsc::Queue` | §5.N |
| Generated `String`-typed payloads | `heapless::String<N>` | this section |

**Generator dispatch.** `generator::Language` already carries
`Language::C11` (shipped at `758aea3f`); §5.J.2 adds the `no_std`
distinction either as a subflag (`std: bool`) on the existing
`Language::Rust` arm or as `Language::RustNoStd`, at SCE's
discretion. CLI `-l` must accept multiple values for one invocation
to produce AP Rust and MCU C in a single pass.

**5.J.3 Backend coverage as a 3-tuple.** A "backend" in the codegen
matrix is `(language, target_os, runtime_crate)`, not just
`language` — the same emitted Rust on AP linux versus AP qnx links
against different runtime crates (`sce_link_runtime_tokio` vs
`sce_link_runtime_qnx`) because mio has no QNX backend (OQ-W20)
and the QNX-native reactor is `dispatch_create()` + channels rather
than epoll-shaped. The OS axis is therefore part of backend
selection, not a downstream link-time concern.

Naming convention (formalized review #13): runtime crates are named
`sce_link_runtime_<os>` and ship per-OS:

| OS axis | Crate | Reactor | Phase |
|---|---|---|---|
| `bare_metal` | `sce_link_runtime_lwip` | ISR-driven + worker tick | B (current MCU baseline) |
| `linux` | `sce_link_runtime_tokio` | mio (epoll) + optional `tokio_uring` (kernel ≥ 5.10) | D.1 |
| `qnx` | `sce_link_runtime_qnx` | QNX `dispatch_create()` + io-sock | D.2 |
| `macos`, `freebsd` | `sce_link_runtime_kqueue` | mio (kqueue) | E |
| `windows` | `sce_link_runtime_iocp` | mio (IOCP) | E |
| `rtos` (Zephyr / FreeRTOS / NuttX) | `sce_link_runtime_<rtos_id>` | RTOS-native | C+ via target plugin |

The backend 3-tuple has phase-gated combinations: `(rust, linux,
sce_link_runtime_tokio)` and `(c11, bare_metal, sce_link_runtime_lwip)`
are the two combinations live in the current Phase A–C track.
`(rust, qnx, sce_link_runtime_qnx)` is reserved Phase D.2 namespace;
authoring `deploy.yaml` against it pre-Phase-D fails with
`deploy/platform-os-not-implemented-in-current-phase` (§5.K).

This means: **the SCE generator's matching of language emitter to
runtime crate is `platform.os`-driven**. Authors do not pick a
runtime crate by name in `deploy.yaml`; they declare `platform.os`
and codegen wires the canonical crate. An override field
`runtime_crate:` exists for the rare case where a deploy needs a
custom runtime (e.g. an in-house QNX reactor variant), but the
default is the OS-canonical crate above.

**5.J.4 Kind × language matrix (parity invariant).** SCE closed
byte-golden parity for the existing eleven Forge kinds across all
six language backends at `758aea3f`. The new kinds in this RFC
divide into two classes by emitter shape, and the table below
fixes each class's expected coverage so additions never silently
regress the parity baseline:

| Kind | Class | Rust | Cpp | Kotlin | Go | Python | C11 |
|---|---|---|---|---|---|---|---|
| §5.A `algorithm` | generic | required | required | required | required | required | required |
| §5.B codec DSL extensions (VLE, variant, present-if, len-prefix, repeat, until-eof, TLV chain, test-vector, parse-mode) | generic | required | required | required | required | required | required |
| §5.B codec DSL extensions, MCU-only sub-features (`dma-burst-align`, `<sce:dma-constraint>`, codec-aggregate-WCET gate) | MCU-class | required | hard error if used | hard error if used | hard error if used | hard error if used | required |
| §5.F build-time const-fold | generic (host interpreter) — output is per-language const data | required | required | required | required | required | required |
| §5.L `bounded-collection` | generic | required (`heapless::Vec` no-std / `Vec` std) | required (`std::array<T,N>` + `std::bitset<N>`) | required (`Array<T?>` + `BooleanArray`) | required (fixed slice + occupancy mask) | required (`tuple` + `bytearray` mask) | required (C array + `uint8_t[]` bitmap) |
| §5.O source traceability | generic | required (`#[doc = "SCE-MAP: ..."]` + sourcemap JSON) | required (`#line` + sourcemap JSON) | required (`// SCE-MAP:` comment + sourcemap JSON) | required (`//line` directive + sourcemap JSON) | required (`# SCE-MAP:` comment + sourcemap JSON) | required (`#line` + sourcemap JSON) |
| §5.C `link` | MCU-class | required (only with `sce_link_runtime_tokio` on linux / `sce_link_runtime_qnx` on qnx / `sce_link_runtime_lwip` on bare_metal) | hard error if used | hard error if used | hard error if used | hard error if used | required (only with `sce_link_runtime_lwip` on bare_metal) |
| §5.D `worker` (timer + worker primitives) | MCU-class | required (cooperative worker on MCU; preemptive on AP) | hard error if used | hard error if used | hard error if used | hard error if used | required |
| §5.E `buffer-pool` | MCU-class | required (`#[no_std]` heapless slot table) | hard error if used | hard error if used | hard error if used | hard error if used | required (linker-fragment-backed pool sections) |
| §5.M reassembly variant | MCU-class | required | hard error if used | hard error if used | hard error if used | hard error if used | required |

"Generic" kinds preserve the §2.4 invariant 1 ("static-first") and
the parity baseline: every generic kind authored in SCXML emits
working code in every language backend. Authoring a generic kind
is decoupled from `platform.os` and `class` — the same kind file
emits all six backends in one `sce-codegen generate` invocation.

"MCU-class" kinds are protocol-synthesis primitives whose emitter
shape is not meaningful on every language backend. Their semantics
(DMA-aligned slot acquisition, ISR-driven RX path, link-time
section placement) bottom out on `(rust|c11)` × `(bare_metal|
linux|qnx)` only — the four other languages have no equivalent
substrate, so binding an MCU-class kind to a Cpp/Kotlin/Go/Python
target is a build-time hard error rather than a silently empty
emit. See §5.J.5 for the language-specific emitter contracts that
implement this matrix.

**5.J.5 Per-language emitter contracts (new generic-class kinds).**
The matrix above commits all six backends to emit each generic-class
kind. Per-language emitter shape is fixed below to anchor the
parity test (§6.2.6 byte-golden + cross-backend semantic parity).
MCU-class kinds restrict to `(rust, c11)` per the matrix; their
per-language shape is in the kind-owning section (§5.C/D/E/M).

| Kind | Rust | Cpp | Kotlin | Go | Python | C11 |
|---|---|---|---|---|---|---|
| §5.A `algorithm` | `pub fn <name>(...) -> T { for/while/if/let-mut }`. `#![no_std]`-clean when no `bytes`-typed param. | `T <name>(...) { for/while/if }` free function in `namespace sce::generated`. No exceptions, no STL containers. | `object <Name> { fun call(...): T }` Kotlin singleton; primitives only on the call boundary. | `func <Name>(...) T { for/if }`; no goroutines, no maps. | `def <name>(...) -> T:` pure function; no globals captured. | `static T <name>(...) { for/while }` with `_Static_assert` on bound constants. |
| §5.B codec extensions (generic sub-features) | `encode/decode<&mut Cursor>` returning `Result<T, CodecError>`; `parse-mode: borrow` lowers to `&'buf [u8]`. | `bool encode(...)` / `optional<T> decode(...)` over `span<uint8_t>`; no exceptions. | `fun encode(buf: ByteBuffer, v: T): Boolean` / `fun decode(buf: ByteBuffer): T?` on `java.nio.ByteBuffer`. | `func encode(buf *Cursor, v T) error` / `func decode(buf *Cursor) (T, error)`. | `def encode(buf: bytearray, v) -> int` / `def decode(buf: memoryview) -> (T, int)` returning consumed-bytes count. | `int encode(sce_cursor_t*, const T*)` / `int decode(sce_cursor_t*, T*)` with `SCE_NEED_MORE_BYTES` return. |
| §5.F const-fold | `pub static <NAME>: [T; N] = [ ... ];` (or `&'static [T]` slice). | `inline constexpr std::array<T, N> <NAME> = { ... };`. | `val <NAME>: <Array> = <Array>(intArrayOf(...))` top-level immutable. | `var <NAME> = [N]T{ ... }` package-level. | `<NAME>: tuple = ( ... )` module-level immutable. | `static const T <NAME>[N] = { ... };` declared inside the per-fixture `.h` (file-scope when the header is included; mirrors the §5.J.1 single-header decision pinned at `758aea3f`). |
| §5.L `bounded-collection` | `heapless::Vec<T, N>` (no-std) or `Vec<T>` capped at N (std), with codegen-checked push/pop wrappers. | `std::array<T, N>` + `std::bitset<N>` occupancy + `size_t len`; iterators expose only occupied slots. | `Array<T?>(N)` + `BooleanArray(N)` occupancy + `var len: Int`. | fixed-length array + parallel occupancy mask + `len int`. | `list` of length N initialized to `None` + `bytearray(N)` mask + `len`. | `T <name>[N]; uint8_t <name>_mask[(N+7)/8]; size_t <name>_len;` with `_Static_assert(N <= UINT16_MAX)`. |
| §5.O sourcemap | per-symbol `#[doc = "SCE-MAP: <state_path>:<line_range>"]`; release-stripped fallback as `// SCE-MAP:` comment; `out/{ap,mcu}/sce_sourcemap.json` shared sidecar. | `#line <n> "<scxml_file>"` directive + sidecar JSON. | header banner comment `// SCE-MAP:` per symbol + sidecar JSON (Kotlin has no preprocessor). | `//line <scxml_file>:<n>` Go directive + sidecar JSON. | `# SCE-MAP:` line comment per symbol + sidecar JSON. | `#line <n> "<scxml_file>"` directive + sidecar JSON. |

The sidecar JSON `out/{ap,mcu}/sce_sourcemap.json` is identical
across all six backends (§5.O is host-side metadata; only the
in-source marker shape differs by language). Cross-backend parity
testing (§6.2.6) checks both the per-language marker presence
*and* the JSON sidecar byte-equivalence to ensure no backend
silently drops traceability.

**Diagnostics:**
- `codegen/kind-unsupported-on-target` — e.g. a kind referencing
  floating-point on a target declared `no-float`
- `codegen/alloc-required-but-no-std` — caught before emission
- `codegen/backend-tuple-not-implemented` — `(language, os,
  runtime_crate)` combination not yet shipped (e.g. `(rust, qnx, *)`
  attempted before Phase D.2 lands). Hard error, references the
  Phase rollout in §7. Auto-resolves once the corresponding crate
  ships
- `codegen/runtime-crate-naming-convention-violation` — a deploy
  declares `runtime_crate:` (override) with a name that does not
  match `sce_link_runtime_<os>` and the override lacks a
  justification reference. Warning; promotable to hard error in
  strict-mode CI
- `codegen/mcu-class-kind-on-non-mcu-language` — an MCU-class kind
  (§5.C link / §5.D worker / §5.E buffer-pool / §5.M reassembly,
  or an MCU-only §5.B sub-feature like `dma-burst-align`) authored
  with a target backend in `{cpp, kotlin, go, python}`. Hard error.
  These kinds are wired only on `(rust, *)` and `(c11, bare_metal)`;
  use of them on other language backends has no defined emitter
  shape and is a configuration error
- `codegen/generic-kind-backend-emit-missing` — a generic-class kind
  (§5.A/§5.B-generic/§5.F/§5.L/§5.O) was authored, the corresponding
  template directory exists for the target language, but the per-kind
  template is absent. Hard error — generic kinds must emit on every
  backend per the §5.J.4 matrix; template absence is an SCE bug, not
  a downstream concern

### 5.K Deploy model extensions

**Problem.** `deploy.yaml` has no representation for platform class,
memory regions, DMA channels, link instances, or buffer-pool
placement.

**Proposal.** Additive sections (does not disturb existing
`machines.<n>.bindings` / SCE Mesh transport fields):

```yaml
machines:
  mcu_node:
    platform:
      class: mcu                   # NEW: ap | mcu
      os: bare_metal               # NEW: linux | qnx | macos | freebsd |
                                   #   windows | bare_metal | rtos.
                                   #   Phase availability:
                                   #   - bare_metal: Phase A onward (MCU
                                   #     zenoh-pico parity track, current).
                                   #   - linux:      Phase D onward (AP
                                   #     baseline; sce_link_runtime_tokio).
                                   #   - qnx:        Phase D+ (AP QNX
                                   #     baseline; sce_link_runtime_qnx).
                                   #   - macos / freebsd / windows / rtos:
                                   #     Phase E+ (deferred AP targets,
                                   #     namespace reserved here so future
                                   #     additions are non-breaking).
                                   #   The OS axis is design-considered
                                   #   from Phase A, implementation-phased
                                   #   per the table above. MCU-first
                                   #   priority is invariant: Phase A–C
                                   #   exclusively serves bare_metal /
                                   #   MCU; AP work begins at Phase D.
                                   #   When `class: mcu`, `os` is
                                   #   `bare_metal` or `rtos` — never
                                   #   linux/qnx/macos/freebsd/windows.
                                   #   When `class: ap`, `os` is one of
                                   #   linux/qnx/macos/freebsd/windows.
      soc: <soc_id>                # NEW: target-specific identifier (TBD)
      core: <core_id>              # NEW: e.g. primary core, deploy-selected
      has_dcache: true             # NEW: drives §5.E cache-policy validation
      dcache_line_size: 32         # NEW: granularity for cache maintenance
      has_speculative_prefetch: true
                                   # NEW: true for cores with speculative
                                   #   load / hardware prefetcher (M7,
                                   #   M85, A-class). Drives §5.E
                                   #   pre-DMA-RX invalidate emission.
                                   #   Defaults: M0/M0+/M3/M4=false,
                                   #   M7+/A-class=true. When unspecified
                                   #   on a `has_dcache: true` core, codegen
                                   #   emits build error rather than guessing.
      core_count: 1                # NEW: 1 | N; enables cross-core ordering checks
      clock_freq_mhz: 400          # NEW: core clock frequency; used for
                                   #   stage-copy WCET (§5.M) and any
                                   #   measurement-class scaling
      memcpy_cycles_per_byte: 1.0  # NEW: per-target memcpy cost. Defaults
                                   #   per architecture (M0/M0+: 4.0,
                                   #   M3/M4: 2.0, M7: 1.0, A-class: 0.5);
                                   #   override when measured. Used by
                                   #   §5.M stage-copy WCET analysis
      vle_decode_cycles_per_byte: 6.0
                                   # NEW: per-byte VLE decode cost (continuation-bit
                                   #   test + shift + accumulate). Defaults per
                                   #   architecture (M0/M0+: 12.0, M3/M4: 8.0,
                                   #   M7: 6.0, A-class: 3.0). Used by §5.B codec
                                   #   aggregate WCET analysis. Required when any
                                   #   codec on the deploy contains a `vle_*` field
                                   #   AND scheduler.kind=cooperative.
      tlv_chain_per_entry_overhead_us: 0.5
                                   # NEW: fixed cost per TLV chain entry beyond
                                   #   the body decode (id-byte + length VLE +
                                   #   dispatch). Defaults per architecture
                                   #   (M0/M0+: 1.5, M3/M4: 0.8, M7: 0.5,
                                   #   A-class: 0.2). Required when any codec on
                                   #   the deploy contains a `tlv-chain` AND
                                   #   scheduler.kind=cooperative.
    scheduler:
      kind: cooperative            # NEW: tokio | cooperative | rt
      tick_period_us: 1000         # NEW
      worker_stack_budget: 4096    # NEW: bytes; used by TLV depth check
      worker_slot_budget_us: 200   # NEW: per-slot WCET ceiling (microseconds);
                                   #   build checks every algorithm/procedure
                                   #   slot's static WCET estimate (or registered
                                   #   measured value, §5.A) against this bound.
                                   #   Required when scheduler.kind=cooperative.
      keepalive_jitter_budget_us: 5000
                                   # NEW: max tolerated drift between scheduled
                                   #   Keepalive tick and actual emission.
                                   #   Sum of worst-case slot budgets in one
                                   #   tick window MUST fit inside this bound.
                                   #   Recommended default: 0.5 × min lease.
    memory:                        # NEW section
      sram_regions:
        dtcm:  { base: 0x20000000, size: 64K, attr: [fast, nocache] }
        sram1: { base: 0x08000000, size: 512K,
                 attr: [dma_coherent, cacheable] }
        sram2: { base: 0x08080000, size: 128K,
                 attr: [dma_coherent, non_cacheable] }
      dma_channels: [DW0_CH0, DW0_CH1, DW0_CH2, DW0_CH3]
    links:                         # NEW section
      udp_scout:
        bind: "224.0.0.224:7446"
        driver: lwip_udp
        mtu_bytes: 1472            # NEW: link-layer MTU; drives RX
                                   #   reassembly sizing checks (§5.M)
                                   #   and TX fragmentation thresholds.
                                   #   Required when this link is bound
                                   #   to any FSM that uses Fragment
                                   #   codec events; optional otherwise
                                   #   (single-frame-only links).
        expected_p99_bytes: 1024   # NEW: optional; declared application
                                   #   p99 payload size on this link.
                                   #   Drives stage-copy rate warning.
                                   #   When absent, the build assumes
                                   #   p99 = mtu_bytes (no warning).
        burst_pps: 200             # NEW: declared peak inbound packets-
                                   #   per-second (worst-case burst, not
                                   #   average). Drives RX pool sizing
                                   #   check (§5.E burst absorption).
                                   #   For multicast: derive from worst
                                   #   peer count × per-peer rate.
        rx_dispatch: isr_to_pool   # NEW: isr_to_pool | worker_tick.
                                   #   isr_to_pool = RX-complete IRQ
                                   #   immediately re-arms next slot
                                   #   from descriptor ring (wire-rate
                                   #   absorption); worker_tick = RX
                                   #   only progresses on cooperative
                                   #   tick (simpler, lower wire-rate
                                   #   ceiling). Default: isr_to_pool
                                   #   on links with burst_pps declared.
        domain_attrs:
          trust_class: session_arming
          # untrusted | session_arming | established_session (§5.M)
          untrusted_source: false  # NEW: true if the link is exposed
                                   #   to a network the deployment
                                   #   does not control (public
                                   #   Internet, untrusted LAN).
                                   #   When true, stateless_accept
                                   #   below is REQUIRED, not optional.
        # --- Accept-side anti-flood (§5.M cross-ref, see also
        #     [docs/session-fsm.md: Trust-class interaction](session-fsm.md#26-trust-class-interaction-cross-ref-to-5m)
        #     / [Accept-side hardening detail](session-fsm.md#27-accept-side-hardening-detail). Required on links
        #     whose trust_class is `session_arming`. Forbidden on
        #     `untrusted` and `established_session` (those classes
        #     never instantiate Accepting.* and these fields would
        #     be dead config — hard error, not silent-ignore).
        session_arming_quota: 8    # NEW: max concurrent half-open
                                   #   Accepting.* slots per link.
                                   #   Bounded-collection capacity.
                                   #   MCU default: 8. AP default: 32.
                                   #   Build-invariant:
                                   #     session_arming_quota
                                   #     × max_handshake_time_s
                                   #     ≤ peer_table.capacity
                                   #   so a slow legitimate handshake
                                   #   can't be evicted by an attacker
                                   #   churning the quota.
        accept_rate_per_sec: 4     # NEW: token-bucket refill rate
                                   #   per (link, src_addr). One
                                   #   token = one Init→Accepting.*
                                   #   transition. Sized so a normal
                                   #   reconnect storm (e.g. AP
                                   #   reboot of N peers) succeeds
                                   #   within seconds, not so high
                                   #   that an attacker can saturate
                                   #   the half-open quota in one
                                   #   tick. Default: 4 (MCU) / 16 (AP).
        accept_rate_burst: 8       # NEW: token-bucket capacity per
                                   #   (link, src_addr). Default
                                   #   2 × accept_rate_per_sec.
        accept_rate_table_capacity: 32
                                   # NEW: capacity of the per-source
                                   #   token-bucket table. Default
                                   #   4 × session_arming_quota.
                                   #   When full, new sources fall
                                   #   through to a single shared
                                   #   bucket (degraded mode) and
                                   #   emit `session/accept-rate-
                                   #   table-saturated`.
        accepting_inactivity_timeout_ms: 1000
                                   # NEW: any Accepting.AwaitingInitSyn
                                   #   or Accepting.SentInitAck slot
                                   #   that has not progressed within
                                   #   this window is forcibly
                                   #   released. Bounds the worst-case
                                   #   "attacker opens TCP, never
                                   #   sends InitSyn" hold time.
        stateless_accept:          # NEW: optional. Recommended for
                                   #   trust_class: session_arming on
                                   #   links facing >0 untrusted
                                   #   peers. REQUIRED when
                                   #   untrusted_source: true.
          mode: cookie_hmac_sha256 #   HMAC-SHA-256 truncated to 32 bytes.
                                   #   Single MVP variant; alternative
                                   #   primitives (e.g. Blake2s for SoCs
                                   #   without SHA-256 acceleration) land
                                   #   as a new enum value when the need
                                   #   is concrete, not preemptively.
          cookie_lifetime_ms: 30000   # cookie validity window. After
                                   #   this, an echoing OpenSyn is
                                   #   silently rejected.
          key_rotation_s: 3600     # HMAC key rotation interval.
                                   #   Previous key honored for one
                                   #   additional cookie_lifetime
                                   #   window after rotation to
                                   #   avoid mid-handshake invalidation.
          hmac_extern: sce_hmac_sha256   # symbol from the intrinsics
                                   #   whitelist (§5.I) OR a target
                                   #   plugin entry. See G-SFM-5 in
                                   #   docs/session-fsm.md §8.1 and
                                   #   OQ-W15 for the open question
                                   #   on which side owns this.
          rng_extern: sce_random_fill    # symbol used to seed key
                                   #   material at FSM-instance
                                   #   startup. Must be a CSPRNG —
                                   #   plugin authors are responsible
                                   #   for the entropy source.
    pool_defaults:                 # NEW section: machine-wide pool policy
      stage_copy_policy: warn      # NEW: warn | error | forbid.
                                   #   - warn (default): §5.M / ARCHITECTURE
                                   #     §9.3 stage-copy-rate gate emits
                                   #     `reassembly/expected-fragmentation-
                                   #     rate-high` as a warning;
                                   #     `<sce:accept-stage-copy-rate>` on a
                                   #     link source suppresses per-link.
                                   #   - error: warning is promoted to a hard
                                   #     error. `<sce:accept-stage-copy-rate>`
                                   #     still allows per-link opt-out, with a
                                   #     justification reference required.
                                   #   - forbid: same as error, AND
                                   #     `<sce:accept-stage-copy-rate>` itself
                                   #     is rejected. For safety-critical
                                   #     deploys (medical / automotive /
                                   #     aerospace) where any stage-copy is
                                   #     unacceptable, period.
                                   #   See ARCHITECTURE §9.3 for the
                                   #   recommended setting per deploy class.
    buffer_pools:                  # NEW section
      scout_rx_pool:
        slot_count: 8
        slot_size: 256
        section: sram1
        alignment: 32              # must be >= platform.dcache_line_size when cache-policy=maintain
        dma_channel: DW0_CH3
        cache_policy: maintain     # NEW: maintain | non-cacheable | none
    extern_symbols:                # NEW section
      lib: sce_intrinsics_runtime_c
      target_plugin: configs/target_extensions.yaml  # optional; see §5.I
```

**Interaction with SCE Mesh.** Orthogonal. A machine may have SCE
Mesh `bindings:` (to call external transports like someip) AND
protocol-synthesis `links:` (raw byte endpoints). The two do not
interfere. Note that SCE Mesh codegen currently targets C++ only
(`sce-build/src/mesh/codegen.rs` dispatches `Language::Cpp` and
rejects other languages); adding Rust/C Mesh emitters is a separate
follow-up outside this RFC's scope.

**Schema-genericness policy.** Fields added to the deploy.yaml
schema by this RFC (`platform.os`, `mtu_bytes`, `expected_p99_bytes`,
`burst_pps`, `rx_dispatch`, `clock_freq_mhz`, `memcpy_cycles_per_byte`,
`vle_decode_cycles_per_byte`, `tlv_chain_per_entry_overhead_us`,
`pool_defaults.stage_copy_policy`,
`linker_flavor`, `has_speculative_prefetch`, etc.) are intentionally
**embedded-networking / hardware generic** — they apply equally to
zenoh, CoAP, MQTT-SN, lwM2M, and any other IP-based MCU transport
that this synthesis framework might target later. None encode
zenoh-specific wire-format knowledge.

Zenoh-specific knowledge (KeyExpr matching, Fragment FSM shape,
ZID, INIT/OPEN handshake) lives in the downstream `sources/` SCXML,
not in the SCE schema. The exception is the Zenoh-flavored
`trust_class: established_session` enum value (§5.M), which is
named after the Zenoh session FSM but is itself a generic
"authenticated-peer post-handshake" trust label that any
session-establishing transport would map onto.

This generic-schema policy is a deliberate response to the concern
that "SCE shouldn't accept domain-specific schema." That concern
applies to wire-format / protocol-mechanic fields, none of which
are added here. Existing precedent: SCE Mesh already has
`mesh/transport/zenoh.rs` (179 lines, integrating
`zenoh::Session`), so SCE has accepted zenoh-specific transport
*integration*; this RFC adds only generic embedded-networking
*deploy schema*, a strictly weaker dependency.

**Diagnostics:**
- `deploy/platform-class-missing`
- `deploy/link-driver-unknown`
- `deploy/memory-region-overlap`
- `deploy/scheduler-incompatible-with-worker-count`
- `deploy/has-dcache-missing` — MCU target without `platform.has_dcache` declared
- `deploy/dcache-line-size-missing` — `has_dcache: true` without `dcache_line_size`
- `deploy/worker-stack-budget-missing` — any `worker` kind declared but no budget set
- `deploy/core-count-zero` — `core_count < 1`; must be at least 1
- `deploy/worker-slot-budget-missing` — `scheduler.kind: cooperative` without
  `worker_slot_budget_us` declared
- `deploy/keepalive-jitter-budget-missing` — `scheduler.kind: cooperative` without
  `keepalive_jitter_budget_us` declared
- `worker/slot-budget-exceeded` — algorithm/procedure static (or measured) WCET
  exceeds `worker_slot_budget_us`; refactor (split into multiple slots, lower the
  bounded-loop cap, or pre-compute) before the build will succeed
- `worker/keepalive-jitter-violation` — sum of worst-case slot budgets queueable
  in one tick window exceeds `keepalive_jitter_budget_us`
- `deploy/multicore-without-target-plugin` — `core_count > 1` and at least one
  cross-core inbox is declared, but no `target_plugin` providing a hardware sync
  primitive (HSEM, spinlock, mailbox) is registered
- `deploy/link-mtu-missing-on-fragmenting-link` — link bound to a Fragment-
  emitting/consuming FSM but `mtu_bytes` not declared. Build cannot size
  reassembly pool slots without it
- `deploy/link-mtu-below-driver-floor` — declared `mtu_bytes` smaller than the
  driver's minimum payload (e.g. UDP/IPv6 56-byte floor); driver default would
  override silently
- `deploy/link-expected-p99-exceeds-mtu` — `expected_p99_bytes > mtu_bytes`
  AND no reassembly pool bound to the link; the p99 message would always
  fragment but no reassembly path exists
- `deploy/session-arming-quota-missing` — link declares
  `trust_class: session_arming` but no `session_arming_quota`. Hard
  error; without a cap an attacker can fill every Accepting.* slot.
- `deploy/accept-rate-config-missing` — `trust_class: session_arming`
  link missing `accept_rate_per_sec` or `accept_rate_burst`. Hard error.
- `deploy/session-arming-fields-on-non-arming-link` —
  `session_arming_quota` / `accept_rate_*` / `stateless_accept`
  declared on a `trust_class: untrusted` or `established_session`
  link where `Accepting.*` is never instantiated. Hard error
  (dead config; suggests author confusion about which link is the
  listener).
- `deploy/session-arming-quota-vs-peer-table-invariant-violated` —
  `session_arming_quota × max_handshake_time_s > peer_table.capacity`.
  A slow legitimate handshake can be evicted under attack. Hard error.
- `deploy/stateless-accept-required-on-untrusted-source` — link with
  `domain_attrs.untrusted_source: true` but no `stateless_accept`
  block. Hard error.
- `deploy/stateless-accept-extern-not-whitelisted` — `hmac_extern`
  or `rng_extern` symbol not present in `sce_intrinsics_runtime`
  core whitelist AND not declared in any loaded `target_plugin`.
  Hard error.
- `deploy/stateless-accept-key-rotation-shorter-than-lifetime` —
  `key_rotation_s × 1000 ≤ 2 × cookie_lifetime_ms`. The previous-key
  honor window cannot bridge a rotation, so handshakes near
  rotation boundaries get spurious cookie rejection. Hard error.
- `session/half-open-cap-exceeded` — runtime informational. Counter
  per (link, recent_window). Spike indicates real attack OR a
  legitimate reconnect storm exceeding `session_arming_quota`.
- `session/accept-rate-exceeded` — runtime informational. Counter
  per (link, src_addr). Spike from many src_addrs indicates spoofed-
  source attack; spike from one src_addr indicates a buggy peer.
- `session/accept-rate-table-saturated` — runtime informational. The
  per-source token-bucket table is full and the link is in degraded
  (single shared bucket) mode. Bumping `accept_rate_table_capacity`
  resolves; or accept that a randomized-source attack triggered
  the fall-through (which is the design-intended behavior).
- `session/cookie-rejected` — runtime informational. Counter per
  (link, reason ∈ {hmac_mismatch, expired, key_unknown}). Steady
  hmac_mismatch traffic indicates active spoofing; expired traffic
  indicates an unhealthy peer or one whose clock skewed.
- `deploy/link-burst-absorption-insufficient` — `burst_pps × 1s` of
  worst-case inbound exceeds the RX pool's drain rate within one
  cooperative tick window (`slot_count × ticks_per_second / burst_pps`
  < 1.0 with safety factor 2.0). Pool will deplete during burst and
  drop packets. Author resolution: raise `slot_count`, lower
  `tick_period_us`, or switch `rx_dispatch: isr_to_pool` if currently
  `worker_tick`
- `deploy/link-rx-dispatch-worker-tick-on-high-burst` —
  `rx_dispatch: worker_tick` declared but `burst_pps × tick_period_us
  > slot_count` (one tick of arrivals overruns the pool). Hard error
  unless author justifies via `<sce:accept-burst-drop-rate>` on the
  link source
- `deploy/link-burst-pps-missing-on-isr-dispatch` — `rx_dispatch:
  isr_to_pool` requires `burst_pps` to size the descriptor ring and
  validate ISR fast-path stack; declaration missing
- `pool/stage-copy-policy-error` — `pool_defaults.stage_copy_policy:
  error` AND the §5.M / ARCHITECTURE §9.3 stage-copy-rate gate fires.
  The warning that would have surfaced as
  `reassembly/expected-fragmentation-rate-high` is promoted to a hard
  error. Author resolution: raise `slot_size`, lower
  `expected_p99_bytes`, or add `<sce:accept-stage-copy-rate>` on the
  affected link source with a justification reference. Last option
  is unavailable under `forbid`
- `pool/stage-copy-accept-rejected-under-forbid` —
  `pool_defaults.stage_copy_policy: forbid` AND a link source carries
  `<sce:accept-stage-copy-rate>`. The opt-out is rejected outright;
  only the structural fixes (raise `slot_size` or lower
  `expected_p99_bytes`) are accepted under `forbid`. Hard error
- `deploy/stage-copy-policy-unknown` — `pool_defaults.stage_copy_policy`
  declared with a value other than `warn` / `error` / `forbid`. Hard
  error (typo guard)
- `deploy/platform-os-missing` — `platform.os` not declared. Hard
  error from review #13 onward. Migration: existing deploys without
  `os` are auto-set to `bare_metal` (when `class: mcu`) or `linux`
  (when `class: ap`) for one release cycle with a deprecation
  warning, then become hard errors
- `deploy/platform-os-class-mismatch` — incompatible `class` × `os`
  combination. `class: mcu` with `os: linux` (or any non-`bare_metal`
  / non-`rtos`), or `class: ap` with `os: bare_metal`. Hard error
- `deploy/platform-os-not-implemented-in-current-phase` — declared
  `os` is namespace-reserved but not yet implemented at the deploy's
  target phase. Currently fires for `os: qnx | macos | freebsd |
  windows | rtos` until the corresponding Phase D+ implementation
  lands. Hard error pre-implementation; downgrades to silent
  acceptance once the runtime crate ships
- `deploy/runtime-crate-mismatch-with-os` — declared
  `runtime_crate:` (or default per `os`) does not match the OS-axis
  contract (e.g. `sce_link_runtime_tokio` declared with `os: qnx`).
  Hard error. The naming convention `sce_link_runtime_<os>` is
  formalized in §5.J

### 5.L New kind: `bounded-collection`

**Problem.** zenoh-pico parity requires runtime subscription
declare/undeclare, runtime queryable declare, and matching over a
table that grows and shrinks at runtime — but MCU forbids heap.
The existing kinds do not express "a typed container whose capacity
is declared at build time but whose occupancy varies at runtime".

**Proposal.** New `ForgeKind::BoundedCollection`.

```xml
<scxml sce:kind="bounded-collection" name="local_sub_table" version="1.0">
  <sce:element-type>SubscriptionEntry</sce:element-type>
  <sce:capacity source="deploy"
                key="machines.<name>.limits.local_subscriptions"/>
  <sce:index-by field="key_expr_id"/>    <!-- optional; enables O(log N) lookup -->
  <sce:on-overflow>diagnostic-event</sce:on-overflow>
  <!-- diagnostic-event | reject | oldest-wins -->
  <sce:ordering>insertion</sce:ordering>
  <!-- insertion | sorted-by(index-by) -->
  <sce:concurrency>single-writer</sce:concurrency>
  <!-- single-writer | multi-writer (multi-writer requires
       §5.I acquire/release atomics on head/tail) -->
</scxml>
```

**Element type resolution.** `<sce:element-type>` references a
codec-kind struct (§5.B) OR a procedure-kind state record. Each of
the six language backends emits a typed fixed-capacity container
per the §5.J.5 emitter table:

- Rust: `heapless::Vec<T, N>` under no_std, `Vec<T>` capped at N
  under std; codegen-checked push/pop wrappers.
- C11: `struct { T slots[N]; uint32_t generation[N];
  uint32_t bitmap[(N+31)/32]; uint32_t count; }` with a generated
  `_insert/_remove/_find_by_index` API; `_Static_assert(N <= UINT16_MAX)`.
- Cpp: `std::array<T, N>` + `std::bitset<N>` occupancy + a
  generation-counter array; iterators expose only occupied slots.
- Kotlin: `Array<T?>(N)` + `BooleanArray(N)` occupancy + `var len: Int`.
- Go: `[N]T` fixed array + `[N]bool` occupancy mask + `len int`.
- Python: `list` of length N initialized to `None` + `bytearray(N)`
  mask + `len` counter.

The `<sce:capacity>` source (deploy or const) is resolved at
codegen time and lowered to a per-language compile-time constant
(see §5.J.5). Cross-backend parity (§6.2.6) verifies that the same
insertion sequence produces the same iterator order on every
backend.

**IR additions** (`forge/model.rs`):

```rust
pub struct BoundedCollectionModel {
    pub element_type: SceType,        // references another kind
    pub capacity: CapacitySource,
    pub index_by: Option<String>,     // field name in element
    pub on_overflow: OverflowPolicy,
    pub ordering: CollectionOrdering,
    pub concurrency: ConcurrencyMode,
}
pub enum CapacitySource {
    DeployKey(String),                // "machines.X.limits.Y"
    CompileConst(u32),                // e.g. <sce:capacity const="8"/>
}
pub enum OverflowPolicy { DiagnosticEvent, Reject, OldestWins }
pub enum CollectionOrdering { Insertion, SortedByIndex }
pub enum ConcurrencyMode { SingleWriter, MultiWriter }
```

**Operations contract.** Each bounded-collection emits:

```
insert(&mut self, elem: T) -> Result<Handle, OverflowError>
remove(&mut self, handle: Handle) -> bool
get(&self, handle: Handle) -> Option<&T>
find_by_index(&self, key: IndexKey) -> Option<Handle>    // if index-by set
iter(&self) -> impl Iterator<Item = &T>
len(&self) -> usize
capacity() -> usize   // compile-time constant
```

`Handle` is a newtype over `u32` carrying slot index + generation
counter (to detect use-after-remove).

**MCU codegen shape (C):**

```c
typedef struct {
    SubscriptionEntry slots[LOCAL_SUB_TABLE_CAPACITY];
    uint32_t generation[LOCAL_SUB_TABLE_CAPACITY];
    uint32_t bitmap[BITMAP_WORDS];
    uint32_t count;
    /* index-by tree, if declared */
} local_sub_table_t;

local_sub_table_handle_t local_sub_table_insert(
    local_sub_table_t* c, const SubscriptionEntry* elem);
```

`LOCAL_SUB_TABLE_CAPACITY` is a `#define` sourced from deploy.yaml
at codegen time, placed in `memory_map.h`.

**Interaction with algorithm kind.** `find_by_index` and iteration
are callable from `algorithm` bodies via `sce:call` (§5.A). Runtime
KeyExpr matching is then an algorithm that iterates the
bounded-collection and calls a `keyexpr_intersect` inner algorithm
on each entry. Bounded loop count = `capacity()`, known at build
time, satisfying `max-iter` requirement.

**Diagnostics:**
- `collection/capacity-unresolved` — deploy key missing from deploy.yaml
- `collection/element-type-not-a-kind` — element-type is a primitive, not a kind reference
- `collection/index-by-field-missing` — index field not in element struct
- `collection/multi-writer-without-atomics` — multi-writer mode without §5.I atomics imported
- `collection/ordering-sorted-requires-index-by`
- `collection/overflow-policy-oldest-wins-requires-ordering-insertion`

### 5.M Fragment and reassembly kinds

**Backend coverage (MCU-class kind, inherits from §5.E).** The
reassembly variant introduced here extends §5.E `buffer-pool`, so
its backend coverage is identical to §5.E: emits only on
`(rust, *)` and `(c11, bare_metal)`. Authoring a reassembly-variant
buffer-pool against a target backend in `{cpp, kotlin, go, python}`
raises `codegen/mcu-class-kind-on-non-mcu-language` (hard error).
The fragmentation-analysis build-time invariants (`max-fragments`
sufficiency, stage-copy rate, slot-size recommendation, stage-copy
WCET) and the per-peer-quota DoS hardening apply to those two
backends only.

**Problem.** zenoh-pico supports fragmented messages on both TX
(large PUT split into Frame/First/Continue/Final) and RX (reassembly
of incoming fragments into a complete message for delivery). This
must be expressible without a heap; the fragments must land in
pre-declared pool slots and the reassembly state must be bounded.

**Proposal.** Two additions, no new top-level kind:

1. **Reassembly-pool variant of buffer-pool (§5.E)**

```xml
<scxml sce:kind="buffer-pool" name="rx_reassembly_pool" version="1.0">
  <sce:variant>reassembly</sce:variant>
  <sce:slot-count>4</sce:slot-count>          <!-- 4 concurrent in-flight reassemblies -->
  <sce:slot-size>4096</sce:slot-size>         <!-- max reassembled message size -->
  <sce:section>sram1</sce:section>
  <sce:alignment>32</sce:alignment>
  <sce:cache-policy>maintain</sce:cache-policy>
  <sce:max-fragments-per-message>16</sce:max-fragments-per-message>
  <sce:reassembly-timeout-ms>500</sce:reassembly-timeout-ms>
  <sce:per-peer-quota>2</sce:per-peer-quota>  <!-- max concurrent reassemblies per peer -->
</scxml>
```

Reassembly pool differs from regular RX pool: each slot tracks
fragment IDs seen, completion state, a timeout, AND the originating
peer's identity. Codegen emits a fragment-index bitmap per slot, a
per-slot deadline field, and a per-slot peer-id field used by the
quota check.

**Trust class requirement (UDP spoofing hardening).** Per-peer
quota only defends against legitimate-but-misbehaving peers if
"peer" is **non-spoofable**. UDP source IP / port is trivially
spoofed, so an attacker sending `Fragment.First` from N random
fake source IPs creates N "new peers" and exhausts the per-peer
quota space (`peer_table.capacity` slots filled with attacker-
created entries) regardless of quota value.

Reassembly therefore requires a **trusted peer identifier**, not
a wire-source identifier. Zenoh provides this naturally: a peer
that has completed the INIT/OPEN session handshake holds a 16-byte
ZID (Zenoh Identifier) that the handshake binds to the link's
source address. Pre-handshake traffic (Scout/Hello, INIT/OPEN
themselves) is small and cannot legally be fragmented in the
wire format — only post-Established Frame messages can be.

The link source declares its **trust class** to encode this:

```yaml
links:
  udp_data:
    bind: "..."
    domain_attrs:
      mtu_bytes: 1472
      trust_class: established_session
      # untrusted | session_arming | established_session
```

| Trust class | Allowed traffic | Reassembly pool binding |
|---|---|---|
| `untrusted` | Scout / Hello only (small, never fragmented) | **forbidden** — hard error if a reassembly pool is bound to this link |
| `session_arming` | INIT / OPEN handshake messages (small) | **forbidden** |
| `established_session` | Frame / data plane traffic (may fragment) | **required** for reassembly to be enabled at all |

Per-peer quota's "peer" identifier on `established_session`
links is the **ZID**, not the wire source address. Spoofing now
requires forging a complete handshake, raising the cost of the
attack from "send N packets" to "complete N handshakes that
each pass the listener's anti-flood gate."

The "anti-flood gate" referenced above is the listener-side
hardening on `trust_class: session_arming` links, specified in
[`docs/session-fsm.md`: Accept-side hardening detail](session-fsm.md#27-accept-side-hardening-detail) and configured per link via the
`session_arming_quota` / `accept_rate_per_sec` /
`accept_rate_burst` / `stateless_accept` fields in §5.K. Three
caps act in series:

1. **Half-open capacity** — `session_arming_quota` bounds the
   number of concurrent `Accepting.*` instances; an attacker
   cannot allocate unbounded handshake-FSM state.
2. **Per-source token bucket** — `accept_rate_per_sec` /
   `accept_rate_burst` rate-limit `Init → Accepting.*`
   transitions per (link, src_addr); a single source cannot
   monopolize the quota.
3. **Stateless accept (optional, recommended)** — `cookie_hmac_sha256`
   defers FSM-state allocation until the OpenSyn echoes a
   validated HMAC cookie, making per-packet unauthenticated work
   O(1) regardless of source-address fan-out.

With (1)+(2)+(3), the cost-to-attacker of "complete N handshakes
to consume N reassembly quota slots" scales as N round-trips from
N reachable addresses, which is qualitatively different from the
unbounded one-way packet flood the §5.M reassembly pool would
otherwise face.

Without (1)+(2)+(3) — e.g. on a `trust_class: session_arming`
link where the deploy author omitted the quota fields — the build
refuses to emit (`deploy/session-arming-quota-missing` and
`deploy/accept-rate-config-missing`, §5.K), so the assertion above
is mechanically backed by the build gate, not by a textual promise.

**Listener-link trust-class lifecycle.** The trust-class table above
is keyed on the *link instance*'s `trust_class` value, but a listener
link declares `trust_class: session_arming` (which gates `Accepting.*`
hardening per §5.K and [`docs/session-fsm.md`: Accept-side hardening detail](session-fsm.md#27-accept-side-hardening-detail)) AND must carry
post-handshake Fragment traffic from any peer whose handshake has
completed — traffic that semantically belongs to `established_session`.
Without further structure the build-time gate
`reassembly/untrusted-link-binding` would either reject the listener
(forbidding the deploy entirely) or become honor-system at compile,
requiring a runtime check at every `Fragment.First`.

To preserve the static semantics, codegen models the listener as
**two logical link-instances** sharing one physical socket:

- The **`session_arming` instance** hosts `Accepting.*` (the three-cap
  anti-flood) and consumes pre-handshake INIT/OPEN traffic. It cannot
  bind to a reassembly pool — the existing
  `reassembly/untrusted-link-binding` hard error stands unchanged.
- The **`established_session` sibling instance** is emitted at codegen
  time alongside the listener. It hosts post-handshake Frame/Fragment
  traffic and is the only side eligible for reassembly-pool binding.
  The unicast session FSM's `Established` entry action
  (`docs/session-fsm.md` §2.3) hands the peer's traffic ownership to
  this sibling at the moment the handshake completes.

The *physical* socket (declared once by `links.<name>.bind` in
deploy.yaml — e.g. `deploy/mcu_target.yaml` `udp_session.bind`)
remains a single endpoint; only the *logical* SCXML link-instances
are two. **deploy.yaml schema is unchanged** — the split is
automatic, not author-declared. Authors write one `links.<name>`
block per listener; codegen emits the sibling.

The build-time gate `reassembly/untrusted-link-binding` is therefore
**link-instance-scoped, not socket-scoped**, and remains a fully
static check. A reassembly-pool binding authored against a listener
kind resolves to the `established_session` sibling instance at
codegen time and passes the gate; a `session_arming` instance that
is not a listener (no sibling exists for it) still fails the gate.
Two new diagnostics back this:

- `link/listener-link-not-paired-with-established-sibling` — codegen
  self-check (template regression guard) that every `session_arming`
  listener instance has its `established_session` sibling emitted.
  Hard error.
- `reassembly/binding-on-unpaired-listener` — author-facing hard
  error for a reassembly-pool binding to a `session_arming` instance
  that is not a listener (no sibling will be emitted, so the binding
  has no valid resolution target).

The link-instance split is a §5.M *semantic* clarification; the
codegen *mechanics* (sibling emission, RX dispatch by peer state on
the shared socket) are specified in §5.C "Codegen contract" with a
back-reference here. Runtime crate header / trait surfaces still see
three trust classes and emit one variant per instance
(`docs/runtime-crate-lwip.md` §4 / `docs/runtime-crate-tokio.md`
§2.4); the listener simply contributes two instance variants instead
of one. G-RFM-2 (`docs/reassembly-fsm.md` §8.1) and OQ-W22 close on
this resolution.

This makes the per-peer quota DoS argument robust:

**Per-peer quota (DoS hardening).** Timeout alone does not stop a
malicious peer from holding the entire pool: an attacker sending
`slot_count` distinct `Fragment.First` packets without follow-ups
keeps the pool full until the timeout fires, then refills it as
soon as slots free up. Any sustained rate above
`slot_count / reassembly_timeout` (e.g. 8 pkt/sec for `slot_count=4`,
`timeout=500ms`) is sufficient — trivially achievable for a
remote attacker without the trust-class restriction above.

`<sce:per-peer-quota>` caps the number of in-flight reassembly
slots a single peer may hold:

- Build-time invariant: `peer_table.capacity × per-peer-quota
  ≥ slot_count`. This guarantees the pool is at least theoretically
  partitionable across peers without false rejection of legitimate
  traffic.
- Runtime: when `Fragment.First` arrives from peer X, the FSM
  scans in-flight slots; if peer X already holds `per-peer-quota`
  slots, the new `Fragment.First` is dropped (peer X only — other
  peers continue normally). The `on-overflow` policy (`reject` /
  `oldest-wins` / `diagnostic-event`) applies *only when even the
  per-peer quota check would have allowed allocation* — i.e. quota
  rejection takes precedence over global pool pressure.
- Quota enforcement is O(slot_count) per `Fragment.First`, bounded
  loop, fits the `algorithm` kind (§5.A `mode="static"` WCET).

The default value of `per-peer-quota` is `ceil(slot_count /
peer_table.capacity)` — the largest value that still satisfies
the build-time invariant. Authors raise it for deployments where
a small number of peers send large fragmented messages
concurrently; lower it for adversarial environments.

2. **Fragment / reassembly FSM authored in SCXML** (no new kind)

Downstream authors a `statechart` that consumes Fragment codec
events and drives reassembly:

```
Idle
  ├─on Fragment.First → Assembling (allocate slot, start timer)
Assembling
  ├─on Fragment.Continue (matching slot) → Assembling (append)
  ├─on Fragment.Final (matching slot) → Complete (emit full message)
  ├─on timeout → FreeSlot (drop, emit diagnostic)
  └─on slot-pool-full → FreeSlot (reject earliest, emit diagnostic)
```

The FSM uses `bounded-collection` (§5.L) as its in-flight slot
table and `reassembly-pool` (§5.E variant) as its byte storage.
No new kind required beyond these two primitives.

**TX fragmentation** is similarly author-level: a `procedure` or
`algorithm` that walks the outbound payload, carves it into
slot-sized chunks, and emits Fragment.First / Continue / Final
codec events. No new kind required.

**Build-time fragmentation analysis.** When a link declares
`mtu_bytes` and (optionally) `expected_p99_bytes` (§5.K), and a
reassembly pool is bound to that link, the build performs three
quantitative checks:

1. **Reassembly capacity check** —
   `slot_size ≥ max-fragments-per-message × mtu_bytes` must hold,
   else worst-case message cannot complete reassembly. This is
   structural; failure is a hard error
   (`reassembly/max-fragments-insufficient-for-mtu`).
2. **Stage-copy rate warning** — when `expected_p99_bytes` is
   declared and exceeds the regular RX pool's `slot_size` (not the
   reassembly pool), the build computes the expected stage-copy
   rate. Default warning threshold is 25%: if
   `(expected_p99_bytes - rx_pool.slot_size) / expected_p99_bytes
   > 0.25`, emit `reassembly/expected-fragmentation-rate-high`.
   Authors silence this by raising RX pool `slot_size`, lowering
   `expected_p99_bytes` (with justification), or accepting the
   warning via `<sce:accept-stage-copy-rate>` on the link source.
   Severity is upgradable per machine via §5.K
   `pool_defaults.stage_copy_policy: error | forbid` —
   `error` promotes to `pool/stage-copy-policy-error`,
   `forbid` additionally rejects the per-link opt-out
   (`pool/stage-copy-accept-rejected-under-forbid`).
3. **Reassembly slot sizing recommendation** — the build emits an
   informational note (`reassembly/slot-size-recommendation`) with
   the computed minimum slot_size to absorb p99 without
   fragmentation: `slot_size_recommended = ceil(expected_p99_bytes
   / mtu_bytes) × mtu_bytes`. Not a warning; surfaced only on
   `sce-codegen build --verbose`.
4. **Stage-copy WCET vs slot budget** — when an oversized RX
   triggers stage copy (ARCHITECTURE §9.3), the copy itself
   executes inside a worker slot. The build computes:
   `stage_copy_wcet_us = expected_p99_bytes ×
   platform.memcpy_cycles_per_byte / platform.clock_freq_mhz`
   and compares against `scheduler.worker_slot_budget_us`. If the
   stage copy alone would blow the slot budget — common for large
   p99 on slow cores (e.g. 16 KB on Cortex-M0+ ≈ 16384 × 4 / 48
   ≈ 1.4 ms, blowing a 200 µs budget by 7×) — the build emits
   `reassembly/stage-copy-wcet-exceeds-slot-budget` as a hard
   error. Author resolutions:
   - Raise `worker_slot_budget_us` (and re-validate every other
     algorithm against the new bound)
   - Lower `expected_p99_bytes` so stage copy is never invoked at
     that size
   - Raise `rx_pool.slot_size` to absorb p99 without stage copy
   - Switch to chunked stage copy (Phase D+; not in MVP — would
     require splitting the copy across multiple ticks with a
     resumable cursor, complicating the FSM)

These checks turn "size to p99 keeps stage path rare" from prose
guidance (ARCHITECTURE §9.3) into mechanical build feedback, so
operators see fragmentation cost at deploy authoring time rather
than at runtime via `link/stage-copy-invoked`.

**Diagnostics:**
- `mem/reassembly-pool-variant-missing-max-fragments`
- `mem/reassembly-pool-variant-missing-timeout`
- `mem/reassembly-slot-size-below-declared-mtu`
- `reassembly/max-fragments-insufficient-for-mtu` — `slot_size <
  max-fragments-per-message × mtu_bytes`; worst-case message
  cannot reassemble within declared bounds (hard error)
- `reassembly/expected-fragmentation-rate-high` — RX pool slot_size
  vs link `expected_p99_bytes` implies > 25% stage-copy rate
  (warning; suppressible via `<sce:accept-stage-copy-rate>`)
- `reassembly/slot-size-recommendation` — informational note with
  the minimum recommended slot size (verbose-only)
- `reassembly/per-peer-quota-build-invariant-violated` —
  `peer_table.capacity × per-peer-quota < slot_count`; the pool
  cannot be partitioned across the maximum peer set without false
  rejection. Hard error
- `reassembly/per-peer-quota-exhausted` — runtime informational
  diagnostic: peer X attempted to allocate beyond its quota.
  Per-peer-bounded; does NOT escalate to global pool pressure.
  Sustained occurrence on a single peer is the signature of a
  fragment-flood attack
- `reassembly/untrusted-link-binding` — reassembly pool bound to a
  link with `trust_class: untrusted` or `session_arming`. **Hard
  error** — fragmentation on these links is forbidden; only
  `established_session` links may carry fragmented traffic. This
  defends against UDP source-IP spoofing exhausting the per-peer
  quota space
- `reassembly/trust-class-missing-on-fragmenting-link` — link bound
  to a reassembly pool but `domain_attrs.trust_class` not declared.
  Hard error; build cannot decide whether the binding is safe.
  Author resolution: declare `trust_class: established_session`
  for data-plane links, or remove the reassembly pool binding for
  control-plane links
- `reassembly/peer-id-not-zid-on-established-session` — internal
  codegen invariant: per-peer quota check on an
  `established_session` link must use ZID (handshake-derived) as
  the peer key, not the wire source address. Codegen guard
  against template regression that would silently fall back to
  spoofable wire ID
- `reassembly/binding-on-unpaired-listener` — a reassembly-pool
  binding has resolved to a `session_arming` link instance whose
  paired `established_session` sibling does not exist. Hard error.
  In well-formed codegen this is unreachable (the listener-link
  sibling emission contract in §5.C guarantees pairing); the
  diagnostic guards SCXML that explicitly targets the
  `session_arming` half (bypassing the auto-resolution) and any
  future schema evolution that introduces non-listener
  `session_arming` instances. Distinct from
  `reassembly/untrusted-link-binding` (which rejects bindings to
  `untrusted` and to standalone `session_arming` non-listeners)
  and from `link/listener-link-not-paired-with-established-sibling`
  (which is the §5.C-side codegen self-check)
- `reassembly/stage-copy-wcet-exceeds-slot-budget` —
  `expected_p99_bytes × memcpy_cycles_per_byte / clock_freq_mhz
  > worker_slot_budget_us`; the implicit memcpy in the stage-copy
  path alone blows the cooperative slot, starving Keepalive and
  other parallel-region timers (ARCHITECTURE §9.3 + §3.4)
- `reassembly/timeout-fired` — informational runtime diagnostic
  emitted on the slot's `Receiving → TimedOut` edge
  ([`docs/reassembly-fsm.md`: Receiving → TimedOut](reassembly-fsm.md#245-receiving--timedout-timer)). One occurrence per timed-out chain;
  sustained per-peer occurrences are the signature of a
  fragment-flood-then-stall attack
- `reassembly/aborted` — informational runtime diagnostic with a
  reason code on the slot's `Receiving → Aborted` edge. Reason
  codes: `incomplete-final` / `evicted` / `codec-error` /
  `reliable-out-of-order` / `unmatched-key` / `out-of-bounds-
  index` / `duplicate-index` (`docs/reassembly-fsm.md`:
  [out-of-order continue](reassembly-fsm.md#244-receiving--aborted-out-of-order-continue)
  / [Router-driven eviction](reassembly-fsm.md#246-receiving--aborted-router-driven-eviction)
  / [codec error](reassembly-fsm.md#247-receiving--aborted-codec-error))
- `reassembly/unmatched-continue` — Router-side runtime
  diagnostic: `Fragment.Continue` arrived with no matching open
  chain. One per occurrence; sustained occurrences indicate
  reordering past the chain timeout or peer misbehavior
- `reassembly/unmatched-final` — Router-side runtime diagnostic:
  `Fragment.Final` arrived with no matching open chain. Same
  shape as unmatched-continue
- `reassembly/slot-pool-full` — Router-side runtime diagnostic:
  `Fragment.First` arrived but the pool is exhausted AND
  `on_overflow != oldest-wins`. Carries the `on_overflow` policy
  reason in the diagnostic data so operators see whether the
  drop was `reject` or `diagnostic-event`
- `reassembly/message-complete` — informational runtime
  diagnostic emitted on `Receiving → Complete` carrying assembly
  duration and fragment count. Off by default; opt-in via
  `<sce:emit-message-complete-diagnostic>` for observability
  builds

### 5.N Multi-link concurrency

**Problem.** zenoh-pico can simultaneously listen on UDP multicast
for scouts, maintain a TCP session with an established peer, and
optionally send over Serial fallback. RFC §5.C currently describes
one link kind at a time; codegen behavior with multiple concurrent
links on one machine is unspecified.

**Proposal.** Clarification, not a new kind.

**Codegen contract (AP).** Each declared link in deploy.yaml's
`machines.<name>.links` becomes one `tokio::spawn` task with its
own driver adapter. Cross-link event routing uses typed mpsc
channels; the generated `LinkBus` struct aggregates sender handles
for each link and is passed to FSM workers.

**Codegen contract (MCU).** Each link becomes one cooperative
scheduler slot. The scheduler's tick loop polls each link's
`driver.poll()` in a fixed round-robin, bounded by deploy.yaml
`scheduler.tick_period_us`. Link-to-FSM event queues are per-link
`heapless::spsc` (Rust) / ring buffers (C); a reader worker
aggregates.

**Starvation guarantee.** Scheduler emits a round-robin visit-order
invariant: no link can starve another. `deploy.yaml`
`scheduler.per-link-budget-us` (optional) caps per-link work per
tick.

**Diagnostics:**
- `link/concurrent-count-exceeds-scheduler-slots` — more links than the cooperative scheduler can accommodate (MCU)
- `link/per-link-budget-exceeds-tick-period`
- `link/inbound-event-queue-unsized` — link declared but downstream FSM inbox depth unset

### 5.O Generated-source traceability

**Problem.** When generated MCU C hard-faults or generated AP Rust
panics in production, the developer must navigate from
`{PC address, symbol name}` back to the authoring SCXML's specific
state, transition, or `<sce:body>` line that emitted the offending
code. Today this requires reverse-engineering the codegen template
by hand: there is no mechanical link from emitted symbol to its
SCXML origin. For a zenoh-pico-parity session stack that generates
~30k LOC of C across ~15 SCXML files, this is the single largest
debugging tax in the production lifecycle, and it scales linearly
with codebase size.

This is a **codegen design-time** concern — the templates that emit
C/Rust must carry source attribution from the start. Adding it
after Phase A backend templates ship would require revising every
emitter; this section pins the contract before §5.J.1's C11 backend
lands. The parallel concern on the SCE side is that the IR must
preserve `(file_id, line, column)` provenance through XInclude
expansion, `sce:template` composition, and any future parametric-
kind instantiation (§5.G), so emitters have something to attribute
to.

**Proposal.** Three layers, each independently consumable:

**(a) Per-line source attribution embedded in generated code.**

C backend emits `#line` directives above each emitted function and
major construct, pointing at the canonical SCXML location:

```c
/* generated from sources/session/session_unicast.scxml */
#line 142 "sources/session/session_unicast.scxml"
static sce_result_t session_unicast__Opening__on_init_ack(
    session_unicast_t* self, const InitAck* msg) {
#line 145 "sources/session/session_unicast.scxml"
    /* <transition target="WaitingOpenAck"> body */
    ...
}
```

`#line` flows into DWARF; standard tools (`addr2line`,
`arm-none-eabi-gdb`) then produce SCXML-relative locations from PC
addresses with no SCE-specific decoder needed at the line-resolution
layer.

Rust has no `#line` directive (the unstable `proc_macro_span`
feature does not affect emitted source-line debug info in the way
C's `#line` does). Rust backend instead emits the mapping as a
`#[doc]` attribute, which survives `cargo build --release` and is
retrievable from `cargo doc --output-format json`:

```rust
#[doc = "SCE-MAP: sources/session/session_unicast.scxml:142 :: Opening :: on_init_ack"]
fn opening__on_init_ack(
    self: &mut SessionUnicast, msg: &InitAck,
) -> Result<(), CodecError> {
    // SCE-MAP: sources/session/session_unicast.scxml:145
    // <transition target="WaitingOpenAck"> body
    ...
}
```

The `// SCE-MAP:` line comment serves as a redundant fallback for
in-line locations where attribute syntax is unwieldy (inside
function bodies). Rust panic traces still point at the generated
`.rs` line, which `addr2sce` (below) re-attributes via the
sourcemap.

OQ-W16 tracks the final mechanism choice for the Rust side
(`#[doc]` vs custom `#[sce_map(...)]` attribute vs comment-only).
Until resolved, codegen MUST emit BOTH `#[doc]` and `//` comment so
neither survival path is closed.

**Cpp** uses the same `#line` directive as C11 — it is a
preprocessor directive predating C99, supported identically in
Cpp; flows into DWARF the same way:

```cpp
#line 142 "sources/session/session_unicast.scxml"
sce_result_t session_unicast::Opening::on_init_ack(const InitAck& msg) {
    ...
}
```

**Kotlin**, **Go**, and **Python** carry per-symbol traceability
as language-idiomatic comments, since none have a `#line`-style
preprocessor directive available to all build pipelines:

- Kotlin: header `// SCE-MAP: <scxml_file>:<line> :: <state> :: <artifact>`
  comment immediately above each emitted top-level declaration.
- Go: a Go `//line` directive (recognized by the Go toolchain for
  `go vet` / panic stack traces) above each function:
  `//line sources/session/session_unicast.scxml:142`.
- Python: `# SCE-MAP: <scxml_file>:<line> :: <state> :: <artifact>`
  comment above each emitted `def` / `class`.

For all six backends the **structured sourcemap JSON sidecar
(`out/{language}/sce_sourcemap.json`) is byte-identical**: the
mapping is a property of the SCXML source and the host symbol-naming
convention, not the per-language source-line marker shape. §6.2.6
parity testing checks both that each backend emits its required
in-source marker AND that all six sourcemap JSONs are byte-equal
(modulo the per-language file-extension key). Authors and tools can
therefore use the JSON sidecar as the single source of truth for
PC/symbol → SCXML resolution; the in-source markers are a
convenience for direct code reading and debugger integration on
each language's native tooling (`addr2line`, Rust panic traces,
Go panic stacks, Python tracebacks, IntelliJ Kotlin gutter, etc.).

**(b) Symbol naming convention.**

Generated identifiers follow a canonical pattern that encodes their
SCXML origin:

```
<machine>__<state_path>__<artifact>
```

Where:
- `<machine>` is the top-level SCXML's `name=` attribute (e.g.
  `session_unicast`).
- `<state_path>` is the dotted path from machine root to the owning
  state, with the path delimiter from OQ-W16 (initial proposal:
  `__`, with `_u_` as the escape for underscores inside SCXML state
  names). Example: state `Established.Keepalive` → `Established__Keepalive`.
- `<artifact>` is one of `entry` / `exit` / `on_<event>` /
  `guard_<n>` / `action_<n>` / `body` for state-level constructs,
  or the algorithm/codec/collection name for kind-level constructs.

Examples:
- `session_unicast__Opening__on_init_ack` — transition handler
- `session_unicast__Established__Keepalive__entry` — entry action of
  the `Established.Keepalive` parallel region
- `crc16_ccitt__body` — algorithm kind body
- `local_sub_table__find_by_index` — bounded-collection operation

Cross-machine collisions (two different SCXML files declaring the
same machine `name=`) are caught by the existing
`forge/duplicate-kind-name` diagnostic. Within-machine collisions
(XInclude composition or `sce:template` expansion producing two
states with identical canonical paths) are the new failure mode →
diagnostic below.

The convention is mechanical, so a developer reading a coredump
symbol name reconstructs the SCXML origin without consulting the
sourcemap. The sourcemap (below) is the structured form for
tooling; the symbol convention is the human-readable form for
on-call.

**(c) Sourcemap artifact.**

`out/{ap,mcu}/sce_sourcemap.json` maps every generated symbol back
to its SCXML origin in a single structured file. Schema:

```json
{
  "version": 1,
  "source_hash":   "<sha256 — same as §6.2.6 generated-source header>",
  "template_hash": "<sha256 — same as §6.2.6>",
  "symbols": {
    "session_unicast__Opening__on_init_ack": {
      "scxml_file":       "sources/session/session_unicast.scxml",
      "scxml_state_path": "Opening",
      "scxml_xpath":      "/scxml/state[@id='Opening']/transition[2]",
      "line_range":       [142, 178],
      "kind":             "transition_handler",
      "event":            "init_ack",
      "wcet_us":          18
    },
    "crc16_ccitt__body": {
      "scxml_file":       "sources/algorithms/crc16_ccitt.scxml",
      "scxml_state_path": null,
      "scxml_xpath":      "/scxml/sce:body",
      "line_range":       [12, 35],
      "kind":             "algorithm_body",
      "wcet_us":          120
    }
  }
}
```

`wcet_us` is present iff the source declares `<sce:wcet-bound>`
(§5.A). The sourcemap travels alongside the generated library and
is checked into version control with `out/`-equivalent provenance —
never edited manually. Its `source_hash` MUST match the
corresponding generated file's §6.2.6 header hash; mismatch is a
build error.

**Tooling — `addr2sce`.**

Ships with `sce-codegen`. Translates from runtime artifacts to
SCXML locations:

```
addr2sce <elf> <pc_or_symbol> [--sourcemap <path>]
  → resolves to scxml_file:line, prints state path / xpath

addr2sce --hard-fault <coredump> <elf>
  → walks the stack, resolves every frame against the sourcemap,
    prints SCXML-relative trace

addr2sce --rust-panic <stderr_capture>
  → consumes 'thread main panicked at out/ap/src/foo.rs:123:5'
    and re-attributes via SCE-MAP attributes + sourcemap
```

The C path uses standard DWARF (`#line` already lands in debug
info); `addr2sce`'s C-side value-add is summarizing across multiple
frames into a state-path narrative ("fault inside
`session_unicast::Established::TxSchedule::on_send_pdu` line 312 of
session_unicast.scxml, called from worker `tx_worker` slot at tick
4218"). On the Rust side `addr2sce` is the only mechanism, since
Rust panic output points at generated `.rs` lines that the
sourcemap translates back.

**Codegen contract.**

- Every C/Rust function emitted by SCE backends MUST carry the
  source attribution above. Templates added after Phase A inherit
  this as a hard requirement.
- The XInclude / `sce:template` composition phase MUST track per-
  element `(file_id, line, column)` and attach it to every IR
  node. Codegen failure to do so is an internal invariant
  violation, surfaced via
  `traceability/scxml-line-range-missing` (codegen-internal; should
  never reach authors).
- Sourcemap emission is a single pass at the end of codegen; it
  consumes the IR's annotation set and serializes one JSON
  document per machine per backend.
- Meta-generated SCXML (ARCHITECTURE §13 #7) preserves provenance
  by emitting `<sce:source-line file="..." line="..."/>` markers
  on each generated state/transition. The meta-generator's
  `tools/meta/expand.py` is responsible; verify.py validates the
  markers exist before sce-codegen runs.

**Interaction with existing sections.**

- **§5.A (algorithm)** — WCET annotations land in the sourcemap's
  `wcet_us` field. `mode="measured"` algorithms whose source-hash
  drift triggered `algorithm/wcet-measured-stale-against-source-hash`
  also fail the sourcemap-hash check, unifying the two staleness
  detections.
- **§5.B (codec)** — `<sce:test-vector>` line ranges land in the
  sourcemap; `codec/test-vector-drift` failures reference SCXML
  location through it.
- **§5.E (pool slot FSM)** — `pool/ownership-violation` diagnostics
  print the offending operation's SCXML location resolved from the
  sourcemap.
- **§5.J.1 (C11 backend)** — `#line` directive emission is part of
  the template tree contract from day one (Phase A5).
- **§5.J.2 (Rust no_std)** — `#[doc]` attribute emission is the same
  mechanism for std and no_std variants; no_std does not strip
  attributes.
- **§6.2.5 (fuzz harness)** — generated harness names follow the
  same convention (`<codec>__fuzz`); crash-finding inputs reference
  symbols that resolve through the sourcemap.
- **§6.2.6 (drift detection)** — the sourcemap is one of the
  verified artifacts; its `source_hash` is computed identically to
  the generated-code header, and `sce-codegen verify` includes it in
  the gate.

**Diagnostics.**

- `traceability/state-id-collision` — XInclude or `sce:template`
  composition produced two states with identical canonical paths
  within one machine. Author resolution: rename one or wrap in a
  sub-state. Reports both source locations.
- `traceability/symbol-name-exceeds-c-identifier-limit` — generated
  symbol exceeds the 31-char external-identifier minimum from C99
  sec. 5.2.4.1. Modern toolchains support far longer identifiers and
  this diagnostic is normally a warning; targets that enforce the
  strict minimum (rare legacy vendor toolchains) opt into hard-
  error treatment via deploy.yaml `platform.strict_c99_identifiers:
  true`. Author resolution: shorten machine or state names.
  Codegen does not auto-truncate because that defeats the round-
  trip from symbol to SCXML origin.
- `traceability/sourcemap-source-hash-mismatch` — emitted
  sourcemap's `source_hash` does not match the generated code's
  §6.2.6 header hash. Codegen invariant violation; should never
  reach authors.
- `traceability/scxml-line-range-missing` — IR node lacks line
  range annotation; XInclude pass dropped it. Codegen-internal
  invariant.
- `traceability/sce-map-attribute-stripped` — Rust `cargo doc
  --output-format json` query for a symbol returns no SCE-MAP
  `#[doc]` attribute. Either the toolchain stripped it (build with
  `-Zstrip=symbols` or similar) or codegen failed to emit. Default:
  warn at sce-build time, fail at addr2sce-time when the attribute
  is absent for a queried symbol.
- `traceability/meta-generated-source-line-marker-missing` —
  meta-generated SCXML (ARCHITECTURE §13 #7) lacks `<sce:source-
  line/>` markers; `tools/meta/verify.py` runs as a CI gate before
  `sce-codegen`.

**Open question.** OQ-W16 (canonical state path delimiter and Rust
SCE-MAP preservation mechanism) — see
`docs/rfc-open-questions-log.md`. Initial proposal: `__` delimiter
with `_u_` escape; `#[doc]` attribute for Rust mapping. Codegen
emits both the attribute and the line comment until OQ-W16 is
resolved, so neither survival path is foreclosed.

---

## §6 Cross-cutting concerns

### 6.1 Diagnostic contract

All new diagnostic codes listed above land simultaneously in:
- `SCE_ERROR_CONTRACT.md` §5
- `docs/SCE_ACCEPTED_SUBSET.md` appendix
- `schemas/sce-diagnostic.v1.schema.json` (if shape changes)

Per the existing §8.1 schema-status rule, additive-only changes keep
`SCHEMA_STATUS = "pre-release"`; any shape revision blocks a "stable"
flip until this RFC is fully implemented. Suggest keeping "pre-release"
through Phase 2.

### 6.2 Testing requirements (new)

#### §6.2.1 Cross-backend parity

Every `algorithm` and `codec` kind
with a `<sce:test-vector>` must round-trip identically in both Rust
and C backends. Enforced by a shared test runner consuming the SCXML
sources, generating both backends, executing both, and diffing byte
vectors.

#### §6.2.2 Wire replay

A new test mode in `sce-build/src/conformance.rs`
accepts a pcap file and a mapping of packet → expected event
sequence, replays bytes into the generated stack, and asserts event
emission and outbound byte shape.

#### §6.2.3 Upstream interop (project-level, not SCE)

`watching-zenoh`
owns CI that runs `zenohd` in Docker and verifies generated AP
backend can SCOUT → HELLO → OPEN → Established. Not an SCE
responsibility but an acceptance criterion for §7 Phase rollout
gating.

#### §6.2.4 No-alloc guard (layered)

MCU-built output enforces the
no-heap invariant through five layers, each catching what the
previous layer's failure mode allows through:

1. **Stub trap symbols** — `malloc`/`calloc`/`realloc`/`free` emit
   `__builtin_trap()`; direct allocation calls fail at link or
   first call.
2. **Linker `--wrap`** — `-Wl,--wrap=malloc` (and the other three)
   so indirect calls from libc land on `__wrap_*` (also trapping).
3. **Libc variant pin** — build configuration pins
   `--specs=nano.specs` (newlib-nano) or picolibc-tiny so
   `printf`/`scanf` family does not link the allocating formatter
   path. Deviation is a build error.
4. **Call-graph reachability analysis** — post-link, codegen runs
   `nm` + objdump call-graph extraction over the linked ELF and
   verifies no symbol reachable from `out/mcu/` entry points calls
   into the SCE-shipped allocating-libc deny-list (`vasprintf`,
   `asprintf`, `strdup`, `getline`, `posix_memalign`, etc.).
5. **Exception/RTTI ban** — build flags `-fno-exceptions -fno-rtti
   -fno-unwind-tables`; libstdc++ exception machinery never links.

Diagnostics:
- `noalloc/libc-variant-not-pinned` — build does not pass
  `--specs=nano.specs` or equivalent; full `printf` may link
- `noalloc/reachable-allocator-from-deny-list` — Layer 4
  reachability hit; the offending call chain is reported (e.g.
  `sce_session_close → log_format_msg → vasprintf`)
- `noalloc/exceptions-not-disabled` — `-fno-exceptions` missing
  from build configuration; libstdc++ exception path could link

#### §6.2.5 Adversarial fuzz harness

Every codec kind that uses VLE,
TLV chain, length-prefix, variant, or borrow `parse-mode` (§5.B)
auto-generates a fuzz target alongside the codec source: a
`cargo fuzz` target on the Rust side and a libFuzzer/AFL harness
on the C side. The contract is:

> Arbitrary byte sequence → either a successfully parsed value
> OR a typed `CodecError` / `SCE_RESULT_*`. Never a panic, trap,
> hang, or out-of-bounds slice.

This makes the "synthesized codec rejects malformed input safely"
property mechanically verifiable. Downstream consumers run the
generated harnesses in CI with coverage-guided seeding from their
pcap fixtures (§6.2.2). Crash-finding inputs are minimized and
captured back into `tests/fuzz/regressions/` as new fixed vectors,
so re-introduction is caught by §6.2.1 cross-backend parity.

A session-FSM-level harness extends the same property to the
statechart layer: arbitrary byte sequences after OPEN must drive
the session into either a valid `Established` state-update or
`Closing` with a typed close reason — never a panic.

**Cross-target build matrix.** The C-side fuzz harness MUST build
in three environments to catch architecture-specific bugs invisible
to a single-host run:

- **F1: x86_64 host + libFuzzer/AFL + ASan/UBSan** — logic bugs,
  OOB slices, panic paths
- **F2: i686 (32-bit) host cross-build + libFuzzer** — `size_t`
  overflow that doesn't trigger on 64-bit, pointer-size-dependent
  padding bugs
- **F3: qemu-system-arm user-mode (Cortex-M3/M4/M7) + harness** —
  unaligned-access traps (Cortex-M0/M0+/M3 strict-align cores),
  ARM-specific undefined behavior

Codegen MUST emit the harness boilerplate parameterized for all
three environments (single SCXML source → three build configs).
F1 is the existing host-fuzz path; F2 requires multilib gcc /
clang and 32-bit libFuzzer; F3 requires qemu-arm and a small
syscall shim layer. Crashes reproducing only on F2 or F3 indicate
architecture-specific bugs that single-host fuzzing cannot find,
and they are exactly the class that hard-faults the MCU at runtime.

Diagnostics:
- `codec/fuzz-harness-not-generated` — codec declares one of the
  high-risk extensions but `<sce:fuzz>skip</sce:fuzz>` was set
  without a justification reference (issue link or RFC pointer)
- `codec/fuzz-harness-stale-against-source-hash` — generated
  fuzz target's recorded source-hash diverges from current codec
  IR; harness must be regenerated
- `codec/fuzz-cross-target-tier-disabled` — F2 or F3 build
  configuration disabled in `deploy.yaml` `fuzz_tiers:` without a
  justification reference; F1-only fuzzing is permitted but warns
  loudly because it cannot find architecture-specific bugs

#### §6.2.6 Generated source drift detection

Every emitted file
carries a header of the form:

```
// SCE-GENERATED — DO NOT EDIT
// source-hash: <sha256 of sorted input SCXML + deploy.yaml>
// template-hash: <sha256 of template tree + Cargo.lock>
// generated-at: <utc timestamp, informational only>
```

`sce-codegen verify <out-dir>` recomputes both hashes from the
current source + template state and compares against the embedded
values. Mismatch is failure. CI runs this as a gate; pre-commit
hook runs it locally.

The policy: **manual edits to `out/` are forbidden**. When
generated code falls short, the path forward is an SCE RFC (or
`sce:extern` if target-specific), never a direct patch. This
preserves the SSoT claim operationally, not just aspirationally.

Exceptions must be tracked in `docs/SCE_ACCEPTED_SUBSET.md` with a
linked RFC and an expiry date.

### 6.3 Tooling

- `sce-codegen` CLI accepts `-l` multiple times: `-l rust -l c11`
- `sce-codegen build <deploy.yaml>` drives the whole multi-machine
  multi-backend generation in one invocation, producing per-machine
  output roots with suggested `Cargo.toml` / `CMakeLists.txt` /
  linker script fragments.
- `sce-codegen watch <dir>` re-runs on source changes.

---

## §7 Phased rollout

Each phase is an independently shippable PR or PR series. Downstream
consumers (watching-zenoh and any others) can adopt phase-by-phase.

The MVP gate for the downstream watching-zenoh project is **full
zenoh-pico feature parity** (§2.2). Phases are sized against that
gate, not against a narrower "leaf subset" framing. Weeks are
SCE-maintainer-side effort estimates; downstream project work
proceeds in parallel once each phase lands.

```
Phase A — Foundation (weeks 1–6)
  A1  Diagnostic code shells, SCE_ERROR_CONTRACT.md additions
  A2  deploy.yaml platform/memory/scheduler fields (§5.K partial,
      including has_dcache / dcache_line_size / core_count /
      worker_stack_budget)
  A3  Algorithm kind (§5.A): XSD, model, parser, Rust+C++ emitters
  A4  Build-time const-fold (§5.F)
  A5  Close §5.J.4 matrix on C11 for the §5.A algorithm kind.
      The C11 backend skeleton itself shipped at `758aea3f` ("close
      C11 byte-golden parity with 5 backends") — the eleven baseline
      kinds are already byte-golden parity on C11. A5 lands the
      C11 algorithm template (`forge/c/algorithm.h.jinja2`) and
      flips `template_ships(Algorithm, C11) = true` in
      `sce-build/src/forge/codegen_matrix.rs`, so the matrix walker
      stops firing `codegen/generic-kind-backend-emit-missing` for
      `(c11, algorithm)` pairs. RFC §5.O (`#line` + sourcemap) is
      a Phase B follow-up — A5 is the matrix-closure cut, not the
      sourcemap rollout
  A6  Test: CRC16 byte-equivalent Rust vs C → permanent acceptance
      gate. Wires the two `algorithm_crc16` / `algorithm_crc16_table`
      fixtures into the Forge conformance harness so cmake-driven
      ctest enforces byte-equivalence on every build:
      - `tests/forge/conformance/fixtures.json` gains `bytes` to its
        canonical-types blurb and 2 `kind: "algorithm"` fixture entries.
      - `tests/forge/conformance/numerical_reference.json` adds 7-vector
        oracle blocks under `pure_functions` (canonical "123456789"
        → 0x29B1, plus empty / single-byte / mixed / 8-zero edge cases),
        identical between bit-by-bit and table forms by contract.
      - `tools/codegen/templates/forge/{rust,c,cpp}/conformance/kinds/algorithm.{rs,c,cpp}.jinja2`
        ship together (cpp fragment lands here for parity since cpp
        algorithm template shipped at A3).
      - `sce-build/src/forge/codegen_matrix.rs::template_ships` becomes
        the single matrix-aware filter for `render_harness` and
        `list-fixtures --language=<lang>` so kotlin/go/python (algorithm
        not yet shipped) silently drop the fixtures while rust/cpp/c11
        run them. A6 also lifts the cpp conformance harness from
        C++17 → C++20 because `std::span` (RFC §5.J.5 algorithm cpp
        emitter shape) requires C++20.
      - Generator hardening: `lower_algorithm_body` now collects
        `<sce:assign target>` roots up-front and the Var arm emits
        `let` (not `let mut`) when the local is never reassigned;
        Rust if/while drop the C-flavoured paren wrap. Both fall out
        from workspace `warnings = "deny"` once algorithm enters the
        rust conformance harness.
      §5.O verification (`addr2sce <elf> crc16_ccitt__body` resolves
      to `sources/algorithms/crc16_ccitt.scxml:<expected_line>`) lands
      with the §5.O sourcemap emitter in Phase B

Phase B — Wire format full (weeks 7–16)
  B1  Codec DSL core: vle, variant, flags, present-if (§5.B)
  B2  Codec DSL rest: len-prefix, repeat, until-eof, test-vector (§5.B)
  B3  Codec DSL: tlv-chain bounds + DMA alignment constraints (§5.B)
  B4  Attachments, timestamps, encoding-info codec shapes (§5.B applied)
  B5  Full Zenoh message set codecs authored in SCXML by downstream
      (~30 message types); SCE-side work: any missing codec primitives
      surfaced by the authoring pass
  B6  Link kind + sce_link_runtime minimal (UDP/TCP) (§5.C)
  B7  Buffer-pool kind + memory placement + cache-policy (§5.E)
  B8  Wire replay harness (§6.2.2)
  B9  Generated source drift detection (§6.2.6)
  B10 Test: VLE ZInt round-trip + Zenoh SCOUT/HELLO/INIT/OPEN pcap replay

Phase C — Dynamic state & concurrency (weeks 17–26)
  C1  Timer kind review/extension (§5.D)
  C2  Worker kind + inbox ordering contract (§5.D)
  C3  Rust no_std variant (§5.J.2)
  C4  sce:extern + concrete intrinsics whitelist with ordering (§5.I)
  C5  Cache maintenance intrinsics wired into §5.E codegen
  C6  Bounded-collection kind (§5.L, new) — backs runtime sub/queryable
      tables and other bounded-dynamic state
  C7  Runtime KeyExpr matching as algorithm kind over bounded-collection
      (compile-time table in §5.F remains available as alternate strategy)
  C8  Multicast session FSM authoring (sibling SCXML to the unicast
      session FSM; no handshake, periodic Join + peer-table learning
      per upstream zenoh 1.5.0 multicast transport). Q13 resolution
      (2026-04-25): peer/client share the unicast FSM — the session
      layer is wire-identical between modes — so the split is by
      **transport class** (unicast vs multicast), not by node mode.
      See `docs/session-fsm.md` §5.3
  C9  Fragment / reassembly kind + reassembly-pool variant of §5.E
  C10 Multi-link concurrent codegen (multiple driver instances per
      machine: e.g. UDP scout + TCP session + Serial fallback)
  C11 Serial + WebSocket link drivers (BLE / Raweth deferred to
      target plugins, not part of SCE core)
  C12 Liveliness token FSM pattern (SCXML authoring; no new SCE kind
      required beyond C6+C7)
  C13 deploy.yaml links/buffer_pools/extern_symbols + all dynamic-state
      bounds (§5.K full)
  C14 Test: watching-zenoh MCU node achieves **zenoh-pico parity** —
      all peer/client operations interop with upstream zenohd and
      upstream zenoh-pico nodes
  C15 Test: multi-core MCU inbox stress (if any target plugin available)

Phase D — AP targets + migration enablers (weeks 27+)
  D.1 — AP Linux baseline (first sub-phase, blocks D.2)
    D.1.1  sce_link_runtime_tokio crate (epoll/kqueue via mio,
           `tokio_udp` / `tokio_tcp` drivers)
    D.1.2  AP code emission integrated with §5.K platform.os: linux
    D.1.3  io_uring opt-in driver (`tokio_uring`, kernel ≥ 5.10);
           pool-like fixed-buffer model maps onto §5.E lifecycle FSM
    D.1.4  Add `unix_socket` / `unix_seqpacket` to §5.C link-class
           enum if and when intra-host AP IPC is required for Phase
           D.1 deliverables (additive enum row + driver + diagnostic
           updates land in the same patch; not pre-reserved)
    D.1.5  Test: AP linux + zenohd interop (SCOUT/HELLO/OPEN)
  D.2 — AP QNX baseline (second sub-phase, depends on D.1 completion)
    D.2.1  sce_link_runtime_qnx crate over QNX-native dispatch + io-sock
           POSIX sockets (mio has no QNX backend; see OQ-W20 resolution)
    D.2.2  Add QNX-native typed-message / shared-memory link-classes to
           §5.C enum if and when concrete Phase D.2 use cases require
           them (e.g. `qnx_msg`, `qnx_shm` would land additively then,
           together with the framer-model RFC and the matching
           `sce_link_runtime_qnx` driver — not pre-reserved)
    D.2.3  realtime scheduler integration on QNX (`scheduler.kind: rt`
           with QNX adaptive partitioning + thread priorities; OQ-W21
           future)
    D.2.4  Test: AP qnx + zenohd interop on a reference QNX target
  D.3 — Migration enablers (no OS dependency)
    D.3.1  Parametric kinds (§5.G) — collapse client/peer FSM variants,
           parameterize VLE, etc.
    D.3.2  Recursive / tree types (§5.H) — bounded recursive data
           traversal (TLV trees, future graph descendants)
    D.3.3  Dynamic aggregation kind — generalization of bounded-
           collection with per-key index lookups; prerequisite for
           AP router mode
    D.3.4  AP-mode unlock for deferred features: router-mode
           prerequisites, subscription aggregation skeleton
    D.3.5  API compat shim examples (`zenoh-c` / `zenoh-pico` drop-in
           wrappers on top of synthesized library)
    D.3.6  Target-plugin examples for BLE and Raweth drivers (not SCE
           core; demonstrates §5.I extensibility)
    D.3.7  Tooling: multi-`-l`, `build`, `watch` (§6.3)

Phase E — Additional AP targets (weeks 50+, post-D)
  E.1  AP macOS / FreeBSD via sce_link_runtime_kqueue
  E.2  AP Windows via sce_link_runtime_iocp
  E.3  RTOS-class targets via target-plugin runtime crates
       (Zephyr / FreeRTOS / NuttX `sce_link_runtime_<rtos_id>`)
```

Phase A is the minimum viable drop for downstream to begin authoring
`algorithm` kinds (CRC, VLE). Phase B delivers wire-format parity
(all zenoh-pico messages representable). Phase C delivers the
**zenoh-pico MVP parity gate** — runtime subscription management,
fragmentation, multi-link, client mode. Phase D opens AP target
implementation (Linux first at D.1, QNX next at D.2) plus migration
enablers; Phase E adds further AP targets.

**MCU-first invariant.** Phase A through C14 (the zenoh-pico parity
gate) is the priority track. AP work begins at Phase D and only
after the C14 gate is green. The OS axis added in review #13
(§5.K `platform.os`, §5.J backend 3-tuple) is **design-only during
Phase A–C** — it reserves namespace and disambiguates schema for
the eventual Phase D entry, but adds zero MCU implementation
burden. A `deploy.yaml` authored during Phase A–C declares
`platform.os: bare_metal` (or `rtos` via target plugin); the AP
OS values exist in the schema enum but emit
`deploy/platform-os-not-implemented-in-current-phase` if attempted
pre-Phase-D. This keeps MCU work uninterrupted while preventing
Phase D from being a refactor cliff.

**New kinds referenced above.** §5.L (`bounded-collection`),
§5.M (reassembly-pool variant + fragment FSM pattern), and §5.N
(multi-link concurrency codegen contract) are fully specified
earlier in this RFC. XSD/IR/emitter work for them is included in
Phase C (C6 for §5.L, C9 for §5.M, C10 for §5.N).

---

## §8 Open questions

These require SCE maintainer input before implementation begins:

**Q1.** Is `algorithm` the right kind name? Alternatives: `routine`,
`function`, `pure`. Concern: `function` overlaps with `FuncSig` in
the type context layer.

**Q2.** Does the existing `Timer` kind already cover §5.D timer
needs? If yes, §5.D collapses to worker-only.

**Q3.** Should `algorithm` allow calls to other `algorithm` kinds
(§5.A `Call` stmt)? This is useful but opens call-graph/cycle
detection scope. MVP could forbid, Phase 2 could allow non-recursive.

**Q4.** Should `<sce:compute-at="build">` be an attribute on existing
`Transform` too, or strictly an `algorithm` / `const` feature? Opens
the door to build-time expression evaluation everywhere, which has
nice but larger implications.

**Q5.** Granularity of the codec DSL extensions — land all of §5.B at
once (one PR), or split by feature (seven PRs)? Split reduces review
risk but stretches schedule.

**Q6.** C11 backend — target C11 strictly, or C99 + stdint for wider
reach? C11 adds `_Static_assert` but loses some legacy toolchains.

**Q7.** `sce:extern` whitelist location — a separate repo/crate, or
in-tree under `sce-build/runtime/sce_intrinsics_runtime/`?

**Q8.** ~~Should the `link` kind's driver set be extensible by
downstream projects?~~ **Answered: Yes.** Drivers are an open set
via the **target-plugin** mechanism introduced in §5.I. Core SCE
ships with `tokio_udp`, `tokio_tcp`, `lwip_udp`, `lwip_tcp`,
`serial_uart`, `websocket_tcp`. Additional drivers (BLE, Raweth,
QUIC, custom) are declared in a target-plugin YAML file referenced
by `deploy.yaml`'s `extern_symbols.target_plugin` entry. Plugin
files list the driver name, required extern symbols, and expected
deploy.yaml configuration schema. Plugin files are part of the
deploy.yaml review scope, not an unbounded escape hatch.

Diagnostic additions (already listed in §5.I):
`extern/target-plugin-symbol-conflict` and a new
`link/driver-not-in-core-or-plugin`.

**Q9.** `deploy.yaml` `memory.sram_regions` format — bytes (64K) or
explicit integer (65536)? Which is less error-prone?

**Q10.** Parametric kinds (§5.G) XSD shape — `sce:type-param` as a
new element, or a convention on existing attributes with a magic
prefix `$W`? XSD validation cleanliness favors the element form.

**Q11.** `bounded-collection` capacity source (§5.L) — always
deploy-sourced, or should `<sce:capacity const="N"/>` also be
allowed for truly fixed structures (e.g., timer wheels)? Proposal:
allow both, with deploy-source preferred when the same capacity
appears in multiple machines so sizing can vary per deployment.

**Q12.** Fragment / reassembly approach (§5.M) — should the
reassembly FSM be author-level SCXML (current proposal), or should
SCE ship a canonical `fragment-reassembly` template? Author-level
preserves flexibility (e.g., different reassembly policies for
different streams); canonical template reduces boilerplate. MVP
preference: author-level with a template library shipped in
`examples/`, no SCE-side kind.

**Q13.** ~~Client vs peer session FSM variants~~ **Answered
2026-04-25.** Upstream zenoh 1.5.0 (`io/zenoh-transport/src/
unicast/establishment/{open.rs, accept.rs}`) shows the session
layer is wire-identical for peer and client — one `OpenLink`
struct drives both, parameterized only by `manager.config.whatami`.
Peer/client differences live in scouting (active vs passive
default) and in the network layer (declaration topology, interest
semantics), not in the session FSM. What is structurally different
at the session layer is **unicast vs multicast**: unicast does a
4-way Init/Open handshake; multicast has no handshake and learns
peers via periodic `Join` (`multicast/establishment.rs` +
`multicast/rx.rs:60 handle_join_from_peer`). MVP therefore
authors two session SCXML files split by transport class —
`session_unicast.scxml` and `session_multicast.scxml` — with
`whatami` as a compile-time constant from deploy (embedded in
`InitSyn`/`Join`, not branched on). No §5.G parametric dependency.
Details and state-level evidence in `docs/session-fsm.md` §5.
Phase C8 retargeted to multicast session authoring (this RFC §7).

**Q14.** `algorithm` kinds calling `bounded-collection` operations
(§5.L interaction with §5.A) — should this be a direct `sce:call`
as proposed (treat collection ops as algorithm-callable methods),
or should collections expose a procedure-kind interface instead?
Proposal: allow both; `sce:call` for synchronous lookups, procedure
interface for event-driven patterns (e.g. "new subscriber arrived").

---

## §9 Impact summary

**MVP criterion recap.** The downstream watching-zenoh project's
MVP gate is **full zenoh-pico feature parity** in peer + client
modes (§2.2). This is larger than an earlier "leaf subset" framing
and requires §5.L (bounded-collection), §5.M (fragment /
reassembly), §5.N (multi-link concurrency) in addition to the
base §5.A–K kinds.

**Files touched (estimate, revised for zenoh-pico parity MVP):**

*New:*
- `tools/codegen/templates/forge/c/**` (~25 files; includes
  bounded-collection, reassembly-pool variant, multi-link dispatch)
- `sce_link_runtime` crate (~800 LOC; UDP/TCP/Serial/WebSocket
  adapters)
- `sce_intrinsics_runtime` crate (~500 LOC; atomics with concrete
  ordering variants, cache maintenance, IRQ save/restore)
- `sce-build/src/verify.rs` (drift detection, ~200 LOC)
- `sce-build/src/forge/bounded_collection.rs` (~400 LOC)
- `sce-build/src/forge/fragment.rs` (reassembly-pool variant
  logic + FSM support, ~350 LOC)
- `sce-build/src/deploy/target_plugin.rs` (~300 LOC)
- `sce-build/src/forge/sourcemap.rs` (~250 LOC; §5.O sourcemap
  IR-to-JSON emission, line-range tracking through XInclude /
  `sce:template`, hash unification with §6.2.6)
- `tools/addr2sce/` (~400 LOC; Rust binary; ELF + DWARF + sourcemap
  resolver, Rust panic re-attribution via rustdoc JSON, coredump
  walker — §5.O traceability tooling)
- Test fixtures (~50 files, covering all zenoh-pico message types
  as test vectors)

*Modified:*
- `schemas/sce-forge-ext.xsd` (~250 lines added; DMA alignment,
  tlv-chain bounds, cache-policy, bounded-collection,
  reassembly-pool variant, multi-link declarations)
- `sce-build/src/forge/model.rs` (+~1300 LOC; new kind models,
  bounded-collection IR, reassembly-pool variant)
- `sce-build/src/forge/parser.rs` (+~1000 LOC)
- `sce-build/src/forge/generator.rs` (+~2500 LOC; per-kind
  emitters for Rust/C including new kinds, cache maintenance
  call-site injection, multi-link scheduler generation)
- `sce-build/src/deploy.rs` (+~400 LOC; platform fields, target
  plugin resolution, link-bus generation)
- `sce-build/src/mesh/` (unchanged — orthogonal per §5.K)
- `SCE_FORGE.md` (~500 lines; new kind sections)
- `SCE_ERROR_CONTRACT.md` (~65 new diagnostic entries)
- `docs/SCE_ACCEPTED_SUBSET.md` (~15 appendix entries)

**LOC budget (SCE-side net addition, all phases):**
**~8500–12500 LOC**. The upward revision from earlier 6000–9500
reflects the expanded MVP scope (bounded-collection, fragment /
reassembly, multi-link concurrency, Serial / WebSocket link
drivers, §5.O traceability sourcemap + addr2sce tool) required to
match zenoh-pico feature parity rather than a narrower leaf subset. Comparable in magnitude to SCE Mesh
landing — and, like that effort, delivered in shippable phases
(A–C gate the MVP; D extends toward AP router mode and the
broader Zenoh feature set).

**Downstream project LOC estimate (watching-zenoh authoring):**
- SCXML sources (FSM, codec, algorithm, link, pool, worker,
  bounded-collection declarations): ~7000 LOC
- Hand-written runtime crates
  (`sce_link_runtime_tokio`, `sce_link_runtime_lwip`,
   `sce_intrinsics_runtime_{rust,c}` impls): ~1700 LOC
- Test fixtures, pcap corpus, CI glue: ~2000 LOC
- **Total downstream authoring: ~10,700 LOC**

vs. hand-written zenoh-pico peer+client parity in two languages:
~35–50K LOC × ongoing drift management. The synthesis approach
delivers roughly **3–5× reduction in authoring volume** plus
**structural drift elimination** between AP and MCU.

**Backward compatibility:** Fully additive. No existing SCXML document
becomes invalid. No existing generated code changes behavior. Existing
tests remain green throughout.

---

## Appendix A — Worked example: CRC16-CCITT

**SCXML source** (abbreviated — see §5.A for full):

```xml
<scxml sce:kind="algorithm" name="crc16_ccitt">
  <sce:signature>
    <sce:param name="data" type="bytes"/>
    <sce:return type="u16"/>
  </sce:signature>
  <sce:const name="TABLE" type="array<u16, 256>" sce:compute-at="build">
    <sce:fold range="0..256" as="i" elem-type="u16"> ... </sce:fold>
  </sce:const>
  <sce:body>
    <sce:var name="crc" type="u16" init="0xFFFF"/>
    <sce:foreach item="b" in="data">
      <sce:assign target="crc"
        expr="TABLE[((crc >> 8) ^ b) &amp; 0xFF] ^ (crc &lt;&lt; 8)"/>
    </sce:foreach>
    <sce:return expr="crc"/>
  </sce:body>
</scxml>
```

**Generated Rust (AP):**

```rust
pub const TABLE: [u16; 256] = [ 0x0000, 0x1021, 0x2042, /* ... */ ];

pub fn crc16_ccitt(data: &[u8]) -> u16 {
    let mut crc: u16 = 0xFFFF;
    for b in data {
        crc = TABLE[(((crc >> 8) ^ (*b as u16)) & 0xFF) as usize] ^ (crc << 8);
    }
    crc
}
```

**Generated C (MCU):**

```c
static const uint16_t TABLE[256] = { 0x0000u, 0x1021u, 0x2042u, /* ... */ };

uint16_t crc16_ccitt(const uint8_t* data, size_t len) {
    uint16_t crc = 0xFFFFu;
    for (size_t i = 0; i < len; ++i) {
        crc = TABLE[(((crc >> 8) ^ data[i]) & 0xFFu)] ^ (uint16_t)(crc << 8);
    }
    return crc;
}
```

**Parity test** (shared across backends):

```xml
<sce:test-vector hex="313233343536373839" value="0x29B1"/>
<!-- "123456789" → 0x29B1, standard CRC16-CCITT vector -->
```

Both backends must return 0x29B1 for this input. Build fails if either
diverges.

---

## Appendix B — Worked example: VLE ZInt u64

```xml
<scxml sce:kind="algorithm" name="vle_u64_encode">
  <sce:signature>
    <sce:param name="v" type="u64"/>
    <sce:param name="cursor" type="cursor"/>
    <sce:return type="Result"/>
  </sce:signature>
  <sce:body>
    <sce:while cond="v > 0x7F" max-iter="10">
      <sce:call target="cursor.write_u8" args="(v &amp; 0x7F) | 0x80"/>
      <sce:assign target="v" expr="v >> 7"/>
    </sce:while>
    <sce:call target="cursor.write_u8" args="v &amp; 0x7F"/>
    <sce:return expr="Ok"/>
  </sce:body>
</scxml>
```

The `max-iter="10"` makes the bound explicit — a u64 VLE is at most
10 bytes. On MCU this becomes a `_Static_assert` friend; the generator
can unroll or keep the loop based on a size-vs-speed attribute.

---

## Appendix C — Honest scope of the end-state

Downstream consumers (watching-zenoh and similar) should document
these boundaries plainly in their own READMEs.

### What `watching-zenoh` WILL deliver when all phases land:

- Zenoh protocol version **pinned** (e.g. Zenoh 1.x wire format).
- **Leaf/peer mode** only. Can initiate peer handshakes, subscribe,
  publish, participate in small peer meshes.
- Interoperable with upstream `zenohd` for the subset.
- QoS selectable per-binding: priority, reliability, express,
  congestion control. Lowered to typed runtime config (AP) or
  `static const` struct (MCU).
- Zero-copy for: TX encode path; RX of single-frame, bounded-size
  payloads; control-plane messages.
- Honest peer-declared KeyExpr intersection at runtime — incoming
  `DeclSubscriber` / `DeclQueryable` / `Interest` wire frames carry
  arbitrary zenoh-style chunked patterns (literal / `*` / `**` / `$*`
  DSL mixes), matched against the local wz-owned KE registry via a
  dedicated chunk-intersect algorithm.

### What `watching-zenoh` will NOT deliver:

- Router mode (no `zenohd` replacement).
- Subscription aggregation across a network (no graph algorithm).
- Fragmentation / reassembly of payloads larger than the largest
  declared pool slot (those paths stage-copy through a dedicated
  buffer and are marked as such in diagnostics and logs).
- Auth / crypto extensions — deferred; not in the MVP wire subset.
- Full KeyExpr wildcard *authoring* for `watching-zenoh`-owned
  keyexprs — the wz-own KE set is compile-time fixed; the runtime
  `declare_*` API accepts only literal keyexprs registered at build
  time (§5.F const-fold). Peer-declared keyexpr wildcards arrive as
  wire-runtime strings and are matched via the runtime intersection
  algorithm (see *What `watching-zenoh` WILL deliver*).
- Drop-in ABI compatibility with `zenoh-c` or `zenoh-pico` APIs;
  compatibility is at the **wire** level, not the API level.

### What zero-copy means in this project specifically:

- TX path: application writes event → codec encodes into pool slot →
  DMA or syscall transmits from pool slot. No intermediate copy.
- RX path, happy case: DMA fills pool slot → codec parses in place
  → event emitted carrying a borrow into the pool slot → slot
  returned to pool after event consumption. No intermediate copy.
- RX path, oversize or fragmented: staged buffer receives
  reassembled bytes, codec parses from staged buffer, event carries
  staged buffer handle. **One copy** between DMA slot and stage.
  Diagnostic `link/stage-copy-invoked` logs occurrence so operators
  can observe when this happens.

This is the honest contract. Anyone reading "zero-copy zenoh on MCU"
elsewhere should mentally append "for bounded control-path traffic."

---

## §10 Acknowledgments

This RFC incorporates feedback from conversation with Claude (Anthropic
Opus 4.7) on 2026-04-24 that corrected an earlier draft's
overestimation of the difficulty (rooted in not distinguishing
leaf/peer mode from router mode, and in underestimating what the
existing SCE expression infrastructure already provides). The final
scope is narrower than the initial sketch and explicit about its
honest boundaries.
