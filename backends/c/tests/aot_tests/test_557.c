// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2025 newmassrael
//
// W3C SCXML test557 — C11 AOT runner.
//
// W3C SCXML B.2 (test557): inline `<data id="var1"><books>…</books></data>`
// and `<data id="var2" src="file:test557.txt"/>` both carry XML — the
// analyzer flips `needs_dom_helper` on the inline `<` first-char (var1),
// which gates the lua_init_engine register hook so DOM metatables are
// installed; the var.content arm and the var.src arm (after `read_data_src`)
// both detect first-char `<` and emit `sce_lua_dom_push_object +
// lua_setglobal` instead of the eval+normalize fallback. The s0 transition
// cond `var1.getElementsByTagName('book')[0].getAttribute('title') ==
// 'title1'` then dereferences through the DOMDocument metatable's
// `getElementsByTagName` callback (Lua C API recursion via dom.c's
// `sce_xml_collect`) and `getAttribute` (linear attribute lookup); s1
// verifies var2 the same way. Mirrors cpp
// `DataModelInitHelper::initializeVariable` isXML branch +
// `LuaDOMBinding::pushDOMObject`
// (sce/src/common/DataModelInitHelper.cpp:103-118 +
// sce/src/scripting/LuaDOMBinding.cpp:74-92), reimplemented in pure C in
// backends/c/runtime/base/dom.c and backends/c/runtime/scripting/
// lua_dom_binding.c.

#include <stdio.h>

#include "../require_started.h"
#include "test557_sm.h"

int main(void) {
    test557_t sm;
    REQUIRE_STARTED(test557_init(&sm), TEST557_START_OK);
    test557_run(&sm);

    int rc = test557_ended_in(&sm, TEST557_STATE_PASS) ? 0 : 1;
    if (rc != 0) {
        fprintf(stderr, "test557: FAIL — active = 0x%08x\n", (unsigned)test557_active_states(&sm));
    }
    test557_destroy(&sm);
    return rc;
}
