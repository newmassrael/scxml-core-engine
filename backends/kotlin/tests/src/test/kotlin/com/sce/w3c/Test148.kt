// SCE-GENERATED — DO NOT EDIT
// source-hash: b1edd275a200b2f8553040c83495e98b687c11a97259eaf4d60667291dcb916a
// template-hash: d4e22b42c8fb86f84b0969d1316ae4d81aa27fbb4a4a2d70f49968ff55378fdc
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
