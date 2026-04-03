// GENERATED — DO NOT EDIT (generate_kotlin_w3c.py)
package com.sce.w3c

import com.sce.generated.test187.Test187Event
import com.sce.generated.test187.Test187State
import com.sce.generated.test187.Test187StateMachine
import org.junit.jupiter.api.DisplayName

// W3C SCXML 6.2: If the SCXML session terminates before the delay interval has elapsed, the SCXML Processor MUST discard the message without attempting to deliver it.
@DisplayName("Test 187 -- W3C SCXML 6.2")
class Test187 : W3CTestBase<Test187State, Test187Event>() {
    override fun createStateMachine() = Test187StateMachine()
    override val expectedPassState: Test187State = Test187State.Pass
    override val timeoutMs: Long = 5000L
}
