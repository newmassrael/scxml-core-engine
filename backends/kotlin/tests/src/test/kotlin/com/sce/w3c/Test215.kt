// SCE-GENERATED — DO NOT EDIT
// source-hash: b1edd275a200b2f8553040c83495e98b687c11a97259eaf4d60667291dcb916a
// template-hash: f8935a2b1ceca80a03ff3489cc9f8dcbccd8c2b85fc58c3b848403d6a2672153
// generated-at: 0
// GENERATED -- DO NOT EDIT (sce-codegen)
// SCE-MAP: test215.scxml:1
package com.sce.w3c

import com.sce.generated.test215.Test215Event
import com.sce.generated.test215.Test215State
import com.sce.generated.test215.Test215StateMachine
import org.junit.jupiter.api.DisplayName

// W3C SCXML 6.4: If the typeexpr attribute is present, the SCXML Processor MUST evaluate it when the parent invoke element is evaluated and treat the result as if it had been entered as the value of 'type'.
@DisplayName("Test 215 -- W3C SCXML 6.4")
class Test215 : W3CTestBase<Test215State, Test215Event>() {
    override fun createStateMachine() = Test215StateMachine(createEngine())
    override val expectedPassState: Test215State = Test215State.Pass
}
