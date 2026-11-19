// SCE-GENERATED — DO NOT EDIT
// source-hash: 26d93a3b07bca6a5bf5e15616de6f8ea4385ee35e934365ba7240b9c941ce42b
// source: test235.scxml
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
