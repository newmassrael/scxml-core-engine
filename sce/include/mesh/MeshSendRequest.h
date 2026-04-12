// SPDX-License-Identifier: LGPL-2.1-or-later OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2025 newmassrael
//
// SCE Mesh MeshSendRequest — data payload for the cross-machine send callback.
//
// Lightweight header intentionally free of engine/runtime dependencies so
// that generated TransportRouter code can wire itself to StaticExecutionEngine
// without dragging the full static engine header into the router unit.

#pragma once

#include <string>

namespace SCE::Mesh {

/// Payload passed to StaticExecutionEngine's onMeshSend callback for each
/// external <send target="#<machine>"/> whose target resolves to a mesh
/// transport (local, shm, someip, …).
///
/// Mirrors HttpSendRequest for BasicHTTP. The generated TransportRouter
/// consumes this struct inside its wireTo() lambda and forwards the
/// send through the configured transport template.
struct MeshSendRequest {
    std::string target;     // as written in SCXML, e.g. "#motor"
    std::string eventName;  // event enum name, e.g. "brake.activate"
    std::string data;       // event data payload (JSON per W3C SCXML 5.10)
    std::string sendId;     // W3C SCXML 5.10.1: _event.sendid
};

}  // namespace SCE::Mesh
