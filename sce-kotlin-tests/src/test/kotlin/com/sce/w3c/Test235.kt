// SCE-GENERATED — DO NOT EDIT
// source-hash: f30ff39ee453ff9c2724b237e7ecc70c10c604254c7a79c1bda4dff30c4daac9
// template-hash: 09c66e0a06202a6ec53b4591ac58670a6615a699910ff161304360792e1e7915
// generated-at: 1780732004
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
