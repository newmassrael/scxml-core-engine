// SCE-GENERATED — DO NOT EDIT
// source-hash: f30ff39ee453ff9c2724b237e7ecc70c10c604254c7a79c1bda4dff30c4daac9
// template-hash: 2337021aa5cf9b8209b5932f23ab0e04a6899271e435f3620bc1da41d7c4d7b7
// generated-at: 1784381545
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
