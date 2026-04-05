// SPDX-License-Identifier: LGPL-2.1-or-later OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2025 newmassrael

//! Logging facade — thin re-exports of the `log` crate macros under SCE names.
//!
//! Ports C++ `sce/include/core/LogMacros.h`. Generated code calls
//! `sce_log_debug!("AOT processTransition: ...")` which expands to
//! `log::debug!(target: "sce", "...")`. Users configure output via standard
//! `log` facade backends (e.g., `env_logger`, `tracing-log`).
//!
//! The `target: "sce"` prefix matches the C++ spdlog logger name, enabling
//! cross-language log filtering (`RUST_LOG=sce=debug` or equivalent).

/// Debug-level log. Matches C++ `SCE_LOG_DEBUG(...)`.
#[macro_export]
macro_rules! sce_log_debug {
    ($($arg:tt)*) => {
        ::log::debug!(target: "sce", $($arg)*)
    };
}

/// Info-level log. Matches C++ `SCE_LOG_INFO(...)`.
#[macro_export]
macro_rules! sce_log_info {
    ($($arg:tt)*) => {
        ::log::info!(target: "sce", $($arg)*)
    };
}

/// Warning-level log. Matches C++ `SCE_LOG_WARN(...)`.
#[macro_export]
macro_rules! sce_log_warn {
    ($($arg:tt)*) => {
        ::log::warn!(target: "sce", $($arg)*)
    };
}

/// Error-level log. Matches C++ `SCE_LOG_ERROR(...)`.
#[macro_export]
macro_rules! sce_log_error {
    ($($arg:tt)*) => {
        ::log::error!(target: "sce", $($arg)*)
    };
}
