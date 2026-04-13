// SPDX-License-Identifier: LGPL-2.1-or-later OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2025 newmassrael
//
// SCE Mesh MeshWireFormat — shared wire encoding for byte-stream transports.
//
// Wire format: [event_name bytes]\0[data bytes]
//
// The null byte separates the event name from the optional JSON payload.
// Receiver splits at the first null byte: everything before is the event
// name, everything after is the data. Empty tail means payload-less event.
//
// This format is used by all byte-stream transports (local, shm, someip,
// zenoh, dds, grpc, ...). Signal-based (CAN, LIN) and field-oriented
// (OPC UA) transports use protocol-native encoding.
//
// See SCE_MESH.md Section 3.2 (Transport Groups) and 7.5 (Mesh-Native
// Serialization).

#pragma once

#include "mesh/MeshSendRequest.h"

#include <cstddef>
#include <cstdint>
#include <cstring>
#include <string_view>
#include <vector>

namespace SCE::Mesh {

/// Encode a MeshSendRequest into the mesh wire format.
///
/// @return byte vector: [event_name\0][data bytes]
inline std::vector<uint8_t> encodeWirePayload(const MeshSendRequest& req) {
    auto name_len = req.eventName.size();
    auto total = name_len + 1 + req.data.size();
    std::vector<uint8_t> buf(total);
    std::memcpy(buf.data(), req.eventName.c_str(), name_len + 1);
    if (!req.data.empty()) {
        std::memcpy(buf.data() + name_len + 1, req.data.data(), req.data.size());
    }
    return buf;
}

/// Decoded view into a mesh wire payload.
///
/// The views point into the original byte buffer — callers must not use
/// them after the underlying buffer is released.
struct WirePayloadView {
    std::string_view eventName;  ///< empty on decode failure
    std::string_view data;        ///< empty if payload-less (valid)
};

/// Decode a mesh wire payload into event name + data views.
///
/// Splits at the first null byte. If no null is found, returns empty
/// eventName to signal a malformed buffer.
///
/// @param raw byte buffer
/// @param len buffer length
inline WirePayloadView decodeWirePayload(const uint8_t* raw, std::size_t len) noexcept {
    if (len == 0) return {};
    auto* name_end = static_cast<const uint8_t*>(std::memchr(raw, '\0', len));
    if (!name_end) return {};
    auto name_len = static_cast<std::size_t>(name_end - raw);
    auto data_offset = name_len + 1;
    return {
        std::string_view(reinterpret_cast<const char*>(raw), name_len),
        (data_offset < len)
            ? std::string_view(reinterpret_cast<const char*>(raw + data_offset), len - data_offset)
            : std::string_view{}
    };
}

}  // namespace SCE::Mesh
