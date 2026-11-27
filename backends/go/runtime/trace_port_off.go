// SPDX-License-Identifier: AGPL-3.0-only WITH AdditionRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael

//go:build !sce_trace_port

package sce

// The trace port of a build that does not ask for one (docs/adr/0115, D4 and
// D8): trace_port.go is compiled only with the build tag sce_trace_port. Here
// the engine's recorder is a field of no size, and every call the engine makes
// at a step's boundary, exit and entry has an empty body the compiler inlines
// and removes, so the engine keeps nothing of the port and spends nothing on
// it, and SetTracePort does not exist.

// traceRecorder holds nothing: there is no port to tell.
type traceRecorder struct{}

func (*Engine[S, E]) traceBeginStart() {}

func (*Engine[S, E]) traceBeginEventless() {}

func (*Engine[S, E]) traceBeginEvent(bool, EventWithMetadata[E]) {}

func (*Engine[S, E]) traceBeginStop() {}

func (*Engine[S, E]) traceFired([]EnabledTransition[S, HistoryID]) {}

func (*Engine[S, E]) traceExited(S) {}

func (*Engine[S, E]) traceEntered(S) {}

func (*Engine[S, E]) traceEnd() {}
