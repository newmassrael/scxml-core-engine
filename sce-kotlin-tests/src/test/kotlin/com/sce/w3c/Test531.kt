// SCE-GENERATED — DO NOT EDIT
// source-hash: f30ff39ee453ff9c2724b237e7ecc70c10c604254c7a79c1bda4dff30c4daac9
// template-hash: 07a1057b89512b0ade7260ce662ea4e6ef3c2abde2d5bd32fb4fe82bd263d4bc
// generated-at: 1780802714
// GENERATED -- DO NOT EDIT (sce-codegen)
// SCE-MAP: test531.scxml:1
package com.sce.w3c

import com.sce.generated.test531.Test531Event
import com.sce.generated.test531.Test531State
import com.sce.generated.test531.Test531StateMachine
import org.junit.jupiter.api.DisplayName

// W3C SCXML C.2: If a single instance of the parameter '_scxmleventname' is present, the SCXML Processor MUST use its value as the name of the SCXML event that it raises.
@DisplayName("Test 531 -- W3C SCXML C.2")
class Test531 : W3CHttpTestBase<Test531State, Test531Event>() {
    override fun createStateMachine() = Test531StateMachine()
    override val expectedPassState: Test531State = Test531State.Pass
}
