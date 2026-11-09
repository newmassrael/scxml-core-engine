// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
//
// W3C SCXML 5.10.1 (docs/adr/0090, test 2): an event no Event I/O Processor
// delivered has no origin — Go AOT path. The host raises `posted` naming no
// party, and its processor answers `host.ask` with `host.reply` naming none; the
// document finds `_event.origin` and `_event.origintype` blank for both, which
// the ECMAScript data model reads as undefined (nil in the Lua it lowers to).
//
// Fixture: integration_resources/an_event_no_processor_delivered_has_no_origin/an_event_no_processor_delivered_has_no_origin.scxml
// (canonical, shared with every channel).
//
// Regeneration (after fixture or template edit):
//
//	scripts/regen_an_event_no_processor_delivered_has_no_origin_go.sh
package an_event_no_processor_delivered_has_no_origin

import (
	"testing"

	sce "github.com/newmassrael/sce-go-runtime"
	scegotest "github.com/newmassrael/sce-go-tests/harness"
)

// hostType is the `<send type>` the build is told the host serves; the
// regeneration script passes the same string as `--host-processor`.
const hostType = "x-sce-host"

func TestAnEventNoProcessorDeliveredHasNoOrigin(t *testing.T) {
	policy := NewAnEventNoProcessorDeliveredHasNoOriginPolicy()
	policy.SessionID = sce.GenerateSessionID()
	policy.ScriptEngine = scegotest.NewLuaEngine()
	engine := sce.NewEngine[AnEventNoProcessorDeliveredHasNoOriginState, AnEventNoProcessorDeliveredHasNoOriginEvent](&policy)
	engine.RegisterEventProcessor(hostType, func(request sce.HostSendRequest) []sce.HostSendResponse {
		if request.EventName != "host.ask" {
			t.Errorf("the host was asked for %q, want host.ask", request.EventName)
		}
		return []sce.HostSendResponse{{EventName: "host.reply"}}
	})
	if err := engine.Initialize(); err != nil {
		t.Fatal(err)
	}
	engine.Step()

	engine.RaiseExternal(AnEventNoProcessorDeliveredHasNoOriginEventPosted, "")
	engine.Step()

	if ended, ok := engine.TerminalState(); !ok || ended != AnEventNoProcessorDeliveredHasNoOriginStateDone {
		t.Errorf("the run must end in `done`")
	}
	observed := []struct {
		name string
		read func() (int64, bool)
		want int64
	}{
		{"postedBlank", policy.PostedBlank, 1},
		{"replyBlank", policy.ReplyBlank, 1},
	}
	for _, o := range observed {
		if got, ok := o.read(); !ok || got != o.want {
			t.Errorf("%s = %d (readable %v), want %d", o.name, got, ok, o.want)
		}
	}
}
