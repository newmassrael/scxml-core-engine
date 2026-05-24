// SCE-GENERATED — DO NOT EDIT
// source-hash: f30ff39ee453ff9c2724b237e7ecc70c10c604254c7a79c1bda4dff30c4daac9
// template-hash: e58c03089e515b4f87df3e09e89234f06d61979361ed8fef1646aeb0069c2169
// generated-at: 1779596481
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
