// SCE-GENERATED — DO NOT EDIT
// source-hash: b1edd275a200b2f8553040c83495e98b687c11a97259eaf4d60667291dcb916a
// template-hash: 42298195b20865d87e273e6a89fd9b7e20af26d02f54273007f21322d047b5d4
// generated-at: 0
// GENERATED -- DO NOT EDIT (sce-codegen)
// SCE-MAP: test448.scxml:1
package com.sce.w3c

import com.sce.generated.test448.Test448Event
import com.sce.generated.test448.Test448State
import com.sce.generated.test448.Test448StateMachine
import org.junit.jupiter.api.DisplayName

// W3C SCXML B.2: In the ECMAScript datamodel, the SCXML Processor must place all variables in a single global ECMAScript scope.
@DisplayName("Test 448 -- W3C SCXML B.2")
class Test448 : W3CTestBase<Test448State, Test448Event>() {
    override fun createStateMachine() = Test448StateMachine(createEngine())
    override val expectedPassState: Test448State = Test448State.Pass
}
