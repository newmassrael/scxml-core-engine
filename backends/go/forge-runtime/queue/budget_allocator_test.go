// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael

package queue

import (
	"math"
	"sync/atomic"
)

// budgetAllocator is the allocator the segmented queues' tests inject (SCE
// Protocol-Synthesis RFC §synth-5-P): it gives at most limit segments at once,
// counting the ones it has out so a run can say that the queue gave every
// segment back. limit is the memory ceiling a bounded allocator stands for. The
// count is an atomic: the queue calls the allocator from several goroutines, and
// a budget taken with a compare-and-swap loop refuses no request on account of
// another's.
//
// Its progress is what a test says the allocator gives, so a test can inject an
// allocator that gives less than a queue's document declared for it.
type budgetAllocator struct {
	limit    int64
	progress Progress
	out      atomic.Int64
	given    atomic.Int64
}

// newBudgetAllocator is an allocator that gives limit segments at once and
// states LockFree.
func newBudgetAllocator(limit int64) *budgetAllocator {
	return &budgetAllocator{limit: limit, progress: LockFree}
}

// unlimitedAllocator is an allocator that never refuses.
func unlimitedAllocator() *budgetAllocator { return newBudgetAllocator(math.MaxInt64) }

func (a *budgetAllocator) Progress() Progress { return a.progress }

func (a *budgetAllocator) Allocate() bool {
	for {
		seen := a.out.Load()
		if seen >= a.limit {
			return false
		}
		if a.out.CompareAndSwap(seen, seen+1) {
			a.given.Add(1)
			return true
		}
	}
}

func (a *budgetAllocator) Deallocate() { a.out.Add(-1) }

// live is the segments the queue holds or has been given and not handed back.
func (a *budgetAllocator) live() int64 { return a.out.Load() }

// granted is how many segments were granted in all.
func (a *budgetAllocator) granted() int64 { return a.given.Load() }
