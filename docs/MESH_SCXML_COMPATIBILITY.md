# MESH SCXML Compatibility Guide

**Audience**: authors writing SCXML documents that must run on SCE Mesh *and* remain parseable/executable on any conforming W3C SCXML 1.0 processor.

**Status**: authoritative for Session E1. Cross-references: `SCE_MESH.md` §§9, 13, 16.

---

## Summary

SCE Mesh extends W3C SCXML 1.0 with exactly **one** construct: the invoke type URI `sce:mesh-rpc`. There are no extension attributes, no extension elements, and no extension namespaces carried by mesh-compatible documents.

| Surface | W3C clause | Compatibility |
|---|---|---|
| `<invoke type="sce:mesh-rpc">` | §6.4 (type is implementation-defined URI) | Unknown processor raises `error.execution`. Document is otherwise parsed normally. |
| `<send target="#machine_id">` | §6.2.4 (IO processor target URIs are implementation-defined) | Unknown target raises `error.execution`. Document is otherwise parsed normally. |
| Event-name conventions (`service.request.*`, `event.notification.*`, `field.get.*`, …) | §5.10 (event names are dot-delimited tokens) | Plain event names. Conventions are SCE-tooling only; a foreign processor sees ordinary strings. |
| `sce:*` attributes on any element | — | **Not used**. Removed in Session E1 (see §13 "Session C/D attribute deprecation"). |

Because the only SCE-specific surface is a single type URI, any conforming W3C SCXML 1.0 processor can parse and execute a mesh-compatible document with only the remote-invoke and remote-send operations degrading locally to `error.execution`. Every other construct works unchanged.

## What works identically

The following constructs have identical semantics on SCE Mesh and on any conforming W3C SCXML 1.0 processor:

- All executable content (`<assign>`, `<raise>`, `<log>`, `<script>`, `<if>`, `<foreach>`, `<send>` to local or standard IO processor targets, `<cancel>`).
- All state-tree constructs (`<state>`, `<parallel>`, `<history>`, `<final>`, transitions with `event`, `cond`, `target`, `type`).
- All data-model operations (`<datamodel>`, `<data>`, expression evaluation in `expr`, `cond`, `location`).
- `<invoke>` whose `type` is a standard W3C URI or unspecified (defaulting to `http://www.w3.org/TR/scxml/` per §6.4).
- Done-state and done-invoke events (`done.state.*`, `done.invoke.*`) with their standard donedata semantics.
- The full macrostep / microstep / RTC execution model (§3.12–§3.13).

No porting work is required for any of these.

## What degrades locally on a foreign processor

### `<invoke type="sce:mesh-rpc">`

W3C §6.4.1 pins the behavior: an unsupported invoke type URI raises `error.execution` at the point of invoke. The document is not rejected; parsing succeeds; execution reaches the `<invoke>` element, fails there, and control returns to the parent state machine, which can handle the error with a standard `<transition event="error.execution">`.

**Portable authoring pattern**:

```xml
<state id="compute">
  <onentry>
    <send event="compute.started"/>
  </onentry>

  <!-- SCE Mesh fulfills this remotely; foreign processor raises error.execution -->
  <invoke type="sce:mesh-rpc" src="#motor" id="motor_inv">
    <param name="_mesh_event" expr="'service.request.compute_force'"/>
    <param name="_mesh_deadline_ms" expr="'250'"/>
    <param name="torque" expr="torque_setpoint"/>
  </invoke>

  <transition event="done.invoke.motor_inv" target="applied"/>
  <transition event="error.communication" target="compute_failed"/>
  <!-- Foreign processor lands here; author controls the fallback -->
  <transition event="error.execution" target="compute_failed"/>
</state>
```

The `error.execution` transition is the portable fallback: a foreign processor takes it, an SCE Mesh processor never synthesizes one for this specific failure class (SCE Mesh uses `error.communication` with the §16.7 reason catalogue). Both outcomes lead the author to a well-defined recovery path.

### `<send target="#machine_id">`

W3C §6.2.4 pins the behavior: an IO processor URI the processor does not recognize raises `error.execution` at the `<send>` site. Again, parsing succeeds; only execution of the specific send fails.

**Portable authoring pattern**:

```xml
<onentry>
  <!-- SCE Mesh routes this to the dashboard machine over its bound transport.
       Foreign processor treats #dashboard as an unknown IO target. -->
  <send event="status.ok" target="#dashboard"/>
</onentry>

<transition event="error.execution" target="degraded_mode"/>
<transition event="error.communication" target="degraded_mode"/>
```

Authors who want strict portability (document compiles AND executes correctly on every conforming processor) should either omit mesh-specific `<send target>` or guard against `error.execution` at the surrounding state.

## Author-side guard rules for portable documents

If you intend a document to run on both SCE Mesh and a foreign processor with graceful degradation, follow these three rules:

1. **Always handle `error.execution` near every mesh-targeted `<send>` and `<invoke>`.** The W3C-compliant handler lets foreign processors fall back cleanly. SCE Mesh uses `error.communication` for transport faults — handle both when portability matters.
2. **Do not rely on `_event.data.reason` from either `error.execution` or `error.communication`.** W3C SCXML 1.0 does not fix `_event.data` shapes for error events (§5.10.1). Foreign processors emit whatever shape they choose. SCE Mesh pins a shape (§10.7.1, §16.7), but those fields are only guaranteed on SCE-generated code.
3. **Treat event names as opaque tokens in receiver transitions.** The `service.request.*`, `event.notification.*`, `field.get.*`, … conventions are for SCE tooling (build-time pattern inference, §8.1). A foreign processor sees ordinary strings — `<transition event="service.request.compute_force">` matches the literal event name, nothing more.

## Verification

A document can be smoke-tested on a foreign SCXML 1.0 processor to confirm graceful degradation:

1. **Parse check**: open the document in any conforming parser (e.g., Apache Commons SCXML, another W3C reference implementation). Parsing MUST succeed without warnings or rejections. If it fails, either the document carries a non-standard construct (file a SCE Mesh bug — we should never emit such constructs) or the XML itself is malformed.
2. **Execution check**: run the document with a test harness that inputs the same external events SCE Mesh would provide. Every `<invoke type="sce:mesh-rpc">` MUST raise `error.execution` at invocation; every `<send target="#...">` to a remote machine MUST raise `error.execution` at send. The document MUST reach a well-defined state via author-provided `error.execution` transitions.

If either check fails on a conforming foreign processor, the document is not portable; correct it by applying the guard rules above.

## Why this is safe

SCE Mesh deliberately uses only W3C-reserved extension points (the implementation-defined `type` URI in §6.4 and the implementation-defined IO processor target URI in §6.2.4). Both extension points carry the same W3C contract: unknown URIs degrade to `error.execution` at the point of use, leaving parsing and all surrounding execution untouched. No foreign namespaces are introduced, no unknown attributes are added, no unknown elements are emitted — the document remains valid against the W3C SCXML 1.0 XSD.

Consequently: **a mesh-compatible document is always a valid W3C SCXML 1.0 document**. Degradation on foreign processors is never a parse failure and is always expressible through standard error-handling transitions.
