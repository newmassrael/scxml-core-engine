// SCE-GENERATED — DO NOT EDIT
// source-hash: b1edd275a200b2f8553040c83495e98b687c11a97259eaf4d60667291dcb916a
// template-hash: d4e22b42c8fb86f84b0969d1316ae4d81aa27fbb4a4a2d70f49968ff55378fdc
// generated-at: 0
// GENERATED -- DO NOT EDIT (sce-codegen)
// SCE-MAP: test551.scxml:1
package com.sce.w3c

import com.sce.generated.test551.Test551Event
import com.sce.generated.test551.Test551State
import com.sce.generated.test551.Test551StateMachine
import org.junit.jupiter.api.DisplayName

// W3C SCXML 5.3: f child content is specified, the Platform MUST assign it as the value of the data element at the time specified by the 'binding' attribute of scxml.
@DisplayName("Test 551 -- W3C SCXML 5.3")
class Test551 : W3CTestBase<Test551State, Test551Event>() {
    override fun createStateMachine() = Test551StateMachine(createEngine())
    override val expectedPassState: Test551State = Test551State.Pass
}
