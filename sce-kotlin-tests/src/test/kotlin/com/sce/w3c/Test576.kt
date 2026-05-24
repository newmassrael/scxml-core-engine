// SCE-GENERATED — DO NOT EDIT
// source-hash: f30ff39ee453ff9c2724b237e7ecc70c10c604254c7a79c1bda4dff30c4daac9
// template-hash: e58c03089e515b4f87df3e09e89234f06d61979361ed8fef1646aeb0069c2169
// generated-at: 1779596481
// GENERATED -- DO NOT EDIT (sce-codegen)
// SCE-MAP: test576.scxml:1
package com.sce.w3c

import com.sce.generated.test576.Test576Event
import com.sce.generated.test576.Test576State
import com.sce.generated.test576.Test576StateMachine
import org.junit.jupiter.api.DisplayName

// W3C SCXML 3.2: At system initialization time, the SCXML Processor MUST enter the states specified by the 'initial' attribute, if it is present.
@DisplayName("Test 576 -- W3C SCXML 3.2")
class Test576 : W3CTestBase<Test576State, Test576Event>() {
    override fun createStateMachine() = Test576StateMachine()
    override val expectedPassState: Test576State = Test576State.Pass
}
