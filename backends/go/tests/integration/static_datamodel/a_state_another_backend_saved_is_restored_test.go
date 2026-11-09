// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael

// SCE Accepted Subset §2.15, "Saving and restoring" (docs/adr/0045, section 8):
// the saved states the shared fixtures hold are the text every backend writes
// and reads, so a state saved by one backend is one another restores. Each
// format-3 text is restored by the generated Go machine of its document and
// saved again, which is the same text byte for byte; each format-1 text, which
// every backend still reads, is restored. Each format-2 reading fixture in
// `saved/format2/` is restored and saved at once, which is the format-3 text
// `saved/format2/read_as/` holds for it (docs/adr/0090, section 7).
//
// The Rust and Kotlin halves are `static_datamodel.rs` and
// `StaticDatamodelTest.kt`; the shared texts are
// `sce-build/tests/fixtures/static_datamodel/saved/`.

package static_datamodel

import (
	"os"
	"path/filepath"
	"strings"
	"testing"

	sce "github.com/newmassrael/sce-go-runtime"

	"github.com/newmassrael/sce-go-tests/integration/static_datamodel/static_enum"
	"github.com/newmassrael/sce-go-tests/integration/static_datamodel/static_history"
	"github.com/newmassrael/sce-go-tests/integration/static_datamodel/static_invoke_tree"
	"github.com/newmassrael/sce-go-tests/integration/static_datamodel/static_list"
	"github.com/newmassrael/sce-go-tests/integration/static_datamodel/static_optional"
	"github.com/newmassrael/sce-go-tests/integration/static_datamodel/static_optional_record"
	"github.com/newmassrael/sce-go-tests/integration/static_datamodel/static_record"
	"github.com/newmassrael/sce-go-tests/integration/static_datamodel/static_schema_pending"
	"github.com/newmassrael/sce-go-tests/integration/static_datamodel/static_schema_revision"
	"github.com/newmassrael/sce-go-tests/integration/static_datamodel/static_send_idlocation"
	"github.com/newmassrael/sce-go-tests/integration/static_datamodel/static_timers"
)

// savedAtMs is the wall-clock moment the shared fixtures were saved at, as the
// Rust and Kotlin suites have it: 2023-11-14T22:13:20Z.
const savedAtMs int64 = 1_700_000_000_000

// restoredAgain restores saved into a fresh machine of one document and saves it
// at the moment it was restored at, answering the text that is.
type restoredAgain func(saved *sce.SavedState, wallNowMs int64) (string, error)

// restoreAndSave is the one round trip every machine makes: the restore of a
// text, then the save of what it restored.
func restoreAndSave[S comparable, E comparable](
	engine *sce.Engine[S, E],
	err error,
	save func(*sce.Engine[S, E], int64) (*sce.SavedState, error),
	wallNowMs int64,
) (string, error) {
	if err != nil {
		return "", err
	}
	again, err := save(engine, wallNowMs)
	if err != nil {
		return "", err
	}
	return again.ToJSON(), nil
}

// machinesBySavedStem is each machine a shared saved state is of, by the stem of
// the document, with what restores and saves it.
var machinesBySavedStem = map[string]restoredAgain{
	"static_enum": func(saved *sce.SavedState, wall int64) (string, error) {
		policy := static_enum.NewStaticEnumPolicy()
		engine, err := static_enum.StaticEnumRestoreAt(&policy, saved, sce.NewManualClock(0), wall)
		return restoreAndSave(engine, err, static_enum.StaticEnumSaveAt, wall)
	},
	"static_history": func(saved *sce.SavedState, wall int64) (string, error) {
		policy := static_history.NewStaticHistoryPolicy()
		engine, err := static_history.StaticHistoryRestoreAt(&policy, saved, sce.NewManualClock(0), wall)
		return restoreAndSave(engine, err, static_history.StaticHistorySaveAt, wall)
	},
	"static_list": func(saved *sce.SavedState, wall int64) (string, error) {
		policy := static_list.NewStaticListPolicy()
		engine, err := static_list.StaticListRestoreAt(&policy, saved, sce.NewManualClock(0), wall)
		return restoreAndSave(engine, err, static_list.StaticListSaveAt, wall)
	},
	"static_optional": func(saved *sce.SavedState, wall int64) (string, error) {
		policy := static_optional.NewStaticOptionalPolicy()
		engine, err := static_optional.StaticOptionalRestoreAt(&policy, saved, sce.NewManualClock(0), wall)
		return restoreAndSave(engine, err, static_optional.StaticOptionalSaveAt, wall)
	},
	"static_optional_record": func(saved *sce.SavedState, wall int64) (string, error) {
		policy := static_optional_record.NewStaticOptionalRecordPolicy()
		engine, err := static_optional_record.StaticOptionalRecordRestoreAt(&policy, saved, sce.NewManualClock(0), wall)
		return restoreAndSave(engine, err, static_optional_record.StaticOptionalRecordSaveAt, wall)
	},
	"static_record": func(saved *sce.SavedState, wall int64) (string, error) {
		policy := static_record.NewStaticRecordPolicy()
		engine, err := static_record.StaticRecordRestoreAt(&policy, saved, sce.NewManualClock(0), wall)
		return restoreAndSave(engine, err, static_record.StaticRecordSaveAt, wall)
	},
	"static_schema_pending": func(saved *sce.SavedState, wall int64) (string, error) {
		policy := static_schema_pending.NewStaticSchemaPendingPolicy()
		engine, err := static_schema_pending.StaticSchemaPendingRestoreAt(&policy, saved, sce.NewManualClock(0), wall)
		return restoreAndSave(engine, err, static_schema_pending.StaticSchemaPendingSaveAt, wall)
	},
	"static_schema_revision": func(saved *sce.SavedState, wall int64) (string, error) {
		policy := static_schema_revision.NewStaticSchemaRevisionPolicy()
		engine, err := static_schema_revision.StaticSchemaRevisionRestoreAt(&policy, saved, sce.NewManualClock(0), wall)
		return restoreAndSave(engine, err, static_schema_revision.StaticSchemaRevisionSaveAt, wall)
	},
	"static_send_idlocation": func(saved *sce.SavedState, wall int64) (string, error) {
		policy := static_send_idlocation.NewStaticSendIdlocationPolicy()
		engine, err := static_send_idlocation.StaticSendIdlocationRestoreAt(&policy, saved, sce.NewManualClock(0), wall)
		return restoreAndSave(engine, err, static_send_idlocation.StaticSendIdlocationSaveAt, wall)
	},
	"static_timers": func(saved *sce.SavedState, wall int64) (string, error) {
		policy := static_timers.NewStaticTimersPolicy()
		engine, err := static_timers.StaticTimersRestoreAt(&policy, saved, sce.NewManualClock(0), wall)
		return restoreAndSave(engine, err, static_timers.StaticTimersSaveAt, wall)
	},
}

// machineOfSavedText is the stem of the document a shared text is of: the file's
// own name up to the part that says which moment of the run it is. A stem that
// another stem begins (`static_optional` and `static_optional_record`) is read as
// the longer one when the name is the longer one's: the document is the one whose
// whole name the text carries.
func machineOfSavedText(name string) string {
	found := ""
	for stem := range machinesBySavedStem {
		if strings.HasPrefix(name, stem) {
			// `static_schema_pending_first` and `static_schema_revision_second` are
			// moments of one document; `static_timers_midway` of another.
			rest := strings.TrimPrefix(name, stem)
			if (rest == "" || strings.HasPrefix(rest, "_")) && len(stem) > len(found) {
				found = stem
			}
		}
	}
	return found
}

func sharedSavedDir(t *testing.T, format1 bool) string {
	t.Helper()
	root := filepath.Join("..", "..", "..", "..", "..", "sce-build", "tests", "fixtures", "static_datamodel", "saved")
	if format1 {
		return filepath.Join(root, "format1")
	}
	return root
}

func sharedSavedTexts(t *testing.T, format1 bool) map[string]string {
	t.Helper()
	entries, err := os.ReadDir(sharedSavedDir(t, format1))
	if err != nil {
		t.Fatalf("the shared saved states: %v", err)
	}
	texts := map[string]string{}
	for _, entry := range entries {
		if entry.IsDir() || filepath.Ext(entry.Name()) != ".json" {
			continue
		}
		raw, err := os.ReadFile(filepath.Join(sharedSavedDir(t, format1), entry.Name()))
		if err != nil {
			t.Fatal(err)
		}
		texts[strings.TrimSuffix(entry.Name(), ".json")] = strings.TrimSpace(string(raw))
	}
	return texts
}

func TestAFormatThreeStateAnotherBackendSavedIsRestoredAndSavedAsItWas(t *testing.T) {
	checked := 0
	for name, text := range sharedSavedTexts(t, false) {
		stem := machineOfSavedText(name)
		if stem == "" {
			// A tree's text is the restore of a parent and its children, which the
			// machines it names hold (a_saved_tree_holds_every_session_where_it_stood).
			continue
		}
		saved, err := sce.SavedStateFromJSON(text)
		if err != nil {
			t.Errorf("%s: refused: %v", name, err)
			continue
		}
		again, err := machinesBySavedStem[stem](saved, savedAtMs)
		if err != nil {
			t.Errorf("%s: restoring and saving again was refused: %v", name, err)
			continue
		}
		if again != text {
			t.Errorf("%s: saved again as\n%s\nthe shared text is\n%s", name, again, text)
		}
		checked++
	}
	if checked < 8 {
		t.Fatalf("only %d of the shared format-3 texts were held to a machine", checked)
	}
}

func TestAFormatOneStateIsRestoredAndTakesTheMachinesOwnSession(t *testing.T) {
	checked := 0
	for name, text := range sharedSavedTexts(t, true) {
		stem := machineOfSavedText(name)
		if stem == "" {
			continue
		}
		saved, err := sce.SavedStateFromJSON(text)
		if err != nil {
			t.Errorf("%s: refused: %v", name, err)
			continue
		}
		// What a format-1 text holds is what a restore reads; saving it again writes
		// format 3, with the machine's own session id, and a text this runtime
		// reads: a waiting send the format-1 text stored with no sender is the
		// send of the session that resumed it (docs/adr/0090, section 7).
		again, err := machinesBySavedStem[stem](saved, savedAtMs)
		if err != nil {
			t.Errorf("%s: restoring was refused: %v", name, err)
			continue
		}
		if !strings.HasPrefix(again, `{"format":3,`) {
			t.Errorf("%s: saved again as %s, which is not format 3", name, again)
		}
		if _, err := sce.SavedStateFromJSON(again); err != nil {
			t.Errorf("%s: saved again as a text that is refused: %v\n%s", name, err, again)
		}
		checked++
	}
	if checked < 10 {
		t.Fatalf("only %d of the shared format-1 texts were held to a machine", checked)
	}
}

// docs/adr/0090, section 7: a waiting send is saved with the origin and the
// sender its event carries when it is delivered. `static_timers` arms its four
// sends on entering `waiting`, as the session `fixture-root`: each `raise` act
// carries that session's pair and id, and the `internal` act carries neither,
// though its scheduled entry keeps the sender, since the event it delivers is an
// internal event. The text is the shared one, byte for byte, as the Rust and
// Kotlin suites save it.
func TestAWaitingSendIsSavedWithTheOriginItsEventWillCarry(t *testing.T) {
	policy := static_timers.NewStaticTimersPolicy()
	if err := policy.SetSessionID("fixture-root"); err != nil {
		t.Fatal(err)
	}
	engine := sce.NewEngine[static_timers.StaticTimersState, static_timers.StaticTimersEvent](&policy)
	engine.SetClock(sce.NewManualClock(0))
	if err := engine.Initialize(); err != nil {
		t.Fatal(err)
	}
	saved, err := static_timers.StaticTimersSaveAt(engine, savedAtMs)
	if err != nil {
		t.Fatalf("saves: %v", err)
	}
	if got, want := saved.ToJSON(), sharedSavedTexts(t, false)["static_timers"]; got != want {
		t.Errorf("saved as\n%s\nthe shared text is\n%s", got, want)
	}
}

// restoredTreeAgain restores saved into a fresh tree of `static_invoke_tree` and
// saves it at the moment it was restored at.
func restoredTreeAgain(saved *sce.SavedState, wall int64) (string, error) {
	policy := static_invoke_tree.NewStaticInvokeTreePolicy()
	engine, err := static_invoke_tree.StaticInvokeTreeRestoreAt(&policy, saved, sce.NewManualClock(0), wall)
	return restoreAndSave(engine, err, static_invoke_tree.StaticInvokeTreeSaveAt, wall)
}

// docs/adr/0090, section 7: a format-2 text is read by the table of section 7
// and a state read from it is written back as format 3. Each reading fixture in
// `saved/format2/` is restored by a machine of its document at the moment the
// shared fixtures were saved at, saved at once, and is, byte for byte, the
// format-3 text `saved/format2/read_as/` holds under the same name: the shared
// tree as the backends other than C11 stored it and as C11 stored it, a tree
// whose queue holds an event for each row of the table, and the timers whose
// internal act stored a sender.
func TestAFormatTwoStateIsRestoredAndSavedAsTheFormatThreeTextItIsReadAs(t *testing.T) {
	directory := filepath.Join(sharedSavedDir(t, false), "format2")
	entries, err := os.ReadDir(directory)
	if err != nil {
		t.Fatalf("the format-2 reading fixtures: %v", err)
	}
	checked := 0
	for _, entry := range entries {
		if entry.IsDir() || filepath.Ext(entry.Name()) != ".json" {
			continue
		}
		name := strings.TrimSuffix(entry.Name(), ".json")
		// The trees are of `static_invoke_tree`, whose texts the tree tests hold;
		// every other fixture is of a machine machinesBySavedStem holds.
		restore := machinesBySavedStem[machineOfSavedText(name)]
		if strings.HasPrefix(name, "static_invoke_tree_working") {
			restore = restoredTreeAgain
		}
		if restore == nil {
			t.Errorf("%s: no machine of its document is held here", name)
			continue
		}
		var texts [2]string
		for at, path := range []string{
			filepath.Join(directory, entry.Name()),
			filepath.Join(directory, "read_as", entry.Name()),
		} {
			raw, err := os.ReadFile(path)
			if err != nil {
				t.Fatal(err)
			}
			texts[at] = strings.TrimSpace(string(raw))
		}
		saved, err := sce.SavedStateFromJSON(texts[0])
		if err != nil {
			t.Errorf("%s: refused: %v", name, err)
			continue
		}
		again, err := restore(saved, savedAtMs)
		if err != nil {
			t.Errorf("%s: restoring and saving at once was refused: %v", name, err)
			continue
		}
		if again != texts[1] {
			t.Errorf("%s: saved at once as\n%s\nread_as/ holds\n%s", name, again, texts[1])
		}
		checked++
	}
	if checked < 4 {
		t.Fatalf("only %d of the format-2 reading fixtures were held to a machine", checked)
	}
}
