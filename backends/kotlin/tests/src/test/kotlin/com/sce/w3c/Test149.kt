// SCE-GENERATED — DO NOT EDIT
// source-hash: b1edd275a200b2f8553040c83495e98b687c11a97259eaf4d60667291dcb916a
// template-hash: 8c07c8492e1d3772f35a6f68b5368478c96a8b79da5206f3742243a191d8e3b5
// generated-at: 0
// GENERATED -- DO NOT EDIT (sce-codegen)
// SCE-MAP: test149.scxml:1
package com.sce.w3c

import com.sce.generated.test149.Test149Event
import com.sce.generated.test149.Test149State
import com.sce.generated.test149.Test149StateMachine
import org.junit.jupiter.api.DisplayName

// W3C SCXML 4.3: When it executes an if element, if no 'cond' attribute evaluates to true and there is no else element, the SCXML processor must not evaluate any executable content within the element.
@DisplayName("Test 149 -- W3C SCXML 4.3")
class Test149 : W3CTestBase<Test149State, Test149Event>() {
    override fun createStateMachine() = Test149StateMachine(createEngine())
    override val expectedPassState: Test149State = Test149State.Pass
}
