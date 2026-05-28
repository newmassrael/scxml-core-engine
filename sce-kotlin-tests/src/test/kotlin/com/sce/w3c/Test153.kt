// SCE-GENERATED — DO NOT EDIT
// source-hash: f30ff39ee453ff9c2724b237e7ecc70c10c604254c7a79c1bda4dff30c4daac9
// template-hash: 5bcd19449227e607bbf4637f80b3a21d971f8561ecea8393b7bab39ff5ce1cc8
// generated-at: 1779976072
// GENERATED -- DO NOT EDIT (sce-codegen)
// SCE-MAP: test153.scxml:1
package com.sce.w3c

import com.sce.generated.test153.Test153Event
import com.sce.generated.test153.Test153State
import com.sce.generated.test153.Test153StateMachine
import org.junit.jupiter.api.DisplayName

// W3C SCXML 4.6: When evaluating foreach, the SCXML processor MUST start with the first item in the collection and proceed to the last item in the iteration order that is defined for the collection. For each item in the collection in turn, the processor MUST assign it to the item variable.
@DisplayName("Test 153 -- W3C SCXML 4.6")
class Test153 : W3CTestBase<Test153State, Test153Event>() {
    override fun createStateMachine() = Test153StateMachine(createEngine())
    override val expectedPassState: Test153State = Test153State.Pass
}
