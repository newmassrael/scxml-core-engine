// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael

// SCE Accepted Subset §2.15, "Saving and restoring", for a hybrid `<invoke>`
// (§2.13). A format-2 saved state records the candidate that was running, with
// its whole state, and a restore resumes it whatever `srcexpr` reads now: the
// arguments were evaluated when the invocation started (§scxml-6.4), and a
// resumed child did not start (docs/adr/0045). A format-1 state named the
// invocation alone, and its restore starts it again from its beginning, reading
// `srcexpr` and the arguments from the restored variables as entering the state
// would: the one case in which a child restarts.
//
// `static_invoke_hybrid_saved.scxml` invokes `watch`, whose `srcexpr` is `pick`
// among `static_hybrid_watcher` (ends on the 8 it is handed) and
// `static_hybrid_holder` (ends on the 7). `base` is 7 when the child starts, and
// `bump` makes it 8 without telling the child; `swap` makes `pick` name the other
// candidate. The Rust and Kotlin halves are
// `a_static_hybrid_invoke_is_started_again_by_a_restore.rs` and
// `AStaticHybridInvokeIsStartedAgainByARestoreTest.kt`.

package static_datamodel

import (
	"reflect"
	"slices"
	"testing"

	sce "github.com/newmassrael/sce-go-runtime"

	"github.com/newmassrael/sce-go-tests/integration/static_datamodel/static_invoke_hybrid_saved"
)

type hybridEngine = sce.Engine[static_invoke_hybrid_saved.StaticInvokeHybridSavedState, static_invoke_hybrid_saved.StaticInvokeHybridSavedEvent]

func hybridStarted() (*hybridEngine, *static_invoke_hybrid_saved.StaticInvokeHybridSavedPolicy) {
	policy := static_invoke_hybrid_saved.NewStaticInvokeHybridSavedPolicy()
	engine := sce.NewEngine[static_invoke_hybrid_saved.StaticInvokeHybridSavedState, static_invoke_hybrid_saved.StaticInvokeHybridSavedEvent](&policy)
	engine.SetClock(sce.NewManualClock(0))
	engine.Initialize()
	return engine, &policy
}

// hybridSettle lets the child run and report to its parent.
func hybridSettle(engine *hybridEngine) {
	for i := 0; i < 5; i++ {
		engine.Tick()
	}
}

func hybridSaved(t *testing.T, engine *hybridEngine) *sce.SavedState {
	t.Helper()
	saved, err := static_invoke_hybrid_saved.StaticInvokeHybridSavedSaveAt(engine, savedAtMs)
	if err != nil {
		t.Fatalf("saves: %v", err)
	}
	return saved
}

func hybridRestoredFrom(t *testing.T, saved *sce.SavedState) (*hybridEngine, *static_invoke_hybrid_saved.StaticInvokeHybridSavedPolicy) {
	t.Helper()
	read, err := sce.SavedStateFromJSON(saved.ToJSON())
	if err != nil {
		t.Fatal(err)
	}
	policy := static_invoke_hybrid_saved.NewStaticInvokeHybridSavedPolicy()
	engine, err := static_invoke_hybrid_saved.StaticInvokeHybridSavedRestoreAt(&policy, read, sce.NewManualClock(0), savedAtMs)
	if err != nil {
		t.Fatalf("restores: %v", err)
	}
	return engine, &policy
}

// hybridRestored is the machine saved and restored, as a host carries it through
// its own storage.
func hybridRestored(t *testing.T, engine *hybridEngine) (*hybridEngine, *static_invoke_hybrid_saved.StaticInvokeHybridSavedPolicy) {
	t.Helper()
	return hybridRestoredFrom(t, hybridSaved(t, engine))
}

// asFormatOne is the same moment as a format-1 state, which names the running
// invocation by its id alone. The document holds no enum, so its format-1 shape is
// the format-2 one the save wrote.
func asFormatOne(saved *sce.SavedState) *sce.SavedState {
	older := *saved
	older.Invokes = sce.SavedInvokes{IDs: saved.Invokes.InvokeIDs()}
	older.SessionID = ""
	older.InvokeStarts = nil
	return &older
}

// hybridDocuments is the document each running child of saved runs.
func hybridDocuments(t *testing.T, saved *sce.SavedState) []string {
	t.Helper()
	if !saved.Invokes.Resumed {
		t.Fatal("a format-2 state names each child's document")
	}
	documents := []string{}
	for _, child := range saved.Invokes.Children {
		documents = append(documents, child.Document)
	}
	return documents
}

func hybridWorking(engine *hybridEngine) bool {
	return slices.Contains(engine.GetActiveStates(), static_invoke_hybrid_saved.StaticInvokeHybridSavedStateWorking)
}

func TestARestoredHybridChildKeepsTheValuesItWasHanded(t *testing.T) {
	engine, _ := hybridStarted()
	hybridSettle(engine)
	// The watcher was handed 7 and waits for 8; `bump` makes `base` 8 and it is not
	// told.
	engine.RaiseExternalByName("bump", "")
	hybridSettle(engine)

	// Resumed, the watcher holds the 7 it was handed: nothing is evaluated again, so
	// it still waits.
	restored, policy := hybridRestored(t, engine)
	hybridSettle(restored)
	if policy.Completed() != 0 || !hybridWorking(restored) {
		t.Errorf("completed %d, working %v", policy.Completed(), hybridWorking(restored))
	}
}

func TestAHybridRestoreResumesTheCandidateThatWasRunningWhateverSrcexprReadsNow(t *testing.T) {
	engine, policy := hybridStarted()
	hybridSettle(engine)
	// The watcher is the child that is running when the machine is saved, though
	// `pick` now names the holder.
	engine.RaiseExternalByName("swap", "")
	engine.RaiseExternalByName("bump", "")
	hybridSettle(engine)
	if policy.Completed() != 0 {
		t.Fatalf("completed is %d", policy.Completed())
	}
	saved := hybridSaved(t, engine)
	if got := hybridDocuments(t, saved); !reflect.DeepEqual(got, []string{"static_hybrid_watcher"}) {
		t.Fatalf("the running child is %v", got)
	}

	restored, _ := hybridRestoredFrom(t, saved)
	hybridSettle(restored)
	if got := hybridDocuments(t, hybridSaved(t, restored)); !reflect.DeepEqual(got, []string{"static_hybrid_watcher"}) {
		t.Errorf("the watcher was resumed, not the holder `pick` names now: %v", got)
	}
	if !hybridWorking(restored) {
		t.Error("the watcher waits for an 8 it was not handed")
	}
}

func TestAHybridChildNothingHasChangedForWaitsAsItDid(t *testing.T) {
	engine, _ := hybridStarted()
	hybridSettle(engine)
	restored, policy := hybridRestored(t, engine)
	hybridSettle(restored)
	// `base` is still 7: the watcher waits for an 8, as it did.
	if policy.Completed() != 0 || !hybridWorking(restored) {
		t.Errorf("completed %d, working %v", policy.Completed(), hybridWorking(restored))
	}
}

func TestAHybridChildThatEndedIsNotStartedAgain(t *testing.T) {
	// In format 1 the watcher started again is handed the 8 `base` holds, and ends.
	engine, _ := hybridStarted()
	hybridSettle(engine)
	engine.RaiseExternalByName("bump", "")
	hybridSettle(engine)
	restarted, restartedPolicy := hybridRestoredFrom(t, asFormatOne(hybridSaved(t, engine)))
	hybridSettle(restarted)
	if restartedPolicy.Completed() != 1 {
		t.Fatalf("completed is %d", restartedPolicy.Completed())
	}

	// Saved once its `done.invoke` was taken, the machine is past `working`, and a
	// restore names no invocation to resume or start.
	if hybridWorking(restarted) {
		t.Fatal("the machine is past `working`")
	}
	again, againPolicy := hybridRestored(t, restarted)
	hybridSettle(again)
	if againPolicy.Completed() != 1 {
		t.Errorf("the child ran twice: completed is %d", againPolicy.Completed())
	}
	if hybridWorking(again) {
		t.Error("the restored machine is past `working`")
	}
}

// ── format 1: the invocation named by its id is started again ───────────────

func TestAFormatOneRestoreHandsTheChildTheValuesTheRestoredMachineHolds(t *testing.T) {
	engine, policy := hybridStarted()
	hybridSettle(engine)
	engine.RaiseExternalByName("bump", "")
	hybridSettle(engine)
	if policy.Completed() != 0 {
		t.Fatalf("completed is %d", policy.Completed())
	}

	// Started again from its beginning, the watcher is handed 8, and ends.
	restarted, restartedPolicy := hybridRestoredFrom(t, asFormatOne(hybridSaved(t, engine)))
	hybridSettle(restarted)
	if restartedPolicy.Completed() != 1 {
		t.Errorf("completed is %d", restartedPolicy.Completed())
	}
}

func TestAFormatOneRestoreReadsTheSrcexprAgain(t *testing.T) {
	engine, policy := hybridStarted()
	hybridSettle(engine)
	engine.RaiseExternalByName("swap", "")
	engine.RaiseExternalByName("bump", "")
	hybridSettle(engine)
	if policy.Completed() != 0 {
		t.Fatalf("completed is %d", policy.Completed())
	}

	// The restored fields name the holder, which is handed 8 and waits for 7. The
	// watcher, which would have ended on that 8, is not what the restore starts.
	restarted, restartedPolicy := hybridRestoredFrom(t, asFormatOne(hybridSaved(t, engine)))
	hybridSettle(restarted)
	if restartedPolicy.Completed() != 0 {
		t.Error("the watcher was started")
	}
	if !hybridWorking(restarted) {
		t.Error("the holder waits for a 7 it is not handed")
	}
	// The holder keeps its child in a slot of its own: the invocation is still
	// running, and a save names it.
	if got := hybridDocuments(t, hybridSaved(t, restarted)); !reflect.DeepEqual(got, []string{"static_hybrid_holder"}) {
		t.Errorf("a save names %v", got)
	}
}
