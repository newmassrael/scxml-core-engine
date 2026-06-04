// SCE-GENERATED — DO NOT EDIT
// source-hash: f30ff39ee453ff9c2724b237e7ecc70c10c604254c7a79c1bda4dff30c4daac9
// template-hash: 7b1a0066fa6a7fefceddfcf4d1e81b9d1fe50e95dd2b02645dfe86a65f3b96fe
// generated-at: 1780606838
// GENERATED -- DO NOT EDIT (sce-codegen)
// SCE-MAP: test185.scxml:1
package com.sce.w3c

import com.sce.generated.test185.Test185Event
import com.sce.generated.test185.Test185State
import com.sce.generated.test185.Test185StateMachine
import org.junit.jupiter.api.DisplayName

// W3C SCXML 6.2: If a delay is specified via 'delay' or 'delayexpr', the SCXML Processor MUST interpret the character string as a time interval.
@DisplayName("Test 185 -- W3C SCXML 6.2")
class Test185 : W3CTestBase<Test185State, Test185Event>() {
    override fun createStateMachine() = Test185StateMachine()
    override val expectedPassState: Test185State = Test185State.Pass
    override val timeoutMs: Long = 5000L
}
