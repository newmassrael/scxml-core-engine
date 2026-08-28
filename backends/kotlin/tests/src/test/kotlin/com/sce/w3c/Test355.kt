// SCE-GENERATED — DO NOT EDIT
// source-hash: b1edd275a200b2f8553040c83495e98b687c11a97259eaf4d60667291dcb916a
// template-hash: 2999a09c910b968e408271dc62f423daf659e11e3dbdea0cdf9857029573f331
// generated-at: 0
// GENERATED -- DO NOT EDIT (sce-codegen)
// SCE-MAP: test355.scxml:1
package com.sce.w3c

import com.sce.generated.test355.Test355Event
import com.sce.generated.test355.Test355State
import com.sce.generated.test355.Test355StateMachine
import org.junit.jupiter.api.DisplayName

// W3C SCXML 3.2: At system initialization time, if the 'initial' attribute is not present, the Processor MUST enter the first state in document order.
@DisplayName("Test 355 -- W3C SCXML 3.2")
class Test355 : W3CTestBase<Test355State, Test355Event>() {
    override fun createStateMachine() = Test355StateMachine()
    override val expectedPassState: Test355State = Test355State.Pass
}
