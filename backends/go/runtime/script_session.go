// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael

package sce

import (
	"errors"
	"fmt"
)

// Why a machine's script session could not be established, which refuses its
// start (docs/adr/0096).
//
// A machine whose document needs a datamodel runs its conditions, assignments
// and scripts in a script session. Establishing it is three steps in this
// order: create the session, load the shared library into it (the ECMAScript
// semantics and the JSON builtins), and bind the system variables
// `_sessionid`, `_name` and `_ioprocessors`. A machine started over a session
// one of them failed for would run with no datamodel, every condition an error
// and every assignment lost, and the document could not even handle it: its
// means of handling an error runs in the session that failed. So the start is
// refused, and the host is handed a *ScriptSessionFailure.
//
// Cross-language siblings: `sce_rust_runtime::ScriptSessionFailure` (Rust) and
// `SCE::ScriptSessionFailure` (C++), whose step names and sentence are these.

// ScriptSessionStage is the step of establishing a script session that failed.
type ScriptSessionStage int

const (
	// ScriptSessionStageCreate: the script engine could not create the
	// session.
	ScriptSessionStageCreate ScriptSessionStage = iota + 1
	// ScriptSessionStageLibrary: the shared library did not load into the new
	// session, which the engine then did not keep. A library chunk that does
	// not load is a defect of the build that shipped it.
	ScriptSessionStageLibrary
	// ScriptSessionStageSystemVariables: the system variables `_sessionid`,
	// `_name` and `_ioprocessors` could not be bound; the session was
	// destroyed.
	ScriptSessionStageSystemVariables
)

// String is the step's name, the same word on every backend: `create`,
// `library` or `system-variables`.
func (s ScriptSessionStage) String() string {
	switch s {
	case ScriptSessionStageCreate:
		return "create"
	case ScriptSessionStageLibrary:
		return "library"
	case ScriptSessionStageSystemVariables:
		return "system-variables"
	}
	return "unknown"
}

// ScriptSessionFailure is why a machine's script session could not be
// established, so its start was refused: no state was entered, no executable
// content ran, no event was taken, and what the failed steps made was
// released.
//
// Engine.Initialize answers it as its error, InitializeAsRoot wraps it (read it
// with errors.As), and EnterAt carries it in its ConfigurationVerdict.
type ScriptSessionFailure struct {
	// Stage is the step that failed.
	Stage ScriptSessionStage
	// Message is what the script engine said.
	Message string
}

// Error is the failure as a sentence: the step that failed, then the engine's
// words, the sentence every backend writes.
func (f *ScriptSessionFailure) Error() string {
	return fmt.Sprintf("the script session could not be established at step '%s': %s", f.Stage, f.Message)
}

// ScriptSessionFailureOfCreation is what IScriptEngine.CreateSession failed
// with: a shared library that did not load (a *ScriptLibraryNotLoaded anywhere
// in err's chain) is the `library` step, any other failure the `create` step.
func ScriptSessionFailureOfCreation(err error) *ScriptSessionFailure {
	stage := ScriptSessionStageCreate
	var library *ScriptLibraryNotLoaded
	if errors.As(err, &library) {
		stage = ScriptSessionStageLibrary
	}
	return &ScriptSessionFailure{Stage: stage, Message: err.Error()}
}

// ScriptSessionFailureOfSystemVariables is what
// IScriptEngine.SetupSystemVariables failed with: the `system-variables` step.
func ScriptSessionFailureOfSystemVariables(err error) *ScriptSessionFailure {
	return &ScriptSessionFailure{Stage: ScriptSessionStageSystemVariables, Message: err.Error()}
}

// ScriptLibraryNotLoaded is what an IScriptEngine's CreateSession answers when
// a chunk of the shared library did not load into the new session. The engine
// keeps no session then, and the start that asked for it is refused at the
// `library` step (ScriptSessionFailureOfCreation), which says the build is at
// fault rather than the machine or the device.
type ScriptLibraryNotLoaded struct {
	// Library names the chunk that did not load, as `ECMAScript semantics`.
	Library string
	// Err is what loading it failed with.
	Err error
}

// Error names the chunk and what loading it failed with.
func (e *ScriptLibraryNotLoaded) Error() string {
	if e.Err == nil {
		return "failed to load " + e.Library
	}
	return "failed to load " + e.Library + ": " + e.Err.Error()
}

// Unwrap is what loading the chunk failed with.
func (e *ScriptLibraryNotLoaded) Unwrap() error { return e.Err }

// ChildStartRefusal is the data of the error.execution a parent raises for the
// child of the `<invoke>` invokeID whose start was refused (docs/adr/0096): the
// invoke, and the step its child's script session failed at. The parent keeps
// no child and raises no done.invoke for it.
func ChildStartRefusal(invokeID string, refused error) string {
	var failure *ScriptSessionFailure
	if errors.As(refused, &failure) {
		return fmt.Sprintf("<invoke id='%s'> did not start: its child's script session failed at step '%s'", invokeID, failure.Stage)
	}
	return fmt.Sprintf("<invoke id='%s'> did not start: %v", invokeID, refused)
}
