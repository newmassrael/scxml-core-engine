// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
//
// W3C SCXML 6.2: a `<param>` payload survives both send paths. Swift AOT path.
//
// Fixture: integration_resources/send_param_payload/send_param_payload.scxml
// (canonical, shared with the C++ / C11 / Rust / Go / Python / Kotlin channels).
//
// Regeneration: scripts/regen_swift_integration.sh

import IntegrationMachines
import SceRuntime
import Testing

// One suite a stem, so two stems that name a test alike do not redeclare one function of the module.
@Suite struct SendParamPayload {
    @Test func sendParamsReachEventDataFromChildAndInternalQueue() throws {
        let engine = Engine(policy: SendParamPayloadPolicy())
        try engine.initialize()
        _ = tickUntil(engine) { engine.isInFinalState }

        let reached = engine.terminalState
        let why: String
        switch reached {
        case .failChildPayload:
            why =
                "`fromChild` arrived without `_event.data.value`: a `datamodel=\"null\"` child needs no script engine, but its `<send>` still has to carry the params it declares. The gate is whether this send folds to literals, not whether the machine needs an engine."
        case .failInternalPayload:
            why =
                "`loopback` arrived without `_event.data.carried`: a `<send target=\"#_internal\">` must raise its params as event data, not build them and drop them at the internal-raise boundary."
        case .failNumberType:
            why =
                "`typed` arrived with `_event.data.n` unequal to 7: `expr=\"7\"` is the Number 7, and a backend that stringifies on the way through delivers \"7\", which `===` finds unequal."
        case .failStringType:
            why =
                "`typed` arrived with `_event.data.s` unequal to 'kept': a param that has to be EVALUATED reaches the runtime serialiser, whose string arm must emit the value rather than an engine spelling of it."
        case .failDuplicateParams:
            why =
                "`typed` did not carry both values of the repeated name `d` with their types: the clause lets a name repeat and every value must be delivered."
        case .failNoParamError:
            why =
                "`withBadParam` arrived with no `error.execution` before it: the error goes on the internal queue while the `<send>` is being evaluated, so it is dequeued first."
        case .failBrokenParamDelivered:
            why =
                "`_event.data.broken` arrived as the empty string: the clause says ignore the name AND the value, so a receiver must find no field at all rather than a placeholder under the name."
        case .failSiblingParamLost:
            why =
                "`_event.data.kept` did not survive alongside the failed param: one `<param>` that will not evaluate costs its own pair and nothing else."
        case .failEscapedText:
            why =
                "`escaped` did not carry `_event.data.text` as 'x\\ny': the payload is JSON, so a control character in a value is escaped; written raw, the payload is not JSON and the receiver reads nothing."
        case .failEscapedMemberName:
            why =
                "`escaped` carried its text but not the members `q\"k` and `b\\k` of `_event.data.obj`: a member name inside a value is a JSON text too, and is escaped as a value is."
        case .pass:
            why = ""
        default:
            why =
                "settled in \(engine.getCurrentState()), which is not a verdict state: the machine never got as far as a payload check, or it discarded a whole `<send>` because one `<param>` would not evaluate (the clause drops the pair, not the message)."
        }

        #expect(reached == .pass, Comment(rawValue: why))
    }
}
