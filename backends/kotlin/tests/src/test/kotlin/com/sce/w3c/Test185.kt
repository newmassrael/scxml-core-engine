// SCE-GENERATED — DO NOT EDIT
// source-hash: b1edd275a200b2f8553040c83495e98b687c11a97259eaf4d60667291dcb916a
// template-hash: 1648c68c7039bcd2d9f4b6a29e08b82f1fcf3cd79ecb3462ff4016858820460c
// generated-at: 0
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
