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
//! * **Validator hook** — §9.6 4c collision detection (Session 4c) calls
//!   [`service_id_for_machine`] from
//!   [`crate::mesh::deploy::parse_deploy_str`] via
//!   [`validate_someip_scxml_invoke_service_id_collisions`](
//!   crate::mesh::deploy::validate_someip_scxml_invoke_service_id_collisions
//!   ) to reject deployments that hash two §9.6 someip peers to the same
//!   low-byte service ID.
//!
//! The codegen template still emits `SCE::Mesh::Someip::serviceIdForMachine`
//! calls directly (deploy-time validator catches collisions at the
//! deploy.yaml boundary; codegen does not need to revalidate). The
//! [`find_colliding_pair`] test helper underwrites the validator's
//! adversarial fixture so a future drift in FNV constants is caught before
//! the C++ side rebuilds against new values.

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
/// machines. The §9.6 Session 4c deploy-time validator
/// ([`crate::mesh::deploy::validate_someip_scxml_invoke_service_id_collisions`])
/// rejects deployments whose §9.6 peer set contains two machine names
/// hashing to the same service ID, so the 256-ID range is observable as a
/// build-time invariant rather than a runtime mis-routing surprise. The
/// unit tests below pin the hash output for representative names so a
/// future change to the FNV constants is caught at
/// `cargo test -p sce-build --lib` before the C++ side rebuilds against
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

// ── Test-only collision discovery helper ───────────────────────────────────

/// Brute-force search for a colliding pair under [`service_id_for_machine`],
/// over short alphanumeric machine names. Used by the §9.6 4c collision
/// validator's adversarial fixture: rather than hard-coding a magic pair
/// of names that happens to collide today, the fixture asks at runtime for
/// "any pair the current FNV constants collapse to the same service ID".
///
/// **Why this shape**: 4-character lowercase alphanumeric (36⁴ ≈ 1.68 M
/// candidates) is far above the 256-slot capacity of the FNV-low-byte
/// projection — the birthday bound says the first collision must surface
/// within roughly the first ~20 enumerated names, not millions. Returning
/// the first pair encountered by a deterministic enumeration order makes
/// the fixture reproducible across machines and runs.
///
/// If a future refactor accidentally widened the projection (e.g. to a
/// full u32 or a 16-bit subrange), this helper would not find a collision
/// in the bounded search space and the adversarial fixture would fail with
/// `expect()` — the failure surface is a *louder* signal than a silently
/// passing collision-free invariant.
///
/// Test-only — production code does not enumerate names; the validator
/// reads the deploy.yaml's declared participant set instead.
#[cfg(test)]
pub(crate) fn find_colliding_pair() -> Option<(String, String)> {
    // Lowercase letters + digits; 36 chars yields predictable enumeration
    // and avoids any case-sensitivity confusion when the pair is later
    // round-tripped through deploy.yaml as a machine name.
    const ALPHABET: &[u8] = b"abcdefghijklmnopqrstuvwxyz0123456789";
    const LEN: usize = 4;

    let mut seen: std::collections::HashMap<u16, String> =
        std::collections::HashMap::with_capacity(256);

    // Lex-ordered enumeration over LEN-character strings: index in base
    // ALPHABET.len() through ALPHABET.len()^LEN - 1. The first colliding
    // pair (under deterministic iteration order) is returned.
    let total = (ALPHABET.len() as u64).pow(LEN as u32);
    let mut buf = [0u8; LEN];
    for n in 0..total {
        let mut x = n;
        for i in (0..LEN).rev() {
            buf[i] = ALPHABET[(x as usize) % ALPHABET.len()];
            x /= ALPHABET.len() as u64;
        }
        let name = std::str::from_utf8(&buf).expect("ALPHABET is ASCII");
        let svc = service_id_for_machine(name);
        if let Some(prev) = seen.get(&svc) {
            return Some((prev.clone(), name.to_string()));
        }
        seen.insert(svc, name.to_string());
    }
    None
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
    fn find_colliding_pair_locates_collision_within_search_space() {
        // §9.6 4c adversarial-fixture seed: the validator's collision
        // test cannot hard-code a "known colliding pair" — that would
        // bake the FNV constants into the test, and a future drift
        // would silently produce a non-colliding pair without trip.
        // Instead the test asks this helper at runtime for any pair
        // the current FNV projection collapses to the same low-byte
        // service ID.
        //
        // The 4-char lowercase alphanumeric search space is 36⁴ ≈
        // 1.68 M; the FNV-low-byte projection has only 256 slots, so
        // by birthday paradox the helper must encounter a collision
        // very quickly (in practice within the first few dozen
        // enumerated names). If this test ever fails, either the
        // alphabet/length parameters changed in a way that no longer
        // covers the slot space, or the projection was widened past
        // 256 slots — either way the validator's adversarial fixture
        // is no longer reliable and needs review.
        let (a, b) =
            find_colliding_pair().expect("FNV-1a low-byte projection must collide in 4-char space");
        assert_ne!(a, b, "collision pair must contain two distinct names");
        assert_eq!(
            service_id_for_machine(&a),
            service_id_for_machine(&b),
            "names returned by find_colliding_pair must hash to the same service ID"
        );
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
