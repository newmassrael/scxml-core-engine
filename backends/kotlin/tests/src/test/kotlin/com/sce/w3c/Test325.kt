// SCE-GENERATED — DO NOT EDIT
// source-hash: 50977319f11c1ff3aac5be1771f46084e92b202125e3d418050cec95e667f58c
// template-hash: f160b18d725f2c0387242c0463da6808a5b8be392d0dc888f0d564e42c83db17
// generated-at: 1785486331
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
