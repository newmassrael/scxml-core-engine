// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2025 newmassrael
//
// W3C SCXML test561 — C11 AOT runner.
//
// W3C SCXML 5.9.2 + B.2 (test561): `<send
// event="foo"><content><books>…</books></content></send>` carries XML in the
// content body — the analyzer flips `needs_dom_helper` on `action.content`
// first-char `<`, `send_content_json` detects the same prefix and carries the
// body on the event's `data[]` as the XML it is (instead of the JSON path
// used for non-XML literal content). The dequeue's `set_current_event` builds
// the DOM from that text, so the cond
// `_event.data.getElementsByTagName('book')[1].getAttribute('title') ==
// 'title2'` resolves through the DOMDocument metatable. Mirrors cpp
// `LuaEngine::setCurrentEvent` event-data XML branch
// (sce/src/scripting/LuaEngine.cpp:1145-1149 — first-char `<` detection at
// delivery time, as here).

#include <stdio.h>

#include "../require_started.h"
#include "test561_sm.h"

int main(void) {
    test561_t sm;
    REQUIRE_STARTED(test561_init(&sm), TEST561_START_OK);
    test561_run(&sm);

    int rc = test561_ended_in(&sm, TEST561_STATE_PASS) ? 0 : 1;
    if (rc != 0) {
        fprintf(stderr, "test561: FAIL — active = 0x%08x\n", (unsigned)test561_active_states(&sm));
    }
    test561_destroy(&sm);
    return rc;
}
