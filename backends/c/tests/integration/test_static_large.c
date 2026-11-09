// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
//
// The shared to-do list (`todo_peer`) built as C at the bounds of a team, a calendar of twenty
// people with a great many events (`sce-codegen --limit merge_values=65536 --limit
// merge_entries=65536`, the bounds of the Rust world's `large` build), replayed through the
// three scenarios `test_static_scalars.c` replays it through at the bounds its document writes
// (docs/adr/0082 §5).
//
// What a failing run would say: a C11 machine holds some list of a bound, or a copy of one, where
// a call or a loop of it runs, and a bound a team needs is a stack that is gone at the call. The
// machine itself is held in static storage, as a host holds a machine of this size; nothing it
// runs may hold its bound on the stack. The scenarios fill no list, so the answers they state
// are the answers at both bounds, and every one of them is asked again here.

#include <stdio.h>

#include "todo_peer_scenario.h"

// The build this program is: a machine generated at the bounds its document writes would replay
// the same scenarios and pass, and prove nothing about a large one.
_Static_assert(TODO_PEER_LIMIT_MERGE_VALUES == UINT32_C(65536), "todo_peer is built at a team's merge_values");
_Static_assert(TODO_PEER_LIMIT_MERGE_ENTRIES == UINT32_C(65536), "todo_peer is built at a team's merge_entries");

int main(void) {
    int bad = 0;
    bad |= todo_peer_scenario("todo_peer_alice", 13);
    bad |= todo_peer_scenario("todo_peer_bob", 13);
    bad |= todo_peer_scenario("todo_peer_carol", 13);
    if (bad != 0) {
        return 1;
    }
    (void)printf("static large: ok (todo_peer_t is %lu bytes)\n", (unsigned long)sizeof(todo_peer_t));
    return 0;
}
