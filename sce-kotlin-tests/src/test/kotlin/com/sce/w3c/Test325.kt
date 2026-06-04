// SCE-GENERATED — DO NOT EDIT
// source-hash: f30ff39ee453ff9c2724b237e7ecc70c10c604254c7a79c1bda4dff30c4daac9
// template-hash: 685fb4e0713193a522c8703edbc4c7f9a7c6eb1a29822dc1f9bfa6c38d3bf333
// generated-at: 1780579912
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
