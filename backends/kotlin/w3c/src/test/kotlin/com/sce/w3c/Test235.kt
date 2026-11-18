// SCE-GENERATED — DO NOT EDIT
// source-hash: 7e40b95c2d842dcf564705fa3efcf7ff2175df2e7887977f32be9e362c2a3f25
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
