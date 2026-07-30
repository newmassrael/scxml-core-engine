// SCE-GENERATED — DO NOT EDIT
// source-hash: f30ff39ee453ff9c2724b237e7ecc70c10c604254c7a79c1bda4dff30c4daac9
// template-hash: 82d5a5b31a2776e65c97ff666726e5d471238b15131eddc7520023d807e91b34
// generated-at: 1785371281
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
