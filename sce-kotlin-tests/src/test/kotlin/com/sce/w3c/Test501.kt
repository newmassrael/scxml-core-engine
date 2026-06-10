// SCE-GENERATED — DO NOT EDIT
// source-hash: f30ff39ee453ff9c2724b237e7ecc70c10c604254c7a79c1bda4dff30c4daac9
// template-hash: f835a323a3abc9cebc80341e1840b22b95739a2efa1726ad2c440477eff36482
// generated-at: 1781089257
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
