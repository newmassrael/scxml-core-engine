// SPDX-License-Identifier: LGPL-2.1-or-later WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
//
//! Which reading `§scxml-B-2-8-1` gave a delivered payload.
//!
//! ⚠ A module of its own rather than a member of `crate::scripting`, and the
//! reason is a compile error rather than taste: that module is
//! `#[cfg(not(feature = "no_std"))]`, because the script-engine impls need
//! `alloc`. [`Engine`](crate::Engine) counts these readings and is exactly the
//! surface a `no_std` MCU consumer builds, so a type it names cannot live
//! behind that gate. Measured 2026-08-22 — `rustdoc-links` refused the
//! `no_std` profile with "not found in the crate root", pointing at
//! `pub use scripting::{… PayloadReading …}` as "an item that was configured
//! out".
//!
//! The C++ port hit the same wall from the other side and moved the same type
//! for the same reason: `StaticExecutionEngine.h` includes no `scripting/`
//! header, so `SCE::PayloadReading` lives in `core/PayloadReading.h`. A
//! reading is a fact about an EVENT's payload, not about the interface that
//! happens to produce it, and both ports now place it that way.
//!
//! Re-exported from `crate::scripting` as well, so a caller that reaches it
//! through the script-engine interface — where it is produced — still can.
//! Both mentions of that module are deliberately plain text rather than
//! intra-doc links: a link to it is itself unresolvable in the profile this
//! module exists for, which is the same gate, one layer up.

/// Which reading of `§scxml-B-2-8-1` a payload actually got.
///
/// The clause gives `_event.data` three readings and no fourth: content the
/// processor can interpret as XML becomes a DOM, content it can interpret as
/// a value becomes that value, and "otherwise, the Processor MUST treat the
/// content as a space-normalized string literal". Every engine here walks that
/// ladder, and until now every engine threw away which rung it landed on — the
/// generated binding call is literally `let _ = se.set_current_event(...)`.
///
/// Throwing it away is what makes a lost payload silent. Measured 2026-08-22
/// on three independent Lua implementations (mlua, go-lua and Lua 5.4), a host
/// that hands over `{["milestone"]="refined"}` — Lua's own table syntax, and
/// the workaround PR-87 replaced — gets the third rung, and a document that
/// then reads `_event.data.milestone` assigns nothing. In the worked
/// supervision loop that silently emptied `start_prompt` as well, so the
/// restarted session was primed with an empty string and the run converged
/// anyway. Nothing failed; the information simply stopped existing.
///
/// So the rung becomes a value the binding RETURNS rather than a fact it
/// discards. A separate "ask me afterwards" accessor was considered and
/// rejected: it can drift out of step with the binding it describes, and a
/// decision handed back by the function that made it cannot.
///
/// [`Undecodable`](Self::Undecodable) is the one a host acts on. It is not the
/// engine guessing from a leading brace — the script engine reports it because
/// it ATTEMPTED a structured read and that read failed, which is a fact only
/// the ladder itself holds.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum PayloadReading {
    /// The event carried no payload, so no rung applies.
    #[default]
    Absent,
    /// `§scxml-B-2-8-1` rung one: read as an XML document, bound as a DOM.
    Dom,
    /// Rung two: read as a value, bound as that value.
    Structured,
    /// Rung three, taken because nothing suggested the content was structured.
    /// A `<content>` element holding prose lands here, and that is correct —
    /// W3C test 562 pins it.
    Text,
    /// Rung three, taken AFTER a structured read was attempted and failed.
    ///
    /// The payload announced itself as structure and the datamodel could not
    /// read it, so `_event.data` holds the raw characters and every
    /// `_event.data.<field>` the document reads is empty. This is the reading
    /// a host is wrong about, and the only one it can act on.
    Undecodable,
}

impl PayloadReading {
    /// Whether this reading is one a host would want to hear about.
    ///
    /// Exactly one is: the other four are the ladder working. Written as a
    /// method rather than left to each caller's `match`, because "which
    /// readings are a problem" is a rule, and a rule with seven spellings is
    /// a rule that can be changed in one of them.
    #[must_use]
    pub const fn is_undecodable(self) -> bool {
        matches!(self, Self::Undecodable)
    }

    /// Which third-rung reading a payload that fell through to text deserves.
    ///
    /// The clause treats prose and a malformed object identically — both are
    /// "otherwise" — and a host does not. This is the one place that rule is
    /// written, so the ladder's implementations mirror a definition instead of
    /// each re-deciding what "looks structured" means.
    ///
    /// The test is the opening character, and deliberately only `{` and `[`.
    /// A number, a bare word or a quoted string is what an author writes in a
    /// `<content>` element, and W3C test 562 requires those to arrive as text
    /// without complaint; an object or an array is what a host CONSTRUCTS, and
    /// nobody constructs one by accident. Widening this to "anything that is
    /// not obviously prose" would report the ladder working as a defect, which
    /// is the failure that gets a diagnostic ignored.
    #[must_use]
    pub fn of_text(payload: &str) -> Self {
        match payload.trim_start().as_bytes().first() {
            Some(b'{' | b'[') => Self::Undecodable,
            _ => Self::Text,
        }
    }
}
