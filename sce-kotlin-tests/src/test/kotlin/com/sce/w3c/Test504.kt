// SCE-GENERATED — DO NOT EDIT
// source-hash: f30ff39ee453ff9c2724b237e7ecc70c10c604254c7a79c1bda4dff30c4daac9
// template-hash: d588114b3294b4cb4d7e02d63e6d31a3c0326d3afa0a691deb12b545b5ff5045
// generated-at: 1779460271
// GENERATED -- DO NOT EDIT (sce-codegen)
// SCE-MAP: test504.scxml:1
package com.sce.w3c

import com.sce.generated.test504.Test504Event
import com.sce.generated.test504.Test504State
import com.sce.generated.test504.Test504StateMachine
import org.junit.jupiter.api.DisplayName

// W3C SCXML 3.13: if [a transition's] 'type' is "external", its exit set consists of all active states that are proper descendents of the Least Common Compound Ancestor (LCCA) of the source and target states.
@DisplayName("Test 504 -- W3C SCXML 3.13")
class Test504 : W3CTestBase<Test504State, Test504Event>() {
    override fun createStateMachine() = Test504StateMachine(createEngine())
    override val expectedPassState: Test504State = Test504State.Pass
}
