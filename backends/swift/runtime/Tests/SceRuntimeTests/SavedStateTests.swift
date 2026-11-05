// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael

import Foundation
import Testing

@testable import SceRuntime

// The saved-state document (`sce-saved-state`, formats 1 and 2): what one backend wrote, every backend reads,
// and what a reader writes back is the text it was given. The shared fixtures under
// `sce-build/tests/fixtures/static_datamodel/saved` are the text the Rust and Kotlin runtimes write and read,
// so reading each and writing it again holds this runtime's reader and writer to theirs, byte for byte.

private func savedFixtures() -> [(name: String, text: String)] {
    var root = URL(fileURLWithPath: #filePath)
    // The file, then SceRuntimeTests, Tests, runtime, swift and backends.
    for _ in 0..<6 {
        root.deleteLastPathComponent()
    }
    let directory = root.appendingPathComponent("sce-build/tests/fixtures/static_datamodel/saved")
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
            #expect(state.formatVersion == (name.hasPrefix("format1/") ? 1 : 2), "\(name) is of the format it is in")
        } catch {
            Issue.record("\(name) is not read: \(error)")
        }
    }
}

/// A session of a format-2 tree that runs no child: a child of a tree is one, with an id of its own.
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
    #expect(two.formatVersion == 2)
    #expect(two.boundSessionID == "root")
    #expect(two.invokeIDs == ["kid"])
    #expect(two.invokeStart("kid") == 1)
    #expect(two.invokeStart("other") == 0, "an invoke that never started has started none")
    let again = try SavedState.fromJSON(two.toJSON())
    #expect(again.formatVersion == 2)
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
    #expect(
        refusal(#"{"format":3}"#)
            == "the saved state is format 3, and this runtime reads formats 1 and 2")
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
    let text = root.toJSON().replacingOccurrences(of: #""session":{"#, with: #""session":{"format":2,"#)
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
