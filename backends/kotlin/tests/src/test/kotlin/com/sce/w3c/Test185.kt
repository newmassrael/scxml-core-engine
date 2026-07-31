// SCE-GENERATED — DO NOT EDIT
// source-hash: 50977319f11c1ff3aac5be1771f46084e92b202125e3d418050cec95e667f58c
// template-hash: 140c4d555915ab51dfdb5b562572972b7994e3bced0046a696f7c65e279b5a12
// generated-at: 1785462747
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
