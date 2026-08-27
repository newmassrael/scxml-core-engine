// SCE-GENERATED — DO NOT EDIT
// source-hash: b1edd275a200b2f8553040c83495e98b687c11a97259eaf4d60667291dcb916a
// template-hash: 42298195b20865d87e273e6a89fd9b7e20af26d02f54273007f21322d047b5d4
// generated-at: 0
// GENERATED -- DO NOT EDIT (sce-codegen)
// SCE-MAP: test325.scxml:1
package com.sce.w3c

import com.sce.generated.test325.Test325Event
import com.sce.generated.test325.Test325State
import com.sce.generated.test325.Test325StateMachine
import org.junit.jupiter.api.DisplayName

// W3C SCXML 5.10: The Processor MUST bind the variable _ioprocessors to a set of values, one for each Event I/O Processor that it supports.
@DisplayName("Test 325 -- W3C SCXML 5.10")
class Test325 : W3CTestBase<Test325State, Test325Event>() {
    override fun createStateMachine() = Test325StateMachine(createEngine())
    override val expectedPassState: Test325State = Test325State.Pass
}
