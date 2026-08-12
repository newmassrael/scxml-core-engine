// SCE-GENERATED — DO NOT EDIT
// source-hash: b1edd275a200b2f8553040c83495e98b687c11a97259eaf4d60667291dcb916a
// template-hash: 74ba562b33766da248288b5dadec1e79a0ebb46a66e38786f6a7a4b2ccd653e3
// generated-at: 0
// GENERATED -- DO NOT EDIT (sce-codegen)
// SCE-MAP: test402.scxml:1
package com.sce.w3c

import com.sce.generated.test402.Test402Event
import com.sce.generated.test402.Test402State
import com.sce.generated.test402.Test402StateMachine
import org.junit.jupiter.api.DisplayName

// W3C SCXML 3.12: The processor MUST process them [error events] like any other event.
@DisplayName("Test 402 -- W3C SCXML 3.12")
class Test402 : W3CTestBase<Test402State, Test402Event>() {
    override fun createStateMachine() = Test402StateMachine(createEngine())
    override val expectedPassState: Test402State = Test402State.Pass
}
