// SCE-GENERATED — DO NOT EDIT
// source-hash: f30ff39ee453ff9c2724b237e7ecc70c10c604254c7a79c1bda4dff30c4daac9
// template-hash: 66bc1c3694f90e60100c842d2a53cd8c05682260c1809ba387d157940d7d6e1d
// generated-at: 1780836426
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
