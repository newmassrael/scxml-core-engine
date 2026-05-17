// SCE-GENERATED — DO NOT EDIT
// source-hash: f30ff39ee453ff9c2724b237e7ecc70c10c604254c7a79c1bda4dff30c4daac9
// template-hash: c1736039ea6628ae1068e428522a9d89bbe2ccef2705503db256c49ec169955e
// generated-at: 1778992486
// GENERATED -- DO NOT EDIT (sce-codegen)
// SCE-MAP: test178.scxml:1
package com.sce.w3c

import com.sce.generated.test178.Test178Event
import com.sce.generated.test178.Test178State
import com.sce.generated.test178.Test178StateMachine
import org.junit.jupiter.api.DisplayName

// W3C SCXML 6.2: The SCXML Processor MUST include all attributes and values provided by param and/or 'namelist' even if duplicates occur.
@DisplayName("Test 178 -- W3C SCXML 6.2")
class Test178 : W3CTestBase<Test178State, Test178Event>() {
    override fun createStateMachine() = Test178StateMachine(createEngine())
    override val expectedPassState: Test178State = Test178State.Final
}
