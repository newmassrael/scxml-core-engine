// SPDX-License-Identifier: AGPL-3.0-only WITH AdditionRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael

package hostfeatures

import (
	sce "github.com/newmassrael/sce-go-runtime"
)

// Serve registers host as the invoker of every standard type on engine.
//
// A start is answered with its completion (a failure is a completion whose error
// is a code) or its refusal, or not at all when the host holds it, and reaches the
// host with the invocation's identity: the session that started it, the invoke's
// id and the start number the engine gave it. The cancel the engine delivers when
// a state exits or a deadline passes reaches the host too.
func Serve[S comparable, E comparable](engine *sce.Engine[S, E], host *ReferenceHost) {
	for _, operation := range Types {
		engine.RegisterInvoker(operation, func(event sce.HostInvokeEvent) *sce.HostInvokeResponse {
			if event.Cancel != nil {
				host.Cancel(event.Cancel.InvokeID)
				return nil
			}
			if event.Start == nil {
				return nil
			}
			params := Params{}
			for name, values := range event.Start.Params {
				if len(values) > 0 {
					params[name] = values[0]
				}
			}
			// docs/adr/0107, D4: the invocation's identity, which a restore starts it
			// again under.
			invocation := Invocation{
				SessionID: event.Start.SessionID, ID: event.Start.InvokeID, Start: event.Start.Start,
			}
			outcome, served := host.Start(operation, invocation, params)
			if !served {
				return nil
			}
			switch outcome.Kind {
			case Done:
				return &sce.HostInvokeResponse{DoneData: &outcome.Data}
			case Refuse:
				return &sce.HostInvokeResponse{Refusal: &outcome.Data}
			default:
				return nil
			}
		})
	}
}
