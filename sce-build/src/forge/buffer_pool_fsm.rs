// SPDX-License-Identifier: LGPL-2.1-or-later WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2025 newmassrael
//
// SCE Forge: Buffer-pool slot-lifecycle FSM (SCE Protocol-Synthesis RFC §synth-5-E
// lines 1116-1180). Single source of truth for the canonical 7-state
// FSM declared by the `buffer-pool` kind itself; consumed by the Rust
// + C11 buffer-pool templates as the contract that pins their
// phantom-typed (Rust) and tag-checked (C11) APIs to the same edge
// set.
//
// Per spec §synth-5-E line 1117-1118: "modeled as a fixed FSM, declared
// canonically by the kind itself (not authored per pool)" — the table
// below is therefore a `const`, not a per-pool field. Future bus
// masters (hardware crypto, compression IP, GPU/NPU DMA) will *add*
// states under §synth-5-E lines 1166-1180 ("FSM extension policy"); the
// existing seven states + eleven edges are stable across that
// extension and the Rust phantom-type API is preserved.

/// One of the seven slot ownership states. Spec §synth-5-E lines 1129-1135.
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

    /// Whether the emitted API can hand author code a handle in this
    /// state — a `Slot<S>` on Rust, a tagged `sce_slot_handle_t` on
    /// C11.
    ///
    /// This is what decides the *shape* of an edge leaving the state,
    /// and both backends read it. An edge leaving a holdable state
    /// consumes the caller's handle, so the old state's handle is
    /// invalidated by the move (Rust) or retagged in place (C11). An
    /// edge leaving a non-holdable state has no handle to consume by
    /// construction — the slot belongs to the DMA controller or the
    /// peripheral, which signal completion with a channel/descriptor
    /// index and nothing else — so those edges are keyed by slot
    /// index and mint the resulting handle instead.
    ///
    /// `free` is not holdable: a free slot is held by the pool, and
    /// handing out a handle to it is precisely the aliasing the
    /// freelist exists to prevent.
    pub fn is_holdable(self) -> bool {
        matches!(self, Self::CpuMut | Self::CpuRef | Self::DmaArmedRx)
    }

    /// Whether a bus master owns the slot in this state rather than the
    /// CPU.
    ///
    /// Distinct from [`Self::is_holdable`], which asks whether the
    /// emitted API can hand author code a handle: `dma-armed-rx` is
    /// both — the peripheral owns the buffer while the author still
    /// holds the handle it needs to write the descriptor.
    ///
    /// An edge *into* one of these is a hand-off, which is what the
    /// spec's deferred cache-clean (line 1154) comes due on.
    pub fn is_dma_owned(self) -> bool {
        matches!(
            self,
            Self::DmaArmedTx | Self::DmaBusyTx | Self::DmaArmedRx | Self::DmaBusyRx
        )
    }

    /// What holding a handle in this state means, as the emitted
    /// marker's doc comment — one entry per rendered `///` line.
    ///
    /// Pre-wrapped rather than one long sentence because the caller is
    /// a template: Jinja cannot reflow, so a single string would emit a
    /// doc line several times the width of every other comment in the
    /// generated file.
    ///
    /// Lives beside the state rather than in the template so a state
    /// added under the §synth-5-E FSM extension policy arrives with its
    /// meaning attached instead of inheriting a generic sentence. Only
    /// holdable states are emitted as markers (see [`Self::is_holdable`]);
    /// the rest state why they are not, which is worth recording where
    /// someone reading the table will ask.
    pub fn holder_doc_lines(self) -> &'static [&'static str] {
        match self {
            Self::Free => &["On the freelist — held by the pool, never by author code."],
            Self::CpuMut => &[
                "Held by author code while the slot is in exclusive",
                "CPU-write state.",
            ],
            Self::DmaArmedTx => &[
                "TX descriptor queued. Owned by the DMA controller, so the",
                "edges leaving it are keyed by slot index.",
            ],
            Self::DmaBusyTx => &[
                "The DMA controller is reading the slot for TX. Reached by",
                "slot index.",
            ],
            Self::DmaArmedRx => &[
                "Returned from `link_arm_rx` so the author can register the",
                "slot index with the peripheral's RX descriptor. Hand it to",
                "`dma_start_rx` when the peripheral begins writing.",
            ],
            Self::DmaBusyRx => &[
                "The peripheral is writing the slot. Reached by slot index;",
                "`rx_complete` yields the readable `cpu-ref` handle.",
            ],
            Self::CpuRef => &[
                "Held by author code while the slot is in shared CPU-read",
                "state (post-RX-IRQ).",
            ],
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

/// Cache-maintenance annotation on a transition edge. Spec §synth-5-E
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

/// One legal edge of the slot-lifecycle FSM. Spec §synth-5-E lines 1141-1156.
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
    /// Identifier form of [`Self::trigger`] — the name both backends
    /// emit for this edge (`Slot::<op_name>` on Rust,
    /// `<pool>_<op_name>` on C11). Rendering from one field is what
    /// keeps the two backends from drifting into different spellings
    /// of the same transition; `pool_return` deliberately repeats
    /// because spec line 1234 gives `cpu-mut → free` and
    /// `cpu-ref → free` a single author-visible operation.
    pub op_name: &'static str,
    /// Line in `docs/spec/synth/rfc-sce-protocol-synthesis.md` declaring this
    /// edge. `spec_line_quotes_its_own_edge` reads the spec body at
    /// this line and fails if the citation drifts off its target — a
    /// misaimed line number is otherwise invisible, since nothing
    /// else reads it.
    pub spec_line: u32,
}

/// The eleven legal transitions, verbatim from spec §synth-5-E lines
/// 1141-1156. Every emitted slot operation maps to one entry; the
/// FSM is total per spec line 1159 ("every emitted operation that
/// touches a slot maps to a transition").
pub const TRANSITIONS: [Transition; 11] = [
    Transition {
        from: SlotState::Free,
        to: SlotState::CpuMut,
        trigger: "pool_acquire_for_encode()",
        op_name: "pool_acquire_for_encode",
        spec_line: 1141,
        cache_op: CacheOp::None,
        author_callable: true,
    },
    Transition {
        from: SlotState::CpuMut,
        to: SlotState::DmaArmedTx,
        trigger: "link_arm_tx(slot)",
        op_name: "link_arm_tx",
        spec_line: 1142,
        cache_op: CacheOp::CleanIfMaintain,
        author_callable: true,
    },
    Transition {
        from: SlotState::DmaArmedTx,
        to: SlotState::DmaBusyTx,
        trigger: "DMA controller signal",
        op_name: "dma_start_tx",
        spec_line: 1144,
        cache_op: CacheOp::None,
        author_callable: false,
    },
    Transition {
        from: SlotState::DmaBusyTx,
        to: SlotState::Free,
        trigger: "TX-complete IRQ; pool_return(slot)",
        op_name: "tx_complete",
        spec_line: 1145,
        cache_op: CacheOp::None,
        author_callable: false,
    },
    Transition {
        from: SlotState::Free,
        to: SlotState::DmaArmedRx,
        trigger: "link_arm_rx(slot)",
        op_name: "link_arm_rx",
        spec_line: 1146,
        cache_op: CacheOp::InvalidateIfMaintain {
            gated_speculative: true,
        },
        author_callable: true,
    },
    Transition {
        from: SlotState::DmaArmedRx,
        to: SlotState::DmaBusyRx,
        trigger: "peripheral start",
        op_name: "dma_start_rx",
        spec_line: 1149,
        cache_op: CacheOp::None,
        author_callable: false,
    },
    Transition {
        from: SlotState::DmaBusyRx,
        to: SlotState::CpuRef,
        trigger: "RX-complete IRQ",
        op_name: "rx_complete",
        spec_line: 1150,
        cache_op: CacheOp::InvalidateIfMaintain {
            gated_speculative: false,
        },
        author_callable: false,
    },
    Transition {
        from: SlotState::CpuRef,
        to: SlotState::Free,
        trigger: "handler complete; pool_return(slot)",
        op_name: "pool_return",
        spec_line: 1152,
        cache_op: CacheOp::None,
        author_callable: true,
    },
    Transition {
        from: SlotState::CpuRef,
        to: SlotState::CpuMut,
        trigger: "in-place mutate",
        op_name: "mutate_in_place",
        spec_line: 1153,
        cache_op: CacheOp::CleanOnNextHandOff,
        author_callable: false,
    },
    Transition {
        from: SlotState::CpuMut,
        to: SlotState::Free,
        trigger: "abort encode (error path)",
        op_name: "pool_return",
        spec_line: 1155,
        cache_op: CacheOp::None,
        author_callable: true,
    },
    Transition {
        from: SlotState::DmaArmedTx,
        to: SlotState::CpuMut,
        trigger: "un-arm before DMA start (error path)",
        op_name: "un_arm_tx",
        spec_line: 1156,
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

/// The edges the runtime owns: the DMA controller signal, the
/// peripheral start, the TX/RX completion IRQs, the un-arm error
/// path, and the in-place mutate. Exactly the complement of
/// [`author_callable_transitions`].
///
/// These are emitted too, as an explicitly-`unsafe` seam that the
/// driver or ISR calls. Leaving them unemitted does not make them
/// unreachable — it makes the pool a one-way sink. `free →
/// dma-armed-{tx,rx}` are author-callable, so a pool with no seam
/// can be drained but never refilled: `dma-busy-tx → free` and
/// `dma-busy-rx → cpu-ref` are the only paths back to the freelist
/// and both live here. Spec line 1159 states the FSM is total over
/// emitted operations, which is only true of an emit that carries
/// every edge.
///
/// `unsafe` is the honest marker rather than a naming convention:
/// calling one of these before the hardware event it names hands out
/// a view of memory the peripheral is still writing.
pub fn runtime_seam_transitions() -> impl Iterator<Item = &'static Transition> {
    TRANSITIONS.iter().filter(|t| !t.author_callable)
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

    /// A deferred cache-clean has to be discharged by whatever edge
    /// comes due, and this is what checks that something does.
    ///
    /// Spec line 1154 attaches no call to `cpu-ref → cpu-mut` itself:
    /// the in-place mutate leaves dirty CPU lines and the obligation
    /// falls on the *next* hand-off to a bus master. Today that is
    /// `link_arm_tx`, which cleans on every hand-off under `maintain`
    /// whether or not the slot arrived through an in-place mutate — so
    /// the obligation is discharged by construction, and nothing says
    /// so or would notice if it stopped being true.
    ///
    /// Making `link_arm_tx`'s clean conditional — on a dirty flag, on
    /// whether the slot was written since acquire — would silently
    /// strip the clean from exactly the path that needs it most,
    /// because that path is the one where the CPU wrote *after* the
    /// slot was last cleaned. Stated as a property over the table so
    /// the check survives an edge set that grows.
    #[test]
    fn a_deferred_cache_clean_is_discharged_by_every_hand_off_out_of_its_target() {
        let deferred: Vec<&Transition> = TRANSITIONS
            .iter()
            .filter(|t| matches!(t.cache_op, CacheOp::CleanOnNextHandOff))
            .collect();
        assert!(
            !deferred.is_empty(),
            "no edge defers a cache-clean; this check has lost its subject",
        );

        let mut hand_offs = 0usize;
        for d in deferred {
            // Every edge leaving the state the deferral lands in and
            // reaching a bus master is a hand-off, and each one must
            // carry the clean the deferral is owed.
            let outgoing: Vec<&Transition> = transitions_from(d.to)
                .filter(|t| t.to.is_dma_owned())
                .collect();
            assert!(
                !outgoing.is_empty(),
                "{} → {} defers a cache-clean to the next hand-off, but no edge \
                 leaves {} for a bus master — the obligation can never come due",
                d.from.spec_name(),
                d.to.spec_name(),
                d.to.spec_name(),
            );
            for h in outgoing {
                hand_offs += 1;
                assert!(
                    matches!(h.cache_op, CacheOp::CleanIfMaintain),
                    "{} → {} defers a cache-clean, but the hand-off {} → {} that \
                     comes due carries {:?} instead of CleanIfMaintain — a slot \
                     mutated in place would reach the bus master with dirty CPU \
                     lines (spec §synth-5-E line 1154)",
                    d.from.spec_name(),
                    d.to.spec_name(),
                    h.from.spec_name(),
                    h.to.spec_name(),
                    h.cache_op,
                );
            }
        }
        assert!(
            hand_offs >= 1,
            "no hand-off edge was examined; the filter stopped selecting",
        );
    }

    /// The two ownership predicates answer different questions, and a
    /// state can be both. Pin the overlap so a future edit cannot
    /// quietly collapse one into the other.
    #[test]
    fn dma_ownership_and_holdability_are_independent_axes() {
        // Owned by a bus master, and the author still holds the handle
        // it needs to write the peripheral's descriptor.
        assert!(SlotState::DmaArmedRx.is_dma_owned());
        assert!(SlotState::DmaArmedRx.is_holdable());
        // CPU-owned and holdable.
        assert!(!SlotState::CpuMut.is_dma_owned());
        assert!(SlotState::CpuMut.is_holdable());
        // Bus-master owned with no handle in author hands.
        assert!(SlotState::DmaBusyRx.is_dma_owned());
        assert!(!SlotState::DmaBusyRx.is_holdable());
        // Neither: the pool holds it.
        assert!(!SlotState::Free.is_dma_owned());
        assert!(!SlotState::Free.is_holdable());
        assert_eq!(STATES.iter().filter(|s| s.is_dma_owned()).count(), 4);
    }

    /// `is_holdable` decides whether an edge leaving a state consumes
    /// a handle or takes a slot index, so both backends branch on it.
    /// Pin the partition and the reason for each side.
    #[test]
    fn holdable_states_are_the_cpu_owned_ones_plus_the_armed_rx_handle() {
        // The pool owns a free slot; handing out a handle to one is
        // the aliasing the freelist prevents.
        assert!(!SlotState::Free.is_holdable());
        // Hardware owns these — a completion IRQ arrives with an
        // index, never with a handle the author could have kept.
        assert!(!SlotState::DmaArmedTx.is_holdable());
        assert!(!SlotState::DmaBusyTx.is_holdable());
        assert!(!SlotState::DmaBusyRx.is_holdable());
        // CPU-owned, plus the armed-RX handle `link_arm_rx` returns so
        // the author can write the peripheral's descriptor.
        assert!(SlotState::CpuMut.is_holdable());
        assert!(SlotState::CpuRef.is_holdable());
        assert!(SlotState::DmaArmedRx.is_holdable());
        assert_eq!(STATES.iter().filter(|s| s.is_holdable()).count(), 3);
    }

    /// Nothing may be declared holdable that no transition produces —
    /// that would emit a `Slot<S>` type with no way to obtain one,
    /// which is the dead-code shape the seam exists to remove.
    #[test]
    fn every_holdable_state_has_a_producer() {
        for s in STATES.iter().filter(|s| s.is_holdable()) {
            assert!(
                TRANSITIONS.iter().any(|t| t.to == *s),
                "{s:?} is holdable but no transition produces it",
            );
        }
    }

    /// The runtime seam is exactly the complement of the author API,
    /// and it is not empty. Six edges: the two DMA starts, the two
    /// completions, the un-arm error path, and the in-place mutate.
    #[test]
    fn runtime_seam_is_the_complement_of_the_author_api() {
        let seam: Vec<(SlotState, SlotState)> =
            runtime_seam_transitions().map(|t| (t.from, t.to)).collect();
        assert_eq!(
            seam.len(),
            TRANSITION_COUNT - author_callable_transitions().count(),
            "seam and author API must partition the edge set",
        );
        assert_eq!(seam.len(), 6, "six runtime-owned edges (spec §5.E)");
        for pair in [
            (SlotState::DmaArmedTx, SlotState::DmaBusyTx),
            (SlotState::DmaBusyTx, SlotState::Free),
            (SlotState::DmaArmedRx, SlotState::DmaBusyRx),
            (SlotState::DmaBusyRx, SlotState::CpuRef),
            (SlotState::CpuRef, SlotState::CpuMut),
            (SlotState::DmaArmedTx, SlotState::CpuMut),
        ] {
            assert!(seam.contains(&pair), "{pair:?} must be a seam edge");
        }
    }

    /// Every state must be reachable from `free` and must have a way
    /// out. This is the property the pre-seam emit violated: three
    /// states had no producer and the two DMA-busy states had no
    /// consumer, so arming a slot removed it from the pool forever.
    ///
    /// Stated over `TRANSITIONS` rather than over a hand-listed set,
    /// so a state added under the §synth-5-E FSM extension policy
    /// (lines 1166-1180) inherits the requirement instead of slipping
    /// in unreachable.
    #[test]
    fn every_state_is_reachable_from_free_and_has_a_way_out() {
        let mut reached = std::collections::HashSet::from([SlotState::Free]);
        // Eleven edges: |STATES| passes is a generous fixpoint bound.
        for _ in 0..STATE_COUNT {
            for t in TRANSITIONS.iter() {
                if reached.contains(&t.from) {
                    reached.insert(t.to);
                }
            }
        }
        for s in STATES.iter() {
            assert!(
                reached.contains(s),
                "{s:?} is not reachable from free — no emitted operation can \
                 ever put a slot into it",
            );
            assert!(
                TRANSITIONS.iter().any(|t| t.from == *s),
                "{s:?} has no outgoing edge — a slot entering it is stranded",
            );
        }
    }

    /// A slot must be able to get back to `free` from every state,
    /// not merely leave it. Without this, "has a way out" is
    /// satisfied by a cycle that never returns the slot to the pool.
    #[test]
    fn every_state_can_return_to_free() {
        // Backward fixpoint: states from which `free` is reachable.
        let mut returns = std::collections::HashSet::from([SlotState::Free]);
        for _ in 0..STATE_COUNT {
            for t in TRANSITIONS.iter() {
                if returns.contains(&t.to) {
                    returns.insert(t.from);
                }
            }
        }
        for s in STATES.iter() {
            assert!(
                returns.contains(s),
                "{s:?} cannot reach free — a slot arriving there leaks",
            );
        }
    }

    /// Two edges leaving the same state must not share an emitted
    /// name: on Rust they would collide in one `impl Slot<From>`
    /// block, on C11 in one function name. Edges leaving *different*
    /// states may share one (`pool_return` covers `cpu-mut → free`
    /// and `cpu-ref → free` per spec line 1234).
    #[test]
    fn op_names_are_unique_per_source_state() {
        for s in STATES.iter() {
            let mut seen = std::collections::HashSet::new();
            for t in transitions_from(*s) {
                assert!(
                    seen.insert(t.op_name),
                    "{:?} has two edges emitting `{}`",
                    s,
                    t.op_name,
                );
            }
        }
    }

    /// Emitted names must be usable as identifiers in both backends.
    #[test]
    fn op_names_are_lower_snake_identifiers() {
        for t in TRANSITIONS.iter() {
            assert!(!t.op_name.is_empty(), "{:?} has an empty op_name", t.from);
            assert!(
                t.op_name
                    .chars()
                    .all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '_'),
                "op_name {:?} must be lower_snake_case",
                t.op_name,
            );
            let first = t.op_name.chars().next().expect("non-empty");
            assert!(
                first.is_ascii_lowercase(),
                "op_name {:?} must start with a letter",
                t.op_name,
            );
        }
    }

    /// Read the spec body at each `spec_line` and require that the
    /// line declares the edge it is attached to.
    ///
    /// A line number that drifts off its target is invisible
    /// otherwise: nothing dereferences it, so a wrong one stays wrong
    /// and reads as documentation. This is the only check that can
    /// catch a misaimed citation, so it reads the file rather than a
    /// copy of it.
    #[test]
    fn spec_line_quotes_its_own_edge() {
        let spec = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../docs/spec/synth/rfc-sce-protocol-synthesis.md");
        let body = std::fs::read_to_string(&spec)
            .unwrap_or_else(|e| panic!("spec must be readable at {}: {e}", spec.display()));
        let lines: Vec<&str> = body.lines().collect();
        let mut checked = 0usize;
        for t in TRANSITIONS.iter() {
            let idx = t.spec_line as usize - 1;
            let line = lines.get(idx).unwrap_or_else(|| {
                panic!(
                    "spec has no line {} (file has {})",
                    t.spec_line,
                    lines.len()
                )
            });
            assert!(
                line.contains(t.from.spec_name()) && line.contains(t.to.spec_name()),
                "spec line {} does not declare {} → {}; it reads:\n  {line}",
                t.spec_line,
                t.from.spec_name(),
                t.to.spec_name(),
            );
            checked += 1;
        }
        assert_eq!(
            checked, TRANSITION_COUNT,
            "every edge must have had its citation checked",
        );
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
