// SCE-GENERATED — DO NOT EDIT
// source-hash: f30ff39ee453ff9c2724b237e7ecc70c10c604254c7a79c1bda4dff30c4daac9
// template-hash: 168f4a554705bdfb42cc51a9fbd01e4e5fc028c49c4d6f47071af9577599e075
// generated-at: 1779449862
// GENERATED -- DO NOT EDIT (sce-codegen)
// SCE-MAP: test287.scxml:1
package com.sce.w3c

import com.sce.generated.test287.Test287Event
import com.sce.generated.test287.Test287State
import com.sce.generated.test287.Test287StateMachine
import org.junit.jupiter.api.DisplayName

// W3C SCXML 5.4: If the location expression of an assign denotes a valid location in the datamodel and if the value specified by 'expr' is a legal value for the location specified, the processor MUST place the specified value at the specified location.
@DisplayName("Test 287 -- W3C SCXML 5.4")
class Test287 : W3CTestBase<Test287State, Test287Event>() {
    override fun createStateMachine() = Test287StateMachine(createEngine())
    override val expectedPassState: Test287State = Test287State.Pass
}
