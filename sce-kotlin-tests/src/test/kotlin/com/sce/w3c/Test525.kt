// GENERATED — DO NOT EDIT (generate_kotlin_w3c.py)
package com.sce.w3c

import com.sce.generated.test525.Test525Event
import com.sce.generated.test525.Test525State
import com.sce.generated.test525.Test525StateMachine
import org.junit.jupiter.api.DisplayName

// W3C SCXML 4.6: The SCXML processor MUST act as if it has made a shallow copy of the collection produced by the evaluation of 'array'. Specifically, modifications to the collection during the execution of foreach MUST NOT affect the iteration behavior.
@DisplayName("Test 525 -- W3C SCXML 4.6")
class Test525 : W3CTestBase<Test525State, Test525Event>() {
    override fun createStateMachine() = Test525StateMachine(createEngine())
    override val expectedPassState: Test525State = Test525State.Pass
}
