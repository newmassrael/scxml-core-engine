// SCE-GENERATED — DO NOT EDIT
// source-hash: 50977319f11c1ff3aac5be1771f46084e92b202125e3d418050cec95e667f58c
// template-hash: e273e083fd84459760e6b7e00629aa0bbc396fdd49f2f0b96778152f02d02625
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
