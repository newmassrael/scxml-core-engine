// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael

// SCE Accepted Subset §2.15, "Saving and restoring": a machine hands the document
// an id for a `<send idlocation>`, the document keeps it, and may name it in a
// `<cancel>` long after. Two ids a machine generates must never be the same, and a
// restore must not start counting again: the document that was saved may still
// hold `_auto_send_1`, and a send waiting in the saved state carries it. A saved
// state says how many ids the machine has generated (`sendseq`), and a restore
// carries on from it.
//
// Driven on a manual clock, no case sleeps. The shared instance
// `saved/statechart_static_host_invoke_running.json` has generated no id, so each
// case that needs a count writes one into it. The Rust and Kotlin halves are
// `a_saved_machine_carries_on_the_send_ids_it_generated.rs` and
// `ASavedMachineCarriesOnTheSendIdsItGeneratedTest.kt`.

package statechart_static_host_invoke

import (
	"strings"
	"testing"

	sce "github.com/newmassrael/sce-go-runtime"
)

func idsStarted() *Engine {
	policy := NewStatechartStaticHostInvokePolicy()
	engine := sce.NewEngine[StatechartStaticHostInvokeState, StatechartStaticHostInvokeEvent](&policy)
	engine.SetClock(sce.NewManualClock(0))
	engine.Initialize()
	return engine
}

// sharedWithCount is the shared instance with `sendseq` written as count.
func sharedWithCount(t *testing.T, count string) string {
	t.Helper()
	return strings.Replace(sharedText(t, false), `"sendseq":"0"`, `"sendseq":"`+count+`"`, 1)
}

func TestAMachineCountsTheIdsItGeneratesFromOne(t *testing.T) {
	engine := idsStarted()
	for _, want := range []string{"_auto_send_1", "_auto_send_2", "_auto_send_3"} {
		if got := engine.NextAutoSendID(); got != want {
			t.Errorf("the next id is %s, not %s", got, want)
		}
	}
}

func TestTwoMachinesDoNotShareACount(t *testing.T) {
	first, second := idsStarted(), idsStarted()
	if first.NextAutoSendID() != "_auto_send_1" || first.NextAutoSendID() != "_auto_send_2" {
		t.Fatal("the first machine counts from one")
	}
	if got := second.NextAutoSendID(); got != "_auto_send_1" {
		t.Errorf("an id does not depend on what else ran beside the machine: %s", got)
	}
}

func TestTheCountIsSavedWithTheMachine(t *testing.T) {
	engine := idsStarted()
	saved, err := StatechartStaticHostInvokeSaveAt(engine, savedAtMs)
	if err != nil {
		t.Fatal(err)
	}
	if saved.AutoSendSeq != 0 {
		t.Errorf("no id generated yet: %d", saved.AutoSendSeq)
	}
	engine.NextAutoSendID()
	engine.NextAutoSendID()
	saved, err = StatechartStaticHostInvokeSaveAt(engine, savedAtMs)
	if err != nil {
		t.Fatal(err)
	}
	if saved.AutoSendSeq != 2 || !strings.Contains(saved.ToJSON(), `"sendseq":"2"`) {
		t.Errorf("%d %s", saved.AutoSendSeq, saved.ToJSON())
	}
}

func TestARestoreCarriesOnFromTheCountItWasSavedWith(t *testing.T) {
	engine, _, _, err := tryRestore(sharedWithCount(t, "5"), 0)
	if err != nil {
		t.Fatal(err)
	}
	if got := engine.NextAutoSendID(); got != "_auto_send_6" {
		t.Errorf("the id a saved machine already handed out is not handed out again: %s", got)
	}
	if got := engine.NextAutoSendID(); got != "_auto_send_7" {
		t.Errorf("%s", got)
	}
}

func TestAMachineRestoredAndSavedAgainBeforeItIsDrivenKeepsTheCount(t *testing.T) {
	engine, _, _, err := tryRestore(sharedWithCount(t, "5"), 0)
	if err != nil {
		t.Fatal(err)
	}
	saved, err := StatechartStaticHostInvokeSaveAt(engine, savedAtMs)
	if err != nil {
		t.Fatal(err)
	}
	if got, want := saved.ToJSON(), sharedWithCount(t, "5"); got != want {
		t.Errorf("repeating the round trip never loses the count:\n%s\nwant\n%s", got, want)
	}
}

func TestTheSendCountIsRequiredAndMustBeACount(t *testing.T) {
	shared := sharedText(t, false)
	swap := func(from, to string) string { return strings.Replace(shared, from, to, 1) }
	for _, c := range []struct{ what, text, expected string }{
		{"no sendseq", swap(`"sendseq":"0",`, ``), "has no 'sendseq'"},
		{"a count that is a number", swap(`"sendseq":"0"`, `"sendseq":0`), "'sendseq' is not a text"},
		{"a count that is not digits", swap(`"sendseq":"0"`, `"sendseq":"-1"`), "'sendseq' (-1) is not a whole number"},
		{"a count past a signed 64-bit count", swap(`"sendseq":"0"`, `"sendseq":"9223372036854775808"`),
			"'sendseq' (9223372036854775808) is not a whole number"},
	} {
		if _, err := sce.SavedStateFromJSON(c.text); err == nil || !strings.Contains(err.Error(), c.expected) {
			t.Errorf("%s: refused as %v, want it to contain %q", c.what, err, c.expected)
		}
	}
}
