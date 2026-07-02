// SPDX-License-Identifier: LGPL-2.1-or-later WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
//
// SCE Mesh two-process orchestration smoke — parent half.
//
// Reads the worker's ephemeral endpoint from MESH_PEER_ENDPOINT (set by
// the orchestrator after handshake), constructs a CustomTcp::Client,
// sends one FireForget envelope, and exits. A non-zero exit surfaces
// through the orchestrator as the ctest failure code.

#include "common/Uuid.h"
#include "mesh/MeshEnvelope.h"
#include "mesh/transports/CustomTcpTransport.h"

#include <cstdio>
#include <cstdlib>
#include <string>

int main() {
    const char *ep = std::getenv("MESH_PEER_ENDPOINT");
    if (ep == nullptr || *ep == '\0') {
        std::fprintf(stderr, "parent: MESH_PEER_ENDPOINT not set "
                             "(orchestrator handshake skipped?)\n");
        return 1;
    }

    SCE::Mesh::CustomTcp::Client client(ep, nullptr);
    SCE::Mesh::MeshEnvelope env;
    env.id = SCE::uuid::v7();
    env.source = "smoke_parent";
    env.type = "smoke.ping";
    env.pattern = SCE::Mesh::PatternKind::FireForget;
    env.datacontenttype = SCE::Mesh::PayloadCodec::None;

    if (!client.send(env)) {
        std::fprintf(stderr, "parent: send to %s failed\n", ep);
        return 2;
    }
    std::fprintf(stderr, "parent: sent envelope to %s\n", ep);
    return 0;
}
