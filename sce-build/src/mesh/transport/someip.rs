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
//! ## What this Rust module owns (RFC F.X-1, RFC F.X-3)
//!
//! Two parallel hybrid (counter + optional author-pin) allocators occupy
//! disjoint sub-ranges of the SCE-reserved §9.6 256-slot space:
//!
//! - [`assign_invoke_service_ids`] (RFC F.X-1) — `[0x8100, 0x817F]`.
//!   Source of truth for §9.6 SOMEIP scxml-invoke service IDs. Invoked
//!   once per build by [`crate::mesh::deploy::assign_someip_invoke_service_ids`]
//!   over the full deploy participant set; produces a deterministic
//!   `BTreeMap<machine_name, service_id>` that codegen consumes via
//!   `mesh::codegen` and emits as named constants in the generated
//!   `<machine>_transport.h` (`SCE_SOMEIP_SERVICE_SELF` and
//!   `SCE_SOMEIP_SERVICE_PEER_<peer_name>`). The deploy-time validator
//!   ([`crate::mesh::deploy::validate_someip_scxml_invoke_service_ids`])
//!   shares this assignment to reject overflow / pin-out-of-range /
//!   pin-vs-pin-collision configurations at parse time.
//! - [`assign_liveness_service_ids`] (RFC F.X-3) — `[0x8180, 0x81FF]`.
//!   Source of truth for §16.4 region-partition liveness service IDs.
//!   Same hybrid pattern, disjoint range, partition-keyed participants
//!   (`<machine>__P__<partition>`). The deploy-time validator
//!   ([`crate::mesh::deploy::validate_someip_liveness_service_ids`])
//!   surfaces overflow / pin-out-of-range / pin-vs-pin-collision with
//!   the same three-shape diagnostic family as F.X-1.
//!
//! The legacy FNV-1a-low-byte derivation
//! (`service_id_for_machine` constexpr / `serviceIdForMachine` C++ helper)
//! has been deleted: the counter scheme is collision-free up to each
//! sub-range's 128-slot ceiling, and the disjoint partition gives
//! cross-subsystem stability without requiring author pins.

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

/// Base of the SCE-reserved §16.4 region-partition liveness service ID
/// sub-range (RFC F.X-3). The upper half of the SCE-reserved 256-slot
/// space, disjoint from [`SCXML_INVOKE_SERVICE_BASE`]'s lower half so
/// liveness participants never shift invoke participants' auto-assigned
/// slots when the deploy participant set changes.
pub const SCXML_LIVENESS_SERVICE_BASE: u16 = 0x8180;

/// Inclusive ceiling of the §16.4 region-partition liveness service ID
/// sub-range under the hybrid (counter + optional pin) allocator
/// (RFC F.X-3). The sub-range is `[0x8180, 0x81FF]` — 128 slots.
pub const SCXML_LIVENESS_SERVICE_CEILING: u16 = 0x81FF;

/// Number of slots in the §16.4 liveness sub-range
/// (`SCXML_LIVENESS_SERVICE_CEILING - SCXML_LIVENESS_SERVICE_BASE + 1`).
/// Used as the overflow ceiling by [`assign_liveness_service_ids`] and
/// named in the
/// [`crate::mesh::error::DeployError::SomeipLivenessServiceIdOverflow`]
/// diagnostic so operators see the exact bound their participant count
/// crossed.
pub const SCXML_LIVENESS_SERVICE_RANGE_SIZE: usize =
    (SCXML_LIVENESS_SERVICE_CEILING - SCXML_LIVENESS_SERVICE_BASE + 1) as usize;

/// Base of the SCE-reserved §16.7 row 8 machine-level liveness service ID
/// sub-range (RFC F.X-4). Disjoint from F.X-1 invoke (`[0x8100, 0x817F]`)
/// and F.X-3 region-liveness (`[0x8180, 0x81FF]`); the gap `[0x8200, 0x827F]`
/// is reserved as documented headroom for a future fourth SCE subsystem
/// (heartbeat / server-pool liveness / etc.) so F.X-4's commit does not
/// have to predict where that axis goes.
pub const SCXML_MACHINE_LIVENESS_SERVICE_BASE: u16 = 0x8280;

/// Inclusive ceiling of the §16.7 row 8 machine-level liveness service ID
/// sub-range under the hybrid (counter + optional pin) allocator
/// (RFC F.X-4). The sub-range is `[0x8280, 0x82FF]` — 128 slots.
pub const SCXML_MACHINE_LIVENESS_SERVICE_CEILING: u16 = 0x82FF;

/// Number of slots in the §16.7 row 8 machine-liveness sub-range
/// (`SCXML_MACHINE_LIVENESS_SERVICE_CEILING - SCXML_MACHINE_LIVENESS_SERVICE_BASE + 1`).
/// Used as the overflow ceiling by [`assign_machine_liveness_service_ids`]
/// and named in the
/// [`crate::mesh::error::DeployError::SomeipMachineLivenessServiceIdOverflow`]
/// diagnostic so operators see the exact bound their participant count
/// crossed.
pub const SCXML_MACHINE_LIVENESS_SERVICE_RANGE_SIZE: usize =
    (SCXML_MACHINE_LIVENESS_SERVICE_CEILING - SCXML_MACHINE_LIVENESS_SERVICE_BASE + 1) as usize;

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

// ── Shared range allocator (RFC F.X-4 D2: extracted at third-consumer threshold) ──

/// Hybrid (counter + optional author-pin) range allocator shared across
/// the three SCE-reserved SOMEIP service-ID subsystems (RFC F.X-1 invoke,
/// RFC F.X-3 region-liveness, RFC F.X-4 machine-liveness). Each subsystem
/// passes its own `[base, ceiling]` sub-range and error-enum constructors;
/// the body is identical across subsystems because F.X-1 D2's
/// cross-subsystem stability property makes the algorithm range-agnostic.
///
/// **Determinism.** `BTreeMap` iteration is lex-sorted; re-running with
/// the same input produces the same output across builds and across deploy
/// graph edits that don't change the participant set.
///
/// **Collision-free by construction.** Pin slots are reserved before the
/// counter advances; the overflow gate up front guarantees the counter
/// never escapes the sub-range.
///
/// **Why generic over `E` (instead of returning a shared enum).** Each
/// subsystem's public error enum carries operator-facing payload field
/// names that match its participant kind (`machine` / `partition_key` /
/// future machine-key) and references its own range constants in the
/// rustdoc. Distinct enums keep the diagnostic chain typed end-to-end at
/// the subsystem boundary; the helper accepts thin closure constructors
/// so each public wrapper builds its own variant.
fn range_alloc<E>(
    participants: &std::collections::BTreeMap<String, Option<u16>>,
    base: u16,
    ceiling: u16,
    err_overflow: impl FnOnce(usize, usize) -> E,
    err_pin_out_of_range: impl FnOnce(String, u16, u16, u16) -> E,
    err_pin_collision: impl FnOnce(Vec<String>, u16) -> E,
) -> Result<std::collections::BTreeMap<String, u16>, E> {
    use std::collections::{BTreeMap, BTreeSet};

    let range_size = (ceiling - base + 1) as usize;

    // 1. Overflow gate. Counter scheme cannot fit more than range_size
    //    participants regardless of pin shape, so reject up front. The
    //    pin checks below assume the participant count is feasible.
    if participants.len() > range_size {
        return Err(err_overflow(participants.len(), range_size));
    }

    // 2. Pin range + collision validation. Reject any pin outside the
    //    sub-range; group remaining pins by id to detect duplicates.
    //    BTreeMap keeps the diagnostic deterministic for byte-stable
    //    error fixtures.
    let mut by_pin: BTreeMap<u16, Vec<String>> = BTreeMap::new();
    for (key, maybe_pin) in participants.iter() {
        if let Some(pin) = maybe_pin {
            if *pin < base || *pin > ceiling {
                return Err(err_pin_out_of_range(key.clone(), *pin, base, ceiling));
            }
            by_pin.entry(*pin).or_default().push(key.clone());
        }
    }
    for (pinned_id, keys) in by_pin.iter() {
        if keys.len() >= 2 {
            return Err(err_pin_collision(keys.clone(), *pinned_id));
        }
    }

    // 3. Reserved set: every pin claims its slot. Counter must skip these.
    let reserved: BTreeSet<u16> = by_pin.keys().copied().collect();

    // 4. Walk participants in lex order. Pinned → use the pin verbatim.
    //    Un-pinned → consume the lowest unreserved slot. The participant-
    //    count overflow gate above guarantees the counter never escapes
    //    the sub-range: total claims = participants.len() <= range_size,
    //    and reserved + auto = participants.len() (disjoint), so the
    //    auto-assignment cannot collide with any pin or run off the end.
    let mut out: BTreeMap<String, u16> = BTreeMap::new();
    let mut next_slot: u16 = base;
    for (key, maybe_pin) in participants.iter() {
        if let Some(pin) = maybe_pin {
            out.insert(key.clone(), *pin);
            continue;
        }
        while reserved.contains(&next_slot) {
            next_slot += 1;
        }
        debug_assert!(
            next_slot <= ceiling,
            "auto-assignment escaped sub-range — overflow gate or pin-reserve invariant broken"
        );
        out.insert(key.clone(), next_slot);
        next_slot += 1;
    }

    Ok(out)
}

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
/// scxml-invoke service IDs. Per RFC F.X-1.
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
    range_alloc(
        participants,
        SCXML_INVOKE_SERVICE_BASE,
        SCXML_INVOKE_SERVICE_CEILING,
        |participant_count, ceiling| AssignInvokeServiceIdError::Overflow {
            participant_count,
            ceiling,
        },
        |machine, pinned_id, range_lo, range_hi| AssignInvokeServiceIdError::PinOutOfRange {
            machine,
            pinned_id,
            range_lo,
            range_hi,
        },
        |machines, pinned_id| AssignInvokeServiceIdError::PinCollision {
            machines,
            pinned_id,
        },
    )
}

// ── Hybrid (counter + optional pin) §16.4 region-liveness assigner ─────────

/// Errors from the hybrid §16.4 region-partition liveness service ID
/// assigner ([`assign_liveness_service_ids`]). Each variant carries the
/// operator-facing payload needed to reach a clear deploy.yaml fix; the
/// deploy-layer validator
/// ([`crate::mesh::deploy::validate_someip_liveness_service_ids`])
/// converts these into [`crate::mesh::error::DeployError`] variants of the
/// same shape so the diagnostic chain stays typed end-to-end.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AssignLivenessServiceIdError {
    /// Total participant count exceeds [`SCXML_LIVENESS_SERVICE_RANGE_SIZE`].
    /// Counter scheme is collision-free up to the ceiling, so the only
    /// overflow shape is "too many participants".
    Overflow {
        participant_count: usize,
        ceiling: usize,
    },
    /// A pinned ID falls outside the liveness sub-range
    /// `[SCXML_LIVENESS_SERVICE_BASE, SCXML_LIVENESS_SERVICE_CEILING]`.
    /// Pins outside the sub-range would either collide with the reserved
    /// invoke range (F.X-1) or with OEM-owned `[0x0000, 0x80FF]` /
    /// `[0x8200, 0xFFFF]` space.
    PinOutOfRange {
        partition_key: String,
        pinned_id: u16,
        range_lo: u16,
        range_hi: u16,
    },
    /// Two or more partitions pinned the same ID. Author error — operator
    /// fix is to repick one of the pins.
    PinCollision {
        partition_keys: Vec<String>,
        pinned_id: u16,
    },
}

/// Hybrid (counter + optional author-pin) assigner for §16.4 region-partition
/// liveness service IDs. Per RFC F.X-3.
///
/// **Input.** `participants` maps each canonical liveness participant key
/// (`<machine>__P__<partition>`, see RFC F.X-3 D2) to its optional pin from
/// deploy.yaml `someip_liveness_service_id:` (per-partition, see D3). The
/// map's keys ARE the canonical participant set — the caller (deploy layer)
/// is responsible for collecting them from every partition belonging to a
/// machine that opts into `liveliness:` and uses SOME/IP transport.
///
/// **Output.** A map from each participant key to its assigned service ID.
/// Pinned partitions get their pinned ID; un-pinned partitions get the
/// lowest unreserved slot in lex order starting from
/// [`SCXML_LIVENESS_SERVICE_BASE`]. The output is collision-free by
/// construction — pinned slots are reserved before the counter advances,
/// and the overflow gate up front guarantees the counter never escapes
/// the sub-range.
///
/// **Determinism.** The output depends only on the keys + pin values of the
/// participant set: `BTreeMap` iteration is lex-sorted, so re-running the
/// assigner against the same input always produces the same output.
/// Re-runs across deploy.yaml edits that don't change the *participant set*
/// (e.g. reordering the YAML, renaming a non-participant) preserve the
/// assignment.
///
/// **Body shared with F.X-1 + F.X-4.** RFC F.X-4 D2 extracted [`range_alloc`]
/// at the third-consumer threshold; this function is now a thin wrapper
/// passing F.X-3's range constants + per-subsystem error constructors.
pub fn assign_liveness_service_ids(
    participants: &std::collections::BTreeMap<String, Option<u16>>,
) -> Result<std::collections::BTreeMap<String, u16>, AssignLivenessServiceIdError> {
    range_alloc(
        participants,
        SCXML_LIVENESS_SERVICE_BASE,
        SCXML_LIVENESS_SERVICE_CEILING,
        |participant_count, ceiling| AssignLivenessServiceIdError::Overflow {
            participant_count,
            ceiling,
        },
        |partition_key, pinned_id, range_lo, range_hi| {
            AssignLivenessServiceIdError::PinOutOfRange {
                partition_key,
                pinned_id,
                range_lo,
                range_hi,
            }
        },
        |partition_keys, pinned_id| AssignLivenessServiceIdError::PinCollision {
            partition_keys,
            pinned_id,
        },
    )
}

// ── Hybrid (counter + optional pin) §16.7 row 8 machine-liveness assigner ──

/// Errors from the hybrid §16.7 row 8 machine-level liveness service ID
/// assigner ([`assign_machine_liveness_service_ids`]). Each variant carries
/// the operator-facing payload needed to reach a clear deploy.yaml fix; the
/// deploy-layer validator
/// ([`crate::mesh::deploy::validate_someip_machine_liveness_service_ids`])
/// converts these into [`crate::mesh::error::DeployError`] variants of the
/// same shape so the diagnostic chain stays typed end-to-end.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AssignMachineLivenessServiceIdError {
    /// Total participant count exceeds [`SCXML_MACHINE_LIVENESS_SERVICE_RANGE_SIZE`].
    /// Counter scheme is collision-free up to the ceiling, so the only
    /// overflow shape is "too many participants".
    Overflow {
        participant_count: usize,
        ceiling: usize,
    },
    /// A pinned ID falls outside the machine-liveness sub-range
    /// `[SCXML_MACHINE_LIVENESS_SERVICE_BASE, SCXML_MACHINE_LIVENESS_SERVICE_CEILING]`.
    /// Pins below the sub-range collide with the F.X-3 region-liveness
    /// reservation or the F.X-1 invoke reservation; pins above escape the
    /// SCE-reserved namespace into OEM-owned space.
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

/// Hybrid (counter + optional author-pin) assigner for §16.7 row 8
/// machine-level liveness service IDs. Per RFC F.X-4.
///
/// **Input.** `participants` maps each canonical liveness participant key
/// (the machine name `<machine>`, see RFC F.X-4 D1) to its optional pin
/// from deploy.yaml `someip_machine_liveness_service_id:` (per-machine,
/// see D3). The map's keys ARE the canonical participant set — the caller
/// (deploy layer) is responsible for collecting them from every machine
/// that opts into `liveliness:` and uses SOME/IP transport.
///
/// **Output.** A map from each machine name to its assigned service ID in
/// the F.X-4 sub-range `[0x8280, 0x82FF]`. Disjoint from the F.X-1 invoke
/// sub-range and the F.X-3 region-liveness sub-range by construction.
///
/// **Body shared with F.X-1 + F.X-3.** Thin wrapper over [`range_alloc`]
/// (RFC F.X-4 D2) — same allocator body as the other two SOMEIP
/// service-ID subsystems with F.X-4's range constants + per-subsystem
/// error constructors.
pub fn assign_machine_liveness_service_ids(
    participants: &std::collections::BTreeMap<String, Option<u16>>,
) -> Result<std::collections::BTreeMap<String, u16>, AssignMachineLivenessServiceIdError> {
    range_alloc(
        participants,
        SCXML_MACHINE_LIVENESS_SERVICE_BASE,
        SCXML_MACHINE_LIVENESS_SERVICE_CEILING,
        |participant_count, ceiling| AssignMachineLivenessServiceIdError::Overflow {
            participant_count,
            ceiling,
        },
        |machine, pinned_id, range_lo, range_hi| {
            AssignMachineLivenessServiceIdError::PinOutOfRange {
                machine,
                pinned_id,
                range_lo,
                range_hi,
            }
        },
        |machines, pinned_id| AssignMachineLivenessServiceIdError::PinCollision {
            machines,
            pinned_id,
        },
    )
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
        assert_eq!(SCXML_INVOKE_METHOD_WIRE14_INVOKE_START, 0x0014);
        assert_eq!(SCXML_INVOKE_METHOD_WIRE15_INVOKE_STARTED, 0x0015);
        assert_eq!(SCXML_INVOKE_METHOD_WIRE16_CHILD_EVENT, 0x0016);
        assert_eq!(SCXML_INVOKE_METHOD_WIRE17_PARENT_EVENT, 0x0017);
        assert_eq!(SCXML_INVOKE_METHOD_WIRE18_INVOKE_DONE, 0x0018);
        assert_eq!(SCXML_INVOKE_METHOD_WIRE19_INVOKE_CANCEL, 0x0019);
        assert_eq!(SCXML_INVOKE_METHOD_WIRE20_INVOKE_ERROR, 0x0020);

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
        let out =
            assign_invoke_service_ids(&p(&[("bravo", None), ("alpha", None), ("charlie", None)]))
                .expect("unpinned-only must succeed");
        assert_eq!(out.get("alpha").copied(), Some(0x8100));
        assert_eq!(out.get("bravo").copied(), Some(0x8101));
        assert_eq!(out.get("charlie").copied(), Some(0x8102));
    }

    #[test]
    fn assigner_all_pinned_uses_pins_verbatim() {
        let out =
            assign_invoke_service_ids(&p(&[("alpha", Some(0x8123)), ("bravo", Some(0x817F))]))
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
        let out =
            assign_invoke_service_ids(&p(&[("alpha", Some(0x8105)), ("bravo", Some(0x8105))]));
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

    // ── §16.4 region-liveness assigner (RFC F.X-3) ──────────────────────

    fn lp(entries: &[(&str, Option<u16>)]) -> BTreeMap<String, Option<u16>> {
        entries.iter().map(|(k, v)| (k.to_string(), *v)).collect()
    }

    #[test]
    fn liveness_constants_partition_invoke_subrange() {
        // RFC F.X-3 D1: invoke and liveness sub-ranges are disjoint
        // halves of the SCE-reserved 256-slot space.
        assert_eq!(
            SCXML_INVOKE_SERVICE_CEILING + 1,
            SCXML_LIVENESS_SERVICE_BASE
        );
        assert_eq!(
            SCXML_LIVENESS_SERVICE_RANGE_SIZE, 128,
            "F.X-3 reserves a 128-slot sub-range mirroring F.X-1"
        );
        assert_eq!(SCXML_LIVENESS_SERVICE_BASE, 0x8180);
        assert_eq!(SCXML_LIVENESS_SERVICE_CEILING, 0x81FF);
    }

    #[test]
    fn liveness_assigner_empty_input_succeeds() {
        let out = assign_liveness_service_ids(&lp(&[])).expect("empty must succeed");
        assert!(out.is_empty());
    }

    #[test]
    fn liveness_assigner_unpinned_lex_counter_from_base() {
        // Three un-pinned partitions; lex order = (alpha__P__l,
        // alpha__P__r, beta__P__top). Each gets the next slot from
        // the liveness base.
        let out = assign_liveness_service_ids(&lp(&[
            ("alpha__P__l", None),
            ("alpha__P__r", None),
            ("beta__P__top", None),
        ]))
        .expect("unpinned must succeed");
        assert_eq!(out["alpha__P__l"], SCXML_LIVENESS_SERVICE_BASE);
        assert_eq!(out["alpha__P__r"], SCXML_LIVENESS_SERVICE_BASE + 1);
        assert_eq!(out["beta__P__top"], SCXML_LIVENESS_SERVICE_BASE + 2);
    }

    #[test]
    fn liveness_assigner_pinned_keeps_pin_unpinned_skips_reserved() {
        // alpha pinned at 0x8181 — auto must skip 0x8181 when filling
        // around it, taking 0x8180 then 0x8182.
        let out = assign_liveness_service_ids(&lp(&[
            ("alpha__P__l", Some(0x8181)),
            ("beta__P__l", None),
            ("gamma__P__l", None),
        ]))
        .expect("pin + auto mix must succeed");
        assert_eq!(out["alpha__P__l"], 0x8181);
        assert_eq!(out["beta__P__l"], 0x8180);
        assert_eq!(out["gamma__P__l"], 0x8182);
    }

    #[test]
    fn liveness_assigner_is_deterministic_across_runs() {
        // Same input → same output across two distinct invocations.
        let a = lp(&[
            ("zoo__P__r", Some(0x819F)),
            ("alpha__P__l", None),
            ("mid__P__l", Some(0x8190)),
            ("beta__P__l", None),
        ]);
        let b = a.clone();
        assert_eq!(
            assign_liveness_service_ids(&a).unwrap(),
            assign_liveness_service_ids(&b).unwrap()
        );
    }

    #[test]
    fn liveness_assigner_overflow_at_one_over_ceiling() {
        let mut input: BTreeMap<String, Option<u16>> = BTreeMap::new();
        for i in 0..(SCXML_LIVENESS_SERVICE_RANGE_SIZE + 1) {
            input.insert(format!("m{i:03}__P__l"), None);
        }
        match assign_liveness_service_ids(&input) {
            Err(AssignLivenessServiceIdError::Overflow {
                participant_count,
                ceiling,
            }) => {
                assert_eq!(participant_count, SCXML_LIVENESS_SERVICE_RANGE_SIZE + 1);
                assert_eq!(ceiling, SCXML_LIVENESS_SERVICE_RANGE_SIZE);
            }
            other => panic!("expected Overflow, got {other:?}"),
        }
    }

    #[test]
    fn liveness_assigner_exact_fill_succeeds() {
        let mut input: BTreeMap<String, Option<u16>> = BTreeMap::new();
        for i in 0..SCXML_LIVENESS_SERVICE_RANGE_SIZE {
            input.insert(format!("m{i:03}__P__l"), None);
        }
        let out = assign_liveness_service_ids(&input).expect("exact-fill must succeed");
        assert_eq!(out.len(), SCXML_LIVENESS_SERVICE_RANGE_SIZE);
        let max = *out.values().max().unwrap();
        assert_eq!(max, SCXML_LIVENESS_SERVICE_CEILING);
    }

    #[test]
    fn liveness_assigner_pin_below_range_rejected() {
        // 0x817F is the highest invoke slot; pinning it as a liveness
        // ID is out-of-range for this allocator.
        let out = assign_liveness_service_ids(&lp(&[("alpha__P__l", Some(0x817F))]));
        match out {
            Err(AssignLivenessServiceIdError::PinOutOfRange {
                partition_key,
                pinned_id,
                range_lo,
                range_hi,
            }) => {
                assert_eq!(partition_key, "alpha__P__l");
                assert_eq!(pinned_id, 0x817F);
                assert_eq!(range_lo, SCXML_LIVENESS_SERVICE_BASE);
                assert_eq!(range_hi, SCXML_LIVENESS_SERVICE_CEILING);
            }
            other => panic!("expected PinOutOfRange (below), got {other:?}"),
        }
    }

    #[test]
    fn liveness_assigner_pin_above_range_rejected() {
        // 0x8200 is the first slot beyond the SCE-reserved range.
        let out = assign_liveness_service_ids(&lp(&[("alpha__P__l", Some(0x8200))]));
        match out {
            Err(AssignLivenessServiceIdError::PinOutOfRange {
                partition_key,
                pinned_id,
                range_lo,
                range_hi,
            }) => {
                assert_eq!(partition_key, "alpha__P__l");
                assert_eq!(pinned_id, 0x8200);
                assert_eq!(range_lo, SCXML_LIVENESS_SERVICE_BASE);
                assert_eq!(range_hi, SCXML_LIVENESS_SERVICE_CEILING);
            }
            other => panic!("expected PinOutOfRange (above), got {other:?}"),
        }
    }

    #[test]
    fn liveness_assigner_pin_collision_rejected() {
        let out = assign_liveness_service_ids(&lp(&[
            ("alpha__P__l", Some(0x8185)),
            ("beta__P__r", Some(0x8185)),
            ("gamma__P__t", None),
        ]));
        match out {
            Err(AssignLivenessServiceIdError::PinCollision {
                partition_keys,
                pinned_id,
            }) => {
                assert_eq!(partition_keys, vec!["alpha__P__l", "beta__P__r"]);
                assert_eq!(pinned_id, 0x8185);
            }
            other => panic!("expected PinCollision, got {other:?}"),
        }
    }

    #[test]
    fn liveness_assigner_disjoint_from_invoke_under_same_keys() {
        // Cross-subsystem invariant: the same key fed to both
        // allocators yields IDs from disjoint ranges. This is the
        // F.X-1 D2 / F.X-3 D1 collision-free property pinned in code.
        let invoke_input = lp(&[("alpha", None), ("beta", None)]);
        let liveness_input = lp(&[("alpha__P__l", None), ("alpha__P__r", None)]);
        let invoke_out = assign_invoke_service_ids(&invoke_input).unwrap();
        let liveness_out = assign_liveness_service_ids(&liveness_input).unwrap();
        for &iid in invoke_out.values() {
            assert!(iid <= SCXML_INVOKE_SERVICE_CEILING);
        }
        for &lid in liveness_out.values() {
            assert!(lid >= SCXML_LIVENESS_SERVICE_BASE);
        }
        // Empty intersection is the load-bearing assertion.
        let invoke_set: std::collections::HashSet<u16> = invoke_out.values().copied().collect();
        let liveness_set: std::collections::HashSet<u16> = liveness_out.values().copied().collect();
        assert!(invoke_set.is_disjoint(&liveness_set));
    }

    // ── §16.7 row 8 machine-liveness assigner (RFC F.X-4) ──────────────

    #[test]
    fn machine_liveness_constants_disjoint_from_invoke_and_region() {
        // RFC F.X-4 D1: machine-liveness sub-range is disjoint from both
        // F.X-1 invoke and F.X-3 region-liveness. The 128-slot gap
        // [0x8200, 0x827F] is documented headroom for a future fourth
        // SCE subsystem.
        assert_eq!(SCXML_MACHINE_LIVENESS_SERVICE_BASE, 0x8280);
        assert_eq!(SCXML_MACHINE_LIVENESS_SERVICE_CEILING, 0x82FF);
        assert_eq!(SCXML_MACHINE_LIVENESS_SERVICE_RANGE_SIZE, 128);
        // Disjoint from F.X-1 invoke [0x8100, 0x817F].
        const { assert!(SCXML_MACHINE_LIVENESS_SERVICE_BASE > SCXML_INVOKE_SERVICE_CEILING) };
        // Disjoint from F.X-3 region-liveness [0x8180, 0x81FF].
        const { assert!(SCXML_MACHINE_LIVENESS_SERVICE_BASE > SCXML_LIVENESS_SERVICE_CEILING) };
        // Documented gap [0x8200, 0x827F] for future fourth subsystem
        // (RFC F.X-4 D1) — not contiguous with F.X-3.
        assert_eq!(SCXML_LIVENESS_SERVICE_CEILING + 1, 0x8200);
        assert_eq!(SCXML_MACHINE_LIVENESS_SERVICE_BASE - 1, 0x827F);
    }

    #[test]
    fn machine_liveness_assigner_empty_input_succeeds() {
        let out = assign_machine_liveness_service_ids(&p(&[])).expect("empty must succeed");
        assert!(out.is_empty());
    }

    #[test]
    fn machine_liveness_assigner_unpinned_lex_counter_from_base() {
        let out = assign_machine_liveness_service_ids(&p(&[
            ("zoo", None),
            ("alpha", None),
            ("mid", None),
        ]))
        .expect("unpinned must succeed");
        assert_eq!(out["alpha"], SCXML_MACHINE_LIVENESS_SERVICE_BASE);
        assert_eq!(out["mid"], SCXML_MACHINE_LIVENESS_SERVICE_BASE + 1);
        assert_eq!(out["zoo"], SCXML_MACHINE_LIVENESS_SERVICE_BASE + 2);
    }

    #[test]
    fn machine_liveness_assigner_pinned_keeps_pin_unpinned_skips_reserved() {
        // alpha pinned at 0x8281 — auto must skip 0x8281, taking 0x8280
        // for beta then 0x8282 for gamma.
        let out = assign_machine_liveness_service_ids(&p(&[
            ("alpha", Some(0x8281)),
            ("beta", None),
            ("gamma", None),
        ]))
        .expect("pin + auto mix must succeed");
        assert_eq!(out["alpha"], 0x8281);
        assert_eq!(out["beta"], 0x8280);
        assert_eq!(out["gamma"], 0x8282);
    }

    #[test]
    fn machine_liveness_assigner_overflow_at_one_over_ceiling() {
        let mut input: std::collections::BTreeMap<String, Option<u16>> =
            std::collections::BTreeMap::new();
        for i in 0..(SCXML_MACHINE_LIVENESS_SERVICE_RANGE_SIZE + 1) {
            input.insert(format!("m{i:03}"), None);
        }
        match assign_machine_liveness_service_ids(&input) {
            Err(AssignMachineLivenessServiceIdError::Overflow {
                participant_count,
                ceiling,
            }) => {
                assert_eq!(
                    participant_count,
                    SCXML_MACHINE_LIVENESS_SERVICE_RANGE_SIZE + 1
                );
                assert_eq!(ceiling, SCXML_MACHINE_LIVENESS_SERVICE_RANGE_SIZE);
            }
            other => panic!("expected Overflow, got {other:?}"),
        }
    }

    #[test]
    fn machine_liveness_assigner_exact_fill_succeeds() {
        let mut input: std::collections::BTreeMap<String, Option<u16>> =
            std::collections::BTreeMap::new();
        for i in 0..SCXML_MACHINE_LIVENESS_SERVICE_RANGE_SIZE {
            input.insert(format!("m{i:03}"), None);
        }
        let out = assign_machine_liveness_service_ids(&input).expect("exact-fill must succeed");
        assert_eq!(out.len(), SCXML_MACHINE_LIVENESS_SERVICE_RANGE_SIZE);
        let max = *out.values().max().unwrap();
        assert_eq!(max, SCXML_MACHINE_LIVENESS_SERVICE_CEILING);
    }

    #[test]
    fn machine_liveness_assigner_pin_below_range_rejected() {
        // 0x827F is the last slot of the documented gap; pinning it as
        // a machine-liveness ID is out-of-range.
        let out = assign_machine_liveness_service_ids(&p(&[("alpha", Some(0x827F))]));
        match out {
            Err(AssignMachineLivenessServiceIdError::PinOutOfRange {
                machine,
                pinned_id,
                range_lo,
                range_hi,
            }) => {
                assert_eq!(machine, "alpha");
                assert_eq!(pinned_id, 0x827F);
                assert_eq!(range_lo, SCXML_MACHINE_LIVENESS_SERVICE_BASE);
                assert_eq!(range_hi, SCXML_MACHINE_LIVENESS_SERVICE_CEILING);
            }
            other => panic!("expected PinOutOfRange (below), got {other:?}"),
        }
    }

    #[test]
    fn machine_liveness_assigner_pin_above_range_rejected() {
        // 0x8300 is the first slot beyond the F.X-4 sub-range.
        let out = assign_machine_liveness_service_ids(&p(&[("alpha", Some(0x8300))]));
        match out {
            Err(AssignMachineLivenessServiceIdError::PinOutOfRange {
                machine,
                pinned_id,
                range_lo,
                range_hi,
            }) => {
                assert_eq!(machine, "alpha");
                assert_eq!(pinned_id, 0x8300);
                assert_eq!(range_lo, SCXML_MACHINE_LIVENESS_SERVICE_BASE);
                assert_eq!(range_hi, SCXML_MACHINE_LIVENESS_SERVICE_CEILING);
            }
            other => panic!("expected PinOutOfRange (above), got {other:?}"),
        }
    }

    #[test]
    fn machine_liveness_assigner_pin_collision_rejected() {
        let out = assign_machine_liveness_service_ids(&p(&[
            ("alpha", Some(0x8285)),
            ("beta", Some(0x8285)),
        ]));
        match out {
            Err(AssignMachineLivenessServiceIdError::PinCollision {
                machines,
                pinned_id,
            }) => {
                assert_eq!(machines, vec!["alpha".to_string(), "beta".to_string()]);
                assert_eq!(pinned_id, 0x8285);
            }
            other => panic!("expected PinCollision, got {other:?}"),
        }
    }

    #[test]
    fn three_subsystem_allocators_yield_disjoint_id_sets() {
        // RFC F.X-4 D1 cross-subsystem disjointness invariant for the
        // three allocators. Same machine name fed into all three yields
        // three distinct IDs from three disjoint sub-ranges.
        let invoke = assign_invoke_service_ids(&p(&[("alpha", None)])).unwrap();
        let region = assign_liveness_service_ids(&p(&[("alpha__P__l", None)])).unwrap();
        let machine = assign_machine_liveness_service_ids(&p(&[("alpha", None)])).unwrap();
        let invoke_id = invoke["alpha"];
        let region_id = region["alpha__P__l"];
        let machine_id = machine["alpha"];
        assert!(invoke_id <= SCXML_INVOKE_SERVICE_CEILING);
        assert!(
            (SCXML_LIVENESS_SERVICE_BASE..=SCXML_LIVENESS_SERVICE_CEILING).contains(&region_id)
        );
        assert!(
            (SCXML_MACHINE_LIVENESS_SERVICE_BASE..=SCXML_MACHINE_LIVENESS_SERVICE_CEILING)
                .contains(&machine_id)
        );
        // Pairwise distinct.
        assert_ne!(invoke_id, region_id);
        assert_ne!(invoke_id, machine_id);
        assert_ne!(region_id, machine_id);
    }
}
