// SCE-GENERATED — DO NOT EDIT
// source-hash: f30ff39ee453ff9c2724b237e7ecc70c10c604254c7a79c1bda4dff30c4daac9
// template-hash: bc7b5b1dd90f65e6c3a4df2e3c4223cf8922d7e6b2d5d124b66683d16074cb6e
// generated-at: 1780362263
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
