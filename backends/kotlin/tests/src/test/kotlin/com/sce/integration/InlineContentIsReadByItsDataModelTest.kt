// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
//
// W3C SCXML B.2: the reading of inline `<content>` text is the data model's —
// Kotlin AOT.
//
// Under `datamodel="ecmascript"` a `<send>`'s `21` is the number 21,
// `{"k": 7}` an object, and text that is no expression the space-normalized
// string, whether or not the sending machine needs a script engine for
// anything else; a `<donedata>`'s XML is a document. The child that sends them
// needs no engine, and this template used to hand its text to an engine such
// a machine is not given.
//
// Fixture: integration_resources/inline_content_is_read_by_its_data_model/inline_content_is_read_by_its_data_model.scxml
// (canonical, shared with every channel).
//
// Regeneration (after fixture or template edit):
//   scripts/materialize.sh kotlin-integration

package com.sce.integration

import com.sce.integration.inline_content_is_read_by_its_data_model.InlineContentIsReadByItsDataModelState
import com.sce.integration.inline_content_is_read_by_its_data_model.InlineContentIsReadByItsDataModelStateMachine
import com.sce.w3c.W3CTestBase
import com.sce.w3c.driveOnVirtualTime
import com.sce.w3c.initializeOnVirtualTime
import org.junit.jupiter.api.Assertions.assertEquals
import org.junit.jupiter.api.DisplayName
import org.junit.jupiter.api.Test

/// W3C SCXML B.2 — inline `<content>` is the value its data model reads (Kotlin AOT).
@DisplayName("InlineContentIsReadByItsDataModel — W3C SCXML B.2")
class InlineContentIsReadByItsDataModelTest {

    @Test
    fun inlineContentIsTheValueItsDataModelReads() {
        val sm = InlineContentIsReadByItsDataModelStateMachine(W3CTestBase.createEngine())
        sm.initializeOnVirtualTime()
        sm.driveOnVirtualTime(2000L)

        val reached = sm.terminalState
        val why = when (reached) {
            InlineContentIsReadByItsDataModelState.FailNumber ->
                "`number` arrived with `_event.data` unequal to 21: under the ECMAScript data " +
                    "model `<content>21</content>` is the number 21, finished at build time, " +
                    "whether or not the sending machine needs a script engine."
            InlineContentIsReadByItsDataModelState.FailText ->
                "`text` arrived with `_event.data` other than 'two words': text that is no " +
                    "expression is the space-normalized string it spells."
            InlineContentIsReadByItsDataModelState.FailObject ->
                "`object` arrived without `_event.data.k === 7`: `<content>{\"k\": 7}</content>` " +
                    "is an object literal, the object it denotes."
            InlineContentIsReadByItsDataModelState.FailDocument ->
                "`done.invoke.reporter` arrived without a document whose `book` carries " +
                    "title=\"x\": a `<donedata>`'s XML `<content>` travels as its text and the " +
                    "receiver builds the DOM; it is not an expression to evaluate."
            InlineContentIsReadByItsDataModelState.Pass -> ""
            else ->
                "settled in ${sm.currentState.value}, which is not a verdict state: the " +
                    "child's events never reached the parent."
        }

        assertEquals(InlineContentIsReadByItsDataModelState.Pass, reached, why)
    }
}
