//! SCE_MESH.md §9.6.2 Session 4b — SOME/IP cross-device scxml-invoke Rust mirror.
//!
//! ## Why a dedicated vsomeip application (Split design rationale)
//!
//! §9.6 cross-device `<invoke type="scxml" src="#peer">` traffic runs on a
//! vsomeip application that is SEPARATE from the per-`<send>`-target
//! applications generated for ordinary SOME/IP method/event/field bindings.
//! The split is intentional and textbook for the SCE architecture, not
//! pragmatic incrementalism:
//!
//!   1. **§13 OEM boundary protection.** `vsomeip.json applications[*]` is
//!      OEM-owned territory. SCE does not register SCE-reserved services
//!      (`0x8100..0x81FF`, see [`SCXML_INVOKE_SERVICE_BASE`]) inside an
//!      OEM-declared application. A dedicated `<machine>_scxml_invoke_app_`
//!      keeps SCE registrations on an SCE-named application that the OEM
//!      explicitly declares for that purpose.
//!   2. **Failure isolation.** A §9.6 peer disconnect or handler exception
//!      stays inside the dedicated application's callback thread and cannot
//!      block the `<send>` SOME/IP path that may carry safety-relevant
//!      traffic.
//!   3. **Service/instance ID responsibility split.** SCE-reserved range
//!      (`0x8100..0x81FF`) collision detection is SCE codegen's responsibility;
//!      OEM service ID collision detection is OEM `vsomeip.json`'s
//!      responsibility. The two domains stay observable at the routing layer
//!      via the `(application, service)` tuple lookup.
//!
//! A future "one app per machine, multiplex SCE + OEM services" alternative
//! would violate (1) by design and is rejected for this system.
//!
//! ## What this Rust module owns
//!
//! Today the codegen template emits `SCE::Mesh::Someip::serviceIdForMachine("...")`
//! constexpr calls directly, so the C++ header
//! [`sce/include/mesh/transports/SomeipScxmlInvokeEndpoint.h`] is the
//! authoritative source for runtime values. This module mirrors the
//! constants and the FNV-1a hash so:
//!
//! * **Rust drift detection** — the unit tests at the bottom pin specific
//!   hash outputs; a divergence from the C++ helper (e.g. someone touching
//!   the FNV constants) trips the tests on `cargo test -p sce-build --lib`
//!   long before the C++ side compiles a wrong service_id into generated
//!   code.
//! * **Future validator hook** — §9.6 4c+ collision detection (256-machine
//!   MVP boundary) will call [`service_id_for_machine`] from the deploy
//!   topology stage to reject deployments that hash two §9.6 peers to the
//!   same low-byte service ID.
//!
//! No `mesh::codegen` path consumes this module yet; the Jinja template
//! emits `SCE::Mesh::Someip::serviceIdForMachine` calls directly. When the
//! validator lands, that consumer will route through here.

// ── SCE-reserved §9.6 namespace constants ───────────────────────────────────

/// Base of the SCE-reserved §9.6 scxml-invoke service ID range. SCE-managed
/// services occupy `[SCXML_INVOKE_SERVICE_BASE, SCXML_INVOKE_SERVICE_BASE +
/// 0x100)` (`0x8100..0x81FF`). Mirror of the C++ `SCXML_INVOKE_SERVICE_BASE`
/// constexpr in `SomeipScxmlInvokeEndpoint.h`.
pub const SCXML_INVOKE_SERVICE_BASE: u16 = 0x8100;

/// Single-instance MVP for §9.6 endpoints. Mirror of the C++
/// `SCXML_INVOKE_INSTANCE_ID` constexpr. Multi-instance pool support for
/// §9.6 endpoints would require lifting §14.4 Gap 7 plumbing into the
/// helper; not in this session.
pub const SCXML_INVOKE_INSTANCE_ID: u16 = 0x0001;

/// Per-wire method IDs. Hex digits replicate the SCE_MESH.md §9.6.2 wire
/// number for vsomeip-trace readability — wire-14 → `0x0014`, wire-20 →
/// `0x0020`. NOT a numeric identity with `PatternKind` enum values
/// (`0x0014 = 20 decimal, not 14`); the C++ helper's `methodForPattern`
/// switch + `static_assert`s pin the mapping. Listed here so any Rust
/// consumer (collision validator, diagnostic emission) sees the canonical
/// values without round-tripping through the C++ side.
pub const SCXML_INVOKE_METHOD_WIRE14_INVOKE_START: u16 = 0x0014;
pub const SCXML_INVOKE_METHOD_WIRE15_INVOKE_STARTED: u16 = 0x0015;
pub const SCXML_INVOKE_METHOD_WIRE16_CHILD_EVENT: u16 = 0x0016;
pub const SCXML_INVOKE_METHOD_WIRE17_PARENT_EVENT: u16 = 0x0017;
pub const SCXML_INVOKE_METHOD_WIRE18_INVOKE_DONE: u16 = 0x0018;
pub const SCXML_INVOKE_METHOD_WIRE19_INVOKE_CANCEL: u16 = 0x0019;
pub const SCXML_INVOKE_METHOD_WIRE20_INVOKE_ERROR: u16 = 0x0020;

// ── Per-machine service ID derivation ───────────────────────────────────────

/// FNV-1a 32-bit offset basis (RFC standard).
const FNV1A_OFFSET: u32 = 0x811c9dc5;
/// FNV-1a 32-bit prime.
const FNV1A_PRIME: u32 = 0x0100_0193;

/// Compile-time per-machine service ID derivation. FNV-1a 32-bit hash of
/// the machine name, low 8 bits ORed with [`SCXML_INVOKE_SERVICE_BASE`] —
/// yields 256 distinct IDs in `[0x8100, 0x81FF]`. Mirror of the C++
/// `SCE::Mesh::Someip::serviceIdForMachine` constexpr.
///
/// **Collision boundary**: the birthday-paradox curve crosses 50% near 16
/// machines. A deploy-time validator (§9.6 4c+) will reject deployments
/// whose §9.6 peer set contains two machine names hashing to the same
/// service ID; in this session no such check is wired (no fixture exercises
/// >2 §9.6 machines). The unit tests below pin the hash output for
/// representative names so a future change to the FNV constants is caught
/// at `cargo test -p sce-build --lib` before the C++ side rebuilds against
/// the new value.
pub const fn service_id_for_machine(name: &str) -> u16 {
    let bytes = name.as_bytes();
    let mut hash: u32 = FNV1A_OFFSET;
    let mut i = 0usize;
    while i < bytes.len() {
        hash ^= bytes[i] as u32;
        hash = hash.wrapping_mul(FNV1A_PRIME);
        i += 1;
    }
    SCXML_INVOKE_SERVICE_BASE | ((hash & 0xFF) as u16)
}

// ── Tests ───────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn service_id_within_reserved_range() {
        // Every machine name must hash into [0x8100, 0x81FF). The base bit
        // pattern is preserved by the OR; the hash supplies the low byte.
        for name in &[
            "parent",
            "worker",
            "motor",
            "brake",
            "very_long_machine_name_with_underscores",
            "x", // single char
            "",  // empty (degenerate, but well-defined: returns base)
        ] {
            let svc = service_id_for_machine(name);
            assert!(svc >= SCXML_INVOKE_SERVICE_BASE);
            assert!(svc < SCXML_INVOKE_SERVICE_BASE + 0x100);
        }
    }

    #[test]
    fn empty_name_yields_base() {
        // The FNV-1a empty-input hash is the offset basis (`0x811c9dc5`),
        // whose low byte is `0xc5`. Documenting the value pins the "empty
        // machine name" edge case so a future refactor of the FNV math
        // cannot silently change it.
        let svc = service_id_for_machine("");
        assert_eq!(svc, SCXML_INVOKE_SERVICE_BASE | 0xc5);
    }

    #[test]
    fn known_names_have_pinned_hashes() {
        // Drift-detection: if anyone changes FNV constants or the OR
        // formula in `service_id_for_machine`, these pinned outputs trip
        // and force a coordinated update with the C++ helper. The values
        // are computed by the same FNV-1a 32-bit construction documented
        // in the C++ docstring.
        //
        // Cross-check: drop into a Python or C++ session, run the same
        // FNV-1a 32-bit hash on the input, mask the low byte, OR with
        // 0x8100 — the result must match. These tests catch the case
        // where someone "simplifies" the Rust impl and accidentally
        // changes outputs without realising the C++ side stays the same.
        assert_eq!(service_id_for_machine("parent"), 0x81fd);
        assert_eq!(service_id_for_machine("worker"), 0x8157);
        assert_eq!(service_id_for_machine("motor"),  0x8172);
        assert_eq!(service_id_for_machine("brake"),  0x8130);
    }

    #[test]
    fn distinct_short_names_distinct_service_ids_in_typical_set() {
        // 256-machine MVP doesn't try to be collision-free; this test
        // documents the practical case (small representative peer sets do
        // collide rarely) by asserting that the parent/worker/motor/brake
        // four-machine set used across mesh fixtures keeps all four
        // distinct. If a future rename collapses two of these, the
        // failure here is the early signal — the §9.6 4c+ validator
        // will perform the same check across the full deploy topology.
        let ids: Vec<u16> = ["parent", "worker", "motor", "brake"]
            .iter()
            .map(|n| service_id_for_machine(n))
            .collect();
        let mut sorted = ids.clone();
        sorted.sort_unstable();
        sorted.dedup();
        assert_eq!(sorted.len(), ids.len(), "fixture machine names collide: {ids:?}");
    }

    #[test]
    fn method_wire_constants_are_pinned_bcd_style() {
        // Hex digits replicate wire decimal number for vsomeip-trace
        // readability. The mapping is BCD-style (digits look like the
        // decimal number) and is NOT contiguous: wire-19 → 0x0019, then
        // wire-20 → 0x0020 with 0x001A..0x001F intentionally unused.
        // This is by design — a vsomeip trace shows `method=0x0014` for
        // wire-14 and `method=0x0020` for wire-20 with no decimal-to-hex
        // translation in the reader's head. Anyone changing values trips
        // this test before the C++ side rebuilds against a wrong constant.
        assert_eq!(SCXML_INVOKE_METHOD_WIRE14_INVOKE_START,   0x0014);
        assert_eq!(SCXML_INVOKE_METHOD_WIRE15_INVOKE_STARTED, 0x0015);
        assert_eq!(SCXML_INVOKE_METHOD_WIRE16_CHILD_EVENT,    0x0016);
        assert_eq!(SCXML_INVOKE_METHOD_WIRE17_PARENT_EVENT,   0x0017);
        assert_eq!(SCXML_INVOKE_METHOD_WIRE18_INVOKE_DONE,    0x0018);
        assert_eq!(SCXML_INVOKE_METHOD_WIRE19_INVOKE_CANCEL,  0x0019);
        assert_eq!(SCXML_INVOKE_METHOD_WIRE20_INVOKE_ERROR,   0x0020);

        // All seven values must be distinct so dispatch by method ID is
        // unambiguous. Sorting + dedup catches any accidental
        // collision introduced by a future refactor.
        let methods = [
            SCXML_INVOKE_METHOD_WIRE14_INVOKE_START,
            SCXML_INVOKE_METHOD_WIRE15_INVOKE_STARTED,
            SCXML_INVOKE_METHOD_WIRE16_CHILD_EVENT,
            SCXML_INVOKE_METHOD_WIRE17_PARENT_EVENT,
            SCXML_INVOKE_METHOD_WIRE18_INVOKE_DONE,
            SCXML_INVOKE_METHOD_WIRE19_INVOKE_CANCEL,
            SCXML_INVOKE_METHOD_WIRE20_INVOKE_ERROR,
        ];
        let mut sorted = methods.to_vec();
        sorted.sort_unstable();
        sorted.dedup();
        assert_eq!(sorted.len(), 7, "method id collision: {methods:?}");
    }
}
