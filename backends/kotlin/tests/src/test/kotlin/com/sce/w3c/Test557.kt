// SCE-GENERATED — DO NOT EDIT
// source-hash: b1edd275a200b2f8553040c83495e98b687c11a97259eaf4d60667291dcb916a
// template-hash: 10c5bb56d60f6d5bc4121611a1230324eaf61d1a5524b71d52c6010f279d5ffd
// generated-at: 0
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
