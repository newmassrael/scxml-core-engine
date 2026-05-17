// SCE-GENERATED — DO NOT EDIT
// source-hash: f30ff39ee453ff9c2724b237e7ecc70c10c604254c7a79c1bda4dff30c4daac9
// template-hash: c1736039ea6628ae1068e428522a9d89bbe2ccef2705503db256c49ec169955e
// generated-at: 1778992486
// GENERATED -- DO NOT EDIT (sce-codegen)
// SCE-MAP: test241.scxml:1
package com.sce.w3c

import com.sce.generated.test241.Test241Event
import com.sce.generated.test241.Test241State
import com.sce.generated.test241.Test241StateMachine
import org.junit.jupiter.api.DisplayName

// W3C SCXML 6.4: Invoked services MUST treat values specified by param and namelist identically.
@DisplayName("Test 241 -- W3C SCXML 6.4")
class Test241 : W3CTestBase<Test241State, Test241Event>() {
    override fun createStateMachine() = Test241StateMachine(createEngine())
    override val expectedPassState: Test241State = Test241State.Pass
    override val timeoutMs: Long = 5000L
}
