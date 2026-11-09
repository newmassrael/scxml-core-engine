// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael

package sce

import (
	"errors"
	"fmt"
	"testing"
)

// docs/adr/0096: the values a refused start hands its host. The refusal itself,
// over a generated machine and a script engine that fails each step, is held in
// backends/go/tests/integration/a_child_may_send_many_events_in_one_tick.

func TestEachStepHasTheNameEveryBackendSpells(t *testing.T) {
	for stage, want := range map[ScriptSessionStage]string{
		ScriptSessionStageCreate:          "create",
		ScriptSessionStageLibrary:         "library",
		ScriptSessionStageSystemVariables: "system-variables",
	} {
		if got := stage.String(); got != want {
			t.Errorf("the step %d is named %q, not %q", int(stage), got, want)
		}
	}
}

func TestALibraryThatDidNotLoadIsTheLibraryStep(t *testing.T) {
	library := ScriptSessionFailureOfCreation(fmt.Errorf("the session: %w",
		&ScriptLibraryNotLoaded{Library: "JSON builtins", Err: errors.New("x")}))
	if library.Stage != ScriptSessionStageLibrary {
		t.Errorf("a library that did not load is the %s step", library.Stage)
	}
	if library.Message != "the session: failed to load JSON builtins: x" {
		t.Errorf("the failure keeps the engine's words, not %q", library.Message)
	}
	if create := ScriptSessionFailureOfCreation(errors.New("y")); create.Stage != ScriptSessionStageCreate {
		t.Errorf("any other creation failure is the create step, not %s", create.Stage)
	}
	if bind := ScriptSessionFailureOfSystemVariables(errors.New("z")); bind.Stage != ScriptSessionStageSystemVariables {
		t.Errorf("a bind that failed is the system-variables step, not %s", bind.Stage)
	}
}

func TestTheFailureNamesItsStepAndTheEnginesWords(t *testing.T) {
	failure := &ScriptSessionFailure{Stage: ScriptSessionStageCreate, Message: "out of memory"}
	want := "the script session could not be established at step 'create': out of memory"
	if got := failure.Error(); got != want {
		t.Errorf("the failure reads %q, not %q", got, want)
	}
}

func TestTheRootStartRefusalForTheSessionCarriesTheFailure(t *testing.T) {
	failure := &ScriptSessionFailure{Stage: ScriptSessionStageLibrary, Message: "no chunk"}
	var refused error = &rootStartScriptSessionRefusal{failure: failure}

	if !errors.Is(refused, RootStartScriptSession) {
		t.Error("the refusal for the session is RootStartScriptSession")
	}
	if errors.Is(refused, RootStartNeedsParent) {
		t.Error("the refusal for the session is not RootStartNeedsParent")
	}
	var carried *ScriptSessionFailure
	if !errors.As(refused, &carried) || carried != failure {
		t.Fatal("the refusal for the session carries the failure")
	}
	if refused.Error() != failure.Error() {
		t.Errorf("the refusal reads %q, the failure's sentence %q", refused.Error(), failure.Error())
	}
	if got := RootStartScriptSession.Error(); got != "the machine's script session could not be established" {
		t.Errorf("RootStartScriptSession reads %q", got)
	}
}

func TestAnEntryVerdictNamesTheFailureThatRefusedIt(t *testing.T) {
	accepted := ConfigurationVerdict{Rejection: ConfigurationAccepted}
	if !accepted.Accepted() {
		t.Error("ConfigurationAccepted is an accepted entry")
	}
	failure := &ScriptSessionFailure{Stage: ScriptSessionStageCreate, Message: "no state"}
	refused := ConfigurationVerdict{Rejection: ConfigurationScriptSessionUnavailable, ScriptSessionFailure: failure}
	if refused.Accepted() {
		t.Error("an entry refused for its session is not accepted")
	}
	if got := refused.String(); got != failure.Error() {
		t.Errorf("the verdict reads %q, not the failure's sentence", got)
	}
	if got := (ConfigurationVerdict{Rejection: ConfigurationEmpty}).String(); got != ConfigurationEmpty.String() {
		t.Errorf("a refused set reads %q, its rule's words", got)
	}
}

func TestARefusedChildIsReportedByItsInvokeAndStep(t *testing.T) {
	failure := &ScriptSessionFailure{Stage: ScriptSessionStageSystemVariables, Message: "no bind"}
	want := "<invoke id='chatty'> did not start: its child's script session failed at step 'system-variables'"
	if got := ChildStartRefusal("chatty", failure); got != want {
		t.Errorf("the refused child reads %q, not %q", got, want)
	}
	other := errors.New("not a session")
	if got := ChildStartRefusal("chatty", other); got != "<invoke id='chatty'> did not start: not a session" {
		t.Errorf("a refusal that is no session failure reads %q", got)
	}
}
