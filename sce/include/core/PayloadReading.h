// SPDX-License-Identifier: LGPL-2.1-or-later WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael

#pragma once

#include <string>

namespace SCE {

/**
 * @brief Which reading of §scxml-B-2-8-1 a payload actually got.
 *
 * The clause gives `_event.data` three readings and no fourth: content the
 * processor can interpret as XML becomes a DOM, content it can interpret as a
 * value becomes that value, and "otherwise, the Processor MUST treat the
 * content as a space-normalized string literal". Every engine here walks that
 * ladder, and until now every engine dropped which rung it landed on — the
 * generated binding call ended in `.get()` with the answer discarded, and the
 * Rust one was literally `let _ = se.set_current_event(...)`.
 *
 * Dropping it is what makes a lost payload silent. Measured 2026-08-22 on
 * three independent Lua implementations (mlua, go-lua and Lua 5.4), a host
 * that hands over `{["milestone"]="refined"}` — Lua's own table syntax, and
 * the workaround PR-87 replaced — gets the third rung, and a document that
 * then reads `_event.data.milestone` assigns nothing. In the worked
 * supervision loop that emptied `start_prompt` as well, so the restarted
 * session was primed with an empty string and the run converged anyway.
 * Nothing failed; the information stopped existing.
 *
 * `Undecodable` is the one a host acts on, and it is not the engine guessing
 * from a leading brace: the script engine reports it because it ATTEMPTED a
 * structured read and that read failed, which is a fact only the ladder holds.
 *
 * ⚠ WHY THIS FILE EXISTS RATHER THAN THE ENUM LIVING IN `IScriptEngine.h`.
 * It started there, beside `SetCurrentEventArgs`, and that is the wrong home
 * for it: the AOT engine counts these readings and is deliberately decoupled
 * from the script-engine interface — `StaticExecutionEngine.h` includes no
 * `scripting/` header, because the policy owns the engine handle and the
 * template never sees the interface. A reading is a fact about an EVENT's
 * payload, so it belongs with `core/EventMetadata.h`, which is what both sides
 * already depend on.
 *
 * Cross-language sibling: `sce_rust_runtime::PayloadReading`.
 */
enum class PayloadReading {
    Absent,      ///< The event carried no payload, so no rung applies.
    Dom,         ///< Rung one: read as an XML document, bound as a DOM.
    Structured,  ///< Rung two: read as a value, bound as that value.
    Text,        ///< Rung three, and nothing suggested the content was structured.
    Undecodable  ///< Rung three, AFTER a structured read was attempted and failed.
};

/**
 * @brief Which third-rung reading a payload that fell through to text deserves.
 *
 * The clause treats prose and a malformed object identically — both are
 * "otherwise" — and a host does not. This is the one place that rule is
 * written, so the ladder's implementations mirror a definition instead of each
 * deciding for itself what "looks structured" means.
 *
 * The test is the opening character, and deliberately only `{` and `[`. A
 * number, a bare word or a quoted string is what an author writes in a
 * `<content>` element, and W3C test 562 requires those to arrive as text
 * without complaint; an object or an array is what a host CONSTRUCTS, and
 * nobody constructs one by accident. Widening this to "anything not obviously
 * prose" would report the ladder working as a defect, which is the failure
 * that gets a diagnostic ignored.
 */
inline PayloadReading payloadReadingOfText(const std::string &payload) {
    const auto first = payload.find_first_not_of(" \t\n\r\f\v");
    if (first == std::string::npos) {
        return PayloadReading::Text;
    }
    const char c = payload[first];
    return (c == '{' || c == '[') ? PayloadReading::Undecodable : PayloadReading::Text;
}

}  // namespace SCE
