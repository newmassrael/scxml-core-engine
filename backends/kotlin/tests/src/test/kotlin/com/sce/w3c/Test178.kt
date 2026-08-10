// SCE-GENERATED — DO NOT EDIT
// source-hash: b1edd275a200b2f8553040c83495e98b687c11a97259eaf4d60667291dcb916a
// template-hash: 06c80ca1f364d1bd47dcf4355438c9eb8afe054b2712ace900a9053d7a3870aa
// generated-at: 0
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
