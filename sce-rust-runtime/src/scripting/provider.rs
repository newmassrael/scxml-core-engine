// SPDX-License-Identifier: LGPL-2.1-or-later OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2025 newmassrael

//! Compile-time script engine selector.
//!
//! 1:1 port of C++ `sce/include/scripting/ScriptEngineProvider.h`. Engine
//! selection happens at **build time** via Cargo features, not at runtime:
//!
//! - `script-engine-lua` (default)   → delegates to `sce-rust-lua`
//! - `script-engine-quickjs` (future) → delegates to `sce-rust-quickjs`
//! - `no-script`                     → compile error / panic if script needed
//!
//! Only one engine feature may be active per build. Exactly matches C++
//! `cmake -DSCE_SCRIPT_ENGINE=lua|quickjs`.
//!
//! Zero overhead: no mutex, no `Box<dyn>`, no runtime factory. Each call
//! resolves to a direct static reference.
//!
//! ## Phase 1 note
//!
//! Phase 1 ships with the provider returning a **panicking stub**. The real
//! Lua engine lives in the `sce-rust-lua` crate, which Phase 1 skeleton
//! implements with `unimplemented!()` stubs. Phase 3 completes the Lua impl.
//! Phase 2 does not call the provider (pure-static generation only).
//!
//! To avoid the cross-crate runtime dependency in Phase 1, `provider.rs`
//! uses a **weak linkage** pattern: it exposes a `set_script_engine()` hook
//! that `sce-rust-lua`'s crate-init function calls to register itself. This
//! lets `sce-rust-runtime` compile without depending on `sce-rust-lua`, and
//! avoids a circular dependency.

use std::sync::OnceLock;

use crate::scripting::i_script_engine::IScriptEngine;

/// The compile-time-configured script engine provider.
///
/// Matches C++ `SCE::ScriptEngineProvider` class 1:1.
pub struct ScriptEngineProvider;

/// Singleton storage for the registered script engine.
///
/// Populated at program startup by the script engine crate (e.g., `sce-rust-lua`
/// calls `set_script_engine(&LUA_ENGINE)` during its static initialization).
/// Subsequent calls to `get()` return this reference with zero overhead.
static SCRIPT_ENGINE: OnceLock<&'static dyn IScriptEngine> = OnceLock::new();

impl ScriptEngineProvider {
    /// Get the compile-time selected script engine instance.
    ///
    /// Matches C++ `ScriptEngineProvider::getScriptEngine()`. Returns the engine
    /// registered via [`set_script_engine`]. Panics if no engine is registered —
    /// indicates a build misconfiguration (no script engine feature enabled).
    pub fn get() -> &'static dyn IScriptEngine {
        *SCRIPT_ENGINE.get().expect(
            "No script engine registered. Ensure the selected feature \
             (`script-engine-lua` or `script-engine-quickjs`) is enabled in Cargo.toml, \
             and that your application calls the engine crate's `register()` function \
             at startup (or links the crate so its `ctor` initializer runs).",
        )
    }

    /// Human-readable engine name (e.g., `"Lua 5.4"`, `"QuickJS"`).
    ///
    /// Matches C++ `getEngineName()`. Determined at compile time via feature flags.
    pub const fn get_engine_name() -> &'static str {
        #[cfg(feature = "script-engine-lua")]
        {
            "Lua 5.4"
        }
        #[cfg(feature = "script-engine-quickjs")]
        {
            "QuickJS"
        }
        #[cfg(not(any(feature = "script-engine-lua", feature = "script-engine-quickjs")))]
        {
            "None"
        }
    }

    /// CMake-style engine ID (e.g., `"lua"`, `"quickjs"`).
    pub const fn get_engine_id() -> &'static str {
        #[cfg(feature = "script-engine-lua")]
        {
            "lua"
        }
        #[cfg(feature = "script-engine-quickjs")]
        {
            "quickjs"
        }
        #[cfg(not(any(feature = "script-engine-lua", feature = "script-engine-quickjs")))]
        {
            "none"
        }
    }
}

/// Register the global script engine singleton.
///
/// Must be called at most once, before any state machine is initialized. Typically
/// called from the script engine crate's `pub fn register()` entrypoint or a
/// `#[ctor]` static initializer.
///
/// Returns `Err(())` if an engine was already registered — this is a programming
/// error and callers should panic.
pub fn set_script_engine(engine: &'static dyn IScriptEngine) -> Result<(), ()> {
    SCRIPT_ENGINE.set(engine).map_err(|_| ())
}

/// Whether a script engine has been registered via [`set_script_engine`].
pub fn has_script_engine() -> bool {
    SCRIPT_ENGINE.get().is_some()
}
