// SCE-GENERATED — DO NOT EDIT
// source-hash: c004aa6d76072d31e9273eed0dc88dc847e351ce7e35f90cc9d56908f01328bd
// GENERATED -- DO NOT EDIT (sce-codegen)
// SCE-MAP: test175.scxml:1
package com.sce.w3c

import com.sce.generated.test175.Test175Event
import com.sce.generated.test175.Test175State
import com.sce.generated.test175.Test175StateMachine
import org.junit.jupiter.api.DisplayName

// W3C SCXML 6.2: If 'delayexpr' is present, the SCXML Processor MUST evaluate it when the parent send element is evaluated and treat the result as if it had been entered as the value of 'delay'.
@DisplayName("Test 175 -- W3C SCXML 6.2")
class Test175 : W3CTestBase<Test175State, Test175Event>() {
    override fun createStateMachine() = Test175StateMachine(createEngine())
    override val expectedPassState: Test175State = Test175State.Pass
    override val timeoutMs: Long = 5000L
}
