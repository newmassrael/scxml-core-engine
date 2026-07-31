// SCE-GENERATED — DO NOT EDIT
// source-hash: 50977319f11c1ff3aac5be1771f46084e92b202125e3d418050cec95e667f58c
// template-hash: f160b18d725f2c0387242c0463da6808a5b8be392d0dc888f0d564e42c83db17
// generated-at: 1785486331
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
