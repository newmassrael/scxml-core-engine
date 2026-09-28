// SCE-GENERATED — DO NOT EDIT
// source-hash: 34aaab241046b30864ac859c2172c0519c3149860e3abe044095473301ada5f1


// SCE-MAP: client.scxml:11 :: _machine
// client's Mesh peers, as deploy.yaml binds them (SCE_MESH.md §mesh-19),
// generated from client.
//
// GENERATED — DO NOT EDIT. `sce-codegen generate --deploy <deploy.yaml> --lang kotlin`.
// A host builds its router with `ClientMeshPeers.MACHINE.router()` and opens
// one connection per `PeerLink` below; every rule about envelopes lives in `com.sce.mesh`.

package com.sce.generated.client

import com.sce.mesh.Delivery
import com.sce.mesh.MeshMachine
import com.sce.mesh.Peer
import com.sce.mesh.PeerConfig
import com.sce.mesh.PeerLink

object ClientMeshPeers {
    /** client's side of the deployment. */
    val MACHINE: MeshMachine = MeshMachine(
        name = "client",
        dedupWindow = 8u,
        gapTimeoutMs = 50L,
        peers = listOf(
            Peer(
                name = "server",
                config = PeerConfig(
                    transport = "wss",
                    buffer = com.sce.mesh.OutboundBuffer(
                        maxPending = 8u,
                        maxAgeMs = 0L,
                    ),
                    retry = null,
                    stampSequence = false,
                    delivery = Delivery(dedup = true, ordered = false),
                ),
                link = PeerLink.WssDial(
                    url = "ws://127.0.0.1:8443",
                    keepaliveMs = 30000u,
                ),
            ),
        ),
    )
}
