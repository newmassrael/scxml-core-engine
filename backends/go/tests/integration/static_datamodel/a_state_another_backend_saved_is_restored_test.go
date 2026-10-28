// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael

// SCE Accepted Subset §2.15, "Saving and restoring" (docs/adr/0045, section 8):
// the saved states the shared fixtures hold are the text every backend writes
// and reads, so a state saved by one backend is one another restores. Each
// format-2 text is restored by the generated Go machine of its document and
// saved again, which is the same text byte for byte; each format-1 text, which
// every backend still reads, is restored.
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

func TestAFormatTwoStateAnotherBackendSavedIsRestoredAndSavedAsItWas(t *testing.T) {
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
		t.Fatalf("only %d of the shared format-2 texts were held to a machine", checked)
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
		// format 2, with the machine's own session id.
		again, err := machinesBySavedStem[stem](saved, savedAtMs)
		if err != nil {
			t.Errorf("%s: restoring was refused: %v", name, err)
			continue
		}
		if !strings.HasPrefix(again, `{"format":2,`) {
			t.Errorf("%s: saved again as %s, which is not format 2", name, again)
		}
		checked++
	}
	if checked < 10 {
		t.Fatalf("only %d of the shared format-1 texts were held to a machine", checked)
	}
}
