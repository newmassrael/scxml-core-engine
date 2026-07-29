// SCE-GENERATED — DO NOT EDIT
// source-hash: f30ff39ee453ff9c2724b237e7ecc70c10c604254c7a79c1bda4dff30c4daac9
// template-hash: aa58405544015ba4d1b8207b13e783fe4f4b991c1d05b4cc1602d85ec7348310
// generated-at: 1785367096
// GENERATED -- DO NOT EDIT (sce-codegen)
// SCE-MAP: test244.scxml:1
package com.sce.w3c

import com.sce.generated.test244.Test244Event
import com.sce.generated.test244.Test244State
import com.sce.generated.test244.Test244StateMachine
import org.junit.jupiter.api.DisplayName

// W3C SCXML 6.4: If the invoked process is of type http://www.w3.org/TR/scxml/ and the key of namelist item in the invoke matches the 'id' of a data element in the top-level data declarations of the invoked session, the SCXML Processor MUST use the corresponding value as the initial value of the corresponding data element.
@DisplayName("Test 244 -- W3C SCXML 6.4")
class Test244 : W3CTestBase<Test244State, Test244Event>() {
    override fun createStateMachine() = Test244StateMachine(createEngine())
    override val expectedPassState: Test244State = Test244State.Pass
}
