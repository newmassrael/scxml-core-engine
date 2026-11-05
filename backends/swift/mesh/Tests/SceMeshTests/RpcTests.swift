// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
//
// The requester's correlation table and error data, case for case with the Kotlin and Rust cores'
// rpc tests. Each JSON expectation is the literal those tests pin, so the cores cannot drift apart
// without one of them failing.

@testable import SceMesh
import Testing

@Suite struct RpcTests {
    private let wire: [UInt8] = [
        0x01, 0x92, 0x00, 0x00, 0x00, 0x00, 0x70, 0x00,
        0x80, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0xab,
    ]
    private let wireText = "01920000-0000-7000-8000-0000000000ab"

    @Test func aFailedReplyNamesItsStatusByTheDeclaredName() {
        #expect(
            invokeErrorData(status: .unavailable, detail: "busy", source: "cloud", wireIDText: uuidText(wire))
                == #"{"errorName":"invoke","reason":"unavailable","detail":"busy","source":"cloud","invoke_id":"01920000-0000-7000-8000-0000000000ab"}"#
        )
    }

    @Test func aSynthesisedDeadlineHasNoSource() {
        #expect(
            invokeErrorData(status: .deadlineExceeded, detail: nil, source: nil, wireIDText: uuidText(wire))
                == #"{"errorName":"invoke","reason":"deadlineExceeded","invoke_id":"01920000-0000-7000-8000-0000000000ab"}"#
        )
    }

    @Test func aRequestWithNoRouteIsSrcNotFound() {
        #expect(
            srcNotFoundData("no binding for '#cloud'")
                == #"{"errorName":"execution","reason":"INVOKE_SRC_NOT_FOUND","detail":"no binding for '#cloud'"}"#)
    }

    @Test func anUndeclaredResponderNamesItsSourceAndTheRequest() {
        #expect(
            Signal.rpcReplyFromUndeclaredPeer(source: "mallory", invokeID: uuidText(wire))
                .eventData(binding: Binding(peer: "hmi", transport: "wss"))
                == #"{"errorName":"communication","reason":"RPC_REPLY_FROM_UNDECLARED_PEER","source":"mallory","invoke_id":"01920000-0000-7000-8000-0000000000ab"}"#
        )
    }

    private func pending(_ invokeID: String, _ token: UInt64, expiresMs: Int64?, target: String = "cloud") -> Pending {
        Pending(invokeID: invokeID, token: token, target: target, responders: ["cloud"], expiresMs: expiresMs)
    }

    private func other() -> [UInt8] {
        var bytes = wire
        bytes[15] = 0xac
        return bytes
    }

    /// §mesh-16.7 row 5: a lost link forgets what was sent on it, and only that.
    @Test func aLostLinkForgetsOnlyTheRequestsSentOnIt() {
        let table = Correlation()
        let other = other()
        table.register(wireID: wire, pending("ask", 1, expiresMs: 100))
        table.register(wireID: other, pending("look", 2, expiresMs: nil, target: "hmi"))
        #expect(table.forgetSentTo("cloud") == [uuidText(wire)])
        #expect(!table.isWaiting(wireID: wire))
        #expect(table.isWaiting(wireID: other))
        #expect(table.expire(nowMs: 1000).isEmpty, "a forgotten request has no deadline")
    }

    @Test func aRequestWhoseLinkIsLostNamesItAndItsTarget() {
        #expect(
            Signal.invokeChildLost(invokeID: uuidText(wire), target: "hmi")
                .eventData(binding: Binding(peer: "hmi", transport: "wss"))
                == #"{"errorName":"communication","reason":"INVOKE_CHILD_LOST","invoke_id":"01920000-0000-7000-8000-0000000000ab","target":"hmi"}"#
        )
    }

    /// §mesh-14.6: only a declared responder retires a request, and a rejected reply leaves it answerable.
    @Test func onlyADeclaredResponderIsAdmitted() {
        let table = Correlation()
        table.register(wireID: wire, pending("ask", 1, expiresMs: nil))
        #expect(table.check(wireID: wire, peer: "mallory") == .undeclared)
        #expect(table.check(wireID: wire, peer: "cloud") == .admitted)
        #expect(table.retire(wireID: wire) != nil)
        #expect(table.check(wireID: wire, peer: "cloud") == .unknown)
    }

    /// A cancel forgets only the start it names: a state entered again starts the same invoke id under a
    /// new token.
    @Test func aCancelForgetsOnlyItsOwnStart() {
        let table = Correlation()
        let second = other()
        table.register(wireID: wire, pending("ask", 1, expiresMs: nil))
        table.register(wireID: second, pending("ask", 2, expiresMs: nil))
        table.cancel(invokeID: "ask", token: 1)
        #expect(table.check(wireID: wire, peer: "cloud") == .unknown)
        #expect(table.check(wireID: second, peer: "cloud") == .admitted)
    }

    @Test func aDeadlineExpiresAtItsInstantAndNotBefore() {
        let table = Correlation()
        table.register(wireID: wire, pending("ask", 1, expiresMs: 100))
        #expect(table.expire(nowMs: 99).isEmpty)
        let expired = table.expire(nowMs: 100)
        #expect(expired.count == 1)
        #expect(expired.first?.pending.invokeID == "ask")
        #expect(expired.first?.wireID == wireText)
        #expect(table.expire(nowMs: 1000).isEmpty)
    }

    @Test func expiredRequestsAreReturnedInWireIDOrder() {
        let table = Correlation()
        table.register(wireID: other(), pending("look", 2, expiresMs: 5))
        table.register(wireID: wire, pending("ask", 1, expiresMs: 5))
        #expect(table.expire(nowMs: 5).map(\.pending.invokeID) == ["ask", "look"])
    }

    @Test func uuidTextReadsBackWhatItWroteInEitherCaseAndNothingElse() {
        #expect(parseUUIDText(uuidText(wire)) == wire)
        #expect(parseUUIDText(uuidText(wire).uppercased()) == wire)
        #expect(parseUUIDText("") == nil)
        #expect(parseUUIDText("ask") == nil)
        // The 32 digits without the dashes are not the canonical text.
        #expect(parseUUIDText(String(repeating: "09", count: 16)) == nil)
        #expect(parseUUIDText("zzzzzzzz-zzzz-zzzz-zzzz-zzzzzzzzzzzz") == nil)
    }
}
