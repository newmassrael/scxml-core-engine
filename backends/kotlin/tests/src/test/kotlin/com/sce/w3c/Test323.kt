// SCE-GENERATED — DO NOT EDIT
// source-hash: b1edd275a200b2f8553040c83495e98b687c11a97259eaf4d60667291dcb916a
// template-hash: 90a17b01a07fa6e2db248839e7823280de5374963642bfa559a2f033a153c586
// generated-at: 0
// GENERATED -- DO NOT EDIT (sce-codegen)
// SCE-MAP: test323.scxml:1
package com.sce.w3c

import com.sce.generated.test323.Test323Event
import com.sce.generated.test323.Test323State
import com.sce.generated.test323.Test323StateMachine
import org.junit.jupiter.api.DisplayName

// W3C SCXML 5.10: The Processor MUST bind the variable _name at load time to the value of the 'name' attribute of the scxml element. 	a
@DisplayName("Test 323 -- W3C SCXML 5.10")
class Test323 : W3CTestBase<Test323State, Test323Event>() {
    override fun createStateMachine() = Test323StateMachine(createEngine())
    override val expectedPassState: Test323State = Test323State.Pass
}
