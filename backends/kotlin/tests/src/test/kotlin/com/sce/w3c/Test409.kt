// SCE-GENERATED — DO NOT EDIT
// source-hash: c004aa6d76072d31e9273eed0dc88dc847e351ce7e35f90cc9d56908f01328bd
// GENERATED -- DO NOT EDIT (sce-codegen)
// SCE-MAP: test409.scxml:1
package com.sce.w3c

import com.sce.generated.test409.Test409Event
import com.sce.generated.test409.Test409State
import com.sce.generated.test409.Test409StateMachine
import org.junit.jupiter.api.DisplayName

// W3C SCXML 3.13: Finally [after the onexits and canceling the invocations], the Processor MUST remove the state from the active state's list.
@DisplayName("Test 409 -- W3C SCXML 3.13")
class Test409 : W3CTestBase<Test409State, Test409Event>() {
    override fun createStateMachine() = Test409StateMachine()
    override val expectedPassState: Test409State = Test409State.Pass
    override val timeoutMs: Long = 5000L
}
