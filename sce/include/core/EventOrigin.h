// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
//
// The reply address an event carries (docs/adr/0090, section 1): the Event I/O
// Processor that delivered it and the address that processor accepts as a
// `<send>` target. The document reads the two as `_event.origintype` and
// `_event.origin`; they are one value because the address means something only
// relative to its processor.
//
// The processor that delivers an event writes the pair once, where it builds the
// event (section 2), and nothing converts it later. An event that no processor
// stamped carries none, never half of a pair.

#pragma once

#include <optional>
#include <string>
#include <utility>

namespace SCE::Core {

/**
 * @brief Where a reply to an event goes: the processor and the address it accepts
 *
 * Constructed whole. There is no default constructor and no constructor from one
 * string, so a pair with one field set cannot be spelled by accident; an event with
 * no origin holds `std::nullopt` instead.
 */
struct EventOrigin {
    /// @param processorType The canonical type URI of the Event I/O Processor
    /// @param processorAddress A URI that processor accepts as a `<send>` target
    EventOrigin(std::string processorType, std::string processorAddress)
        : processor(std::move(processorType)), address(std::move(processorAddress)) {}

    /// The canonical type URI of the Event I/O Processor that delivered the event
    /// (`_event.origintype`).
    std::string processor;
    /// A URI that processor accepts as a `<send>` target (`_event.origin`).
    std::string address;

    friend bool operator==(const EventOrigin &a, const EventOrigin &b) {
        return a.processor == b.processor && a.address == b.address;
    }

    friend bool operator!=(const EventOrigin &a, const EventOrigin &b) {
        return !(a == b);
    }
};

/// `_event.origin` of an event that carries @p origin: its address, or the empty
/// string when the event has none.
inline std::string originAddressOf(const std::optional<EventOrigin> &origin) {
    return origin.has_value() ? origin->address : std::string();
}

/// `_event.origintype` of an event that carries @p origin: its processor, or the
/// empty string when the event has none.
inline std::string originProcessorOf(const std::optional<EventOrigin> &origin) {
    return origin.has_value() ? origin->processor : std::string();
}

}  // namespace SCE::Core
