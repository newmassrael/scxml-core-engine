// SCE-GENERATED — DO NOT EDIT
// source-hash: f30ff39ee453ff9c2724b237e7ecc70c10c604254c7a79c1bda4dff30c4daac9
// template-hash: f835a323a3abc9cebc80341e1840b22b95739a2efa1726ad2c440477eff36482
// generated-at: 1781089257
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
