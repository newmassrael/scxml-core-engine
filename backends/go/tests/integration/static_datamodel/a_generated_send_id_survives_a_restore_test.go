// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael

// SCE Accepted Subset §2.15, "Saving and restoring": the id a machine generates
// for a `<send idlocation>` is held by the document in a variable, and by the send
// that waits under it, so a saved state carries both and the count the next id
// comes from. A machine restored from it cancels the send by the id the document
// holds, and numbers the ids it generates next after the count: an id the saved
// state already holds is never generated again, which would let one `<cancel>`
// remove two sends.
//
// `static_send_idlocation.scxml`: `arm` sends two delayed events, `first.due` and
// `second.due`, each under an id written to `first` and `second`, and
// `cancel.first` and `cancel.second` remove the send whose id the variable holds.
// Everything runs on a host-owned clock, so no case sleeps. The shared fixture
// `saved/static_send_idlocation_armed.json` is the format-2 text every backend
// writes (docs/adr/0045), its root given the id the fixtures name it by, and
// `saved/format1/` holds the format-1 text of the same run, which still restores.
// The Rust and Kotlin halves are `a_generated_send_id_survives_a_restore.rs` and
// `AGeneratedSendIdSurvivesARestoreTest.kt`.

package static_datamodel

import (
	"strings"
	"testing"

	sce "github.com/newmassrael/sce-go-runtime"

	"github.com/newmassrael/sce-go-tests/integration/static_datamodel/static_send_idlocation"
)

type sendIdEngine = sce.Engine[static_send_idlocation.StaticSendIdlocationState, static_send_idlocation.StaticSendIdlocationEvent]

// sendIdArmed is a machine that has run `arm` once, on a clock the test owns.
func sendIdArmed(t *testing.T) (*sendIdEngine, *static_send_idlocation.StaticSendIdlocationPolicy) {
	t.Helper()
	policy := static_send_idlocation.NewStaticSendIdlocationPolicy()
	if err := policy.SetSessionID("fixture-root"); err != nil {
		t.Fatal(err)
	}
	engine := sce.NewEngine[static_send_idlocation.StaticSendIdlocationState, static_send_idlocation.StaticSendIdlocationEvent](&policy)
	engine.SetClock(sce.NewManualClock(0))
	engine.Initialize()
	sendIdSend(engine, "arm")
	return engine, &policy
}

func sendIdRestoredFrom(t *testing.T, text string) (*sendIdEngine, *static_send_idlocation.StaticSendIdlocationPolicy) {
	t.Helper()
	saved, err := sce.SavedStateFromJSON(text)
	if err != nil {
		t.Fatal(err)
	}
	policy := static_send_idlocation.NewStaticSendIdlocationPolicy()
	engine, err := static_send_idlocation.StaticSendIdlocationRestoreAt(&policy, saved, sce.NewManualClock(0), savedAtMs)
	if err != nil {
		t.Fatalf("restores: %v", err)
	}
	return engine, &policy
}

func sendIdSend(engine *sendIdEngine, name string) {
	engine.RaiseExternalByName(name, "")
	engine.Tick()
}

func sharedArmed(t *testing.T, format1 bool) string {
	t.Helper()
	text := sharedSavedTexts(t, format1)["static_send_idlocation_armed"]
	if text == "" {
		t.Fatal("the shared fixture static_send_idlocation_armed")
	}
	return text
}

func TestAMachineThatArmedTwoSendsSavesTheIdsItGenerated(t *testing.T) {
	engine, _ := sendIdArmed(t)
	saved, err := static_send_idlocation.StaticSendIdlocationSaveAt(engine, savedAtMs)
	if err != nil {
		t.Fatal(err)
	}
	if saved.AutoSendSeq != 2 || len(saved.Pending) != 2 {
		t.Fatalf("two ids were generated and two sends wait: seq %d, %d pending", saved.AutoSendSeq, len(saved.Pending))
	}
	text := saved.ToJSON()
	for _, id := range []string{"_auto_send_1", "_auto_send_2"} {
		if !strings.Contains(text, `"sendid":"`+id+`"`) {
			t.Errorf("a waiting send carries %s: %s", id, text)
		}
	}
	// The text every backend writes for the same run, byte for byte: the variables
	// hold the ids the waiting sends are known by.
	if want := sharedArmed(t, false); text != want {
		t.Errorf("saved as\n%s\nthe shared text is\n%s", text, want)
	}
}

func TestARestoredSendIsCancelledByTheIdTheDocumentHolds(t *testing.T) {
	// From the format-2 text, and from the format-1 text of the same run.
	for _, text := range []string{sharedArmed(t, false), sharedArmed(t, true)} {
		engine, policy := sendIdRestoredFrom(t, text)
		sendIdSend(engine, "cancel.first")
		engine.AdvanceTimeMs(250)
		if policy.FirstFired() != 0 {
			t.Errorf("its send was removed: first fired %d", policy.FirstFired())
		}
		if policy.SecondFired() != 1 {
			t.Errorf("the other came due: second fired %d", policy.SecondFired())
		}
	}
}

func TestARestoredMachineNumbersItsNextIdsAfterTheSavedCount(t *testing.T) {
	engine, policy := sendIdRestoredFrom(t, sharedArmed(t, false))
	// The ids the saved state holds are 1 and 2: `arm` again writes 3 and 4 to the
	// variables, and a `<cancel>` by `first` removes the send of id 3 and not the one
	// of id 1, which is still waiting.
	sendIdSend(engine, "arm")
	saved, err := static_send_idlocation.StaticSendIdlocationSaveAt(engine, savedAtMs)
	if err != nil {
		t.Fatal(err)
	}
	if saved.AutoSendSeq != 4 {
		t.Errorf("four ids were generated: %d", saved.AutoSendSeq)
	}
	if !strings.Contains(saved.ToJSON(), `"first":"_auto_send_3","second":"_auto_send_4"`) {
		t.Errorf("%s", saved.ToJSON())
	}

	sendIdSend(engine, "cancel.first")
	engine.AdvanceTimeMs(250)
	if policy.FirstFired() != 1 {
		t.Errorf("the send of id 1 came due, the one of id 3 was removed: first fired %d", policy.FirstFired())
	}
	if policy.SecondFired() != 2 {
		t.Errorf("ids 2 and 4 came due: second fired %d", policy.SecondFired())
	}
}
