// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael

// SCE Accepted Subset §2.15, "Saving and restoring": a delayed `<send>` the HOST
// serves is a wait a saved machine carries, with the request it will make when
// the wait ends, measured through the machine a generator wrote.
//
// `statechart_static_delayed_host_send.scxml` is a `datamodel="sce-static"`
// document, generated with the processor declaration: the send is armed by the
// code the generator emitted, with its two typed `<param>`s read from the
// machine's fields, and saved and restored by the `Save` and `Restore` the
// generator emitted. The Rust and Kotlin halves are
// `a_generated_machines_waiting_host_send_survives_a_restore.rs` and
// `AGeneratedMachinesWaitingHostSendSurvivesARestoreTest.kt`. The text this
// backend writes is the format-2
// `saved/statechart_static_delayed_host_send_waiting.json` (docs/adr/0045), its
// root given the id the fixtures name it by; the format-1 text of the same run,
// under `saved/format1/`, still restores.
//
// The run `start`, `bump` changes `job` after the send was armed, so the request
// the wait carries (`job` 7) is told from one evaluated again when the wait ends
// (`job` 8): a restore hands the host the request the document made, and the
// machine itself carries on with the `job` it had.
//
// Driven entirely on a manual clock: no case sleeps, and none can be decided by
// how loaded the build machine is.

package statechart_static_delayed_host_send

import (
	"os"
	"path/filepath"
	"reflect"
	"strings"
	"testing"

	sce "github.com/newmassrael/sce-go-runtime"
)

const (
	declaredType = "x-sce-host"
	// root is the session id the shared format-2 fixtures give their root.
	root = "fixture-root"
	// savedAtMs is the wall-clock moment the shared instance was saved at.
	savedAtMs int64 = 1_700_000_000_000
	// waitMs is how long the document's send waits, and waitedMs how much of that
	// has gone by when the machine is saved.
	waitMs   int64 = 500
	waitedMs int64 = 100
)

type Engine = sce.Engine[StatechartStaticDelayedHostSendState, StatechartStaticDelayedHostSendEvent]

func sharedText(t *testing.T, format1 bool) string {
	t.Helper()
	dir := filepath.Join("..", "..", "..", "..", "..", "sce-build", "tests", "fixtures", "host_processor", "saved")
	if format1 {
		dir = filepath.Join(dir, "format1")
	}
	raw, err := os.ReadFile(filepath.Join(dir, "statechart_static_delayed_host_send_waiting.json"))
	if err != nil {
		t.Fatalf("the shared saved state: %v", err)
	}
	return strings.TrimSpace(string(raw))
}

// call is what a restored machine's handler is asked: the engine's reading of
// "now" at the moment, and the request.
type call struct {
	at      int64
	request sce.HostSendRequest
}

type host struct {
	calls []call
	now   int64
}

func (h *host) register(engine *Engine) {
	engine.RegisterEventProcessor(declaredType, func(request sce.HostSendRequest) []sce.HostSendResponse {
		h.calls = append(h.calls, call{at: h.now, request: request})
		return nil
	})
}

func (h *host) advanceTo(t *testing.T, engine *Engine, toMs int64) {
	t.Helper()
	if toMs < h.now {
		t.Fatal("time does not run backwards in these cases")
	}
	from := h.now
	h.now = toMs
	engine.AdvanceTimeMs(toMs - from)
}

func drive(engine *Engine, event string) {
	engine.RaiseExternalByName(event, "")
	engine.Tick()
}

// waiting is a machine that armed its send with `job` 7, saw `bump`, and has waited
// waitedMs of the waitMs.
func waiting(t *testing.T) *Engine {
	t.Helper()
	policy := NewStatechartStaticDelayedHostSendPolicy()
	if err := policy.SetSessionID(root); err != nil {
		t.Fatal(err)
	}
	engine := sce.NewEngine[StatechartStaticDelayedHostSendState, StatechartStaticDelayedHostSendEvent](&policy)
	engine.SetClock(sce.NewManualClock(0))
	engine.RegisterEventProcessor(declaredType, func(sce.HostSendRequest) []sce.HostSendResponse { return nil })
	engine.Initialize()
	drive(engine, "start")
	drive(engine, "bump")
	engine.AdvanceTimeMs(waitedMs)
	if engine.GetCurrentState() != StatechartStaticDelayedHostSendStateWaiting {
		t.Fatalf("the machine stands in %v", engine.GetCurrentState())
	}
	return engine
}

func restored(t *testing.T, text string, elapsedMs int64) (*Engine, *host) {
	t.Helper()
	saved, err := sce.SavedStateFromJSON(text)
	if err != nil {
		t.Fatalf("the saved text reads: %v", err)
	}
	policy := NewStatechartStaticDelayedHostSendPolicy()
	engine, err := StatechartStaticDelayedHostSendRestoreAt(&policy, saved, sce.NewManualClock(0), savedAtMs+elapsedMs)
	if err != nil {
		t.Fatalf("the restore was refused: %v", err)
	}
	h := &host{}
	h.register(engine)
	return engine, h
}

// jobOf is the `job` of a request, from the text the wire carries it as.
func jobOf(request sce.HostSendRequest) string {
	if values := request.Params["job"]; len(values) > 0 {
		return values[0]
	}
	return ""
}

func TestAWaitingSendIsSavedWithTheRequestItWasArmedWith(t *testing.T) {
	saved, err := StatechartStaticDelayedHostSendSaveAt(waiting(t), savedAtMs)
	if err != nil {
		t.Fatal(err)
	}
	if len(saved.Pending) != 1 {
		t.Fatalf("one send is waiting: %+v", saved.Pending)
	}
	entry := saved.Pending[0]
	if entry.Due != savedAtMs+waitMs-waitedMs {
		t.Errorf("due is %d", entry.Due)
	}
	act := entry.Act
	if act.Kind != sce.SavedActHost || act.ProcessorType != declaredType || act.Event != "audit" ||
		act.Target != "job://report" || act.SendID != "a" {
		t.Errorf("a host-served send: %+v", act)
	}
	// Armed with `job` 7: `bump` ran after, and the request does not follow it.
	want := map[string][]string{"job": {"7"}, "label": {"report"}}
	if !reflect.DeepEqual(act.Params, want) {
		t.Errorf("params are %v, want %v", act.Params, want)
	}
	// The text every backend writes for the same machine, byte for byte. It also
	// says the machine itself carries the `job` it has now (8), apart from the 7
	// the request was armed with.
	if got, want := saved.ToJSON(), sharedText(t, false); got != want {
		t.Errorf("saved as\n%s\nthe shared text is\n%s", got, want)
	}
}

func TestARestoredSendIsMadeWhenItsMomentComesWithTheJobItWasArmedWith(t *testing.T) {
	// Back 50 ms after the save: the send has 350 ms left of the 400 it had.
	engine, h := restored(t, sharedText(t, false), 50)
	if engine.GetCurrentState() != StatechartStaticDelayedHostSendStateWaiting {
		t.Fatalf("standing in %v", engine.GetCurrentState())
	}
	h.advanceTo(t, engine, 349)
	if len(h.calls) != 0 {
		t.Fatalf("nothing is due yet: %+v", h.calls)
	}
	h.advanceTo(t, engine, 350)
	if len(h.calls) != 1 {
		t.Fatalf("the send is made once: %+v", h.calls)
	}
	made := h.calls[0]
	if made.at != 350 {
		t.Errorf("at the moment it came due: %d", made.at)
	}
	request := made.request
	if request.ProcessorType != declaredType || request.EventName != "audit" || request.Target != "job://report" ||
		request.SendID != "a" {
		t.Errorf("the request is %+v", request)
	}
	if jobOf(request) != "7" {
		t.Errorf("the request the document made, not one evaluated again with the job the machine holds now: %q", jobOf(request))
	}
	if request.EventData != `{"job":7,"label":"report"}` {
		t.Errorf("event data is %s", request.EventData)
	}
}

func TestAWaitingSendSavedInFormatOneIsMadeWhenItsMomentComes(t *testing.T) {
	// The format-1 text of the same run, which a backend wrote before trees.
	engine, h := restored(t, sharedText(t, true), 50)
	h.advanceTo(t, engine, 350)
	var jobs []string
	for _, made := range h.calls {
		jobs = append(jobs, jobOf(made.request))
	}
	if !reflect.DeepEqual(jobs, []string{"7"}) {
		t.Errorf("%+v", h.calls)
	}
}

func TestASendAlreadyDueWhenTheMachineComesBackIsMadeWhenItIsDriven(t *testing.T) {
	// A minute away: the wait ran out while the process was dead.
	engine, h := restored(t, sharedText(t, false), 60_000)
	if len(h.calls) != 0 {
		t.Fatal("restoring makes no send")
	}
	h.advanceTo(t, engine, 0)
	if len(h.calls) != 1 || h.calls[0].at != 0 {
		t.Errorf("%+v", h.calls)
	}
}

func TestARestoredSendCanStillBeCancelledByItsId(t *testing.T) {
	engine, h := restored(t, sharedText(t, false), 0)
	drive(engine, "cancel")
	h.advanceTo(t, engine, 10_000)
	if len(h.calls) != 0 {
		t.Errorf("the cancel found the send the saved state carried: %+v", h.calls)
	}
}

func TestARestoredMachineCarriesOnWithTheJobItHad(t *testing.T) {
	// `job` is 8 in the saved state; a `bump` after the restore makes it 9, and a
	// new wait armed from `idle` reads the machine's own field. The restored wait
	// is over (at 400 ms) before the second is armed: a send under an id still
	// waiting is replaced by one backend and left beside it by another, and this
	// case is not about that.
	engine, h := restored(t, sharedText(t, false), 0)
	h.advanceTo(t, engine, 400)
	drive(engine, "stop")
	drive(engine, "bump")
	drive(engine, "start")
	h.advanceTo(t, engine, 10_000)
	var jobs []string
	for _, made := range h.calls {
		jobs = append(jobs, jobOf(made.request))
	}
	if !reflect.DeepEqual(jobs, []string{"7", "9"}) {
		t.Errorf("the restored wait first, then the one armed with the job the restored field held: %+v", h.calls)
	}
}

func TestASavedStateRoundTripsThroughItsOwnText(t *testing.T) {
	engine, _ := restored(t, sharedText(t, false), 0)
	saved, err := StatechartStaticDelayedHostSendSaveAt(engine, savedAtMs)
	if err != nil {
		t.Fatal(err)
	}
	if got, want := saved.ToJSON(), sharedText(t, false); got != want {
		t.Errorf("saved as\n%s\nwant\n%s", got, want)
	}
}
