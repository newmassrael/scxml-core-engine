// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
//
// The shared to-do list (`todo_peer`) bound to the scenario driver
// (`static_scenario_harness.h`): its readers and `todo_peer_scenario`, whatever bounds the
// machine was built at. `test_static_scalars.c` replays it at the bounds its document writes and
// `test_static_large.c` at the bounds of a team (docs/adr/0082); the scenarios fill no list, so
// one answer holds at both.

#ifndef SCE_TESTS_TODO_PEER_SCENARIO_H
#define SCE_TESTS_TODO_PEER_SCENARIO_H

#include "static_scenario_harness.h"
#include "todo_peer_sm.h"

// todo_peer: one copy of a shared to-do list, replayed through the afternoon the
// world simulation ran it through, as each of the three that took part saw it.
// A to-do's fields are values the copy keeps, each stamped; the adds no write-over
// removed (`adds`) say which stamp of a field is live, merged by the standard
// `orset_union` and `orset_live`. Stamps are 64-bit and cross as their digits; a
// field's text travels with its value; and each change a copy sends its group is
// held send by send.
#define TODO_VALUE_FIELDS(N, E)                                                                                        \
    E(id, wide_digits)                                                                                                 \
    E(field, wide_digits)                                                                                              \
    E(value, wide_signed_digits)                                                                                       \
    E(text, todo_peer_text_text)                                                                                       \
    E(wallTime, wide_signed_digits)                                                                                    \
    N(counter)                                                                                                         \
    E(nodeId, wide_digits)
VARIABLE_READER(todo_peer, refusals)
VARIABLE_READER(todo_peer, agendaNext)
VARIABLE_READER(todo_peer, full)
VARIABLE_READER(todo_peer, denied)

static const char *todo_peer_text_text(sce_static_string_96_t text) {
    static char copy[sizeof(text.data)];
    memcpy(copy, text.data, sizeof(copy));
    return copy;
}

RECORD_READER(todo_peer, clock, todo_peer_record_hlc_t, HLC_FIELDS)
RECORD_LIST_READER(todo_peer, adds, todo_peer_record_entry_view_t, todo_peer_record_entry_t, ORSET_ENTRY_FIELDS)
RECORD_LIST_READER(todo_peer, values, todo_peer_record_value_view_t, todo_peer_record_value_t, TODO_VALUE_FIELDS)
RECORD_LIST_READER(todo_peer, agenda, todo_peer_record_occurrence_view_t, todo_peer_record_occurrence_t,
                   OCCURRENCE_FIELDS)

static const char *todo_peer_text(void *sm, const char *name) {
    if (strcmp(name, "node") == 0) {
        return wide_digits(todo_peer_get_node((const todo_peer_t *)sm));
    }
    return NULL;
}

static const name_value_t todo_peer_states[] = {
    {"open", TODO_PEER_STATE_OPEN},
};
static const variable_t todo_peer_variables[] = {
    {"refusals", todo_peer_read_refusals},
    {"agendaNext", todo_peer_read_agendaNext},
    {"full", todo_peer_read_full},
    {"denied", todo_peer_read_denied},
};
static const record_variable_t todo_peer_records[] = {RECORD_ROW(todo_peer, clock),
                                                      RECORD_LIST_ROW(todo_peer, adds),
                                                      RECORD_LIST_ROW(todo_peer, values),
                                                      RECORD_LIST_ROW(todo_peer, agenda),
                                                      {NULL, NULL, NULL, NULL}};
STATIC_SCENARIO_PEERED(todo_peer, todo_peer_states, todo_peer_variables, todo_peer_text, no_lists, todo_peer_records)

#endif
