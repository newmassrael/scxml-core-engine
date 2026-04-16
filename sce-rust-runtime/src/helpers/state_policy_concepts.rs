// SPDX-License-Identifier: LGPL-2.1-or-later WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2025 newmassrael

//! Rust equivalents for C++20 concepts in `sce/include/core/StatePolicyConcepts.h`.
//!
//! ## Background
//!
//! C++ uses SFINAE-style detection traits like:
//! ```cpp
//! template <typename P, typename = void>
//! struct HasDataModelInitTrait : std::false_type {};
//! template <typename P, typename Engine>
//! struct HasDataModelInitTrait<P, std::void_t<
//!     decltype(std::declval<P>().initializeDataModel(std::declval<Engine&>()))
//! >> : std::true_type {};
//! ```
//! and then branches on them with `if constexpr (HasDataModelInit<P, Engine>) { ... }`.
//!
//! Rust has no structural typing. Instead, optional capabilities are signaled via
//! **associated `const bool` flags** on [`StatePolicy`](crate::StatePolicy):
//!
//! - `StatePolicy::NEEDS_DATA_MODEL_INIT`
//! - `StatePolicy::HAS_INVOKE_SUPPORT`
//! - `StatePolicy::HAS_FINALIZE`
//! - `StatePolicy::HAS_PARALLEL_STATES`
//! - `StatePolicy::HAS_AUTOFORWARD`
//! - `StatePolicy::HAS_CHILD_TICK`
//! - `StatePolicy::HAS_EXTERNAL_EVENT_FLAG`
//! - `StatePolicy::HAS_ACTIVE_STATES`
//!
//! The engine branches on these constants via `if P::HAS_X { ... }`. Rust's const
//! propagation eliminates the dead branch entirely — no runtime cost, matching C++
//! `if constexpr`.
//!
//! ## Why this module exists
//!
//! This file documents the mapping from C++ concept names to Rust flag names so
//! that code reviewers can cross-reference with C++ `StatePolicyConcepts.h`. It
//! also provides inline const helpers for readability at call sites.

use crate::policy::StatePolicy;

/// C++ `SCE::Core::HasDataModelInit<StatePolicy, Engine>` equivalent.
pub const fn has_data_model_init<P: StatePolicy>() -> bool {
    P::NEEDS_DATA_MODEL_INIT
}

/// C++ `SCE::Core::HasInvokeSupport<StatePolicy, Engine>` equivalent.
pub const fn has_invoke_support<P: StatePolicy>() -> bool {
    P::HAS_INVOKE_SUPPORT
}

/// C++ `SCE::Core::HasFinalize<StatePolicy, EventWithMetadata, Engine>` equivalent.
pub const fn has_finalize<P: StatePolicy>() -> bool {
    P::HAS_FINALIZE
}

/// C++ `SCE::Core::HasActiveStates<StatePolicy>` equivalent.
pub const fn has_active_states<P: StatePolicy>() -> bool {
    P::HAS_ACTIVE_STATES
}

/// C++ `SCE::Core::HasExternalEventFlag<StatePolicy>` equivalent.
pub const fn has_external_event_flag<P: StatePolicy>() -> bool {
    P::HAS_EXTERNAL_EVENT_FLAG
}

/// C++ `SCE::Core::HasAutoforward<StatePolicy, Engine>` equivalent.
pub const fn has_autoforward<P: StatePolicy>() -> bool {
    P::HAS_AUTOFORWARD
}

/// C++ `SCE::Core::HasChildTick<StatePolicy, Engine>` equivalent.
pub const fn has_child_tick<P: StatePolicy>() -> bool {
    P::HAS_CHILD_TICK
}

/// C++ `SCE::Core::ParallelStatePolicy<StatePolicy>` equivalent.
pub const fn is_parallel_state_policy<P: StatePolicy>() -> bool {
    P::HAS_PARALLEL_STATES
}
