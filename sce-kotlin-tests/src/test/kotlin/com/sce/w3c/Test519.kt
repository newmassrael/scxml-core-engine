// SCE-GENERATED — DO NOT EDIT
// source-hash: f30ff39ee453ff9c2724b237e7ecc70c10c604254c7a79c1bda4dff30c4daac9
// template-hash: a5d5c62df04659924e14ff2b6c6771228646739eefc82472964b6d7b318ffce2
// generated-at: 1782568712
// GENERATED -- DO NOT EDIT (sce-codegen)
// SCE-MAP: test519.scxml:1
package com.sce.w3c

import com.sce.generated.test519.Test519Event
import com.sce.generated.test519.Test519State
import com.sce.generated.test519.Test519StateMachine
import org.junit.jupiter.api.DisplayName

// W3C SCXML C.2: If one or more param children are present [in send], the SCXML Processor MUST map their names (i.e. name attributes) and values to HTTP POST parameters
@DisplayName("Test 519 -- W3C SCXML C.2")
class Test519 : W3CHttpTestBase<Test519State, Test519Event>() {
    override fun createStateMachine() = Test519StateMachine(createEngine())
    override val expectedPassState: Test519State = Test519State.Pass
}
