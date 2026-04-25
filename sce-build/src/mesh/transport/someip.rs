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
//!      OEM-declared application. The consolidated `<machine>[_<partition>]_sce_app_`
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
//! ## What this Rust module owns (RFC F.X-1)
//!
//! The hybrid (counter + optional author-pin) allocator
//! ([`assign_invoke_service_ids`]) is the source of truth for §9.6 SOMEIP
//! scxml-invoke service IDs. It is invoked once per build by
//! [`crate::mesh::deploy::assign_someip_invoke_service_ids`] over the full
//! deploy participant set, producing a deterministic
//! `BTreeMap<machine_name, service_id>` that codegen consumes via
//! `mesh::codegen` and emits as named constants in the generated
//! `<machine>_transport.h` (`SCE_SOMEIP_SERVICE_SELF` and
//! `SCE_SOMEIP_SERVICE_PEER_<peer_name>`). The deploy-time validator
//! ([`crate::mesh::deploy::validate_someip_scxml_invoke_service_ids`])
//! shares this assignment to reject overflow / pin-out-of-range /
//! pin-vs-pin-collision configurations at parse time.
//!
//! The legacy FNV-1a-low-byte derivation
//! (`service_id_for_machine` constexpr / `serviceIdForMachine` C++ helper)
//! has been deleted: the counter scheme is collision-free up to the
//! 128-slot ceiling of the §9.6 invoke sub-range `[0x8100, 0x817F]`, and
//! the upper half `[0x8180, 0x81FF]` is reserved for the §16.4
//! region-liveness landing (RFC F.X-3).

// ── SCE-reserved §9.6 namespace constants ───────────────────────────────────

/// Base of the SCE-reserved §9.6 scxml-invoke service ID range. SCE-managed
/// services occupy `[SCXML_INVOKE_SERVICE_BASE, SCXML_INVOKE_SERVICE_BASE +
/// 0x100)` (`0x8100..0x81FF`). Mirror of the C++ `SCXML_INVOKE_SERVICE_BASE`
/// constexpr in `SomeipScxmlInvokeEndpoint.h`.
pub const SCXML_INVOKE_SERVICE_BASE: u16 = 0x8100;

/// Inclusive ceiling of the §9.6 scxml-invoke service ID sub-range under the
/// hybrid (counter + optional pin) allocator (RFC F.X-1). The sub-range is
/// `[0x8100, 0x817F]` — 128 slots. The upper half `[0x8180, 0x81FF]` of the
/// SCE-reserved 256-slot space is reserved for the §16.4 region-liveness
/// landing (F.X-3); subsystem range partitioning gives F.X-1 invoke IDs
/// stability across F.X-3's later landing without requiring author pins.
pub const SCXML_INVOKE_SERVICE_CEILING: u16 = 0x817F;

/// Number of slots in the §9.6 invoke sub-range (`SCXML_INVOKE_SERVICE_CEILING -
/// SCXML_INVOKE_SERVICE_BASE + 1`). Used as the overflow ceiling by
/// [`assign_invoke_service_ids`] and named in the
/// [`crate::mesh::error::DeployError::SomeipScxmlInvokeServiceIdOverflow`]
/// diagnostic so operators see the exact bound their participant count
/// crossed.
pub const SCXML_INVOKE_SERVICE_RANGE_SIZE: usize =
    (SCXML_INVOKE_SERVICE_CEILING - SCXML_INVOKE_SERVICE_BASE + 1) as usize;

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

// ── Hybrid (counter + optional pin) service ID assigner ────────────────────

/// Errors from the hybrid §9.6 SOMEIP scxml-invoke service ID assigner
/// ([`assign_invoke_service_ids`]). Each variant carries the operator-facing
/// payload needed to reach a clear deploy.yaml fix; the deploy-layer
/// validator
/// ([`crate::mesh::deploy::validate_someip_scxml_invoke_service_ids`])
/// converts these into [`crate::mesh::error::DeployError`] variants of the
/// same shape so the diagnostic chain stays typed end-to-end.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AssignInvokeServiceIdError {
    /// Total participant count exceeds [`SCXML_INVOKE_SERVICE_RANGE_SIZE`].
    /// Counter scheme is collision-free up to the ceiling, so the only
    /// overflow shape is "too many participants".
    Overflow {
        participant_count: usize,
        ceiling: usize,
    },
    /// A pinned ID falls outside the invoke sub-range
    /// `[SCXML_INVOKE_SERVICE_BASE, SCXML_INVOKE_SERVICE_CEILING]`. Pins
    /// outside the sub-range would either collide with the reserved
    /// liveness range (F.X-3) or with OEM-owned `[0x0000, 0x80FF]` /
    /// `[0x8200, 0xFFFF]` space.
    PinOutOfRange {
        machine: String,
        pinned_id: u16,
        range_lo: u16,
        range_hi: u16,
    },
    /// Two or more machines pinned the same ID. Author error — operator
    /// fix is to repick one of the pins.
    PinCollision {
        machines: Vec<String>,
        pinned_id: u16,
    },
}

/// Hybrid (counter + optional author-pin) assigner for §9.6 SOMEIP
/// scxml-invoke service IDs. Per RFC F.X-1
/// (`claudedocs/rfc-someip-service-id-counter.md`).
///
/// **Input.** `participants` maps each canonical participant's machine name
/// to its optional pin from deploy.yaml `someip_service_id:`. The map's keys
/// ARE the canonical participant set — the caller (deploy layer) is
/// responsible for collecting them from the deploy.yaml structure (every
/// machine that declares a peer-shape `bindings["#X"].transport: someip` for
/// a declared peer `X`, plus the named peers themselves; same definition as
/// the legacy 4c collision validator's participant projection).
///
/// **Output.** A map from each participant's machine name to its assigned
/// service ID. Pinned machines get their pinned ID; un-pinned machines get
/// the lowest unreserved slot in lex order starting from
/// [`SCXML_INVOKE_SERVICE_BASE`]. The output is collision-free by
/// construction: pinned and auto-assigned slots are disjoint because the
/// auto-assignment counter skips slots already claimed by pins.
///
/// **Determinism.** The assignment is fully deterministic given a fixed
/// participant set: `BTreeMap` iteration is lex-sorted, so re-running the
/// assigner against the same input always produces the same output. Re-runs
/// across deploy.yaml edits that don't change the *participant set* (e.g.
/// reordering the YAML, renaming a non-participant) preserve the
/// assignment.
pub fn assign_invoke_service_ids(
    participants: &std::collections::BTreeMap<String, Option<u16>>,
) -> Result<std::collections::BTreeMap<String, u16>, AssignInvokeServiceIdError> {
    use std::collections::{BTreeMap, BTreeSet};

    // 1. Overflow gate. Counter scheme cannot fit more than RANGE_SIZE
    //    participants regardless of pin shape, so reject up front. The
    //    pin checks below assume the participant count is feasible.
    if participants.len() > SCXML_INVOKE_SERVICE_RANGE_SIZE {
        return Err(AssignInvokeServiceIdError::Overflow {
            participant_count: participants.len(),
            ceiling: SCXML_INVOKE_SERVICE_RANGE_SIZE,
        });
    }

    // 2. Pin range + collision validation. Reject any pin outside the
    //    invoke sub-range; group remaining pins by id to detect duplicates.
    //    BTreeMap keeps the diagnostic deterministic for byte-stable
    //    error fixtures.
    let mut by_pin: BTreeMap<u16, Vec<String>> = BTreeMap::new();
    for (name, maybe_pin) in participants.iter() {
        if let Some(pin) = maybe_pin {
            if *pin < SCXML_INVOKE_SERVICE_BASE || *pin > SCXML_INVOKE_SERVICE_CEILING {
                return Err(AssignInvokeServiceIdError::PinOutOfRange {
                    machine: name.clone(),
                    pinned_id: *pin,
                    range_lo: SCXML_INVOKE_SERVICE_BASE,
                    range_hi: SCXML_INVOKE_SERVICE_CEILING,
                });
            }
            by_pin.entry(*pin).or_default().push(name.clone());
        }
    }
    for (pinned_id, machines) in by_pin.iter() {
        if machines.len() >= 2 {
            return Err(AssignInvokeServiceIdError::PinCollision {
                machines: machines.clone(),
                pinned_id: *pinned_id,
            });
        }
    }

    // 3. Reserved set: every pin claims its slot. Counter must skip these.
    let reserved: BTreeSet<u16> = by_pin.keys().copied().collect();

    // 4. Walk participants in lex order. Pinned → use the pin verbatim.
    //    Un-pinned → consume the lowest unreserved slot. The participant-
    //    count overflow gate above guarantees the counter never escapes
    //    the sub-range: total claims = participants.len() <= RANGE_SIZE,
    //    and reserved + auto = participants.len() (disjoint), so the
    //    auto-assignment cannot collide with any pin or run off the end.
    let mut out: BTreeMap<String, u16> = BTreeMap::new();
    let mut next_slot: u16 = SCXML_INVOKE_SERVICE_BASE;
    for (name, maybe_pin) in participants.iter() {
        if let Some(pin) = maybe_pin {
            out.insert(name.clone(), *pin);
            continue;
        }
        while reserved.contains(&next_slot) {
            next_slot += 1;
        }
        debug_assert!(
            next_slot <= SCXML_INVOKE_SERVICE_CEILING,
            "auto-assignment escaped invoke sub-range — overflow gate or pin-reserve invariant broken"
        );
        out.insert(name.clone(), next_slot);
        next_slot += 1;
    }

    Ok(out)
}

// ── Tests ───────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

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

    // ── Hybrid assigner tests (RFC F.X-1) ──────────────────────────────────

    use std::collections::BTreeMap;

    fn p(entries: &[(&str, Option<u16>)]) -> BTreeMap<String, Option<u16>> {
        entries.iter().map(|(n, p)| (n.to_string(), *p)).collect()
    }

    #[test]
    fn assigner_empty_set_returns_empty_map() {
        let out = assign_invoke_service_ids(&p(&[])).expect("empty must succeed");
        assert!(out.is_empty());
    }

    #[test]
    fn assigner_all_unpinned_walks_counter_from_base() {
        // Lex order: alpha, bravo, charlie. IDs 0x8100, 0x8101, 0x8102.
        let out = assign_invoke_service_ids(&p(&[
            ("bravo", None),
            ("alpha", None),
            ("charlie", None),
        ]))
        .expect("unpinned-only must succeed");
        assert_eq!(out.get("alpha").copied(), Some(0x8100));
        assert_eq!(out.get("bravo").copied(), Some(0x8101));
        assert_eq!(out.get("charlie").copied(), Some(0x8102));
    }

    #[test]
    fn assigner_all_pinned_uses_pins_verbatim() {
        let out = assign_invoke_service_ids(&p(&[
            ("alpha", Some(0x8123)),
            ("bravo", Some(0x817F)),
        ]))
        .expect("all-pinned must succeed");
        assert_eq!(out.get("alpha").copied(), Some(0x8123));
        assert_eq!(out.get("bravo").copied(), Some(0x817F));
    }

    #[test]
    fn assigner_mix_skips_pinned_slots() {
        // alpha pins 0x8101 (would be bravo's natural slot). Counter must
        // skip 0x8101: alpha=0x8101 (pin), bravo=0x8100, charlie=0x8102,
        // delta=0x8103. Lex order: alpha, bravo, charlie, delta.
        let out = assign_invoke_service_ids(&p(&[
            ("alpha", Some(0x8101)),
            ("bravo", None),
            ("charlie", None),
            ("delta", None),
        ]))
        .expect("mixed must succeed");
        assert_eq!(out.get("alpha").copied(), Some(0x8101));
        assert_eq!(out.get("bravo").copied(), Some(0x8100));
        assert_eq!(out.get("charlie").copied(), Some(0x8102));
        assert_eq!(out.get("delta").copied(), Some(0x8103));
    }

    #[test]
    fn assigner_is_deterministic_under_yaml_reordering() {
        // Same participant set in two different insertion orders into
        // BTreeMap must produce identical output. (BTreeMap iteration is
        // lex-sorted, so this is a property of the assigner using the
        // BTree order rather than the call-site order.)
        let a = p(&[("alpha", None), ("bravo", None), ("charlie", None)]);
        let b = p(&[("charlie", None), ("alpha", None), ("bravo", None)]);
        assert_eq!(
            assign_invoke_service_ids(&a).unwrap(),
            assign_invoke_service_ids(&b).unwrap()
        );
    }

    #[test]
    fn assigner_overflow_rejects_at_ceiling_plus_one() {
        // 129 unpinned participants — one over the 128-slot ceiling. Reject.
        let mut input: BTreeMap<String, Option<u16>> = BTreeMap::new();
        for i in 0..(SCXML_INVOKE_SERVICE_RANGE_SIZE + 1) {
            input.insert(format!("m{i:04}"), None);
        }
        match assign_invoke_service_ids(&input) {
            Err(AssignInvokeServiceIdError::Overflow {
                participant_count,
                ceiling,
            }) => {
                assert_eq!(participant_count, SCXML_INVOKE_SERVICE_RANGE_SIZE + 1);
                assert_eq!(ceiling, SCXML_INVOKE_SERVICE_RANGE_SIZE);
            }
            other => panic!("expected Overflow, got {other:?}"),
        }
    }

    #[test]
    fn assigner_overflow_at_ceiling_exactly_succeeds() {
        // 128 participants — exactly fills the range, no overflow.
        let mut input: BTreeMap<String, Option<u16>> = BTreeMap::new();
        for i in 0..SCXML_INVOKE_SERVICE_RANGE_SIZE {
            input.insert(format!("m{i:04}"), None);
        }
        let out = assign_invoke_service_ids(&input).expect("exact-fill must succeed");
        assert_eq!(out.len(), SCXML_INVOKE_SERVICE_RANGE_SIZE);
        // Highest assigned ID equals the ceiling.
        let max = *out.values().max().unwrap();
        assert_eq!(max, SCXML_INVOKE_SERVICE_CEILING);
    }

    #[test]
    fn assigner_pin_below_range_rejected() {
        let out = assign_invoke_service_ids(&p(&[("alpha", Some(0x80FF))]));
        match out {
            Err(AssignInvokeServiceIdError::PinOutOfRange {
                machine,
                pinned_id,
                range_lo,
                range_hi,
            }) => {
                assert_eq!(machine, "alpha");
                assert_eq!(pinned_id, 0x80FF);
                assert_eq!(range_lo, SCXML_INVOKE_SERVICE_BASE);
                assert_eq!(range_hi, SCXML_INVOKE_SERVICE_CEILING);
            }
            other => panic!("expected PinOutOfRange (below), got {other:?}"),
        }
    }

    #[test]
    fn assigner_pin_above_range_rejected() {
        // 0x8180 is the first slot of the F.X-3 liveness range — out of
        // range for invoke participants.
        let out = assign_invoke_service_ids(&p(&[("alpha", Some(0x8180))]));
        match out {
            Err(AssignInvokeServiceIdError::PinOutOfRange {
                machine,
                pinned_id,
                range_lo,
                range_hi,
            }) => {
                assert_eq!(machine, "alpha");
                assert_eq!(pinned_id, 0x8180);
                assert_eq!(range_lo, SCXML_INVOKE_SERVICE_BASE);
                assert_eq!(range_hi, SCXML_INVOKE_SERVICE_CEILING);
            }
            other => panic!("expected PinOutOfRange (above), got {other:?}"),
        }
    }

    #[test]
    fn assigner_pin_collision_rejected() {
        // Two machines pin the same ID — operator error.
        let out = assign_invoke_service_ids(&p(&[
            ("alpha", Some(0x8105)),
            ("bravo", Some(0x8105)),
        ]));
        match out {
            Err(AssignInvokeServiceIdError::PinCollision {
                machines,
                pinned_id,
            }) => {
                assert_eq!(pinned_id, 0x8105);
                // Order is BTreeMap lex (alpha < bravo).
                assert_eq!(machines, vec!["alpha".to_string(), "bravo".to_string()]);
            }
            other => panic!("expected PinCollision, got {other:?}"),
        }
    }

    #[test]
    fn assigner_unpinned_does_not_collide_with_pinned() {
        // Stress: 128 participants total, half pinned at the high end,
        // half un-pinned. Auto must fill the low end without colliding.
        let mut input: BTreeMap<String, Option<u16>> = BTreeMap::new();
        // 64 pins occupying the upper half of the range.
        for i in 0..64 {
            input.insert(
                format!("p{i:03}"),
                Some(SCXML_INVOKE_SERVICE_BASE + 64 + i as u16),
            );
        }
        // 64 un-pinned participants.
        for i in 0..64 {
            input.insert(format!("a{i:03}"), None);
        }
        let out = assign_invoke_service_ids(&input).expect("128-participant mix must succeed");
        assert_eq!(out.len(), 128);
        // All assignments unique (collision-free invariant).
        let mut ids: Vec<u16> = out.values().copied().collect();
        ids.sort_unstable();
        ids.dedup();
        assert_eq!(ids.len(), 128, "auto-assignment collided with a pin");
        // All assignments inside the sub-range.
        for sid in out.values() {
            assert!(*sid >= SCXML_INVOKE_SERVICE_BASE);
            assert!(*sid <= SCXML_INVOKE_SERVICE_CEILING);
        }
    }
}
