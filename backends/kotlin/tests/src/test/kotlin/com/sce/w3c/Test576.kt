// SCE-GENERATED — DO NOT EDIT
// source-hash: b1edd275a200b2f8553040c83495e98b687c11a97259eaf4d60667291dcb916a
// template-hash: 90ac0b7250dd34a7e14136bc481cc93d6f1302dcf207c461738cfaee4b475c98
// generated-at: 0
// GENERATED -- DO NOT EDIT (sce-codegen)
// SCE-MAP: test576.scxml:1
package com.sce.w3c

import com.sce.generated.test576.Test576Event
import com.sce.generated.test576.Test576State
import com.sce.generated.test576.Test576StateMachine
import org.junit.jupiter.api.DisplayName

// W3C SCXML 3.2: At system initialization time, the SCXML Processor MUST enter the states specified by the 'initial' attribute, if it is present.
@DisplayName("Test 576 -- W3C SCXML 3.2")
class Test576 : W3CTestBase<Test576State, Test576Event>() {
    override fun createStateMachine() = Test576StateMachine()
    override val expectedPassState: Test576State = Test576State.Pass
}
