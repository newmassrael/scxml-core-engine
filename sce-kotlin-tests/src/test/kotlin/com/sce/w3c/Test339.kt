// GENERATED — DO NOT EDIT (generate_kotlin_w3c.py)
package com.sce.w3c

import com.sce.generated.test339.Test339Event
import com.sce.generated.test339.Test339State
import com.sce.generated.test339.Test339StateMachine
import org.junit.jupiter.api.DisplayName

// W3C SCXML 5.10: If an event is not generated from an invoked child process, the Processor MUST leave the invokeid field blank.
@DisplayName("Test 339 -- W3C SCXML 5.10")
class Test339 : W3CTestBase<Test339State, Test339Event>() {
    override fun createStateMachine() = Test339StateMachine()
    override val expectedPassState: Test339State = Test339State.Pass
}
