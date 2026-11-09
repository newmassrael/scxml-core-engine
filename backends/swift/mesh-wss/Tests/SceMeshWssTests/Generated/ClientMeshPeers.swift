// SCE-GENERATED — DO NOT EDIT
// source-hash: da24085fe06f94f61466f2d529c0aacec24fac38de0680c7b04dbb0aacaaa1cd


// SCE-MAP: client.scxml:13 :: _machine
// client's Mesh peers, as deploy.yaml binds them (SCE_MESH.md §mesh-19),
// generated from client.
//
// GENERATED — DO NOT EDIT. `sce-codegen generate --deploy <deploy.yaml> --language swift`.
// A host builds its router with `ClientMeshPeers.machine.router()` and opens
// one connection per `PeerLink` below; every rule about envelopes lives in `SceMesh`.

import SceMesh

public enum ClientMeshPeers {
    /// client's side of the deployment.
    public static let machine = MeshMachine(
        name: "client",
        dedupWindow: 8,
        gapTimeoutMs: 50,
        peers: [
            Peer(
                name: "server",
                config: PeerConfig(
                    transport: "wss",
                    buffer: OutboundBuffer(maxPending: 8, maxAgeMs: 0),
                    retry: nil,
                    stampSequence: false,
                    delivery: Delivery(dedup: true, ordered: false),
                    responders: ["server"],
                    deadlineMs: nil,
                    replyEvents: [],
                    causal: nil
                ),
                link: .wssDial(
                    url: "ws://127.0.0.1:8443",
                    keepaliveMs: 30000
                )
            ),
        ]
    )
}
