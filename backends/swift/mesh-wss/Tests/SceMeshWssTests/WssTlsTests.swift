// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
//
// The `wss://` half of the binding's client (SCE_MESH.md §mesh-18.1): what a link does when the
// socket is TLS. A certificate is made when the test runs, as the Rust core's tests make theirs, so
// no private key is committed; the server presents it and the client either trusts it or does not.

import Crypto
import Foundation
import NIOCore
import NIOSSL
import SceMesh
import SceMeshWss
import SwiftASN1
import Testing
import X509

/// A self-signed certificate for `host`, valid for an hour either side of now, and the key it was
/// signed with.
private func selfSigned(for host: String) throws -> (certificate: NIOSSLCertificate, key: NIOSSLPrivateKey) {
    let key = P256.Signing.PrivateKey()
    let name = try DistinguishedName { CommonName(host) }
    let now = Date()
    let certificate = try Certificate(
        version: .v3,
        serialNumber: Certificate.SerialNumber(),
        publicKey: Certificate.PublicKey(key.publicKey),
        notValidBefore: now.addingTimeInterval(-3600),
        notValidAfter: now.addingTimeInterval(3600),
        issuer: name,
        subject: name,
        signatureAlgorithm: .ecdsaWithSHA256,
        extensions: try Certificate.Extensions {
            SubjectAlternativeNames([.dnsName(host)])
        },
        issuerPrivateKey: Certificate.PrivateKey(key)
    )
    let certificatePEM = try certificate.serializeAsPEM().pemString
    return (
        try NIOSSLCertificate(bytes: Array(certificatePEM.utf8), format: .pem),
        try NIOSSLPrivateKey(bytes: Array(key.pemRepresentation.utf8), format: .pem)
    )
}

private func serving(_ pair: (certificate: NIOSSLCertificate, key: NIOSSLPrivateKey)) throws -> NIOSSLContext {
    try NIOSSLContext(
        configuration: .makeServerConfiguration(
            certificateChain: [.certificate(pair.certificate)], privateKey: .privateKey(pair.key)))
}

@Suite(.serialized) struct WssTlsTests {
    private func client(
        port: Int, events: Inbox<LinkEvent>, trusting certificate: NIOSSLCertificate?
    ) throws -> WssClient {
        var configuration = WssConfiguration()
        if let certificate {
            configuration.tls.trustRoots = .certificates([certificate])
        }
        configuration.connectTimeout = .seconds(5)
        return try WssClient(
            base: "wss://localhost:\(port)", own: "client", server: "server", configuration: configuration
        ) { events.push($0) }
    }

    @Test func aLinkCarriesEnvelopesOverTLSTheClientTrusts() throws {
        let pair = try selfSigned(for: "localhost")
        let server = try TestServer(tls: serving(pair))
        defer { server.shutdown() }
        let events = Inbox<LinkEvent>()
        let link = try client(port: server.port, events: events, trusting: pair.certificate)
        link.connect()
        #expect(events.next() == .ready(peer: "server"))
        #expect(server.paths.next() == "/sce-mesh/1/client")

        #expect(link.transmit(peer: "server", bytes: [1, 2, 3]) == nil)
        #expect(server.received.next() == [1, 2, 3])
        try #require(server.sockets.next()).send(binary: [4, 5])
        #expect(events.next() == .received(peer: "server", bytes: [4, 5]))
        link.close()
    }

    /// A server whose certificate the client was not told to trust is not a server the link opens to.
    @Test func aServerWhoseCertificateIsNotTrustedIsALostLink() throws {
        let pair = try selfSigned(for: "localhost")
        let server = try TestServer(tls: serving(pair))
        defer { server.shutdown() }
        let events = Inbox<LinkEvent>()
        try client(port: server.port, events: events, trusting: nil).connect()
        let event = try #require(events.next(within: 10))
        guard case .lost = event else {
            throw Unexpected(description: "expected a lost link, got \(event)")
        }
        #expect(server.paths.next(within: 0.3) == nil, "the upgrade was sent over a link that was not trusted")
    }

    /// A trusted certificate for another host is not the server the URL names.
    @Test func aCertificateForAnotherHostIsALostLink() throws {
        let pair = try selfSigned(for: "other.example")
        let server = try TestServer(tls: serving(pair))
        defer { server.shutdown() }
        let events = Inbox<LinkEvent>()
        try client(port: server.port, events: events, trusting: pair.certificate).connect()
        let event = try #require(events.next(within: 10))
        guard case .lost = event else {
            throw Unexpected(description: "expected a lost link, got \(event)")
        }
    }
}
