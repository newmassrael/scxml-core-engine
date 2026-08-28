// SCE-GENERATED — DO NOT EDIT
// source-hash: b1edd275a200b2f8553040c83495e98b687c11a97259eaf4d60667291dcb916a
// template-hash: 2999a09c910b968e408271dc62f423daf659e11e3dbdea0cdf9857029573f331
// generated-at: 0
// GENERATED -- DO NOT EDIT (sce-codegen)
// SCE-MAP: test337.scxml:1
package com.sce.w3c

import com.sce.generated.test337.Test337Event
import com.sce.generated.test337.Test337State
import com.sce.generated.test337.Test337StateMachine
import org.junit.jupiter.api.DisplayName

// W3C SCXML 5.10: For internal and platform events, the Processor MUST leave the origintype field blank.
@DisplayName("Test 337 -- W3C SCXML 5.10")
class Test337 : W3CTestBase<Test337State, Test337Event>() {
    override fun createStateMachine() = Test337StateMachine()
    override val expectedPassState: Test337State = Test337State.Pass
}
