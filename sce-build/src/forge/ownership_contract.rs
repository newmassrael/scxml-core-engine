// SPDX-License-Identifier: LGPL-2.1-or-later WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2025 newmassrael
//
// SCE Forge: analyzer-layer annotation contract (SCE Protocol-Synthesis
// RFC §synth-5-E lines 1349-1365). Single source of truth for the
// pointer-ownership facts that commercial static analyzers consume on
// toolchains where the typestate layer is inert — which, on C, is all
// of them.
//
// ── Why this is data rather than hand-written comments ─────────────
//
// PC-lint Plus and Coverity express the *same* fact in two syntaxes
// that disagree on argument numbering: PC-lint's `-sem` positions are
// 1-based, Coverity's `arg-N` positions are 0-based. Spec lines
// 1354-1357 show both forms side by side, and the spec's own example
// carried a defect that a hand-maintained pair cannot catch:
//
//     /*lint -function(sce_sample_take, sce_sample_payload) */
//
// PC-lint's `-function(f1, f2)` copies *all* of f1's semantics onto f2.
// Applied to this pair it would tell the analyzer that the borrow
// accessor `sce_sample_payload` also takes custody of its argument —
// the precise opposite of the borrow contract the header states with
// `callable_when("unconsumed")`, and a false-positive source on
// correct caller code. Rendering both syntaxes from one contract makes
// that class of divergence unrepresentable: `Custodial` appears in the
// PC-lint output only where it appears in the Coverity output, and the
// index conversion lives in exactly one function.
//
// ── Analyzer coverage, stated honestly ─────────────────────────────
//
// Spec line 1351 names "PC-Lint / Coverity / Polyspace". Only the
// first two consume in-source function models:
//
//   * PC-lint Plus — `-sem(f, sem...)` in a `/*lint ... */` comment.
//   * Coverity — `/* coverity[+free : arg-N] */` immediately preceding
//     the declaration.
//   * Polyspace — in-source `/* polyspace ... */` comments justify
//     *findings*; they cannot declare function behaviour. Behaviour
//     mapping is `-code-behavior-specifications`, a separate XML file
//     whose schema ships with the installation. SCE therefore emits
//     nothing Polyspace-specific rather than emitting a comment that
//     the tool silently ignores — a silently-inert hook is the failure
//     mode §2.4 invariant 1 forbids. Polyspace users get the
//     `SCE_WARN_UNUSED` result-check contract through MISRA C:2012
//     Rule 17.7, which every conforming MISRA checker implements.
//
// `PointerEffect::Borrow` renders a PC-lint null-check semantic but no
// Coverity annotation: Coverity's model primitives describe *state
// transitions* (allocate, free, kill, taint), and "reads without
// consuming" is the absence of a transition. Emitting a speculative
// primitive here would be an unverified claim about the tool.

/// What a function does to one pointer argument.
///
/// Ordering matters to no consumer; the discriminants exist so the
/// renderers can match exhaustively and a future effect cannot be
/// added without visiting both syntaxes.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum PointerEffect {
    /// The callee takes custody: after the call the caller must not
    /// use the pointee. Spec §synth-5-E lines 1360-1362 — "argument 1
    /// becomes invalid after the call; subsequent
    /// `sce_sample_payload(sample)` in the same scope is flagged".
    Custodial,
    /// The callee reads through the pointer and returns; the caller
    /// retains ownership. This is the
    /// `callable_when("unconsumed")` accessor shape.
    Borrow,
    /// The callee writes through the pointer (an out-parameter). The
    /// caller retains ownership and must not assume the pointee holds
    /// a meaningful value before the call.
    OutParam,
    /// The callee reads the pointee and writes it back — PC-lint's
    /// `inout(i)`. Distinct from [`Self::OutParam`]: the tag-checked
    /// pool API reads a handle's `state` to decide whether the
    /// transition is legal *and then* invalidates it, so an analyzer
    /// told the parameter is write-only would treat the caller's
    /// pre-call handle initialisation as dead.
    InOut,
}

impl PointerEffect {
    /// Stable lower-snake-case name used in diagnostics and test
    /// failure messages.
    pub fn spec_name(self) -> &'static str {
        match self {
            Self::Custodial => "custodial",
            Self::Borrow => "borrow",
            Self::OutParam => "out-param",
            Self::InOut => "inout",
        }
    }
}

/// One pointer argument's ownership contract.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ParamContract {
    /// Argument position, **0-based** — the storage convention. Both
    /// renderers convert from this single origin, so a numbering
    /// mistake cannot affect one analyzer without affecting the other.
    pub position: u8,
    /// What the callee does to the pointee.
    pub effect: PointerEffect,
    /// Whether the caller must guarantee a non-null pointer. False
    /// when the callee itself null-checks and returns a failure value;
    /// annotating such a parameter as null-hostile would make the
    /// analyzer flag the callee's own defensive branch as dead.
    pub caller_guarantees_non_null: bool,
}

/// The analyzer-layer contract for one C function.
///
/// The name borrows rather than owning so per-pool contracts — whose
/// names are assembled at codegen time from the pool's snake_case name
/// — render through the same code path as the `const` runtime table
/// without leaking a `&'static str` per pool.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct OwnershipContract<'a> {
    /// Function name as it appears in the emitted declaration.
    pub name: &'a str,
    /// Total declared parameter count, including non-pointer
    /// parameters absent from `params`. Bounds-checks every
    /// `ParamContract::position`.
    pub arity: u8,
    /// Pointer parameters carrying an ownership effect. Non-pointer
    /// parameters (`size_t dst_cap`) are intentionally absent.
    pub params: &'static [ParamContract],
    /// Whether the function may return a null pointer as a documented
    /// failure signal. Renders PC-lint's `r_null` so callers that skip
    /// the null check are flagged.
    pub may_return_null: bool,
}

impl OwnershipContract<'_> {
    /// Render the PC-lint Plus `-sem` annotation, or `None` when the
    /// contract carries no fact PC-lint can consume.
    ///
    /// Spec §synth-5-E line 1354 fixes the shape
    /// (`/*lint -sem(sce_sample_take, custodial(1)) */`). PC-lint
    /// argument positions are **1-based** while [`ParamContract`]
    /// stores 0-based positions, so `position + 1` here is the only
    /// place the two conventions meet.
    pub fn pc_lint_annotation(&self) -> Option<String> {
        let mut sems: Vec<String> = Vec::new();

        // Custody first: it is the fact the whole layer exists to
        // state, and reading it at the head of the option keeps the
        // rendered line legible in a diff.
        for p in self.params {
            if p.effect == PointerEffect::Custodial {
                sems.push(format!("custodial({})", p.position + 1));
            }
        }

        // `inout(i)` — read-then-written. Stated before the null-check
        // semantics so the option reads outermost-fact-first.
        for p in self.params {
            if p.effect == PointerEffect::InOut {
                sems.push(format!("inout({})", p.position + 1));
            }
        }

        if self.may_return_null {
            sems.push("r_null".to_string());
        }

        // `ip` (e.g. `2p`) tells PC-lint the ith argument is
        // dereferenced and must not be null at the call site.
        for p in self.params {
            if p.caller_guarantees_non_null {
                sems.push(format!("{}p", p.position + 1));
            }
        }

        if sems.is_empty() {
            return None;
        }
        Some(format!(
            "/*lint -sem({}, {}) */",
            self.name,
            sems.join(", ")
        ))
    }

    /// Render the Coverity function-model annotations, one per line,
    /// to be placed immediately before the declaration.
    ///
    /// Spec §synth-5-E line 1356 fixes the shape
    /// (`/* coverity[+free : arg-0] */`). Coverity argument positions
    /// are **0-based**, matching [`ParamContract`] storage, so no
    /// conversion happens here — the asymmetry against
    /// [`Self::pc_lint_annotation`] is the reason both renderers read
    /// from one contract instead of from each other.
    ///
    /// Only `Custodial` maps to a Coverity primitive (`+free`).
    /// `Borrow` and `OutParam` describe the absence of a state
    /// transition, which Coverity models by omission.
    pub fn coverity_annotations(&self) -> Vec<String> {
        self.params
            .iter()
            .filter(|p| p.effect == PointerEffect::Custodial)
            .map(|p| format!("/* coverity[+free : arg-{}] */", p.position))
            .collect()
    }

    /// Every rendered line for this contract, PC-lint first, in the
    /// order they appear above a declaration.
    pub fn rendered_lines(&self) -> Vec<String> {
        let mut lines = Vec::new();
        if let Some(sem) = self.pc_lint_annotation() {
            lines.push(sem);
        }
        lines.extend(self.coverity_annotations());
        lines
    }
}

/// Analyzer-layer contracts for the three `<sce/sample.h>` runtime
/// declarations (spec §synth-5-E lines 1318-1334).
///
/// `sce_sub_callback_t` is a typedef rather than a declaration and
/// therefore carries no annotation: PC-lint's `-sem` and Coverity's
/// model primitives both key on a *function* name, and the callback's
/// ownership contract is stated in the typestate spelling via
/// `param_typestate("unconsumed")` on the typedef's parameter.
pub const RUNTIME_CONTRACTS: [OwnershipContract<'static>; 2] = [
    // `const uint8_t *sce_sample_payload(const sce_sample_t *sample)`.
    // Borrow accessor — the declaration the spec's `-function(...)`
    // example would have mislabelled as custodial.
    OwnershipContract {
        name: "sce_sample_payload",
        arity: 1,
        params: &[ParamContract {
            position: 0,
            effect: PointerEffect::Borrow,
            caller_guarantees_non_null: true,
        }],
        may_return_null: false,
    },
    // `sce_result_t sce_sample_take(const sce_sample_t *sample,
    //                               uint8_t *dst, size_t dst_cap,
    //                               size_t *out_len)`.
    // Argument 2 (`dst_cap`) is a scalar and carries no pointer
    // effect, which is why positions jump 1 → 3.
    OwnershipContract {
        name: "sce_sample_take",
        arity: 4,
        params: &[
            ParamContract {
                position: 0,
                effect: PointerEffect::Custodial,
                caller_guarantees_non_null: true,
            },
            ParamContract {
                position: 1,
                effect: PointerEffect::OutParam,
                caller_guarantees_non_null: true,
            },
            ParamContract {
                position: 3,
                effect: PointerEffect::OutParam,
                caller_guarantees_non_null: true,
            },
        ],
        may_return_null: false,
    },
];

/// A per-pool function's contract. The pool's snake_case name
/// is a codegen-time value, so the contract stores the suffix and
/// [`pool_contract`] assembles the full name at render time.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PoolFnContract {
    /// Appended to the pool's snake_case name, e.g. `_slot_read`.
    pub suffix: &'static str,
    /// Declared parameter count.
    pub arity: u8,
    /// Pointer parameters carrying an ownership effect.
    pub params: &'static [ParamContract],
    /// Whether the function returns null on a tag-check failure.
    pub may_return_null: bool,
}

/// Analyzer-layer contracts for the tag-checked per-pool author API
/// (spec §synth-5-E lines 1239-1242).
///
/// The transition functions declare `InOut`, not `Custodial`: they
/// invalidate the *handle's tag* (`handle->state = SCE_SLOT_INVALID`)
/// rather than the handle's storage, which the caller still owns and
/// commonly reuses for the next acquire. Declaring custody here would
/// make the analyzer flag correct handle reuse.
///
/// The accessors (`_slot_read` / `_slot_write`) declare `Borrow` — they
/// read the handle's tag and return a pointer into pool storage without
/// touching the handle — plus `may_return_null`, which is the fact
/// the typestate layer cannot state at all: Clang's family has no
/// return-nullability attribute, so on a Clang build this contract is
/// strictly additive rather than a fallback.
///
/// None of them declares `caller_guarantees_non_null`: each opens with
/// an explicit `if (handle == NULL) return ...;` branch, and marking
/// the parameter null-hostile would render that branch unreachable to
/// the analyzer — turning a defensive check into a dead-code finding.
/// ── Runtime-seam entries ──────────────────────────────────────────
///
/// The six edges `buffer_pool_fsm::runtime_seam_transitions` yields are
/// emitted too, but only two of them appear here, and the split is the
/// one `SlotState::is_holdable` already draws.
///
/// An edge leaving a holdable state takes the caller's handle and
/// retags it in place, so it has a pointer argument to describe and
/// declares the same `InOut` shape as `_link_arm_tx`. An edge leaving a
/// DMA-owned state has no handle to take — the completion signal
/// carries a slot index and nothing else — so it reads `size_t idx` and
/// hands the resulting handle back *by value*, with `INVALID_HANDLE` as
/// the failure sentinel. That is the shape `pool_acquire_for_encode`
/// and `link_arm_rx` already use, and like them it carries no pointer
/// argument, so there is nothing for this table to say about it.
///
/// The by-value return was not the first design. An out-parameter was,
/// and `pool_functions_do_not_declare_their_guarded_parameters_null_hostile`
/// rejected it: a nullable out-parameter renders no annotation at all
/// (`OutParam` alone maps to no PC-lint semantic and no Coverity
/// primitive), and the only way to make it render was to declare it
/// non-null — which contradicts that test's standing claim that every
/// pool function guards its pointers. Returning by value removes the
/// pointer, and with it the choice between an empty contract and a
/// false one.
///
/// ── Address-publication entries ───────────────────────────────────
///
/// The hand-off states publish their slot's address, in the shape
/// `is_holdable` implies — and only the holdable one has a pointer
/// argument to describe. `dma-armed-rx` reads the caller's handle and
/// returns a pointer into pool storage without touching it, which is
/// `_slot_read`'s shape exactly: `Borrow` plus `may_return_null`.
/// `dma-armed-tx` hands out no handle, so its accessor takes a slot
/// index and there is nothing here to say about it, the same as the
/// index-keyed seam edges.
pub const POOL_CONTRACTS: [PoolFnContract; 7] = [
    // `bool <pool>_link_arm_tx(sce_slot_handle_t *handle)` —
    // spec §synth-5-E line 1142 (`cpu-mut → dma-armed-tx`).
    PoolFnContract {
        suffix: "_link_arm_tx",
        arity: 1,
        params: &[ParamContract {
            position: 0,
            effect: PointerEffect::InOut,
            caller_guarantees_non_null: false,
        }],
        may_return_null: false,
    },
    // `bool <pool>_pool_return(sce_slot_handle_t *handle)` —
    // spec §synth-5-E lines 1152, 1155.
    PoolFnContract {
        suffix: "_pool_return",
        arity: 1,
        params: &[ParamContract {
            position: 0,
            effect: PointerEffect::InOut,
            caller_guarantees_non_null: false,
        }],
        may_return_null: false,
    },
    // `const uint8_t *<pool>_slot_read(const sce_slot_handle_t *handle)`
    // — returns NULL when the handle's tag is not a CPU-visible state.
    PoolFnContract {
        suffix: "_slot_read",
        arity: 1,
        params: &[ParamContract {
            position: 0,
            effect: PointerEffect::Borrow,
            caller_guarantees_non_null: false,
        }],
        may_return_null: true,
    },
    // `uint8_t *<pool>_slot_write(sce_slot_handle_t *handle)` —
    // returns NULL unless the handle's tag is `cpu-mut`.
    PoolFnContract {
        suffix: "_slot_write",
        arity: 1,
        params: &[ParamContract {
            position: 0,
            effect: PointerEffect::Borrow,
            caller_guarantees_non_null: false,
        }],
        may_return_null: true,
    },
    // `bool <pool>_dma_start_rx(sce_slot_handle_t *handle)` —
    // spec §synth-5-E line 1149 (`dma-armed-rx -> dma-busy-rx`). Leaves a holdable
    // state, so the caller's handle is read then retagged.
    PoolFnContract {
        suffix: "_dma_start_rx",
        arity: 1,
        params: &[ParamContract {
            position: 0,
            effect: PointerEffect::InOut,
            caller_guarantees_non_null: false,
        }],
        may_return_null: false,
    },
    // `uint8_t *<pool>_dma_armed_rx_ptr(const sce_slot_handle_t *handle)`
    // — publishes the slot's address for the peripheral's descriptor.
    // Returns NULL unless the handle's tag is `dma-armed-rx`.
    PoolFnContract {
        suffix: "_dma_armed_rx_ptr",
        arity: 1,
        params: &[ParamContract {
            position: 0,
            effect: PointerEffect::Borrow,
            caller_guarantees_non_null: false,
        }],
        may_return_null: true,
    },
    // `bool <pool>_mutate_in_place(sce_slot_handle_t *handle)` —
    // spec §synth-5-E line 1153 (`cpu-ref -> cpu-mut`). Leaves a holdable
    // state, so the caller's handle is read then retagged.
    PoolFnContract {
        suffix: "_mutate_in_place",
        arity: 1,
        params: &[ParamContract {
            position: 0,
            effect: PointerEffect::InOut,
            caller_guarantees_non_null: false,
        }],
        may_return_null: false,
    },
];

impl PoolFnContract {
    /// The function's emitted name for a pool called `pool_snake_name`.
    pub fn qualified_name(&self, pool_snake_name: &str) -> String {
        format!("{}{}", pool_snake_name, self.suffix)
    }

    /// Render this contract's annotation lines for a concrete pool,
    /// reusing [`OwnershipContract`]'s renderers so per-pool output and
    /// runtime-header output cannot diverge in shape.
    pub fn rendered_lines(&self, pool_snake_name: &str) -> Vec<String> {
        let name = self.qualified_name(pool_snake_name);
        OwnershipContract {
            name: &name,
            arity: self.arity,
            params: self.params,
            may_return_null: self.may_return_null,
        }
        .rendered_lines()
    }
}

/// Render every per-pool annotation block for `snake_name`, keyed by
/// the fully-qualified function name so the template can look each one
/// up at its declaration site rather than positionally.
pub fn pool_annotations(snake_name: &str) -> Vec<(String, Vec<String>)> {
    POOL_CONTRACTS
        .iter()
        .map(|c| (c.qualified_name(snake_name), c.rendered_lines(snake_name)))
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Every declared position must address a real parameter. A
    /// contract that indexes past the declaration would render an
    /// annotation the analyzer rejects (PC-lint) or silently drops
    /// (Coverity) — the latter being the dangerous direction.
    #[test]
    fn every_param_position_is_within_arity() {
        for c in RUNTIME_CONTRACTS.iter() {
            for p in c.params {
                assert!(
                    p.position < c.arity,
                    "{}: param position {} is outside arity {}",
                    c.name,
                    p.position,
                    c.arity
                );
            }
        }
        for c in POOL_CONTRACTS.iter() {
            for p in c.params {
                assert!(
                    p.position < c.arity,
                    "{}: param position {} is outside arity {}",
                    c.suffix,
                    p.position,
                    c.arity
                );
            }
        }
    }

    /// No parameter may carry two effects. The runtime tables are
    /// `const` so this is a structural check on the literals, but it
    /// is the invariant the renderers assume when they filter by
    /// effect independently.
    #[test]
    fn no_parameter_carries_two_effects() {
        for c in RUNTIME_CONTRACTS.iter() {
            for (i, a) in c.params.iter().enumerate() {
                for b in c.params.iter().skip(i + 1) {
                    assert_ne!(
                        a.position,
                        b.position,
                        "{}: position {} declared twice ({} and {})",
                        c.name,
                        a.position,
                        a.effect.spec_name(),
                        b.effect.spec_name()
                    );
                }
            }
        }
    }

    /// The two syntaxes must agree on *which* argument is consumed,
    /// across the 1-based / 0-based boundary. This is the check the
    /// spec's hand-written pair could not have: it reconstructs the
    /// Coverity index from the PC-lint text and compares.
    #[test]
    fn pc_lint_and_coverity_agree_on_the_custodial_argument() {
        for c in RUNTIME_CONTRACTS.iter() {
            let custodial: Vec<u8> = c
                .params
                .iter()
                .filter(|p| p.effect == PointerEffect::Custodial)
                .map(|p| p.position)
                .collect();

            let sem = c.pc_lint_annotation().unwrap_or_default();
            let cov = c.coverity_annotations();

            assert_eq!(
                cov.len(),
                custodial.len(),
                "{}: expected one Coverity +free per custodial argument, got {:?}",
                c.name,
                cov
            );

            for pos in custodial {
                assert!(
                    sem.contains(&format!("custodial({})", pos + 1)),
                    "{}: PC-lint annotation must carry 1-based custodial({}); got {sem}",
                    c.name,
                    pos + 1
                );
                assert!(
                    cov.iter().any(|l| l.contains(&format!("arg-{pos}"))),
                    "{}: Coverity annotation must carry 0-based arg-{pos}; got {cov:?}",
                    c.name
                );
            }
        }
    }

    /// The defect the spec's example carried: a borrow accessor must
    /// never be told it consumes its argument. Guards both the
    /// contract table and the renderers.
    #[test]
    fn borrow_accessors_never_render_a_consuming_annotation() {
        for c in RUNTIME_CONTRACTS.iter() {
            let borrow_only = c
                .params
                .iter()
                .all(|p| p.effect != PointerEffect::Custodial);
            if !borrow_only {
                continue;
            }
            let rendered = c.rendered_lines().join("\n");
            assert!(
                !rendered.contains("custodial"),
                "{}: borrow-only declaration rendered a custodial semantic:\n{rendered}",
                c.name
            );
            assert!(
                !rendered.contains("+free"),
                "{}: borrow-only declaration rendered a Coverity +free:\n{rendered}",
                c.name
            );
        }
    }

    /// PC-lint's `-function(f1, f2)` copies every semantic from f1 to
    /// f2. Rendering it for any pair in this contract would propagate
    /// `sce_sample_take`'s custody onto whatever f2 is — the spec's
    /// defect. No renderer may emit the option.
    #[test]
    fn no_renderer_emits_the_semantics_copying_function_option() {
        let mut all: Vec<String> = RUNTIME_CONTRACTS
            .iter()
            .flat_map(|c| c.rendered_lines())
            .collect();
        all.extend(
            pool_annotations("demo_pool")
                .into_iter()
                .flat_map(|(_, lines)| lines),
        );
        for line in all {
            assert!(
                !line.contains("-function("),
                "`-function(f1, f2)` copies all of f1's semantics onto f2 and must \
                 never be rendered; got: {line}"
            );
        }
    }

    /// `sce_sample_take` is the declaration the whole layer exists for.
    /// Pin its exact rendering so a refactor of the renderers is
    /// visible in the diff rather than silently reshaping what ships
    /// to analyzers.
    #[test]
    fn sample_take_renders_the_spec_shape() {
        let take = RUNTIME_CONTRACTS
            .iter()
            .find(|c| c.name == "sce_sample_take")
            .expect("sce_sample_take contract");
        assert_eq!(
            take.rendered_lines(),
            vec![
                "/*lint -sem(sce_sample_take, custodial(1), 1p, 2p, 4p) */".to_string(),
                "/* coverity[+free : arg-0] */".to_string(),
            ]
        );
    }

    /// A parameter the callee itself null-checks must not be declared
    /// null-hostile: PC-lint would report the callee's own guard as
    /// unreachable. Every pool function opens with such a guard.
    #[test]
    fn pool_functions_do_not_declare_their_guarded_parameters_null_hostile() {
        for c in POOL_CONTRACTS.iter() {
            for p in c.params {
                assert!(
                    !p.caller_guarantees_non_null,
                    "{}: parameter {} is null-checked by the callee; declaring it \
                     null-hostile makes the guard look like dead code",
                    c.suffix, p.position
                );
            }
        }
    }

    /// The null-returning pool accessors must render `r_null` so a
    /// caller that skips the check is flagged. This is the fact the
    /// typestate layer
    /// cannot state — Clang's typestate family has no return-nullability
    /// attribute.
    #[test]
    fn null_returning_pool_accessors_render_r_null() {
        let rendered = pool_annotations("demo_pool");
        for (name, lines) in &rendered {
            let suffix_contract = POOL_CONTRACTS
                .iter()
                .find(|c| name.ends_with(c.suffix))
                .expect("every rendered name maps to a contract");
            let joined = lines.join("\n");
            if suffix_contract.may_return_null {
                assert!(
                    joined.contains("r_null"),
                    "{name}: returns NULL on tag mismatch but rendered no r_null:\n{joined}"
                );
            } else {
                assert!(
                    !joined.contains("r_null"),
                    "{name}: never returns NULL but rendered r_null:\n{joined}"
                );
            }
        }
    }

    /// Every contract must render at least one line. A contract whose
    /// effects happen to map to no analyzer semantic renders nothing,
    /// the codegen self-check passes vacuously, and the declaration
    /// ships with no analyzer coverage while looking covered — the
    /// failure this table exists to prevent. `_link_arm_tx` and
    /// `_pool_return` sat in exactly that state until `InOut` was
    /// separated from `OutParam`.
    #[test]
    fn every_contract_renders_at_least_one_annotation() {
        for c in RUNTIME_CONTRACTS.iter() {
            assert!(
                !c.rendered_lines().is_empty(),
                "{}: contract renders no annotation — the declaration would ship \
                 with no analyzer coverage while the tables claim it is covered",
                c.name
            );
        }
        for c in POOL_CONTRACTS.iter() {
            let lines = c.rendered_lines("demo_pool");
            assert!(
                !lines.is_empty(),
                "{}: contract renders no annotation — the declaration would ship \
                 with no analyzer coverage while the tables claim it is covered",
                c.suffix
            );
        }
    }

    /// Every emitted transition that takes a handle pointer must have
    /// a contract, and every one that does not must not.
    ///
    /// Read off `buffer_pool_fsm::TRANSITIONS` rather than a copy of
    /// it, so an edge added under the spec's FSM extension policy
    /// cannot reach the header uncovered. `is_holdable` on the source
    /// state is what decides which side an edge falls on, and it is
    /// the same predicate the templates branch on — so this test fails
    /// if the table and the emitted shape ever disagree about an edge.
    ///
    /// Both directions matter.
    /// `check_analyzer_annotations_emitted_c11` verifies the template
    /// renders every contract in this table; it cannot see a function
    /// the table never mentions. Only this test looks that way.
    #[test]
    fn contract_coverage_follows_the_handle_taking_edges() {
        use crate::forge::buffer_pool_fsm as fsm;
        let suffixes: std::collections::HashSet<&str> =
            POOL_CONTRACTS.iter().map(|c| c.suffix).collect();
        let (mut with_handle, mut by_value) = (0usize, 0usize);

        for t in fsm::TRANSITIONS.iter() {
            let suffix = format!("_{}", t.op_name);
            if t.from.is_holdable() {
                with_handle += 1;
                assert!(
                    suffixes.contains(suffix.as_str()),
                    "edge {} → {} emits `<pool>{suffix}(sce_slot_handle_t *)` but \
                     POOL_CONTRACTS has no entry — the declaration would ship with no \
                     analyzer coverage",
                    t.from.spec_name(),
                    t.to.spec_name(),
                );
            } else {
                by_value += 1;
                assert!(
                    !suffixes.contains(suffix.as_str()),
                    "edge {} → {} leaves a DMA-owned state, so it takes a slot index \
                     and returns by value — a pointer contract for `<pool>{suffix}` \
                     would describe an argument that does not exist",
                    t.from.spec_name(),
                    t.to.spec_name(),
                );
            }
        }

        // Lower bounds: a predicate that stopped selecting anything
        // would satisfy both loops vacuously. The numbers are counted
        // off the current table — five edges leave a holdable state
        // (`cpu-mut → dma-armed-tx`, `dma-armed-rx → dma-busy-rx`,
        // `cpu-ref → free`, `cpu-ref → cpu-mut`, `cpu-mut → free`) and
        // six leave one the API never hands out.
        assert_eq!(
            with_handle + by_value,
            fsm::TRANSITION_COUNT,
            "every edge must fall on exactly one side",
        );
        assert!(
            with_handle >= 5,
            "only {with_handle} handle-taking edges seen; is_holdable narrowed",
        );
        assert!(
            by_value >= 6,
            "only {by_value} by-value edges seen; is_holdable widened",
        );
    }

    /// The address accessors split on the same predicate the edges do,
    /// and their contracts have to follow.
    ///
    /// A hand-off state that hands out a handle publishes its address
    /// through that handle, so the accessor takes a pointer and needs a
    /// contract; one that does not takes a slot index, and a pointer
    /// contract for it would describe an argument that does not exist.
    /// Read off `STATES` so a bus master added under the FSM extension
    /// policy cannot reach the header with its accessor uncovered.
    #[test]
    fn address_accessor_contracts_follow_the_holdable_hand_off_states() {
        use crate::forge::buffer_pool_fsm as fsm;
        let suffixes: std::collections::HashSet<&str> =
            POOL_CONTRACTS.iter().map(|c| c.suffix).collect();
        let mut checked = 0usize;

        for st in fsm::STATES.iter().filter(|s| s.publishes_bus_address()) {
            let suffix = format!(
                "_{}",
                st.address_op_name()
                    .expect("a publishing state names its accessor")
            );
            if st.is_holdable() {
                assert!(
                    suffixes.contains(suffix.as_str()),
                    "{} publishes its address through a handle, so `<pool>{suffix}` \
                     takes `const sce_slot_handle_t *` — but POOL_CONTRACTS has no \
                     entry, and the declaration would ship with no analyzer coverage",
                    st.spec_name(),
                );
                let c = POOL_CONTRACTS
                    .iter()
                    .find(|c| c.suffix == suffix)
                    .expect("just asserted present");
                assert!(
                    c.may_return_null,
                    "{suffix} returns NULL on a tag mismatch; without may_return_null \
                     a caller that dereferences unchecked goes unflagged",
                );
            } else {
                assert!(
                    !suffixes.contains(suffix.as_str()),
                    "{} hands out no handle, so `<pool>{suffix}` takes a slot index — \
                     a pointer contract would describe an argument that does not exist",
                    st.spec_name(),
                );
            }
            checked += 1;
        }
        assert!(
            checked >= 2,
            "only {checked} hand-off states examined; the filter narrowed",
        );
    }

    /// The tag-checked transition functions read the handle before
    /// writing it, so they must render `inout`. A regression to
    /// `OutParam` would tell the analyzer the pre-call handle value is
    /// unused and flag correct callers.
    #[test]
    fn handle_transition_functions_render_inout() {
        for suffix in ["_link_arm_tx", "_pool_return"] {
            let c = POOL_CONTRACTS
                .iter()
                .find(|c| c.suffix == suffix)
                .unwrap_or_else(|| panic!("{suffix} contract"));
            let rendered = c.rendered_lines("demo_pool").join("\n");
            assert!(
                rendered.contains("inout(1)"),
                "{suffix}: reads the handle's tag before invalidating it and must \
                 render inout(1); got:\n{rendered}"
            );
        }
    }

    /// Pool names flow into the rendered text, so a pool named such
    /// that the suffix collides must still resolve to distinct
    /// annotations.
    #[test]
    fn pool_annotations_carry_the_fully_qualified_name() {
        let rendered = pool_annotations("rx_frames");
        assert_eq!(rendered.len(), POOL_CONTRACTS.len());
        for (name, lines) in rendered {
            assert!(name.starts_with("rx_frames_"), "unqualified name: {name}");
            for line in lines {
                assert!(
                    line.contains(&name),
                    "annotation for {name} does not name it: {line}"
                );
            }
        }
    }
}
