// SCE-GENERATED — DO NOT EDIT
// source-hash: f30ff39ee453ff9c2724b237e7ecc70c10c604254c7a79c1bda4dff30c4daac9
// template-hash: 168f4a554705bdfb42cc51a9fbd01e4e5fc028c49c4d6f47071af9577599e075
// generated-at: 1779449862
// GENERATED -- DO NOT EDIT (sce-codegen)
// SCE-MAP: test444.scxml:1
package com.sce.w3c

import com.sce.generated.test444.Test444Event
import com.sce.generated.test444.Test444State
import com.sce.generated.test444.Test444StateMachine
import org.junit.jupiter.api.DisplayName

// W3C SCXML B.2: In the ECMAScript datamodel, for each data element in the document, the SCXML Processor must create an ECMAScript variable object whose name is the value of the id attribute of the data element.
@DisplayName("Test 444 -- W3C SCXML B.2")
class Test444 : W3CTestBase<Test444State, Test444Event>() {
    override fun createStateMachine() = Test444StateMachine(createEngine())
    override val expectedPassState: Test444State = Test444State.Pass
}
