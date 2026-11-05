// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
//
// The address of one session of a Mesh machine: `mesh://<device>/<machine>/<session>`
// (SCE_MESH.md, the `_event` wiring of a remote child's events and of its parent's).
//
// A remote `<invoke type="scxml">` runs its child on another machine, perhaps on
// another device, and the two sessions answer each other at these addresses: the
// child's events carry the child's endpoint as their origin, the parent's events
// at the child carry the parent's, and a `<send>` whose target is one of them, of
// the SCXML Event I/O Processor's type, reaches that session (docs/adr/0090).
//
// The transport writes an endpoint once — the child's when it starts the
// session, the parent's when it asks for the child — and every later reader
// compares the text it was handed, so this codec only spells an endpoint and
// recognises the shape; it never takes one apart to route by its pieces.
//
// Kept beside the SCXML processor's own address codec (`IOProcessorHelper`) and
// free of the transport's headers, because the generated engine, which includes
// none of them, has to tell an endpoint from the other values a target can take.

#pragma once

#include "common/UrlEncodingHelper.h"

#include <cstddef>
#include <string>

namespace SCE::Common {

/// The scheme of every Mesh address an event's origin can carry.
inline constexpr const char *MESH_ENDPOINT_SCHEME = "mesh://";

/**
 * @brief The endpoint of session @p sessionId of @p machine, hosted on @p device
 *
 * Each segment is percent-encoded as an SCXML location's session id is, so a
 * session id such as `parent:invoke` stays one segment.
 */
inline std::string meshSessionEndpoint(const std::string &device, const std::string &machine,
                                       const std::string &sessionId) {
    return std::string(MESH_ENDPOINT_SCHEME) + UrlEncodingHelper::urlEncode(device) + "/" +
           UrlEncodingHelper::urlEncode(machine) + "/" + UrlEncodingHelper::urlEncode(sessionId);
}

/**
 * @brief Whether @p uri has the shape of a session endpoint: the Mesh scheme and
 *        three non-empty segments
 *
 * A peer's address (`mesh://<machine>`, one segment) is not one: it names a
 * machine, not a session.
 */
inline bool isMeshSessionEndpoint(const std::string &uri) {
    const std::string scheme(MESH_ENDPOINT_SCHEME);
    if (uri.size() <= scheme.size() || uri.compare(0, scheme.size(), scheme) != 0) {
        return false;
    }
    std::size_t segments = 1;
    std::size_t length = 0;
    for (std::size_t at = scheme.size(); at < uri.size(); ++at) {
        if (uri[at] == '/') {
            if (length == 0) {
                return false;
            }
            ++segments;
            length = 0;
        } else {
            ++length;
        }
    }
    return segments == 3 && length > 0;
}

}  // namespace SCE::Common
