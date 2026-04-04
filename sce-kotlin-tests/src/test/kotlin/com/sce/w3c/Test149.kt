// GENERATED — DO NOT EDIT (generate_kotlin_w3c.py)
package com.sce.w3c

import com.sce.generated.test149.Test149Event
import com.sce.generated.test149.Test149State
import com.sce.generated.test149.Test149StateMachine
import org.junit.jupiter.api.DisplayName

// W3C SCXML 4.3: When it executes an if element, if no 'cond' attribute evaluates to true and there is no else element, the SCXML processor must not evaluate any executable content within the element.
@DisplayName("Test 149 -- W3C SCXML 4.3")
class Test149 : W3CTestBase<Test149State, Test149Event>() {
    override fun createStateMachine() = Test149StateMachine(createEngine())
    override val expectedPassState: Test149State = Test149State.Pass
}
