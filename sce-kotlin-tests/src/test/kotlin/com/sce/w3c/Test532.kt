// SCE-GENERATED — DO NOT EDIT
// source-hash: f30ff39ee453ff9c2724b237e7ecc70c10c604254c7a79c1bda4dff30c4daac9
// template-hash: c1736039ea6628ae1068e428522a9d89bbe2ccef2705503db256c49ec169955e
// generated-at: 1778992486
// GENERATED -- DO NOT EDIT (sce-codegen)
// SCE-MAP: test532.scxml:1
package com.sce.w3c

import com.sce.generated.test532.Test532Event
import com.sce.generated.test532.Test532State
import com.sce.generated.test532.Test532StateMachine
import org.junit.jupiter.api.DisplayName

// W3C SCXML C.2: If _scxmleventname is not present, the Processor MUST the name of the HTTP method that was used to deliver the event as name of the SCXML event that it raises
@DisplayName("Test 532 -- W3C SCXML C.2")
class Test532 : W3CHttpTestBase<Test532State, Test532Event>() {
    override fun createStateMachine() = Test532StateMachine()
    override val expectedPassState: Test532State = Test532State.Pass
}
