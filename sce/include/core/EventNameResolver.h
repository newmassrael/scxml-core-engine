// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
//
// This file is part of SCE (SCXML Core Engine).
//
// Dual Licensed:
// 1. AGPL-3.0 + Linking Exception: Free for unmodified use (see LICENSE-EXCEPTION.md)
// 2. Commercial: For modifications (contact newmassrael@gmail.com)
//
// Commercial License:
//   Individual: $100 cumulative
//   Enterprise: $500 cumulative
//   Contact: https://github.com/newmassrael
//
// Full terms: https://github.com/newmassrael/scxml-core-engine/blob/main/LICENSE

#pragma once

#include <string>
#include <string_view>

namespace SCE::Core {

/**
 * @brief The event an event that arrives BY NAME is delivered as (§scxml-3.12.1)
 *
 * An event arrives by name from outside the document — a child's autoforward, a
 * host, a Mesh peer — so the names it can arrive under are open, while the
 * generated `Event` enum holds only the names the document writes. A
 * transition's descriptor matches an event by whole tokens: `request` matches
 * `request.new` whether or not the document writes `request.new`. Every
 * descriptor that matches an arriving name is a token prefix of it, so is one
 * of the document's names, so is a prefix of the LONGEST of those — which
 * therefore matches exactly what the arriving name would. That is the event this
 * answers: the name itself when the document writes it, else the longest token
 * prefix of it the document does, else the wildcard member — the one
 * `event="*"` stands for, which `exact` keeps under the name `*` (no document
 * can write that as an event's name) — when the document has such a
 * transition, else the empty optional: no transition the document has could
 * match it, and it is dropped.
 *
 * `exact` is the policy's `getEventFromName`, which stays the exact table the
 * generated code writes; the rule is written once, here, for every place a name
 * crosses into a machine. The Rust, Go and Kotlin twins are
 * `StatePolicy::resolve_event_by_name`, `Engine.resolveEventByName` and
 * `StateMachineEngine.resolveArrivingEvent`; C11's is inside
 * `<machine>_raise_external_forwarded`.
 *
 * @tparam Lookup Callable taking a `const std::string &` and returning the
 *         `std::optional<Event>` of the document's own event of that name
 */
template <typename Lookup> auto resolveArrivingEventName(std::string_view name, Lookup &&exact) {
    std::string candidate(name);
    for (;;) {
        if (auto event = exact(candidate)) {
            return event;
        }
        const auto at = candidate.rfind('.');
        if (at == std::string::npos) {
            return exact(std::string("*"));
        }
        candidate.resize(at);
    }
}

}  // namespace SCE::Core
