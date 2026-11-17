// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
//
// SCE::Common::errorData: the `_event.data` of an `error.*` event the
// processor raises for itself, the record SCE_MESH.md section 10.7.1 pins
// (docs/adr/0106, #111): `errorName` ("execution" or "communication"),
// `reason` (the code) and `detail` (the sentence the raise site wrote).
//
// One form for both C++ engines: the AOT engine's
// `EventWithMetadata::platformError` and the Interpreter's
// `IEventRaiser::raiseError` build their data here. The code is a member of
// the enum generated from sce:std/engine/error_reason.scxml
// (tools/codegen/trees/runtime-cpp.toml), spelled by its declared-name
// accessor, so the document is the one spelling of it.
//
// Header-only and free of nlohmann, as JsonText.h is, because generated code
// includes it through the static engine.

#pragma once

#include <cstdint>
#include <string>
#include <string_view>

#include "common/JsonText.h"
#include "generated/error_reason.h"

namespace SCE::Common {

/// The reasons an engine raises `error.execution` and `error.communication`
/// for: the enum the vocabulary document declares.
using ErrorReason = ::SCE::Generated::ErrorReason::ErrorReason;

/// The first value of the communication half of the vocabulary: 1 to 63 are
/// raised as `error.execution`, 64 to 127 as `error.communication`.
inline constexpr uint8_t kCommunicationReasonsFrom = 64;

/// The `errorName` of an `error.*` event raised for `reason`: read from the
/// code's value range, so a code and the event it is raised as cannot disagree.
constexpr const char *errorName(ErrorReason reason) noexcept {
    return ::SCE::Generated::ErrorReason::to_underlying(reason) < kCommunicationReasonsFrom ? "execution"
                                                                                            : "communication";
}

/// The name of the event `reason` is raised as: `error.execution` or
/// `error.communication`.
inline std::string errorEventName(ErrorReason reason) {
    return std::string("error.") + errorName(reason);
}

/// The `_event.data` of an `error.*` event raised for `reason`, as JSON text:
/// `{"errorName":...,"reason":...,"detail":...}`, `detail` escaped as every
/// engine escapes a JSON string.
inline std::string errorData(ErrorReason reason, std::string_view detail) {
    const char *name = ::SCE::Generated::ErrorReason::declared_name(reason);
    std::string data;
    data.reserve(detail.size() + 64);
    data += "{\"errorName\":\"";
    data += errorName(reason);
    data += "\",\"reason\":\"";
    // A value no member names is not one the vocabulary declares; the enum
    // can hold it only through a cast, and the record then names no code.
    data += name != nullptr ? name : "";
    data += "\",\"detail\":";
    data += ::SCE::JsonText::quoted(detail);
    data += '}';
    return data;
}

}  // namespace SCE::Common
