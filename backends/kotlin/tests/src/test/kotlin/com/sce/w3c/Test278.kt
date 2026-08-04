// SCE-GENERATED — DO NOT EDIT
// source-hash: b1edd275a200b2f8553040c83495e98b687c11a97259eaf4d60667291dcb916a
// template-hash: bf9f012bd8272e352f46f4d8064cf0cf3b743ab6fffdf8c941cc03f3254cb15f
// generated-at: 0
// GENERATED -- DO NOT EDIT (sce-codegen)
// SCE-MAP: test278.scxml:1
package com.sce.w3c

import com.sce.generated.test278.Test278Event
import com.sce.generated.test278.Test278State
import com.sce.generated.test278.Test278StateMachine
import org.junit.jupiter.api.DisplayName

// W3C SCXML B.2: In the ECMAScript data model, the SCXML processor MUST allow any data element to be accessed from any state.
@DisplayName("Test 278 -- W3C SCXML B.2")
class Test278 : W3CTestBase<Test278State, Test278Event>() {
    override fun createStateMachine() = Test278StateMachine(createEngine())
    override val expectedPassState: Test278State = Test278State.Pass
}
