// SCE-GENERATED — DO NOT EDIT
// source-hash: b1edd275a200b2f8553040c83495e98b687c11a97259eaf4d60667291dcb916a
// template-hash: 80cbaef5e101fcdca529f6185da2e031307f67f6aef9e4f014a25f5e4c08bb8b
// generated-at: 0
// GENERATED -- DO NOT EDIT (sce-codegen)
// SCE-MAP: test449.scxml:1
package com.sce.w3c

import com.sce.generated.test449.Test449Event
import com.sce.generated.test449.Test449State
import com.sce.generated.test449.Test449StateMachine
import org.junit.jupiter.api.DisplayName

// W3C SCXML B.2: In the ECMAScript datamodel, the SCXML Processor must convert ECMAScript expressions used in conditional expressions into their effective boolean value using the ToBoolean operator as described in Section 9.2 of [ECMASCRIPT-262].
@DisplayName("Test 449 -- W3C SCXML B.2")
class Test449 : W3CTestBase<Test449State, Test449Event>() {
    override fun createStateMachine() = Test449StateMachine()
    override val expectedPassState: Test449State = Test449State.Pass
}
