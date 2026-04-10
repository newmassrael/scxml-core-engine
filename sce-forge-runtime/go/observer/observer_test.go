// SPDX-License-Identifier: LGPL-2.1-or-later OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2025 newmassrael

package observer

import "testing"

type alertTag int

const (
	alertLow alertTag = iota
	alertHigh
)

func TestThresholdStateTransitions(t *testing.T) {
	var st ThresholdState
	if !st.EnterIf(true) {
		t.Error("EnterIf(true) on inactive should return true")
	}
	if st.EnterIf(true) {
		t.Error("EnterIf(true) on active should return false")
	}
	if !st.LeaveIf(true) {
		t.Error("LeaveIf(true) on active should return true")
	}
	if st.Active() {
		t.Error("Active() should be false after LeaveIf")
	}
}

func TestEventQueuePushAndIterate(t *testing.T) {
	q := NewEventQueue[alertTag]()
	if !q.IsEmpty() {
		t.Error("new queue should be empty")
	}
	q.Push(alertHigh)
	q.Push(alertLow)
	if q.Len() != 2 {
		t.Errorf("Len = %d, want 2", q.Len())
	}
	if q.Get(0) != alertHigh {
		t.Errorf("Get(0) = %v, want alertHigh", q.Get(0))
	}
	if q.Get(1) != alertLow {
		t.Errorf("Get(1) = %v, want alertLow", q.Get(1))
	}
}

func TestEventQueueClear(t *testing.T) {
	q := NewEventQueue[alertTag]()
	q.Push(alertHigh)
	q.Clear()
	if !q.IsEmpty() {
		t.Error("queue should be empty after Clear()")
	}
}
