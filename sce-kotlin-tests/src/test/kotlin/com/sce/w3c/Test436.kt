// SCE-GENERATED — DO NOT EDIT
// source-hash: f30ff39ee453ff9c2724b237e7ecc70c10c604254c7a79c1bda4dff30c4daac9
// template-hash: dade9f8de6d0c296ea9dd537c4a48e14404d516e6b96273faf48e4d26f58db4f
// generated-at: 1782564443
// GENERATED -- DO NOT EDIT (sce-codegen)
// SCE-MAP: test436.scxml:1
package com.sce.w3c

import com.sce.generated.test436.Test436Event
import com.sce.generated.test436.Test436State
import com.sce.generated.test436.Test436StateMachine
import org.junit.jupiter.api.DisplayName

// W3C SCXML B.1: When the "datamodel" attribute of the scxml element has the value "null", the In() predicate must return 'true' if and only if that state is in the current state configuration.
@DisplayName("Test 436 -- W3C SCXML B.1")
class Test436 : W3CTestBase<Test436State, Test436Event>() {
    override fun createStateMachine() = Test436StateMachine()
    override val expectedPassState: Test436State = Test436State.Pass
}
