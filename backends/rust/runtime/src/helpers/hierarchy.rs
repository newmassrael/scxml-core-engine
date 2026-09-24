// SPDX-License-Identifier: LGPL-2.1-or-later WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2025 newmassrael

//! The bounded chain every configuration, and every set of states Appendix D's
//! procedures build, is held in.
//!
//! The walks over the hierarchy — domains, exit sets, entry sets — are
//! [`microstep`](super::microstep)'s. They used to be here as an LCA and a pair
//! of entry/exit chain builders, which could name one target and one region at
//! a time; the appendix's procedures replaced them, and what is left is the
//! storage they share.
//!
//! ## Cycle detection
//!
//! Every walk up the hierarchy is bounded at `MAX_HIERARCHY_DEPTH` (16 levels)
//! to detect cyclic parent relationships. A cycle indicates a generator bug or
//! corrupted SCXML; the walk panics with a diagnostic message. Matches C++
//! `throw std::runtime_error` semantics — both are fatal, unrecoverable errors.
//!
//! ## no_std variant (SCE Protocol-Synthesis RFC §synth-5-J-2)
//!
//! Under `--features=no_std`, [`StateChain`] is a stack-allocated `heapless::Vec`
//! capped at [`MAX_HIERARCHY_DEPTH`] (= 16), so the no_std push paths are
//! infallible for any configuration the document can hold — they `.expect()`
//! only as a generator-bug tripwire. No new capacity constant is introduced;
//! the std `Vec<S>` and no_std `heapless::Vec<S, 16>` share the single existing
//! `MAX_HIERARCHY_DEPTH` invariant.

/// Maximum supported state hierarchy depth. Prevents infinite loops from cyclic
/// parent relationships and bounds stack/heap allocation. Matches C++ `MAX_DEPTH = 16`.
///
/// W3C SCXML has no normative depth limit; 16 covers every real-world document we've
/// encountered (typical: 1-5, complex: up to 10).
pub const MAX_HIERARCHY_DEPTH: usize = 16;

/// Compile-time bounded chain of states: a configuration, an exit or entry set,
/// a list of effective targets, a history record.
///
/// - **std build**: [`Vec<S>`] — unbounded heap allocation, capacity hint only.
/// - **no_std build**: `heapless::Vec<S, MAX_HIERARCHY_DEPTH>` — stack-allocated,
///   compile-time-capped at 16 elements. Every one of those is a set of states
///   that are, or are about to be, active, so it fits the configuration's bound;
///   a heapless push failure indicates a configuration larger than the bound or
///   a generator bug.
///
/// SCE Protocol-Synthesis RFC §synth-5-J-2 (lines 1989-1994): reuses [`MAX_HIERARCHY_DEPTH`] rather
/// than introducing a new capacity constant — the same depth invariant bounds both
/// the iteration count and the heapless allocation.
#[cfg(not(feature = "no_std"))]
pub type StateChain<S> = ::std::vec::Vec<S>;
/// no_std variant of [`StateChain`]: stack-allocated `heapless::Vec` capped
/// at [`MAX_HIERARCHY_DEPTH`]. See the std-variant doc-comment above for the
/// full contract.
#[cfg(feature = "no_std")]
pub type StateChain<S> = ::heapless::Vec<S, MAX_HIERARCHY_DEPTH>;

/// Push into a [`StateChain`] uniformly under std and no_std.
///
/// Under std this is `Vec::push`. Under no_std this is `heapless::Vec::push` with
/// an `.expect()` tripwire — every chain holds states of one configuration, which
/// [`MAX_HIERARCHY_DEPTH`] bounds, so the push failure path is unreachable under
/// valid input.
///
/// `pub` so generated state machines (`tools/codegen/templates/rust/`) can call
/// the cfg-branched push body without each fixture inlining the branch. Single
/// source of truth shared between the runtime (`Engine::get_active_states`,
/// `helpers::microstep`) and the template-emitted active-set writer.
#[inline]
pub fn push_chain<S: core::fmt::Debug>(chain: &mut StateChain<S>, item: S) {
    #[cfg(not(feature = "no_std"))]
    {
        chain.push(item);
    }
    #[cfg(feature = "no_std")]
    {
        chain.push(item).expect(
            "hierarchy: chain capacity exhausted — depth check at MAX_HIERARCHY_DEPTH should have fired first (generator bug or corrupted hierarchy)",
        );
    }
}

/// Construct an empty [`StateChain`].
///
/// Under std this is `Vec::with_capacity(8)` (preserves the existing pre-allocation
/// hint for typical depths 1-5). Under no_std this is `heapless::Vec::new()` — the
/// capacity is fixed at compile time so the hint is a no-op.
///
/// `pub` so generated state machines (`tools/codegen/templates/rust/`) can construct
/// empty chains via the cfg-branched path that resolves identically across std and
/// no_std builds.
#[inline]
pub fn new_chain<S>() -> StateChain<S> {
    #[cfg(not(feature = "no_std"))]
    {
        ::std::vec::Vec::with_capacity(8)
    }
    #[cfg(feature = "no_std")]
    {
        ::heapless::Vec::new()
    }
}

/// Construct a [`StateChain`] from a compile-time-sized array of items.
///
/// Replaces the `vec![a, b, c]` macro — `vec!` is a std-only macro (`alloc`
/// doesn't re-export it without the `alloc` feature, and heapless has no
/// equivalent) — so a host builds the configuration
/// [`Engine::enter_at`](crate::Engine::enter_at) takes the same way on either
/// runtime profile.
///
/// `N` must be `<= MAX_HIERARCHY_DEPTH` (= 16); larger arrays trigger the same
/// capacity-exhausted panic as [`push_chain`].
#[inline]
pub fn state_chain_from_slice<S: core::fmt::Debug, const N: usize>(items: [S; N]) -> StateChain<S> {
    let mut chain = new_chain::<S>();
    for item in items {
        push_chain(&mut chain, item);
    }
    chain
}
