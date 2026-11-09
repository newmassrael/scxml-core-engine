// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
//
// W3C SCXML B.2: the reading of inline `<content>` text is the data model's —
// C++ AOT.
//
// Under `datamodel="ecmascript"` a `<send>`'s `21` is the number 21,
// `{"k": 7}` an object, and text that is no expression the space-normalized
// string, whether or not the sending machine needs a script engine for
// anything else; a `<donedata>`'s XML is a document. The child that sends them
// needs no engine.
//
// Fixture:
// integration_resources/inline_content_is_read_by_its_data_model/inline_content_is_read_by_its_data_model.scxml
// (canonical, shared with every channel).
//
// Regeneration: automatic at CMake build time via
// `sce_generate_static_integration_test(inline_content_is_read_by_its_data_model ...)`.

#include "inline_content_is_read_by_its_data_model_sm.h"
#include "scripting/ScriptEngineProvider.h"

#include <chrono>
#include <gtest/gtest.h>
#include <memory>

namespace SCE::Tests {

TEST(InlineContentIsReadByItsDataModelAotTest, InlineContentIsTheValueItsDataModelReads) {
    using SM = SCE::Generated::inline_content_is_read_by_its_data_model::inline_content_is_read_by_its_data_model;

    SM sm;
    if constexpr (SM::PolicyType::NEEDS_SCRIPT_ENGINE) {
        // Non-owning view of the ScriptEngineProvider singleton, as the
        // sibling AOT drivers hold it.
        sm.setScriptEngine(std::shared_ptr<::SCE::IScriptEngine>(&::SCE::ScriptEngineProvider::getScriptEngine(),
                                                                 [](::SCE::IScriptEngine *) {}));
    }

    EXPECT_EQ(sm.initialize(), std::nullopt);
    const bool reachedFinal = sm.runUntilCompletion(std::chrono::seconds(3));

    EXPECT_TRUE(reachedFinal) << "the parent did not reach a final state within the timeout: the child's "
                                 "events never reached it";
    EXPECT_NE(sm.terminalState(), SM::State::FailNumber)
        << "`number` arrived with `_event.data` unequal to 21: under the ECMAScript data model "
           "`<content>21</content>` is the number 21, finished at build time, whether or not the sending "
           "machine needs a script engine.";
    EXPECT_NE(sm.terminalState(), SM::State::FailText)
        << "`text` arrived with `_event.data` other than 'two words': text that is no expression is the "
           "space-normalized string it spells.";
    EXPECT_NE(sm.terminalState(), SM::State::FailObject)
        << "`object` arrived without `_event.data.k === 7`: `<content>{\"k\": 7}</content>` is an object "
           "literal, the object it denotes.";
    EXPECT_NE(sm.terminalState(), SM::State::FailDocument)
        << "`done.invoke.reporter` arrived without a document whose `book` carries title=\"x\": a "
           "`<donedata>`'s XML `<content>` travels as its text and the receiver builds the DOM; it is not "
           "an expression to evaluate.";
    EXPECT_EQ(sm.terminalState(), SM::State::Pass);
}

}  // namespace SCE::Tests
