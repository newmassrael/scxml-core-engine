// SCE-GENERATED — DO NOT EDIT
// source-hash: f30ff39ee453ff9c2724b237e7ecc70c10c604254c7a79c1bda4dff30c4daac9
// template-hash: 168f4a554705bdfb42cc51a9fbd01e4e5fc028c49c4d6f47071af9577599e075
// generated-at: 1779449862
// GENERATED -- DO NOT EDIT (sce-codegen)
// SCE-MAP: test313.scxml:1
package com.sce.w3c

import com.sce.generated.test313.Test313Event
import com.sce.generated.test313.Test313State
import com.sce.generated.test313.Test313StateMachine
import org.junit.jupiter.api.DisplayName

// W3C SCXML 5.9: The SCXML processor MAY reject documents containing syntactically ill-formed expressions at document load time, or it MAY wait and place error.execution in the internal event queue at runtime when the expressions are evaluated.
@DisplayName("Test 313 -- W3C SCXML 5.9")
class Test313 : W3CTestBase<Test313State, Test313Event>() {
    override fun createStateMachine() = Test313StateMachine(createEngine())
    override val expectedPassState: Test313State = Test313State.Pass
}
