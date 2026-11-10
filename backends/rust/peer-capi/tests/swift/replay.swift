// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael

// Replays the recorded scenario of the peer core (backends/rust/peer-capi/tests/fixtures) through the
// C ABI, from Swift, with no relay: the cores of two devices are made, given the administrator's acts,
// the machine's sends and the rounds' answers, and one is sealed and made again from its sealed state
// with no key given again; every text a core gives (each request, each outcome, each standing, the
// genesis, the sealed state, the reason of a refusal) must be the recorded one.
//
// The header is reached as the Clang module `CScePeer` (include/module.modulemap), the binding a
// Swift host writes over the C header. The scenario is read as the whitespace-separated tokens the C
// replay reads (the Rust test writes them from the JSON file); tests/c/replay.c describes them.
//
// usage: replay <script>   (built with swiftc -parse-as-library -I include -lsce_peer_capi)

import CScePeer
import Foundation

final class Replayer {
    private var tokens: [Substring] = []
    private var at = 0
    private var steps = 0

    func die(_ what: String) -> Never {
        let reason = String(cString: sce_peer_last_error())
        FileHandle.standardError.write(Data("replay: step \(steps): \(what) (\(reason))\n".utf8))
        exit(1)
    }

    func check(_ status: Int32, _ what: String) {
        if status != Int32(SCE_PEER_OK) {
            die("\(what) failed with \(status)")
        }
    }

    func token() -> Substring? {
        guard at < tokens.count else {
            return nil
        }
        at += 1
        return tokens[at - 1]
    }

    func need() -> Substring {
        guard let next = token() else {
            die("the scenario ends too soon")
        }
        return next
    }

    func number() -> UInt64 {
        guard let value = UInt64(need()) else {
            die("a number is expected")
        }
        return value
    }

    func unhex(_ hex: Substring) -> [UInt8] {
        var bytes: [UInt8] = []
        var index = hex.startIndex
        while index < hex.endIndex {
            let after = hex.index(index, offsetBy: 2, limitedBy: hex.endIndex) ?? hex.endIndex
            guard let byte = UInt8(hex[index..<after], radix: 16) else {
                die("a hex digit is expected")
            }
            bytes.append(byte)
            index = after
        }
        return bytes
    }

    /// A text token: x and the text's UTF-8 bytes in hex.
    func text() -> String {
        let written = need()
        guard written.first == "x" else {
            die("a text is expected")
        }
        return String(decoding: unhex(written.dropFirst()), as: UTF8.self)
    }

    /// The text of a buffer the library gave, given back; nil for an empty buffer.
    func taken(_ buf: ScePeerBuf) -> String? {
        guard let ptr = buf.ptr else {
            return nil
        }
        let bytes = UnsafeBufferPointer(start: UnsafePointer(ptr), count: Int(buf.len))
        let given = String(decoding: bytes, as: UTF8.self)
        sce_peer_buf_free(buf)
        return given
    }

    /// The bytes of a buffer the library gave, given back; nil for an empty buffer.
    func takenBytes(_ buf: ScePeerBuf) -> [UInt8]? {
        guard let ptr = buf.ptr else {
            return nil
        }
        let bytes = Array(UnsafeBufferPointer(start: UnsafePointer(ptr), count: Int(buf.len)))
        sce_peer_buf_free(buf)
        return bytes
    }

    /// A data key token: 32 bytes in hex.
    func dataKey() -> [UInt8] {
        let key = unhex(need())
        guard key.count == 32 else {
            die("a data key is not 32 bytes")
        }
        return key
    }

    func expect(_ got: String?, _ want: String, _ what: String) {
        let have = got ?? ""
        if have != want {
            let report = "replay: step \(steps): \(what) is not the recorded one\n  got:  \(have)\n  want: \(want)\n"
            FileHandle.standardError.write(Data(report.utf8))
            exit(1)
        }
    }

    func kept(_ name: Substring) -> Int32 {
        switch name {
        case "taken": return Int32(SCE_PEER_KEPT_TAKEN)
        case "held": return Int32(SCE_PEER_KEPT_HELD)
        case "not_signed": return Int32(SCE_PEER_KEPT_NOT_SIGNED)
        case "refused": return Int32(SCE_PEER_KEPT_REFUSED)
        case "waiting": return Int32(SCE_PEER_KEPT_WAITING)
        default: die("an answer to a send the replay does not know")
        }
    }

    func profile(_ name: Substring) -> Int32 {
        switch name {
        case "untrusted": return Int32(SCE_PEER_PROFILE_UNTRUSTED)
        case "trusted_hub": return Int32(SCE_PEER_PROFILE_TRUSTED_HUB)
        default: die("a profile the replay does not know")
        }
    }

    /// The bytes as lowercase hex.
    func hex(_ bytes: [UInt8]) -> String {
        let digits = Array("0123456789abcdef")
        var text = ""
        for byte in bytes {
            text.append(digits[Int(byte >> 4)])
            text.append(digits[Int(byte & 0x0f)])
        }
        return text
    }

    func kind(_ name: Substring) -> Int32 {
        switch name {
        case "sync": return Int32(SCE_PEER_ROUND_SYNC)
        case "resync": return Int32(SCE_PEER_ROUND_RESYNC)
        case "reread": return Int32(SCE_PEER_ROUND_REREAD)
        default: die("a kind of round the replay does not know")
        }
    }

    func run(_ path: String) {
        guard UInt32(sce_peer_abi_version()) == UInt32(SCE_PEER_ABI_VERSION) else {
            die("the library is another ABI version")
        }
        guard let data = FileManager.default.contents(atPath: path) else {
            die("the script cannot be read")
        }
        tokens = String(decoding: data, as: UTF8.self).split(whereSeparator: { $0.isWhitespace })

        let list = text()
        let devices = Int(number())
        var nodes: [UInt64] = []
        var seeds: [[UInt8]] = []
        var geneses: [[UInt8]?] = []
        var administrators: [[UInt8]?] = []
        for _ in 0..<devices {
            nodes.append(number())
            let seed = unhex(need())
            guard seed.count == 32 else {
                die("a seed is not 32 bytes")
            }
            seeds.append(seed)
            let genesis = need()
            if genesis == "-" {
                geneses.append(nil)
            } else {
                let hash = unhex(genesis)
                guard hash.count == 32 else {
                    die("a genesis hash is not 32 bytes")
                }
                geneses.append(hash)
            }
            let administrator = need()
            if administrator == "-" {
                administrators.append(nil)
            } else {
                let seed = unhex(administrator)
                guard seed.count == 32 else {
                    die("an administrator's seed is not 32 bytes")
                }
                administrators.append(seed)
            }
        }

        var peers: [OpaquePointer?] = Array(repeating: nil, count: devices)
        /// The core of a device given nothing but the list and the genesis.
        func bare(_ device: Int) -> OpaquePointer {
            if let old = peers[device] {
                sce_peer_free(old)
            }
            let made: OpaquePointer?
            if let genesis = geneses[device] {
                made = sce_peer_invited(list, genesis)
            } else {
                made = sce_peer_open(list)
            }
            guard let core = made else {
                die("the core is not made")
            }
            peers[device] = core
            return core
        }
        while let op = token() {
            let device = Int(number())
            guard device < devices else {
                die("a step names no device")
            }
            if op == "restore" {
                // The device started again: a core given nothing but the list and the genesis.
                let key = dataKey()
                let state = unhex(need())
                let want = number()
                let core = bare(device)
                var count: UInt64 = .max
                check(sce_peer_restore_sealed(core, nodes[device], state, state.count, key, &count), "restore_sealed")
                if count != want {
                    die("the restore kept another number of operations than was recorded")
                }
                steps += 1
                continue
            }
            if op == "make" {
                let core = bare(device)
                check(sce_peer_set_identity(core, nodes[device], seeds[device]), "set_identity")
                if let administrator = administrators[device] {
                    check(sce_peer_set_administrator(core, administrator), "set_administrator")
                }
                steps += 1
                continue
            }
            if op == "make_invited" {
                // The device started from the text of its invitation alone.
                let invitation = Array(text().utf8)
                if let old = peers[device] {
                    sce_peer_free(old)
                }
                guard let core = sce_peer_invited_by(invitation, invitation.count) else {
                    die("the core is not made from the invitation")
                }
                peers[device] = core
                check(sce_peer_set_identity(core, nodes[device], seeds[device]), "set_identity")
                if let administrator = administrators[device] {
                    check(sce_peer_set_administrator(core, administrator), "set_administrator")
                }
                steps += 1
                continue
            }
            guard let peer = peers[device] else {
                die("the device's core is not made")
            }
            var out = ScePeerBuf()
            switch op {
            case "open":
                let chosen = profile(need())
                let want = String(need())
                var genesis = [UInt8](repeating: 0, count: 32)
                check(sce_peer_open_list(peer, chosen, &genesis), "open_list")
                expect(hex(genesis), want, "the genesis")
            case "grant":
                let node = number()
                guard let role = UInt32(exactly: number()) else {
                    die("a role is not a 32-bit number")
                }
                let key = unhex(need())
                guard key.count == 32 else {
                    die("a key is not 32 bytes")
                }
                check(sce_peer_grant(peer, node, role, key), "grant")
            case "remove":
                check(sce_peer_remove_member(peer, number()), "remove_member")
            case "standing":
                let want = text()
                check(sce_peer_standing(peer, &out), "standing")
                expect(taken(out), want, "the standing")
            case "outbound":
                let event = text()
                let data = text()
                let want = kept(need())
                let reason = text()
                var answer: Int32 = -1
                check(sce_peer_outbound(peer, event, data, &answer, &out), "outbound")
                if answer != want {
                    die("the core did not keep the send as recorded")
                }
                expect(taken(out), reason, "the reason of the refusal")
            case "start":
                check(sce_peer_start(peer, kind(need())), "start")
            case "login":
                check(sce_peer_login(peer, text()), "login")
            case "request":
                let want = text()
                check(sce_peer_poll_request(peer, &out), "poll_request")
                guard let request = taken(out) else {
                    die("the core asks nothing where a request was recorded")
                }
                expect(request, want, "the request")
            case "respond":
                check(sce_peer_respond(peer, text()), "respond")
            case "fail":
                check(sce_peer_fail(peer, text()), "fail")
            case "outcome":
                let want = text()
                check(sce_peer_poll_request(peer, &out), "poll_request")
                if taken(out) != nil {
                    die("the core still asks where the exchange was over")
                }
                var ended = ScePeerBuf()
                check(sce_peer_take_outcome(peer, &ended), "take_outcome")
                guard let outcome = taken(ended) else {
                    die("no exchange ended where an outcome was recorded")
                }
                expect(outcome, want, "the outcome")
            case "save":
                let key = dataKey()
                let want = String(need())
                check(sce_peer_save_sealed_under(peer, key, &out), "save_sealed_under")
                guard let state = takenBytes(out) else {
                    die("the sealed state is empty")
                }
                expect(hex(state), want, "the sealed state")
            case "invite":
                let relay = text()
                let audience = text()
                let node = number()
                let nowMs = number()
                let validMs = number()
                let secret = dataKey()
                let want = text()
                check(
                    sce_peer_invite_under(peer, relay, audience, node, nowMs, validMs, secret, &out),
                    "invite_under")
                expect(taken(out), want, "the invitation")
            case "invitation":
                let invitation = Array(text().utf8)
                let want = text()
                check(sce_peer_invitation_read(invitation, invitation.count, &out), "invitation_read")
                expect(taken(out), want, "what the invitation says")
            case "join":
                let invitation = Array(text().utf8)
                let want = text()
                check(sce_peer_join(peer, invitation, invitation.count, &out), "join")
                expect(taken(out), want, "the fingerprint")
            case "joined":
                let node = number()
                let want = text()
                check(sce_peer_joined(peer, node, &out), "joined")
                expect(taken(out), want, "the devices that joined")
            case "made":
                let data = text()
                let want = kept(need())
                let reason = text()
                var answer: Int32 = -1
                check(sce_peer_made(peer, data, &answer, &out), "made")
                if answer != want {
                    die("the core did not give the verdict recorded")
                }
                expect(taken(out), reason, "the reason of the refusal or of the wait")
            case "records":
                let want = text()
                check(sce_peer_take_records(peer, &out), "take_records")
                expect(taken(out), want, "the records")
            case "recorded":
                check(sce_peer_recorded(peer, number()), "recorded")
            case "restore_records":
                let records = text()
                let want = number()
                var count: UInt64 = .max
                check(sce_peer_restore_records(peer, records, &count), "restore_records")
                if count != want {
                    die("the records given back held another number of new ones than was recorded")
                }
            case "history":
                let want = text()
                check(sce_peer_history(peer, nil, &out), "history")
                expect(taken(out), want, "the history")
            case "stable_cut":
                let want = text()
                check(sce_peer_stable_cut(peer, &out), "stable_cut")
                expect(taken(out), want, "the stable cut")
            case "mark":
                let want = number()
                var seq: UInt64 = .max
                check(sce_peer_mark(peer, &seq), "mark")
                if seq != want {
                    die("the core signed another mark than was recorded")
                }
            case "own_stamp":
                guard let wantWall = Int64(need()),
                      let wantCounter = UInt32(need()),
                      let wantReadBack = Int32(need()) else {
                    die("an own stamp is three numbers")
                }
                var wallTime: Int64 = -1
                var counter: UInt32 = .max
                var readBack: Int32 = -1
                check(sce_peer_own_stamp(peer, &wallTime, &counter, &readBack), "own_stamp")
                if wallTime != wantWall || counter != wantCounter || readBack != wantReadBack {
                    die("the core's own stamp is not the recorded one")
                }
            default:
                die("a step the replay does not know: \(op)")
            }
            steps += 1
        }
        for peer in peers {
            if let peer = peer {
                sce_peer_free(peer)
            }
        }
        print("replayed \(steps) steps")
    }
}

@main
struct Replay {
    static func main() {
        let arguments = CommandLine.arguments
        guard arguments.count == 2 else {
            FileHandle.standardError.write(Data("usage: replay <script>\n".utf8))
            exit(2)
        }
        Replayer().run(arguments[1])
    }
}
