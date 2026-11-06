// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael

import Foundation
import Testing

@testable import SceRuntime

// The saved-state document (`sce-saved-state`, formats 1, 2 and 3): what one backend wrote, every backend reads,
// and what a reader writes back is the text it was given, or, for a format-2 text, the format-3 text it is read
// as (docs/adr/0090, section 7). The shared fixtures under `sce-build/tests/fixtures/static_datamodel/saved` are
// the text every runtime writes and reads, so reading each and writing it again holds this runtime's reader and
// writer to theirs, byte for byte.

/// `sce-build/tests/fixtures/static_datamodel/saved`, found from this file's own path.
private func savedDirectory() -> URL {
    var root = URL(fileURLWithPath: #filePath)
    // The file, then SceRuntimeTests, Tests, runtime, swift and backends.
    for _ in 0..<6 {
        root.deleteLastPathComponent()
    }
    return root.appendingPathComponent("sce-build/tests/fixtures/static_datamodel/saved")
}

private func savedFixtures() -> [(name: String, text: String)] {
    let directory = savedDirectory()
    var found: [(name: String, text: String)] = []
    for sub in ["", "format1"] {
        let folder = sub.isEmpty ? directory : directory.appendingPathComponent(sub)
        let names = (try? FileManager.default.contentsOfDirectory(atPath: folder.path)) ?? []
        for name in names.sorted() where name.hasSuffix(".json") {
            let path = folder.appendingPathComponent(name)
            if let text = try? String(contentsOf: path, encoding: .utf8) {
                found.append((name: sub.isEmpty ? name : "\(sub)/\(name)", text: text))
            }
        }
    }
    return found
}

private func refusal(_ text: String) -> String? {
    do {
        _ = try SavedState.fromJSON(text)
        return nil
    } catch let refusal as StateRefusal {
        return refusal.message
    } catch {
        return "\(error)"
    }
}

/// A format-1 state with the members every one holds, and `extra` appended inside the object.
private func format1(_ extra: String = "") -> String {
    """
    {"format":1,"shape":"\(String(repeating: "a", count: 64))","configuration":["a"],"current":"a",\
    "variables":{},"history":{},"pending":[],"invokes":[],"hostinvokes":[],"hostinvoketoken":"0",\
    "sendseq":"0","external":[]\(extra)}
    """
}

@Test func everySharedSavedStateIsReadAndWrittenBackAsTheTextItWas() {
    let fixtures = savedFixtures()
    #expect(fixtures.count >= 20, "\(fixtures.count) saved states read: a test that read none would pass")
    for (name, text) in fixtures {
        do {
            let state = try SavedState.fromJSON(text)
            #expect(state.toJSON() + "\n" == text, "\(name) is written back as it was read")
            #expect(state.formatVersion == (name.hasPrefix("format1/") ? 1 : 3), "\(name) is of the format it is in")
        } catch {
            Issue.record("\(name) is not read: \(error)")
        }
    }
}

@Test func everyFormat2TextIsWrittenBackAsTheFormat3TextItIsReadAs() throws {
    // docs/adr/0090, section 7: each origin of a format-2 text is read by the table of that section, and the
    // state is written back as format 3. `format2/read_as/` holds, under the same name, the text each is read as.
    let directory = savedDirectory().appendingPathComponent("format2")
    let names = try FileManager.default.contentsOfDirectory(atPath: directory.path)
        .filter { $0.hasSuffix(".json") }.sorted()
    #expect(names.count == 4, "\(names.count) format-2 texts read: a test that read none would pass")
    for name in names {
        let text = try String(contentsOf: directory.appendingPathComponent(name), encoding: .utf8)
        let readAs = try String(
            contentsOf: directory.appendingPathComponent("read_as").appendingPathComponent(name), encoding: .utf8)
        let state = try SavedState.fromJSON(text)
        #expect(state.formatVersion == SavedState.format3, "\(name) is written as format 3")
        #expect(state.toJSON() + "\n" == readAs, "\(name) is written as the text it is read as")
    }
}

/// A session of a tree that runs no child: a child of a tree is one, with an id of its own.
private func leaf(_ id: String) -> SavedState {
    SavedState(
        shape: String(repeating: "a", count: 64), configuration: ["a"], current: "a", variables: [],
        invokes: .resumed([]), sessionID: id)
}

@Test func aStateIsOfTheFormatItsRunningChildrenAreSavedIn() throws {
    let one = try SavedState.fromJSON(format1())
    #expect(one.formatVersion == 1)
    #expect(one.boundSessionID == nil, "a format-1 state holds no session id to bind")
    let child = SavedChild(id: "kid", start: 1, invokeID: "s.1.kid", document: "kid", session: leaf("root.kid"))
    let two = SavedState(
        shape: one.shape, configuration: ["a"], current: "a", variables: [], invokes: .resumed([child]),
        sessionID: "root", invokeStarts: [SavedEntry("kid", 1)])
    #expect(two.formatVersion == 3, "a tree is written as format 3")
    #expect(two.boundSessionID == "root")
    #expect(two.invokeIDs == ["kid"])
    #expect(two.invokeStart("kid") == 1)
    #expect(two.invokeStart("other") == 0, "an invoke that never started has started none")
    #expect(two.toJSON().hasPrefix(#"{"format":3,"#))
    let again = try SavedState.fromJSON(two.toJSON())
    #expect(again.formatVersion == 3)
    #expect(again == two)
}

@Test func aVariableIsAskedForByItsId() throws {
    let state = SavedState(
        shape: "s", configuration: [], current: "", variables: [SavedEntry("count", .number("2"))])
    #expect(try state.variable("count") == .number("2"))
    #expect(throws: StateRefusal("the saved state has no variable 'missing'")) { try state.variable("missing") }
}

@Test func aRefusedTextSaysWhyAndNamesWhatIsWrong() {
    #expect(refusal("not json")?.hasPrefix("the saved state is not JSON") == true)
    #expect(refusal("[]") == "the saved state is not an object")
    #expect(refusal("{}") == "the saved state has no 'format'")
    // A format of the future is refused by its `format`, before anything else of the text is read.
    #expect(
        refusal(#"{"format":4}"#)
            == "the saved state is format 4, and this runtime reads formats 1, 2 and 3")
    #expect(
        refusal(#"{"format":"3"}"#)
            == #"the saved state is format "3", and this runtime reads formats 1, 2 and 3"#)
    #expect(refusal(format1().replacingOccurrences(of: #""current":"a","#, with: "")) == "the saved state has no 'current'")
    #expect(refusal(format1().replacingOccurrences(of: #""configuration":["a"]"#, with: #""configuration":"a""#))
        == "'configuration' is not an array")
}

@Test func aSendToTheParentOrAChildIsAnActOfFormat2Only() {
    let send = #"{"due":"5","act":"parent","event":"e","data":"","sendid":"s","origin":"o"}"#
    let text = format1().replacingOccurrences(of: #""pending":[]"#, with: #""pending":[\#(send)]"#)
    #expect(
        refusal(text)
            == "'pending[0].act' is 'parent', which is not raise, internal or host, the acts of format 1")
    let unknown = text.replacingOccurrences(of: #""act":"parent""#, with: #""act":"later""#)
    #expect(
        refusal(unknown)
            == "'pending[0].act' is 'later', which is not raise, internal or host, the acts of format 1")
}

@Test func aCountATokenAndAMomentAreWholeNumbersWrittenAsTexts() {
    #expect(
        refusal(format1().replacingOccurrences(of: #""sendseq":"0""#, with: #""sendseq":"-1""#))
            == "'sendseq' (-1) is not a whole number a token can be")
    #expect(
        refusal(format1().replacingOccurrences(of: #""sendseq":"0""#, with: #""sendseq":0"#))
            == "'sendseq' is not a text")
    let late = #"{"due":"x5","act":"raise","event":"e","data":"","sendid":"s","origin":"o"}"#
    #expect(
        refusal(format1().replacingOccurrences(of: #""pending":[]"#, with: #""pending":[\#(late)]"#))
            == "'pending[0].due' (x5) is not a whole number of milliseconds")
}

@Test func aTreeIsRefusedAsAWholeAndTheReasonNamesTheSessionByItsPath() throws {
    let child = leaf("root.kid")
    let grandchild = SavedChild(id: "kid", start: 1, invokeID: "s.1.kid", document: "kid", session: child)
    let root = SavedState(
        shape: child.shape, configuration: ["a"], current: "a", variables: [], invokes: .resumed([grandchild]),
        sessionID: "root", invokeStarts: [SavedEntry("kid", 1)])
    // The child's session states a format of its own, which only the root of a tree does.
    let text = root.toJSON().replacingOccurrences(of: #""session":{"#, with: #""session":{"format":3,"#)
    #expect(refusal(text) == "'invokes[0].session' states a format, which only the root of a tree does")
    // A refusal inside the child's own session is named by the path to it.
    let broken = root.toJSON().replacingOccurrences(of: #""hostinvoketoken":"0""#, with: #""hostinvoketoken":"x""#)
    let said = refusal(broken)
    #expect(said?.hasPrefix("the session at invokes[0].session: ") == true, "\(said ?? "none")")
}

@Test func aSessionIdThatIsNoNmtokenAndACountThatIsNoStartAreRefused() throws {
    let child = try SavedState.fromJSON(format1())
    let root = SavedState(
        shape: child.shape, configuration: ["a"], current: "a", variables: [], invokes: .resumed([]),
        sessionID: "root")
    #expect(
        refusal(root.toJSON().replacingOccurrences(of: #""sessionid":"root""#, with: #""sessionid":"a b""#))
            == "'sessionid' (a b) is not an XML Nmtoken, which a session id is")
    let counted = SavedState(
        shape: child.shape, configuration: ["a"], current: "a", variables: [], invokes: .resumed([]),
        sessionID: "root", invokeStarts: [SavedEntry("kid", 1)])
    #expect(
        refusal(counted.toJSON().replacingOccurrences(of: #""kid":"1""#, with: #""kid":"01""#))
            == "'invokestarts.kid' (01) is not a count of starts, a whole number from 1")
}

@Test func revisionsAndFeaturesAreOptionalAndHeldToTheirRules() throws {
    let digest = String(repeating: "b", count: 64)
    let revised = format1(#","revisions":{"event_a":{"revision":2,"fields":"\#(digest)"}}"#)
    let state = try SavedState.fromJSON(revised)
    #expect(state.revisions == [SavedEntry("event_a", SavedRevision(revision: 2, fields: digest))])
    #expect(state.toJSON() == revised)
    #expect(try SavedState.fromJSON(format1()).revisions.isEmpty)
    #expect(
        refusal(format1(#","revisions":{}"#))
            == "'revisions' names no schema; a state whose schemas are all in their first revision leaves it out")
    #expect(
        refusal(format1(#","revisions":{"e":{"revision":1,"fields":"\#(digest)"}}"#))?
            .contains("is not a revision past the first") == true)
    #expect(
        refusal(format1(#","revisions":{"e":{"revision":2,"fields":"ABC"}}"#))
            == "'revisions.e.fields' is not a SHA-256 written as lowercase hex")

    #expect(try SavedState.fromJSON(format1()).features == nil, "a machine of no product line records none")
    #expect(try SavedState.fromJSON(format1(#","features":[]"#)).features == [])
    let featured = format1(#","features":["a","b"]"#)
    #expect(try SavedState.fromJSON(featured).toJSON() == featured)
    #expect(
        refusal(format1(#","features":["b","a"]"#))
            == "'features' is not in order or names a feature twice; it is a set, sorted")
    #expect(
        refusal(format1(#","features":["a","a"]"#))
            == "'features' is not in order or names a feature twice; it is a set, sorted")
}

@Test func aHostInvokeAndAHostSendKeepTheirParamsInTheOrderOfTheNamesBytes() throws {
    let state = SavedState(
        shape: "s", configuration: ["a"], current: "a", variables: [],
        pending: [
            SavedSend(
                due: 7,
                act: .host(
                    processorType: "t", event: "e", target: "", content: "", params: ["z": ["1"], "a": ["2", "3"]],
                    sendID: "s", data: "", invokeID: ""))
        ],
        hostInvokes: [SavedHostInvoke(processorType: "t", invokeID: "i", params: ["y": ["1"], "b": ["2"]], due: 9)])
    let text = state.toJSON()
    #expect(text.contains(#""params":{"a":["2","3"],"z":["1"]}"#), "\(text)")
    #expect(text.contains(#""params":{"b":["2"],"y":["1"]}"#), "\(text)")
    let read = try SavedState.fromJSON(text)
    #expect(read == state)
}

// MARK: - An event's origin in format 3 (docs/adr/0090, section 7)

/// A format-3 state of the session `root`, with `pending` and `external` inside its two arrays.
private func format3(pending: String = "", external: String = "") -> String {
    """
    {"format":3,"shape":"\(String(repeating: "a", count: 64))","sessionid":"root","configuration":["a"],\
    "current":"a","variables":{},"history":{},"pending":[\(pending)],"invokes":[],"invokestarts":{},\
    "hostinvokes":[],"hostinvoketoken":"0","sendseq":"0","external":[\(external)]}
    """
}

/// The text of a pair, `processor` then `address`.
private func pairText(_ processor: String, _ address: String) -> String {
    #"{"processor":"\#(processor)","address":"\#(address)"}"#
}

/// The text of the pair the SCXML Event I/O Processor writes for the session `id`.
private func scxmlPairText(_ id: String) -> String {
    pairText(scxmlEventProcessorType, scxmlProcessorLocation(sessionID: id))
}

/// A queued event whose `origin` and `sender` members are the texts given.
private func queuedText(origin: String, sender: String) -> String {
    #"{"name":"e","data":"","type":"external","sendid":"","origin":\#(origin),"sender":\#(sender),"invokeid":"","hostinvoketoken":null}"#
}

@Test func aFormat3OriginIsReadAsWrittenAndWrittenBackAsItWasRead() throws {
    let sent = format3(external: queuedText(origin: scxmlPairText("peer:1"), sender: #""peer:1""#))
    let read = try SavedState.fromJSON(sent)
    #expect(read.external.first?.origin == scxmlOrigin(sender: "peer:1"))
    #expect(read.external.first?.origin?.address == "sce://scxml/peer%3A1", "the id is percent-encoded")
    #expect(read.external.first?.sender == "peer:1")
    // A host's own pair, an SCXML pair at an address that names no session, and none: no sender beside any.
    let host = format3(external: queuedText(origin: pairText("x-sce-host", "motor"), sender: "null"))
    #expect(try SavedState.fromJSON(host).external.first?.origin == EventOrigin(processor: "x-sce-host", address: "motor"))
    let mesh = format3(external: queuedText(origin: pairText(scxmlEventProcessorType, "mesh://ecu"), sender: "null"))
    #expect(try SavedState.fromJSON(mesh).external.first?.sender == nil)
    let none = format3(external: queuedText(origin: "null", sender: "null"))
    #expect(try SavedState.fromJSON(none).external.first?.origin == nil)
    // The sender is the session that sent the event and the origin its reply address, two fields no rule ties
    // together: a remote child's event the Mesh transport delivers names the child beside its endpoint.
    let remote = format3(
        external: queuedText(
            origin: pairText(scxmlEventProcessorType, "mesh://dev/machine/kid"), sender: #""root.work.1.kid""#))
    let fromRemote = try SavedState.fromJSON(remote).external.first
    #expect(fromRemote?.origin == EventOrigin(processor: scxmlEventProcessorType, address: "mesh://dev/machine/kid"))
    #expect(fromRemote?.sender == "root.work.1.kid")
    let otherAddress = format3(external: queuedText(origin: scxmlPairText("other"), sender: #""kid""#))
    #expect(try SavedState.fromJSON(otherAddress).external.first?.sender == "kid")
    let hostAddress = format3(external: queuedText(origin: pairText("x-sce-host", "motor"), sender: #""kid""#))
    #expect(try SavedState.fromJSON(hostAddress).external.first?.origin?.processor == "x-sce-host")
    // A session's own send carries its pair and its id; an internal act carries neither.
    let raise = #"{"due":"5","act":"raise","event":"e","data":"","sendid":"s","origin":\#(scxmlPairText("root")),"sender":"root"}"#
    let inner = #"{"due":"6","act":"internal","event":"i","data":"","sendid":"t","origin":null,"sender":null}"#
    let acts = format3(pending: "\(raise),\(inner)")
    #expect(
        try SavedState.fromJSON(acts).pending.map(\.act)
            == [.raise(event: "e", data: "", sendID: "s", sender: "root"), .internal(event: "i", data: "", sendID: "t")])
    for text in [sent, host, mesh, none, remote, otherAddress, hostAddress, acts] {
        #expect(try SavedState.fromJSON(text).toJSON() == text, "\(text) is written back as it was read")
    }
}

@Test func aFormat3OriginThatIsNotAWholePairOrASenderWithNoOriginIsRefused() {
    func event(_ origin: String, _ sender: String) -> String? {
        refusal(format3(external: queuedText(origin: origin, sender: sender)))
    }
    #expect(event(#""kid""#, "null") == "'external[0].origin' is neither null nor a pair of a processor and an address")
    #expect(event(#"{"processor":"p"}"#, "null") == "'external[0].origin' has no 'address'")
    #expect(event(#"{"address":"a"}"#, "null") == "'external[0].origin' has no 'processor'")
    #expect(
        event(#"{"processor":"","address":"a"}"#, "null")
            == "'external[0].origin.processor' is empty, and a pair names both its members")
    #expect(
        event(#"{"processor":"p","address":""}"#, "null")
            == "'external[0].origin.address' is empty, and a pair names both its members")
    #expect(
        event(#"{"processor":"p","address":"a","port":"1"}"#, "null")
            == "'external[0].origin' holds 'port', which a pair does not")
    #expect(event(#"{"processor":"p","address":7}"#, "null") == "'external[0].origin.address' is not a text")
    #expect(event(scxmlPairText("kid"), #""a b""#) == "'external[0].sender' (a b) is not a session id, an XML Nmtoken")
    #expect(event(scxmlPairText("kid"), "7") == "'external[0].sender' is neither null nor a session id")
    // A session that sent an event has an address a reply reaches, so a sender beside no origin is refused.
    #expect(
        event("null", #""kid""#)
            == "'external[0].sender' is 'kid' beside a null 'origin', and the session that sent an event has an address a reply reaches"
    )
    // Both members are always written.
    let unsent = #"{"name":"e","data":"","type":"external","sendid":"","origin":null,"invokeid":"","hostinvoketoken":null}"#
    #expect(refusal(format3(external: unsent)) == "'external[0]' has no 'sender'")

    func act(_ members: String) -> String? {
        refusal(format3(pending: #"{"due":"5",\#(members)}"#))
    }
    let mine = scxmlPairText("root")
    #expect(
        act(#""act":"raise","event":"e","data":"","sendid":"s","origin":null,"sender":null"#)
            == "'pending[0]' is a raise act with no sender, which a session's own send names")
    #expect(
        act(#""act":"parent","event":"e","data":"","sendid":"s","origin":\#(mine),"sender":null"#)
            == "'pending[0]' is a parent act with no sender, which a session's own send names")
    #expect(
        act(#""act":"child","invokeid":"k","event":"e","data":"","sendid":"s","origin":null,"sender":"root""#)
            == "'pending[0].sender' is 'root' beside a null 'origin', and the session that sent an event has an address a reply reaches"
    )
    // A session's own send keeps its own-pair rule, which a queued event does not have.
    let notRoots = "'pending[0].origin' is not the SCXML pair of its sender 'root', which a session's own send carries"
    #expect(
        act(#""act":"raise","event":"e","data":"","sendid":"s","origin":\#(scxmlPairText("other")),"sender":"root""#)
            == notRoots)
    #expect(
        act(#""act":"parent","event":"e","data":"","sendid":"s","origin":\#(pairText("x-sce-host", "sce://scxml/root")),"sender":"root""#)
            == notRoots)
    #expect(
        act(#""act":"child","invokeid":"k","event":"e","data":"","sendid":"s","origin":\#(pairText(scxmlEventProcessorType, "mesh://dev/m/root")),"sender":"root""#)
            == notRoots)
    #expect(
        act(#""act":"internal","event":"e","data":"","sendid":"s","origin":\#(mine),"sender":null"#)
            == "'pending[0].origin' is not null, and an internal act's event has no origin and no sender")
    #expect(
        act(#""act":"internal","event":"e","data":"","sendid":"s","origin":null,"sender":"root""#)
            == "'pending[0].sender' is not null, and an internal act's event has no origin and no sender")
}

@Test func aFormat1TextIsWrittenBackAsFormat1WithTheOriginTextFormat2Wrote() throws {
    // docs/adr/0090, section 7: format 1 has no place for the pair, so a state read from it and written back
    // without a machine writes `origin` as the sender's id for a session's send and the address otherwise, and
    // `origintype` as the pair's processor.
    let host = #"{"name":"e","data":"","type":"external","sendid":"","origin":"motor","origintype":"x-sce-host","invokeid":"","hostinvoketoken":null}"#
    let sent = #"{"name":"f","data":"","type":"external","sendid":"","origin":"kid","origintype":"\#(scxmlEventProcessorType)","invokeid":"","hostinvoketoken":null}"#
    let raise = #"{"due":"5","act":"raise","event":"e","data":"","sendid":"s","origin":"kid"}"#
    let text = format1()
        .replacingOccurrences(of: #""external":[]"#, with: #""external":[\#(host),\#(sent)]"#)
        .replacingOccurrences(of: #""pending":[]"#, with: #""pending":[\#(raise)]"#)
    let state = try SavedState.fromJSON(text)
    #expect(
        state.external.map(\.origin) == [EventOrigin(processor: "x-sce-host", address: "motor"), scxmlOrigin(sender: "kid")])
    #expect(state.external.map(\.sender) == [nil, "kid"])
    #expect(state.pending.map(\.act) == [.raise(event: "e", data: "", sendID: "s", sender: "kid")])
    #expect(state.formatVersion == SavedState.format1)
    #expect(state.toJSON() == text, "a format-1 state is written back as format 1")
    // An empty address is none whatever type was stored beside it, and none is written as two empty texts. An
    // internal act's stored origin is not its event's, which has none.
    let blank = #"{"name":"g","data":"","type":"external","sendid":"","origin":"","origintype":"\#(scxmlEventProcessorType)","invokeid":"","hostinvoketoken":null}"#
    let inner = #"{"due":"5","act":"internal","event":"i","data":"","sendid":"t","origin":"kid"}"#
    let stored = format1()
        .replacingOccurrences(of: #""external":[]"#, with: #""external":[\#(blank)]"#)
        .replacingOccurrences(of: #""pending":[]"#, with: #""pending":[\#(inner)]"#)
    let written = try SavedState.fromJSON(stored)
    #expect(written.external.first?.origin == nil && written.external.first?.sender == nil)
    #expect(written.pending.map(\.act) == [.internal(event: "i", data: "", sendID: "t")])
    #expect(
        written.toJSON()
            == stored.replacingOccurrences(of: #""origintype":"\#(scxmlEventProcessorType)""#, with: #""origintype":"""#)
            .replacingOccurrences(of: #""origin":"kid""#, with: #""origin":"""#))
}

@Test func aFormat3SessionsActSentByAnotherSessionIsRefusedInTheRootAndInAChild() {
    // docs/adr/0090, section 7, "Text form": a `raise`, `parent` or `child` act carries this session's pair and
    // this session's id, so one that names another session is no text this session saved.
    let other = #"{"due":"5","act":"raise","event":"e","data":"","sendid":"s","origin":\#(scxmlPairText("other")),"sender":"other"}"#
    #expect(
        refusal(format3(pending: other))
            == "'pending[0].sender' is 'other', and this session is 'root': a raise act is this session's own send")
    let shape = String(repeating: "a", count: 64)
    let child = SavedState(
        shape: shape, configuration: ["a"], current: "a", variables: [],
        pending: [SavedSend(due: 9, act: .parent(event: "up", data: "", sendID: "u", sender: "root"))],
        invokes: .resumed([]), sessionID: "root.kid")
    let root = SavedState(
        shape: shape, configuration: ["a"], current: "a", variables: [],
        invokes: .resumed([SavedChild(id: "kid", start: 1, invokeID: "s.1.kid", document: "kid", session: child)]),
        sessionID: "root", invokeStarts: [SavedEntry("kid", 1)])
    #expect(
        refusal(root.toJSON())
            == "the session at invokes[0].session: 'pending[0].sender' is 'root', and this session is 'root.kid': a parent act is this session's own send"
    )
}

@Test func aStoredSendWhoseOriginNamesNoSessionIsRefusedAndAQueuedEventIsReadAsSaved() throws {
    // A session's own send that stored an origin names the session that made it: what the origin reads as is a
    // session id, an XML Nmtoken, in format 2 and in format 1. The prefix alone reads as itself, another
    // processor's address as itself, and `a%20b` decodes to `a b`, so each is refused; a queued event that
    // stored the prefix alone is the pair as saved, with no sender.
    let state = SavedState(
        shape: String(repeating: "a", count: 64), configuration: ["a"], current: "a", variables: [],
        pending: [SavedSend(due: 5, act: .raise(event: "e", data: "", sendID: "s", sender: "root"))],
        invokes: .resumed([]), sessionID: "root")
    func format2(_ origin: String) -> String {
        state.toJSON()
            .replacingOccurrences(of: #"{"format":3,"#, with: #"{"format":2,"#)
            .replacingOccurrences(of: #""origin":\#(scxmlPairText("root")),"sender":"root""#, with: #""origin":"\#(origin)""#)
    }
    func format1Text(_ origin: String) -> String {
        let act = #"{"due":"5","act":"raise","event":"e","data":"","sendid":"s","origin":"\#(origin)"}"#
        return format1().replacingOccurrences(of: #""pending":[]"#, with: #""pending":[\#(act)]"#)
    }
    #expect(format2("sce://scxml/").contains(#""sendid":"s","origin":"sce://scxml/"}"#), "\(format2("sce://scxml/"))")
    for origin in ["sce://scxml/", "mesh://elsewhere", "sce://scxml/a%20b"] {
        let refused =
            "'pending[0].origin' is '\(origin)', which names no session: a raise act is a session's own send, and its stored origin is that session's id or its SCXML location"
        #expect(refusal(format2(origin)) == refused)
        #expect(refusal(format1Text(origin)) == refused)
    }
    // A neighbour that names a session is read as its sender: the id, or the id its SCXML location names.
    for origin in ["peer:1", "sce://scxml/peer%3A1"] {
        let two = try SavedState.fromJSON(format2(origin))
        #expect(two.pending.map(\.act) == [.raise(event: "e", data: "", sendID: "s", sender: "peer:1")])
        let one = try SavedState.fromJSON(format1Text(origin))
        #expect(one.pending.map(\.act) == [.raise(event: "e", data: "", sendID: "s", sender: "peer:1")])
    }
    let queued = #"{"name":"go","data":"","type":"external","sendid":"","origin":"sce://scxml/","origintype":"","invokeid":"","hostinvoketoken":null}"#
    let read = try SavedState.fromJSON(
        format1().replacingOccurrences(of: #""external":[]"#, with: #""external":[\#(queued)]"#))
    #expect(read.external.first?.origin == EventOrigin(processor: scxmlEventProcessorType, address: "sce://scxml/"))
    #expect(read.external.first?.sender == nil)
}

@Test func aFormat2SendThatStoredNoOriginIsReadAsSentByTheSessionThatHoldsIt() throws {
    // docs/adr/0090, section 7: a `raise`, `parent` or `child` act of a format-2 text whose stored `origin` is
    // empty is the send of the session it sits in, its `sessionid`, so the text it is written back as is one a
    // format-3 reader reads.
    let shape = String(repeating: "a", count: 64)
    let child = SavedState(
        shape: shape, configuration: ["a"], current: "a", variables: [],
        pending: [SavedSend(due: 9, act: .parent(event: "up", data: "", sendID: "u", sender: "root.kid"))],
        invokes: .resumed([]), sessionID: "root.kid")
    let root = SavedState(
        shape: shape, configuration: ["a"], current: "a", variables: [],
        pending: [SavedSend(due: 5, act: .raise(event: "e", data: "", sendID: "s", sender: "root"))],
        invokes: .resumed([SavedChild(id: "kid", start: 1, invokeID: "s.1.kid", document: "kid", session: child)]),
        sessionID: "root", invokeStarts: [SavedEntry("kid", 1)])
    let written = root.toJSON()
    let stored = written
        .replacingOccurrences(of: #"{"format":3,"#, with: #"{"format":2,"#)
        .replacingOccurrences(of: #""origin":\#(scxmlPairText("root")),"sender":"root""#, with: #""origin":"""#)
        .replacingOccurrences(of: #""origin":\#(scxmlPairText("root.kid")),"sender":"root.kid""#, with: #""origin":"""#)
    #expect(stored.components(separatedBy: #""origin":"""#).count == 3, "both acts stored no origin: \(stored)")
    let read = try SavedState.fromJSON(stored)
    #expect(read == root, "each act is its own session's send")
    #expect(read.toJSON() == written)
    #expect(try SavedState.fromJSON(read.toJSON()) == root, "a format-3 reader reads the text written back")
    // A format-1 text names no session, so its act that stored no origin holds no sender: the machine that
    // restores it arms it as its own.
    let unnamed = #"{"due":"5","act":"raise","event":"e","data":"","sendid":"s","origin":""}"#
    let one = try SavedState.fromJSON(format1().replacingOccurrences(of: #""pending":[]"#, with: #""pending":[\#(unnamed)]"#))
    #expect(one.pending.map(\.act) == [.raise(event: "e", data: "", sendID: "s", sender: nil)])
}

@Test func aSessionsOwnWaitingSendIsSavedAsSentByTheSessionAndAnInternalOneAsSentByNone() throws {
    // docs/adr/0090, section 7, "Text form": a `raise`, `parent` or `child` act carries the pair and the id of the
    // session saved, and an `internal` act neither, whatever sender the waiting entry kept.
    let (engine, _, _) = makeEngine { _ in }
    engine.scheduleEvent(.go, delay: .milliseconds(5), sendID: "s", eventData: "", sender: "")
    engine.scheduleRoutedEvent(
        .go, delay: .milliseconds(6), sendID: "t", eventData: "", sender: "root",
        route: ScheduledRoute(kind: .internalQueue, eventName: "go"))
    engine.scheduleRoutedEvent(
        nil, delay: .milliseconds(7), sendID: "u", eventData: "", sender: "root",
        route: ScheduledRoute(kind: .parent, eventName: "up"))
    let saved = try engine.savedPendingSends(wallNowMs: 1000, sessionID: "root")
    #expect(
        saved.map(\.act) == [
            .raise(event: "go", data: "", sendID: "s", sender: "root"),
            .internal(event: "go", data: "", sendID: "t"),
            .parent(event: "up", data: "", sendID: "u", sender: "root"),
        ])
}

@Test func aSessionsOwnSendThatStoredNoSenderIsArmedAsSentByTheRestoredSession() throws {
    // A format-1 text stored an empty `origin` for a send of a machine that held no session id: the send is the
    // restored session's own, and it is armed as that session's, as the save of it writes it.
    let (engine, _, _) = makeEngine { _ in }
    let sends = [
        SavedSend(due: 5, act: .raise(event: "go", data: "", sendID: "a", sender: nil)),
        SavedSend(due: 6, act: .raise(event: "go", data: "", sendID: "b", sender: "peer")),
        SavedSend(due: 7, act: .parent(event: "up", data: "", sendID: "c", sender: nil)),
        SavedSend(due: 8, act: .internal(event: "go", data: "", sendID: "d")),
    ]
    let arms = try engine.judgeSavedSends(sends, invokeIDs: [], sessionID: "root")
    #expect(arms.map(\.sender) == ["root", "peer", "root", ""])
}
