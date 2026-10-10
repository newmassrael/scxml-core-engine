// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael

//go:build amd64 || arm64 || loong64 || mips64 || mips64le || ppc64 || ppc64le || riscv64 || s390x || wasm

package queue

// The list of SCQ rings' runs written as histories (SCE Protocol-Synthesis RFC
// §synth-5-P, verification layer 2), under the architectures the queue exists
// on. See segmented_history_test.go for what is recorded and how.

import (
	"fmt"
	"testing"
)

func TestTheLscqRunsAreWrittenAsHistories(t *testing.T) {
	dir, ok := historyDir(t)
	if !ok {
		return
	}
	// The shapes are the C arm's: segments of two over rings of four, with few
	// and with several goroutines a side, short runs of many segments each.
	shapes := []struct {
		producers, consumers int
		perProducer          uint64
	}{
		{2, 1, 60},
		{1, 2, 60},
		{2, 2, 50},
		{3, 3, 30},
	}
	for _, s := range shapes {
		for run := 0; run < runsPerSegmentedShape; run++ {
			allocator := unlimitedAllocator()
			q, built := NewLscq[uint64](2, 4, allocator, LockFree)
			if !built {
				t.Fatal("the allocator gives a segment")
			}
			pushing := make([]producing, s.producers)
			for i := range pushing {
				producer, ok := q.Producer()
				if !ok {
					t.Fatal("a ring of four has a place for every producer of a shape")
				}
				pushing[i] = producing{producer.TryPush, producer.Release}
			}
			popping := make([]consuming, s.consumers)
			for i := range popping {
				consumer, ok := q.Consumer()
				if !ok {
					t.Fatal("a ring of four has a place for every consumer of a shape")
				}
				popping[i] = consuming{consumer.TryPop, consumer.Release}
			}
			document := recordSegmentedRun(t, pushing, popping, s.perProducer)
			if allocator.live() != 1 {
				t.Fatalf("the queue gave back every segment but its newest: %d still out", allocator.live())
			}
			writeHistory(t, dir, fmt.Sprintf("go_segmented_lscq_n2_r4_p%d_c%d_%d", s.producers, s.consumers, run), document)
		}
	}
}
