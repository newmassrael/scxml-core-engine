// SCE-GENERATED — DO NOT EDIT
// source-hash: 7f418d70ffbf9e34303ec5b694ace0e8ad8138948007a395fa2c2a2380179efc
// GENERATED -- DO NOT EDIT (sce-codegen)
// SCE-MAP: test321.scxml:1
package com.sce.w3c

import com.sce.generated.test321.Test321Event
import com.sce.generated.test321.Test321State
import com.sce.generated.test321.Test321StateMachine
import org.junit.jupiter.api.DisplayName

// W3C SCXML 5.10: The Processor MUST bind the variable _sessionid at load time to the system-generated id for the current SCXML session.
@DisplayName("Test 321 -- W3C SCXML 5.10")
class Test321 : W3CTestBase<Test321State, Test321Event>() {
    override fun createStateMachine() = Test321StateMachine(createEngine())
    override val expectedPassState: Test321State = Test321State.Pass
}
