// SCE-GENERATED — DO NOT EDIT
// source-hash: b1edd275a200b2f8553040c83495e98b687c11a97259eaf4d60667291dcb916a
// template-hash: d4e22b42c8fb86f84b0969d1316ae4d81aa27fbb4a4a2d70f49968ff55378fdc
// generated-at: 0
// GENERATED -- DO NOT EDIT (sce-codegen)
// SCE-MAP: test232.scxml:1
package com.sce.w3c

import com.sce.generated.test232.Test232Event
import com.sce.generated.test232.Test232State
import com.sce.generated.test232.Test232StateMachine
import org.junit.jupiter.api.DisplayName

// W3C SCXML 6.4: he invoked external service MAY return multiple events while it is processing
@DisplayName("Test 232 -- W3C SCXML 6.4")
class Test232 : W3CTestBase<Test232State, Test232Event>() {
    override fun createStateMachine() = Test232StateMachine()
    override val expectedPassState: Test232State = Test232State.Pass
}
