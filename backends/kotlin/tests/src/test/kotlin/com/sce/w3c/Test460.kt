// SCE-GENERATED — DO NOT EDIT
// source-hash: b1edd275a200b2f8553040c83495e98b687c11a97259eaf4d60667291dcb916a
// template-hash: d4e22b42c8fb86f84b0969d1316ae4d81aa27fbb4a4a2d70f49968ff55378fdc
// generated-at: 0
// GENERATED -- DO NOT EDIT (sce-codegen)
// SCE-MAP: test460.scxml:1
package com.sce.w3c

import com.sce.generated.test460.Test460Event
import com.sce.generated.test460.Test460State
import com.sce.generated.test460.Test460StateMachine
import org.junit.jupiter.api.DisplayName

// W3C SCXML B.2: In the ECMAScript data model, since shallow copy is required for the foreach element, foreach assignment is equivalent to item = array_name[index] in ECMAScript.
@DisplayName("Test 460 -- W3C SCXML B.2")
class Test460 : W3CTestBase<Test460State, Test460Event>() {
    override fun createStateMachine() = Test460StateMachine(createEngine())
    override val expectedPassState: Test460State = Test460State.Pass
}
