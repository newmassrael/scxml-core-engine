// SCE-GENERATED — DO NOT EDIT
// source-hash: 7e40b95c2d842dcf564705fa3efcf7ff2175df2e7887977f32be9e362c2a3f25
// GENERATED -- DO NOT EDIT (sce-codegen)
// SCE-MAP: test558.scxml:1
package com.sce.w3c

import com.sce.generated.test558.Test558Event
import com.sce.generated.test558.Test558State
import com.sce.generated.test558.Test558StateMachine
import org.junit.jupiter.api.DisplayName

// W3C SCXML B.2: In the ECMAScript datamodel, if either the 'src' attribute or in-line content is provided in the data element, and the content (whether fetched or provided in-line) is not an XML document or JSON (or the
@DisplayName("Test 558 -- W3C SCXML B.2")
class Test558 : W3CTestBase<Test558State, Test558Event>() {
    override fun createStateMachine() = Test558StateMachine(createEngine())
    override val expectedPassState: Test558State = Test558State.Pass
}
