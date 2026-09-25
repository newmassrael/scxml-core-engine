// SPDX-License-Identifier: LGPL-2.1-or-later WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
//
// W3C SCXML 5.3: typed readers of `<data>` ids that are keywords or no
// identifier in some backend. C++ AOT.
//
// The C++ reader keeps the author's spelling, and used to keep all of it:
// `<data id="screen-rules">` was emitted as a member named `screen-rules()`,
// and `<data id="auto">` as `auto()`. `sce-build/src/reader_names.rs` folds the
// XML-name delimiters to `_` and gives no reader to an id C++ has no escape for
// — nor to any id another backend cannot spell (`self`, `new`, `start`, `t`,
// `a-b` beside `a_b`), since a reader's existence does not depend on the
// backend. This file compiling is what shows none of them was emitted.
//
// Fixture: integration_resources/typed_reader_names/typed_reader_names.scxml
// (canonical, shared with the C11 / Go / Kotlin / Python / Rust channels).

#include "scripting/ScriptEngineProvider.h"
#include "typed_reader_names_sm.h"

#include <gtest/gtest.h>
#include <memory>
#include <optional>

namespace SCE::Tests {
namespace {

using SM = SCE::Generated::typed_reader_names::typed_reader_names;

std::unique_ptr<SM> started() {
    auto sm = std::make_unique<SM>();
    sm->setScriptEngine(std::shared_ptr<::SCE::IScriptEngine>(&::SCE::ScriptEngineProvider::getScriptEngine(),
                                                              [](::SCE::IScriptEngine *) {}));
    sm->initialize();
    return sm;
}

}  // namespace

// Each reader reads the value its own variable was declared with.
TEST(TypedReaderNamesAotTest, AReaderReadsItsOwnVariable) {
    auto sm = started();
    const auto &p = sm->getPolicy();
    EXPECT_EQ(p.box(), std::optional<int64_t>(1));
    EXPECT_EQ(p.object(), std::optional<int64_t>(2));
    EXPECT_EQ(p.pass(), std::optional<int64_t>(3));
    EXPECT_EQ(p.screen_rules(), std::optional<int64_t>(4));
    EXPECT_EQ(p.a_b(), std::optional<int64_t>(10));
}

// And the value it holds now: `bump` adds 10 to four of them, and leaves
// `screen-rules` — which no ECMAScript expression can name — alone. Read
// through the engine's forwarders, the other place a reader is emitted.
TEST(TypedReaderNamesAotTest, AReaderReadsTheLiveValue) {
    auto sm = started();
    sm->processEvent(SM::Event::Bump);
    EXPECT_EQ(sm->box(), std::optional<int64_t>(11));
    EXPECT_EQ(sm->object(), std::optional<int64_t>(12));
    EXPECT_EQ(sm->pass(), std::optional<int64_t>(13));
    EXPECT_EQ(sm->screen_rules(), std::optional<int64_t>(4));
    EXPECT_EQ(sm->a_b(), std::optional<int64_t>(20));
}

}  // namespace SCE::Tests
