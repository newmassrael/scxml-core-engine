// SCE-GENERATED — DO NOT EDIT
// source-hash: b1edd275a200b2f8553040c83495e98b687c11a97259eaf4d60667291dcb916a
// template-hash: 1cfb591080ee0f7028d74f99302d8ee6d7a5b2416447e2ddc2e71e093c1a3c98
// generated-at: 0
// GENERATED -- DO NOT EDIT (sce-codegen)
// SCE-MAP: test558.scxml:1
package com.sce.w3c

import com.sce.generated.test558.Test558Event
import com.sce.generated.test558.Test558State
import com.sce.generated.test558.Test558StateMachine
import org.junit.jupiter.api.DisplayName

// W3C SCXML B.2: In the ECMAScript datamodel, if either the 'src' attribute or in-line content is provided in the data element, and the content (whether fetched or provided in-line) is not an XML document or JSON (or the
@DisplayName("Test 558 -- W3C SCXML B.2")
class Test558 : W3CTestBase<Test558State, Test558Event>() {
    override fun createStateMachine() = Test558StateMachine(createEngine())
    override val expectedPassState: Test558State = Test558State.Pass
}
