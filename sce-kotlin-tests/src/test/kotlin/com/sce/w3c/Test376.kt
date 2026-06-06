// SCE-GENERATED — DO NOT EDIT
// source-hash: f30ff39ee453ff9c2724b237e7ecc70c10c604254c7a79c1bda4dff30c4daac9
// template-hash: 09c66e0a06202a6ec53b4591ac58670a6615a699910ff161304360792e1e7915
// generated-at: 1780732004
// GENERATED -- DO NOT EDIT (sce-codegen)
// SCE-MAP: test376.scxml:1
package com.sce.w3c

import com.sce.generated.test376.Test376Event
import com.sce.generated.test376.Test376State
import com.sce.generated.test376.Test376StateMachine
import org.junit.jupiter.api.DisplayName

// W3C SCXML 3.8: The SCXML processor MUST treat each [onentry] handler as a separate block of executable content.
@DisplayName("Test 376 -- W3C SCXML 3.8")
class Test376 : W3CTestBase<Test376State, Test376Event>() {
    override fun createStateMachine() = Test376StateMachine(createEngine())
    override val expectedPassState: Test376State = Test376State.Pass
}
