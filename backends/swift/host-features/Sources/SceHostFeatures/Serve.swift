// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael

import SceRuntime

/// Registers `host` as the invoker of every standard type on `engine`.
///
/// A start is answered with its completion (a failure is a completion whose error is a
/// code) or its refusal, or not at all when the host holds it. The cancel the engine
/// delivers when a state exits or a deadline passes reaches the host too.
public func serve<Policy: StatePolicy>(_ host: ReferenceHost, on engine: Engine<Policy>) {
    for operation in ReferenceHost.types {
        engine.registerInvoker(operation) { event in
            switch event {
            case .cancel(let cancel):
                host.cancel(cancel.invokeID)
                return nil
            case .start(let start):
                var params: HostParams = [:]
                for (name, values) in start.params {
                    if let first = values.first {
                        params[name] = first
                    }
                }
                switch host.start(operation: operation, id: start.invokeID, params: params) {
                case .done(let data)?:
                    return .done(data)
                case .refuse(let reason)?:
                    return .refused(reason)
                case .hold?, nil:
                    return nil
                }
            }
        }
    }
}
