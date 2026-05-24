// SCE-GENERATED — DO NOT EDIT
// source-hash: f30ff39ee453ff9c2724b237e7ecc70c10c604254c7a79c1bda4dff30c4daac9
// template-hash: e58c03089e515b4f87df3e09e89234f06d61979361ed8fef1646aeb0069c2169
// generated-at: 1779596481
// GENERATED -- DO NOT EDIT (sce-codegen)
// SCE-MAP: test501.scxml:1
package com.sce.w3c

import com.sce.generated.test501.Test501Event
import com.sce.generated.test501.Test501State
import com.sce.generated.test501.Test501StateMachine
import org.junit.jupiter.api.DisplayName

// W3C SCXML C.1: The 'location' field inside the entry for the SCXML Event I/O Processor in the _ioprocessors system variable MUST hold an address that external entities can use to communicate with this SCXML session using the SCXML Event I/O Processor.
@DisplayName("Test 501 -- W3C SCXML C.1")
class Test501 : W3CTestBase<Test501State, Test501Event>() {
    override fun createStateMachine() = Test501StateMachine(createEngine())
    override val expectedPassState: Test501State = Test501State.Pass
}
