// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2025 newmassrael

#pragma once

#include "SCXMLTypes.h"
#include "core/EventOrigin.h"
#include <optional>
#include <string>
#include <utility>

namespace SCE::Core {

/**
 * @brief Event metadata container for §scxml-5.10 compliance
 *
 * Single Source of Truth for event metadata shared between:
 * - Interpreter engine (StateMachine)
 * - AOT engine (StaticExecutionEngine)
 *
 * Consolidates all event-related metadata into a single structure to simplify
 * API and reduce parameter coupling. The text fields are empty when not set.
 *
 * §scxml-5.10 Event Object Fields:
 * - name: Event name (required)
 * - type: Event type ("internal", "platform", "external")
 * - sendid: Send ID from <send> element
 * - origin, origintype: the reply address the delivering processor wrote, as
 *   one value (docs/adr/0090); none for an event no processor stamped
 * - invokeid: Invoke ID from <invoke> element that generated this event
 * - data: Event payload data
 *
 * `sender` is not an `_event` field. It is the id of the session that sent the
 * event, written by the processor that delivered it from that session: the
 * SCXML Event I/O Processor for a local session, and the Mesh transport for a
 * remote child or parent (SCE_MESH.md). Any other sender leaves it empty. It
 * is what the session-keyed lookups read (`<finalize>`, a cancelled
 * invocation's events, the Interpreter's queue choice). Nothing reads the
 * origin's address to find a session, and no rule ties that address to it.
 */
struct EventMetadata {
    std::string name;                      ///< Event name (required)
    std::string data;                      ///< Event data as JSON string
    std::string type;                      ///< Event type ("internal", "platform", "external")
    std::string sendId;                    ///< Send ID from <send> element
    std::string invokeId;                  ///< Invoke ID from <invoke> element
    std::optional<EventOrigin> origin;     ///< `_event.origin` and `_event.origintype`, none when unstamped
    std::string sender;                    ///< Sending session's id, empty unless a session sent it
    std::optional<ScriptValue> typedData;  ///< Typed event data (engine-agnostic, avoids JSON round-trip)

    /**
     * @brief Construct minimal event metadata with name and data
     */
    EventMetadata(const std::string &eventName = "", const std::string &eventData = "")
        : name(eventName), data(eventData) {}

    /**
     * @brief Construct full event metadata with all fields
     */
    EventMetadata(const std::string &eventName, const std::string &eventData, const std::string &eventType,
                  const std::string &sendId = "", const std::string &invokeId = "",
                  std::optional<EventOrigin> eventOrigin = std::nullopt, std::string senderSessionId = "")
        : name(eventName), data(eventData), type(eventType), sendId(sendId), invokeId(invokeId),
          origin(std::move(eventOrigin)), sender(std::move(senderSessionId)) {}
};

}  // namespace SCE::Core
