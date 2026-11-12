// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
//
// W3C SCXML B.2: the reading of inline `<content>` text is the data model's —
// C11 AOT.
//
// Under `datamodel="ecmascript"` a `<send>`'s `21` is the number 21,
// `{"k": 7}` an object, and text that is no expression the space-normalized
// string, whether or not the sending machine needs a script engine for
// anything else; a `<donedata>`'s XML is a document. The child that sends them
// needs no engine, and this template used to hand its text to a Lua state
// such a machine is not given.
//
// Fixture:
// integration_resources/inline_content_is_read_by_its_data_model/inline_content_is_read_by_its_data_model.scxml
// (canonical, shared with every channel).
//
// Regeneration: automatic at CMake build time via
// `sce_generate_static_integration_c_test(inline_content_is_read_by_its_data_model ...)`
// in `backends/c/tests/CMakeLists.txt`.

#include <stdint.h>
#include <stdio.h>

#include "../require_started.h"
#include "inline_content_is_read_by_its_data_model_sm.h"

int main(void) {
    inline_content_is_read_by_its_data_model_t sm;
    REQUIRE_STARTED(inline_content_is_read_by_its_data_model_init(&sm),
                    INLINE_CONTENT_IS_READ_BY_ITS_DATA_MODEL_START_OK);

    // No `<send delay>` here: the child sends during its own `_init`.
    inline_content_is_read_by_its_data_model_run(&sm);

    const int pass =
        inline_content_is_read_by_its_data_model_ended_in(&sm, INLINE_CONTENT_IS_READ_BY_ITS_DATA_MODEL_STATE_PASS);
    const int fail_number = inline_content_is_read_by_its_data_model_ended_in(
        &sm, INLINE_CONTENT_IS_READ_BY_ITS_DATA_MODEL_STATE_FAILNUMBER);
    const int fail_text =
        inline_content_is_read_by_its_data_model_ended_in(&sm, INLINE_CONTENT_IS_READ_BY_ITS_DATA_MODEL_STATE_FAILTEXT);
    const int fail_object = inline_content_is_read_by_its_data_model_ended_in(
        &sm, INLINE_CONTENT_IS_READ_BY_ITS_DATA_MODEL_STATE_FAILOBJECT);
    const int fail_document = inline_content_is_read_by_its_data_model_ended_in(
        &sm, INLINE_CONTENT_IS_READ_BY_ITS_DATA_MODEL_STATE_FAILDOCUMENT);

    if (!pass) {
        if (fail_number) {
            fprintf(stderr, "inline_content_is_read_by_its_data_model: FAIL: `number` arrived "
                            "with `_event.data` unequal to 21. Under the ECMAScript data model "
                            "`<content>21</content>` is the number 21, finished at build time, "
                            "whether or not the sending machine needs a script engine.\n");
        } else if (fail_text) {
            fprintf(stderr, "inline_content_is_read_by_its_data_model: FAIL: `text` arrived "
                            "with `_event.data` other than 'two words'. Text that is no "
                            "expression is the space-normalized string it spells.\n");
        } else if (fail_object) {
            fprintf(stderr, "inline_content_is_read_by_its_data_model: FAIL: `object` arrived "
                            "without `_event.data.k === 7`. `<content>{\"k\": 7}</content>` is an "
                            "object literal, the object it denotes.\n");
        } else if (fail_document) {
            fprintf(stderr, "inline_content_is_read_by_its_data_model: FAIL: `done.invoke.reporter` "
                            "arrived without a document whose `book` carries title=\"x\". A "
                            "`<donedata>`'s XML `<content>` travels as its text and the receiver "
                            "builds the DOM; it is not an expression to evaluate.\n");
        } else {
            fprintf(stderr, "inline_content_is_read_by_its_data_model: FAIL: settled in no "
                            "verdict state: the child's events never reached the parent.\n");
        }
    }

    inline_content_is_read_by_its_data_model_destroy(&sm);
    return pass ? 0 : 1;
}
