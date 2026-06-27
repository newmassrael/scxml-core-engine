// SCE-GENERATED — DO NOT EDIT
// source-hash: f30ff39ee453ff9c2724b237e7ecc70c10c604254c7a79c1bda4dff30c4daac9
// template-hash: dade9f8de6d0c296ea9dd537c4a48e14404d516e6b96273faf48e4d26f58db4f
// generated-at: 1782564443
// GENERATED -- DO NOT EDIT (sce-codegen)
// SCE-MAP: test150.scxml:1
package com.sce.w3c

import com.sce.generated.test150.Test150Event
import com.sce.generated.test150.Test150State
import com.sce.generated.test150.Test150StateMachine
import org.junit.jupiter.api.DisplayName

// W3C SCXML 4.6: In the foreach element, the SCXML processor MUST declare a new variable if the one specified by 'item' is not already defined.
@DisplayName("Test 150 -- W3C SCXML 4.6")
class Test150 : W3CTestBase<Test150State, Test150Event>() {
    override fun createStateMachine() = Test150StateMachine(createEngine())
    override val expectedPassState: Test150State = Test150State.Pass
}
