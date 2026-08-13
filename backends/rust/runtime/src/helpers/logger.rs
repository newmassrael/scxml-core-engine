// SPDX-License-Identifier: LGPL-2.1-or-later WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2025 newmassrael

//! Logging facade — thin re-exports of the `log` crate macros under SCE names.
//!
//! Ports C++ `sce/include/core/LogMacros.h`. Generated code calls
//! `::sce_rust_runtime::sce_log_debug!("AOT processTransition: ...")`, which
//! expands to `log::debug!(target: "sce", "...")`. Users configure output via
//! standard `log` facade backends (e.g., `env_logger`, `tracing-log`).
//!
//! The `target: "sce"` prefix matches the C++ spdlog logger name, enabling
//! cross-language log filtering (`RUST_LOG=sce=debug` or equivalent).
//!
//! # Why `$crate::log` and not `::log`
//!
//! These are `#[macro_export]` macros, so their bodies resolve paths in the
//! *calling* crate. A `::log::debug!` expansion therefore demands that every
//! consumer of a generated machine declare `log` in its own `Cargo.toml` —
//! a dependency contract that appears in no manifest, no README, and no
//! generated header. `$crate` resolves to `sce_rust_runtime` at every call
//! site, so `$crate::log` reaches the [re-export](crate::log) the runtime
//! already carries and the consumer needs no `log` entry at all.
//!
//! This keeps logging under the same rule as the rest of the generated
//! surface: emitted code names one crate, `sce-rust-runtime`, and reaches
//! everything else through it.

/// Debug-level log. Matches C++ `SCE_LOG_DEBUG(...)`.
#[macro_export]
macro_rules! sce_log_debug {
    ($($arg:tt)*) => {
        $crate::log::debug!(target: "sce", $($arg)*)
    };
}

/// Info-level log. Matches C++ `SCE_LOG_INFO(...)`.
#[macro_export]
macro_rules! sce_log_info {
    ($($arg:tt)*) => {
        $crate::log::info!(target: "sce", $($arg)*)
    };
}

/// Warning-level log. Matches C++ `SCE_LOG_WARN(...)`.
#[macro_export]
macro_rules! sce_log_warn {
    ($($arg:tt)*) => {
        $crate::log::warn!(target: "sce", $($arg)*)
    };
}

/// Error-level log. Matches C++ `SCE_LOG_ERROR(...)`.
#[macro_export]
macro_rules! sce_log_error {
    ($($arg:tt)*) => {
        $crate::log::error!(target: "sce", $($arg)*)
    };
}
