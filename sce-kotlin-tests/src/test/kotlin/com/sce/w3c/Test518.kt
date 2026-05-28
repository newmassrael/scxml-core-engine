// SCE-GENERATED — DO NOT EDIT
// source-hash: f30ff39ee453ff9c2724b237e7ecc70c10c604254c7a79c1bda4dff30c4daac9
// template-hash: d578a9cfec09708cd26393ca0d01ceccd7a2c1ee3a13c2911d4850d61b99f2ce
// generated-at: 1779985213
// GENERATED -- DO NOT EDIT (sce-codegen)
// SCE-MAP: test518.scxml:1
package com.sce.w3c

import com.sce.generated.test518.Test518Event
import com.sce.generated.test518.Test518State
import com.sce.generated.test518.Test518StateMachine
import org.junit.jupiter.api.DisplayName

// W3C SCXML C.2: If the namelist attribute is defined [in send], the SCXML Processor MUST map its variable names and values to HTTP POST parameters
@DisplayName("Test 518 -- W3C SCXML C.2")
class Test518 : W3CHttpTestBase<Test518State, Test518Event>() {
    override fun createStateMachine() = Test518StateMachine(createEngine())
    override val expectedPassState: Test518State = Test518State.Pass
}
