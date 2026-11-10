// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
//
// W3C SCXML B.2: the reading of inline `<content>` text is the data model's —
// Rust AOT.
//
// Under `datamodel="ecmascript"` a `<send>`'s `21` is the number 21,
// `{"k": 7}` an object, and text that is no expression the space-normalized
// string, whether or not the sending machine needs a script engine for
// anything else; a `<donedata>`'s XML is a document. The child that sends them
// needs no engine, which is the shape where the template used to take the
// last rung for a send, and handed donedata XML to an engine as an expression.
//
// Each reading lands in its own failure state, so a failure names it.
//
// Fixture: integration_resources/inline_content_is_read_by_its_data_model/inline_content_is_read_by_its_data_model.scxml
// (canonical, shared with every channel).
//
// Regeneration (after fixture or template edit):
//   scripts/materialize.sh rust-integration

use std::time::Duration;

use sce_rust_tests::integration::inline_content_is_read_by_its_data_model::{
    InlineContentIsReadByItsDataModelPolicy, InlineContentIsReadByItsDataModelState,
};

#[test]
fn inline_content_is_the_value_its_data_model_reads() {
    let script_engine: std::sync::Arc<dyn sce_rust_runtime::IScriptEngine> =
        std::sync::Arc::new(sce_rust_lua::LuaEngine::new());
    let policy = InlineContentIsReadByItsDataModelPolicy::new(script_engine);
    let mut engine = sce_rust_runtime::Engine::new(policy);
    engine
        .initialize()
        .expect("the machine's script session is established");

    let completed = engine.run_until_completion(Duration::from_secs(2), Duration::from_millis(10));
    assert!(
        completed,
        "inline_content_is_read_by_its_data_model timed out before reaching a final \
         state: the child's events never reached the parent"
    );

    match engine.terminal_state() {
        Some(InlineContentIsReadByItsDataModelState::Pass) => {}
        Some(InlineContentIsReadByItsDataModelState::FailNumber) => panic!(
            "`number` arrived with `_event.data` unequal to 21: under the ECMAScript data \
             model `<content>21</content>` is the number 21, finished at build time, \
             whether or not the sending machine needs a script engine"
        ),
        Some(InlineContentIsReadByItsDataModelState::FailText) => panic!(
            "`text` arrived with `_event.data` other than 'two words': text that is no \
             expression is the space-normalized string it spells"
        ),
        Some(InlineContentIsReadByItsDataModelState::FailObject) => panic!(
            "`object` arrived without `_event.data.k === 7`: `<content>{{\"k\": 7}}</content>` \
             is an object literal, the object it denotes"
        ),
        Some(InlineContentIsReadByItsDataModelState::FailDocument) => panic!(
            "`done.invoke.reporter` arrived without a document whose `book` carries \
             title=\"x\": a `<donedata>`'s XML `<content>` travels as its text and the \
             receiver builds the DOM; it is not an expression to evaluate"
        ),
        other => panic!(
            "inline_content_is_read_by_its_data_model ended in {other:?}, which is not a \
             verdict state"
        ),
    }
}
