// SCE-GENERATED — DO NOT EDIT
// source-hash: f30ff39ee453ff9c2724b237e7ecc70c10c604254c7a79c1bda4dff30c4daac9
// template-hash: 9c6a4f8dfaed131dd8c4550407375e80f92b4e4373728b55d22f59422722a6ba
// generated-at: 1779372462
// GENERATED -- DO NOT EDIT (sce-codegen)
// SCE-MAP: test557.scxml:1
package com.sce.w3c

import com.sce.generated.test557.Test557Event
import com.sce.generated.test557.Test557State
import com.sce.generated.test557.Test557StateMachine
import org.junit.jupiter.api.DisplayName

// W3C SCXML B.2: In the ECMAScript data model, if either the 'src' attribute or in-line content is provided in the data element, then if the content (whether fetched or provided in-line) is an XML document, the SCXML Processor MUST create the corresponding DOM structure and assign it as the value of the data element.
@DisplayName("Test 557 -- W3C SCXML B.2")
class Test557 : W3CTestBase<Test557State, Test557Event>() {
    override fun createStateMachine() = Test557StateMachine(createEngine())
    override val expectedPassState: Test557State = Test557State.Pass
}
