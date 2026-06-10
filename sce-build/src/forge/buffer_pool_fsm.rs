// SPDX-License-Identifier: LGPL-2.1-or-later WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2025 newmassrael
//
// SCE Forge: Buffer-pool slot-lifecycle FSM (watching-zenoh RFC §5.E
// lines 1116-1180). Single source of truth for the canonical 7-state
// FSM declared by the `buffer-pool` kind itself; consumed by the Rust
// + C11 buffer-pool templates as the contract that pins their
// phantom-typed (Rust) and tag-checked (C11) APIs to the same edge
// set.
//
// Per spec §5.E line 1117-1118: "modeled as a fixed FSM, declared
// canonically by the kind itself (not authored per pool)" — the table
// below is therefore a `const`, not a per-pool field. Future bus
// masters (hardware crypto, compression IP, GPU/NPU DMA) will *add*
// states under §5.E lines 1166-1180 ("FSM extension policy"); the
// existing seven states + eleven edges are stable across that
// extension and the Rust phantom-type API is preserved.

/// One of the seven slot ownership states. Spec §5.E lines 1129-1135.
///
/// Discriminant values are stable: the C11 backend emits these as
/// `sce_slot_state_t` enum values for runtime tag checks, and the
/// generated `STATE_COUNT` constant in both backends must match
/// `STATES.len()`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[repr(u8)]
pub enum SlotState {
    /// On freelist, no holder.
    Free = 0,
    /// Exclusive CPU write (encode TX, build reassembly).
    CpuMut = 1,
    /// TX descriptor queued, DMA not started.
    DmaArmedTx = 2,
    /// DMA actively reading slot for TX.
    DmaBusyTx = 3,
    /// RX descriptor armed, peripheral not yet writing.
    DmaArmedRx = 4,
    /// DMA actively writing slot from peripheral.
    DmaBusyRx = 5,
    /// Shared CPU read (parse decoded slot, dispatch handler).
    CpuRef = 6,
}

impl SlotState {
    /// Stable lower-snake-case name matching the spec body. Used by
    /// the Rust template to render phantom-marker names and by C11
    /// to render the enum-variant identifier.
    pub fn spec_name(self) -> &'static str {
        match self {
            Self::Free => "free",
            Self::CpuMut => "cpu-mut",
            Self::DmaArmedTx => "dma-armed-tx",
            Self::DmaBusyTx => "dma-busy-tx",
            Self::DmaArmedRx => "dma-armed-rx",
            Self::DmaBusyRx => "dma-busy-rx",
            Self::CpuRef => "cpu-ref",
        }
    }

    /// PascalCase form for Rust phantom-marker types.
    pub fn pascal_name(self) -> &'static str {
        match self {
            Self::Free => "Free",
            Self::CpuMut => "CpuMut",
            Self::DmaArmedTx => "DmaArmedTx",
            Self::DmaBusyTx => "DmaBusyTx",
            Self::DmaArmedRx => "DmaArmedRx",
            Self::DmaBusyRx => "DmaBusyRx",
            Self::CpuRef => "CpuRef",
        }
    }

    /// SCREAMING_SNAKE_CASE form for C11 enum variants
    /// (`SCE_SLOT_<name>` is the full identifier).
    pub fn c_enum_suffix(self) -> &'static str {
        match self {
            Self::Free => "FREE",
            Self::CpuMut => "CPU_MUT",
            Self::DmaArmedTx => "DMA_ARMED_TX",
            Self::DmaBusyTx => "DMA_BUSY_TX",
            Self::DmaArmedRx => "DMA_ARMED_RX",
            Self::DmaBusyRx => "DMA_BUSY_RX",
            Self::CpuRef => "CPU_REF",
        }
    }
}

/// Stable enumeration of all states. Order matches discriminant.
pub const STATES: [SlotState; 7] = [
    SlotState::Free,
    SlotState::CpuMut,
    SlotState::DmaArmedTx,
    SlotState::DmaBusyTx,
    SlotState::DmaArmedRx,
    SlotState::DmaBusyRx,
    SlotState::CpuRef,
];

/// Cache-maintenance annotation on a transition edge. Spec §5.E
/// lines 1182-1228. The IR carries this annotation; per-edge
/// emission has no consumer — the shipped cache-maintenance emit
/// (item C5) gates on the pool-level `cache-policy: maintain` flag
/// instead, so per-edge gating is not implemented until a consumer
/// needs it.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CacheOp {
    /// No cache maintenance on this edge.
    None,
    /// `cache_clean_by_addr(slot, len)` flushes pending CPU writes
    /// before DMA reads.
    CleanIfMaintain,
    /// `cache_invalidate_by_addr(slot, len)` evicts speculative /
    /// post-DMA stale lines. The Rx-arm pre-invalidate is gated on
    /// `platform.has_speculative_prefetch == true` (line 1190);
    /// post-Rx invalidate is unconditional under `maintain`.
    InvalidateIfMaintain { gated_speculative: bool },
    /// `cache_clean_by_addr(slot, len)` on next CPU→DMA hand-off
    /// after in-place mutate (cpu-ref → cpu-mut). Spec line 1154.
    CleanOnNextHandOff,
}

/// One legal edge of the slot-lifecycle FSM. Spec §5.E lines 1141-1156.
#[derive(Debug, Clone, Copy)]
pub struct Transition {
    /// Source state.
    pub from: SlotState,
    /// Target state.
    pub to: SlotState,
    /// Human-readable trigger label from the spec body — mirrors the
    /// right-hand side of the spec's transition table verbatim.
    pub trigger: &'static str,
    /// Cache-maintenance annotation (spec lines 1182-1228; carried
    /// in the IR, not yet consumed by emission — see [`CacheOp`]).
    pub cache_op: CacheOp,
    /// Whether author code can directly invoke this edge through
    /// the public author-visible API (spec lines 1232-1237). False
    /// for IRQ-driven, peripheral-driven, and intermediate edges
    /// that the runtime owns.
    pub author_callable: bool,
}

/// The eleven legal transitions, verbatim from spec §5.E lines
/// 1141-1156. Every emitted slot operation maps to one entry; the
/// FSM is total per spec line 1159 ("every emitted operation that
/// touches a slot maps to a transition").
pub const TRANSITIONS: [Transition; 11] = [
    Transition {
        from: SlotState::Free,
        to: SlotState::CpuMut,
        trigger: "pool_acquire_for_encode()",
        cache_op: CacheOp::None,
        author_callable: true,
    },
    Transition {
        from: SlotState::CpuMut,
        to: SlotState::DmaArmedTx,
        trigger: "link_arm_tx(slot)",
        cache_op: CacheOp::CleanIfMaintain,
        author_callable: true,
    },
    Transition {
        from: SlotState::DmaArmedTx,
        to: SlotState::DmaBusyTx,
        trigger: "DMA controller signal",
        cache_op: CacheOp::None,
        author_callable: false,
    },
    Transition {
        from: SlotState::DmaBusyTx,
        to: SlotState::Free,
        trigger: "TX-complete IRQ; pool_return(slot)",
        cache_op: CacheOp::None,
        author_callable: false,
    },
    Transition {
        from: SlotState::Free,
        to: SlotState::DmaArmedRx,
        trigger: "link_arm_rx(slot)",
        cache_op: CacheOp::InvalidateIfMaintain {
            gated_speculative: true,
        },
        author_callable: true,
    },
    Transition {
        from: SlotState::DmaArmedRx,
        to: SlotState::DmaBusyRx,
        trigger: "peripheral start",
        cache_op: CacheOp::None,
        author_callable: false,
    },
    Transition {
        from: SlotState::DmaBusyRx,
        to: SlotState::CpuRef,
        trigger: "RX-complete IRQ",
        cache_op: CacheOp::InvalidateIfMaintain {
            gated_speculative: false,
        },
        author_callable: false,
    },
    Transition {
        from: SlotState::CpuRef,
        to: SlotState::Free,
        trigger: "handler complete; pool_return(slot)",
        cache_op: CacheOp::None,
        author_callable: true,
    },
    Transition {
        from: SlotState::CpuRef,
        to: SlotState::CpuMut,
        trigger: "in-place mutate",
        cache_op: CacheOp::CleanOnNextHandOff,
        author_callable: false,
    },
    Transition {
        from: SlotState::CpuMut,
        to: SlotState::Free,
        trigger: "abort encode (error path)",
        cache_op: CacheOp::None,
        author_callable: true,
    },
    Transition {
        from: SlotState::DmaArmedTx,
        to: SlotState::CpuMut,
        trigger: "un-arm before DMA start (error path)",
        cache_op: CacheOp::None,
        author_callable: false,
    },
];

/// Number of declared states. Constant for codegen consumers; the
/// emitted `STATE_COUNT` in both backends must match this.
pub const STATE_COUNT: usize = STATES.len();

/// Number of declared transitions. Constant for codegen consumers;
/// the emitted `TRANSITION_COUNT` in both backends must match this.
pub const TRANSITION_COUNT: usize = TRANSITIONS.len();

/// True iff `(from, to)` is one of the declared transitions.
/// Linear scan is fine for an 11-element table; this is a codegen-
/// time helper, not a hot-path query.
pub fn is_allowed(from: SlotState, to: SlotState) -> bool {
    TRANSITIONS.iter().any(|t| t.from == from && t.to == to)
}

/// All transitions whose `from` state matches `state`. Used by the
/// codegen layer to enumerate the legal next-edges from a given
/// holder state when emitting phantom-typed accessor methods.
pub fn transitions_from(state: SlotState) -> impl Iterator<Item = &'static Transition> {
    TRANSITIONS.iter().filter(move |t| t.from == state)
}

/// All transitions that author code can directly invoke via the
/// public author-visible API (spec lines 1232-1237). Excludes
/// IRQ-driven and intermediate edges. Used by the codegen layer
/// to pick which edges become public methods on `Slot<state>` /
/// the C11 handle.
pub fn author_callable_transitions() -> impl Iterator<Item = &'static Transition> {
    TRANSITIONS.iter().filter(|t| t.author_callable)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn states_count_matches_spec() {
        assert_eq!(
            STATE_COUNT, 7,
            "spec §5.E lines 1129-1135 declare seven states"
        );
    }

    #[test]
    fn transitions_count_matches_spec() {
        assert_eq!(
            TRANSITION_COUNT, 11,
            "spec §5.E lines 1141-1156 declare eleven legal transitions"
        );
    }

    #[test]
    fn each_state_has_a_distinct_discriminant() {
        let mut seen = std::collections::HashSet::new();
        for s in STATES.iter() {
            assert!(seen.insert(*s as u8), "discriminant collision for {:?}", s);
        }
        assert_eq!(seen.len(), STATE_COUNT);
    }

    #[test]
    fn spec_names_are_unique_and_lower_kebab() {
        let mut seen = std::collections::HashSet::new();
        for s in STATES.iter() {
            let name = s.spec_name();
            assert!(seen.insert(name), "duplicate spec_name for {:?}", s);
            assert!(
                name.chars().all(|c| c.is_ascii_lowercase() || c == '-'),
                "spec_name {:?} must be lower-kebab-case",
                name,
            );
        }
    }

    #[test]
    fn pascal_names_are_unique_and_pascal_case() {
        let mut seen = std::collections::HashSet::new();
        for s in STATES.iter() {
            let name = s.pascal_name();
            assert!(seen.insert(name), "duplicate pascal_name for {:?}", s);
            // First char uppercase, no hyphen / underscore.
            let first = name.chars().next().expect("pascal_name non-empty");
            assert!(
                first.is_ascii_uppercase(),
                "pascal_name {:?} must start uppercase",
                name
            );
            assert!(
                !name.contains('-'),
                "pascal_name {:?} must not contain '-'",
                name
            );
            assert!(
                !name.contains('_'),
                "pascal_name {:?} must not contain '_'",
                name
            );
        }
    }

    #[test]
    fn c_enum_suffixes_are_unique_and_screaming_snake() {
        let mut seen = std::collections::HashSet::new();
        for s in STATES.iter() {
            let name = s.c_enum_suffix();
            assert!(seen.insert(name), "duplicate c_enum_suffix for {:?}", s);
            assert!(
                name.chars().all(|c| c.is_ascii_uppercase() || c == '_'),
                "c_enum_suffix {:?} must be SCREAMING_SNAKE",
                name,
            );
        }
    }

    /// Every transition declared in the spec is reachable through
    /// `is_allowed`. Pins the eleven edges verbatim.
    #[test]
    fn all_eleven_spec_transitions_are_allowed() {
        let expected: &[(SlotState, SlotState)] = &[
            (SlotState::Free, SlotState::CpuMut),
            (SlotState::CpuMut, SlotState::DmaArmedTx),
            (SlotState::DmaArmedTx, SlotState::DmaBusyTx),
            (SlotState::DmaBusyTx, SlotState::Free),
            (SlotState::Free, SlotState::DmaArmedRx),
            (SlotState::DmaArmedRx, SlotState::DmaBusyRx),
            (SlotState::DmaBusyRx, SlotState::CpuRef),
            (SlotState::CpuRef, SlotState::Free),
            (SlotState::CpuRef, SlotState::CpuMut),
            (SlotState::CpuMut, SlotState::Free),
            (SlotState::DmaArmedTx, SlotState::CpuMut),
        ];
        assert_eq!(
            expected.len(),
            TRANSITION_COUNT,
            "test fixture must list every edge",
        );
        for (from, to) in expected {
            assert!(
                is_allowed(*from, *to),
                "transition {:?} -> {:?} must be allowed (spec §5.E)",
                from,
                to,
            );
        }
    }

    /// Self-edges are forbidden — no state has a no-op transition
    /// to itself in the spec.
    #[test]
    fn self_edges_are_forbidden() {
        for s in STATES.iter() {
            assert!(
                !is_allowed(*s, *s),
                "self-edge {:?} -> {:?} must not be a declared transition",
                s,
                s,
            );
        }
    }

    /// Representative invalid edges that real code might attempt.
    /// Pinning these prevents accidental table edits from quietly
    /// admitting them.
    #[test]
    fn representative_forbidden_edges_are_rejected() {
        // Direct CPU read of a free slot bypassing acquire — would
        // mean handing out a Slot<CpuRef> from acquire(), which
        // breaks the "free → cpu-mut only" rule.
        assert!(!is_allowed(SlotState::Free, SlotState::CpuRef));
        // Skipping the IRQ-driven Rx pipeline.
        assert!(!is_allowed(SlotState::Free, SlotState::CpuRef));
        assert!(!is_allowed(SlotState::DmaArmedRx, SlotState::CpuRef));
        assert!(!is_allowed(SlotState::DmaBusyRx, SlotState::Free));
        // TX un-arm after the DMA controller has already started —
        // spec only permits un-arm from `dma-armed-tx`, not
        // `dma-busy-tx`.
        assert!(!is_allowed(SlotState::DmaBusyTx, SlotState::CpuMut));
        // Direct cpu-mut → cpu-ref or cpu-mut → dma-armed-rx are
        // not legal hand-offs — must go through `free` first.
        assert!(!is_allowed(SlotState::CpuMut, SlotState::CpuRef));
        assert!(!is_allowed(SlotState::CpuMut, SlotState::DmaArmedRx));
        // Direct dma-armed-rx → cpu-ref bypasses the IRQ pipeline.
        assert!(!is_allowed(SlotState::DmaArmedRx, SlotState::CpuRef));
    }

    #[test]
    fn transitions_from_free_yields_two_edges() {
        let edges: Vec<_> = transitions_from(SlotState::Free).collect();
        assert_eq!(
            edges.len(),
            2,
            "free has two outgoing edges (cpu-mut, dma-armed-rx)"
        );
        let targets: Vec<_> = edges.iter().map(|t| t.to).collect();
        assert!(targets.contains(&SlotState::CpuMut));
        assert!(targets.contains(&SlotState::DmaArmedRx));
    }

    #[test]
    fn transitions_from_cpu_mut_yields_two_edges() {
        let edges: Vec<_> = transitions_from(SlotState::CpuMut).collect();
        assert_eq!(
            edges.len(),
            2,
            "cpu-mut has two outgoing edges (dma-armed-tx, free)"
        );
        let targets: Vec<_> = edges.iter().map(|t| t.to).collect();
        assert!(targets.contains(&SlotState::DmaArmedTx));
        assert!(targets.contains(&SlotState::Free));
    }

    #[test]
    fn transitions_from_cpu_ref_yields_two_edges() {
        let edges: Vec<_> = transitions_from(SlotState::CpuRef).collect();
        assert_eq!(
            edges.len(),
            2,
            "cpu-ref has two outgoing edges (free, cpu-mut)"
        );
    }

    #[test]
    fn cache_annotations_match_spec_lines_1182_1228() {
        // cpu-mut → dma-armed-tx: clean if maintain (spec line 1186-1188)
        let tx_arm = TRANSITIONS
            .iter()
            .find(|t| t.from == SlotState::CpuMut && t.to == SlotState::DmaArmedTx)
            .expect("tx-arm edge present");
        assert!(matches!(tx_arm.cache_op, CacheOp::CleanIfMaintain));

        // free → dma-armed-rx: invalidate if maintain && speculative (spec line 1189-1194)
        let rx_arm = TRANSITIONS
            .iter()
            .find(|t| t.from == SlotState::Free && t.to == SlotState::DmaArmedRx)
            .expect("rx-arm edge present");
        assert!(matches!(
            rx_arm.cache_op,
            CacheOp::InvalidateIfMaintain {
                gated_speculative: true
            }
        ));

        // dma-busy-rx → cpu-ref: invalidate if maintain (spec line 1195-1197, unconditional)
        let rx_complete = TRANSITIONS
            .iter()
            .find(|t| t.from == SlotState::DmaBusyRx && t.to == SlotState::CpuRef)
            .expect("rx-complete edge present");
        assert!(matches!(
            rx_complete.cache_op,
            CacheOp::InvalidateIfMaintain {
                gated_speculative: false
            }
        ));

        // cpu-ref → cpu-mut: clean on next hand-off (spec line 1154)
        let in_place = TRANSITIONS
            .iter()
            .find(|t| t.from == SlotState::CpuRef && t.to == SlotState::CpuMut)
            .expect("in-place mutate edge present");
        assert!(matches!(in_place.cache_op, CacheOp::CleanOnNextHandOff));
    }

    #[test]
    fn author_callable_transitions_match_spec_lines_1232_1237() {
        // Spec lines 1232-1237 list four author-visible operations:
        //   pool_acquire_for_encode  (free → cpu-mut)
        //   pool_return              (cpu-mut|cpu-ref → free) — two edges
        //   link_arm_tx              (cpu-mut → dma-armed-tx)
        //   link_arm_rx              (free → dma-armed-rx)
        // → five author-callable edges total.
        let count = author_callable_transitions().count();
        assert_eq!(
            count, 5,
            "five author-visible edges per spec lines 1232-1237"
        );

        let pairs: Vec<(SlotState, SlotState)> = author_callable_transitions()
            .map(|t| (t.from, t.to))
            .collect();
        assert!(pairs.contains(&(SlotState::Free, SlotState::CpuMut)));
        assert!(pairs.contains(&(SlotState::CpuMut, SlotState::DmaArmedTx)));
        assert!(pairs.contains(&(SlotState::Free, SlotState::DmaArmedRx)));
        assert!(pairs.contains(&(SlotState::CpuMut, SlotState::Free)));
        assert!(pairs.contains(&(SlotState::CpuRef, SlotState::Free)));
    }

    /// IRQ-driven and peripheral-driven edges must NOT be marked
    /// author-callable — exposing them would let authors call into
    /// `pool_complete_dma_tx()` etc. directly, breaking the
    /// IRQ-handler-as-sole-source contract.
    #[test]
    fn irq_driven_edges_are_not_author_callable() {
        for t in TRANSITIONS.iter() {
            if t.from == SlotState::DmaArmedTx && t.to == SlotState::DmaBusyTx
                || t.from == SlotState::DmaBusyTx && t.to == SlotState::Free
                || t.from == SlotState::DmaArmedRx && t.to == SlotState::DmaBusyRx
                || t.from == SlotState::DmaBusyRx && t.to == SlotState::CpuRef
            {
                assert!(
                    !t.author_callable,
                    "edge {:?} -> {:?} is IRQ/peripheral-driven and must not be author-callable",
                    t.from, t.to,
                );
            }
        }
    }
}
