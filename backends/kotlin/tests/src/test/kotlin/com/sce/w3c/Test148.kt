// SCE-GENERATED — DO NOT EDIT
// source-hash: b1edd275a200b2f8553040c83495e98b687c11a97259eaf4d60667291dcb916a
// template-hash: 419df244c5f8e83941772fe0e162c3decc43983c72d904462cbbb6425fb07338
// generated-at: 0
// GENERATED -- DO NOT EDIT (sce-codegen)
// SCE-MAP: test148.scxml:1
package com.sce.w3c

import com.sce.generated.test148.Test148Event
import com.sce.generated.test148.Test148State
import com.sce.generated.test148.Test148StateMachine
import org.junit.jupiter.api.DisplayName

// W3C SCXML 4.3: When the if element is executed, if no 'cond'attribute evaluates to true, the SCXML Processor must execute the partition defined by the else tag, if there is one.
@DisplayName("Test 148 -- W3C SCXML 4.3")
class Test148 : W3CTestBase<Test148State, Test148Event>() {
    override fun createStateMachine() = Test148StateMachine(createEngine())
    override val expectedPassState: Test148State = Test148State.Pass
}
