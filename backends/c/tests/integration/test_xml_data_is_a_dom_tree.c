// SPDX-License-Identifier: LGPL-2.1-or-later WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
//
// W3C SCXML B.2: a `<data>` element's XML content is a DOM structure a
// document can walk — C11 AOT.
//
// The appendix obliges the Processor to create "the corresponding DOM
// structure". Measured 2026-08-18, what every backend created was an object
// carrying three methods — `getElementsByTagName`, `getAttribute` and a
// non-standard `getTagName`, which are the two names the W3C IRP suite reads
// plus one — so `doc.tagName`, `doc.firstChild` and `doc.childNodes.length`
// answered nil on all seven channels with 230/230 C11 tests green.
//
// This channel is the one where the fixture is the ONLY witness. The other six
// bindings are measured directly against
// `tests/ecmascript/dom_read_surface.json`; here `sce_lua_dom_push_object` and
// its metatable have no caller but generated code, so nothing short of a
// document reaches them.
//
// Fixture: integration_resources/xml_data_is_a_dom_tree/xml_data_is_a_dom_tree.scxml
// (canonical, shared with the C++ / Rust / Go / Kotlin / Python channels).
//
// Regeneration: automatic at CMake build time via
// `sce_generate_static_integration_c_test(xml_data_is_a_dom_tree ...)`
// in `backends/c/tests/CMakeLists.txt`.

#include <stdio.h>

#include "xml_data_is_a_dom_tree_sm.h"

int main(void) {
    xml_data_is_a_dom_tree_t sm;
    xml_data_is_a_dom_tree_init(&sm);
    // Every transition is eventless, so the run reaches the verdict without an
    // event; no payload path is involved.
    xml_data_is_a_dom_tree_run(&sm);

    if (xml_data_is_a_dom_tree_in_state(&sm, XML_DATA_IS_A_DOM_TREE_STATE_NOTADOCUMENT)) {
        fprintf(stderr, "FAIL: the variable did not hold a document — nodeType === 9, nodeName === "
                        "'#document', documentElement.tagName === 'books' or hasAttribute('count') "
                        "did not hold\n");
        return 1;
    }
    if (xml_data_is_a_dom_tree_in_state(&sm, XML_DATA_IS_A_DOM_TREE_STATE_WRONGTREE)) {
        fprintf(stderr, "FAIL: the document element's children are not the two <book> elements in "
                        "document order — the whitespace between them may have become nodes, or a "
                        "sibling/parent link is missing\n");
        return 1;
    }
    if (xml_data_is_a_dom_tree_in_state(&sm, XML_DATA_IS_A_DOM_TREE_STATE_NOTEXT)) {
        fprintf(stderr, "FAIL: character data did not report itself as a text node, or textContent "
                        "did not read the text below the element\n");
        return 1;
    }
    if (!xml_data_is_a_dom_tree_in_state(&sm, XML_DATA_IS_A_DOM_TREE_STATE_SETTLED)) {
        fprintf(stderr, "FAIL: the machine reached none of its four verdicts, so the guards did not "
                        "evaluate at all\n");
        return 1;
    }

    printf("PASS: a <data> element's XML is a DOM tree the document walked\n");
    return 0;
}
