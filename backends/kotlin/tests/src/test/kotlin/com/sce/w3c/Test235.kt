// SCE-GENERATED — DO NOT EDIT
// source-hash: b1edd275a200b2f8553040c83495e98b687c11a97259eaf4d60667291dcb916a
// template-hash: d4e22b42c8fb86f84b0969d1316ae4d81aa27fbb4a4a2d70f49968ff55378fdc
// generated-at: 0
// GENERATED -- DO NOT EDIT (sce-codegen)
// SCE-MAP: test235.scxml:1
package com.sce.w3c

import com.sce.generated.test235.Test235Event
import com.sce.generated.test235.Test235State
import com.sce.generated.test235.Test235StateMachine
import org.junit.jupiter.api.DisplayName

// W3C SCXML 6.4: Once the invoked external service has finished processing it MUST return a special event 'done.invoke.id' to the external event queue of the invoking process, where id is the invokeid for the corresponding invoke element.
@DisplayName("Test 235 -- W3C SCXML 6.4")
class Test235 : W3CTestBase<Test235State, Test235Event>() {
    override fun createStateMachine() = Test235StateMachine()
    override val expectedPassState: Test235State = Test235State.Pass
}
