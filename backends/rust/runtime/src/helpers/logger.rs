// SPDX-License-Identifier: AGPL-3.0-only WITH AdditionRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
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
//! site, so `$crate::log` reaches the `sce_rust_runtime::log` re-export the
//! runtime carries and the consumer needs no `log` entry at all.
//!
//! This keeps logging under the same rule as the rest of the generated
//! surface: emitted code names one crate, `sce-rust-runtime`, and reaches
//! everything else through it.
//!
//! # Without the `log` feature
//!
//! The facade is the runtime's `log` feature (docs/adr/0053, amendment of
//! 2026-11-25). Without it the crate neither depends on `log` nor re-exports
//! it, and every macro here expands to a block that type-checks its arguments
//! as `format_args!` does and never evaluates them: a call site that compiles
//! in one build compiles in the other, and the image carries no logger.
//!
//! Which of the two expansions a call site gets is decided by the runtime's own
//! build, because the `cfg` stands on the definition of `__sce_log!` below
//! and not inside its body: a `cfg!` in a `#[macro_export]` body would read the
//! features of the crate that calls it.

/// The one expansion of the `sce_log_*!` family: a record of the facade's
/// `$level` under the target `sce`. Not part of the surface; reached only
/// through the four macros below.
#[cfg(feature = "log")]
#[doc(hidden)]
#[macro_export]
macro_rules! __sce_log {
    ($level:ident, $($arg:tt)*) => {
        $crate::log::$level!(target: "sce", $($arg)*)
    };
}

/// The one expansion of the `sce_log_*!` family in a build without the `log`
/// feature: the arguments are checked as a format string and its values, and
/// the branch that holds them is never taken, so nothing is evaluated and
/// nothing is written.
#[cfg(not(feature = "log"))]
#[doc(hidden)]
#[macro_export]
macro_rules! __sce_log {
    ($level:ident, $($arg:tt)*) => {{
        if false {
            let _ = ::core::format_args!($($arg)*);
        }
    }};
}

/// Debug-level log. Matches C++ `SCE_LOG_DEBUG(...)`.
#[macro_export]
macro_rules! sce_log_debug {
    ($($arg:tt)*) => {
        $crate::__sce_log!(debug, $($arg)*)
    };
}

/// Info-level log. Matches C++ `SCE_LOG_INFO(...)`.
#[macro_export]
macro_rules! sce_log_info {
    ($($arg:tt)*) => {
        $crate::__sce_log!(info, $($arg)*)
    };
}

/// Warning-level log. Matches C++ `SCE_LOG_WARN(...)`.
#[macro_export]
macro_rules! sce_log_warn {
    ($($arg:tt)*) => {
        $crate::__sce_log!(warn, $($arg)*)
    };
}

/// Error-level log. Matches C++ `SCE_LOG_ERROR(...)`.
#[macro_export]
macro_rules! sce_log_error {
    ($($arg:tt)*) => {
        $crate::__sce_log!(error, $($arg)*)
    };
}
