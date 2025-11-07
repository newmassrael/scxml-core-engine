#include "scripting/JSEngine.h"
#include <gtest/gtest.h>

/**
 * @brief W3C SCXML Appendix B.1 ECMAScript Data Model Compliance Tests
 *
 * Tests comprehensive ECMAScript operator, type coercion, and built-in object
 * functionality required by W3C SCXML ECMAScript data model specification.
 */
class ECMAScriptComplianceTest : public ::testing::Test {
protected:
    void SetUp() override {
        engine_ = &SCE::JSEngine::instance();
        engine_->reset();

        sessionId_ = "test_session_ecmascript";
        bool result = engine_->createSession(sessionId_, "");
        ASSERT_TRUE(result) << "Failed to create session";
    }

    void TearDown() override {
        if (engine_) {
            engine_->destroySession(sessionId_);
            engine_->shutdown();
        }
    }

    SCE::JSEngine *engine_;
    std::string sessionId_;
};

// ============================================================================
// W3C SCXML B.1: Array Iteration Methods (P0 - Critical)
// ============================================================================

TEST_F(ECMAScriptComplianceTest, W3C_ArrayMethods_ForEach) {
    // Setup test array
    auto setupResult = engine_->executeScript(sessionId_, "var arr = [1, 2, 3]; var sum = 0;").get();
    ASSERT_TRUE(setupResult.isSuccess()) << "Failed to setup test array";

    // Test forEach with accumulation
    auto forEachResult = engine_->executeScript(sessionId_, "arr.forEach(function(val) { sum += val; }); sum").get();
    ASSERT_TRUE(forEachResult.isSuccess()) << "forEach execution failed";
    EXPECT_EQ(forEachResult.getValue<double>(), 6.0) << "forEach should accumulate 1+2+3=6";

    // Verify array unchanged
    auto arrayCheckResult = engine_->evaluateExpression(sessionId_, "arr.length").get();
    ASSERT_TRUE(arrayCheckResult.isSuccess());
    EXPECT_EQ(arrayCheckResult.getValue<double>(), 3.0) << "forEach should not modify array";
}

TEST_F(ECMAScriptComplianceTest, W3C_ArrayMethods_Map) {
    // Setup test array
    auto setupResult = engine_->executeScript(sessionId_, "var arr = [1, 2, 3];").get();
    ASSERT_TRUE(setupResult.isSuccess());

    // Test map transformation
    auto mapResult =
        engine_->executeScript(sessionId_, "var doubled = arr.map(function(val) { return val * 2; }); doubled").get();
    ASSERT_TRUE(mapResult.isSuccess()) << "map execution failed";
    EXPECT_TRUE(mapResult.isArray()) << "map should return array";

    // Verify mapped values
    auto firstResult = engine_->evaluateExpression(sessionId_, "doubled[0]").get();
    ASSERT_TRUE(firstResult.isSuccess());
    EXPECT_EQ(firstResult.getValue<double>(), 2.0) << "First element should be doubled";

    auto secondResult = engine_->evaluateExpression(sessionId_, "doubled[1]").get();
    ASSERT_TRUE(secondResult.isSuccess());
    EXPECT_EQ(secondResult.getValue<double>(), 4.0) << "Second element should be doubled";

    auto thirdResult = engine_->evaluateExpression(sessionId_, "doubled[2]").get();
    ASSERT_TRUE(thirdResult.isSuccess());
    EXPECT_EQ(thirdResult.getValue<double>(), 6.0) << "Third element should be doubled";

    // Verify original array unchanged
    auto origResult = engine_->evaluateExpression(sessionId_, "arr[0]").get();
    ASSERT_TRUE(origResult.isSuccess());
    EXPECT_EQ(origResult.getValue<double>(), 1.0) << "map should not modify original array";
}

TEST_F(ECMAScriptComplianceTest, W3C_ArrayMethods_Filter) {
    // Setup test array with mixed values
    auto setupResult = engine_->executeScript(sessionId_, "var arr = [1, 2, 3, 4, 5, 6];").get();
    ASSERT_TRUE(setupResult.isSuccess());

    // Test filter for even numbers
    auto filterResult =
        engine_->executeScript(sessionId_, "var evens = arr.filter(function(val) { return val % 2 === 0; }); evens")
            .get();
    ASSERT_TRUE(filterResult.isSuccess()) << "filter execution failed";
    EXPECT_TRUE(filterResult.isArray()) << "filter should return array";

    // Verify filtered array length
    auto lengthResult = engine_->evaluateExpression(sessionId_, "evens.length").get();
    ASSERT_TRUE(lengthResult.isSuccess());
    EXPECT_EQ(lengthResult.getValue<double>(), 3.0) << "Should filter 3 even numbers (2, 4, 6)";

    // Verify filtered values
    auto firstResult = engine_->evaluateExpression(sessionId_, "evens[0]").get();
    ASSERT_TRUE(firstResult.isSuccess());
    EXPECT_EQ(firstResult.getValue<double>(), 2.0);

    auto secondResult = engine_->evaluateExpression(sessionId_, "evens[1]").get();
    ASSERT_TRUE(secondResult.isSuccess());
    EXPECT_EQ(secondResult.getValue<double>(), 4.0);

    auto thirdResult = engine_->evaluateExpression(sessionId_, "evens[2]").get();
    ASSERT_TRUE(thirdResult.isSuccess());
    EXPECT_EQ(thirdResult.getValue<double>(), 6.0);
}

TEST_F(ECMAScriptComplianceTest, W3C_ArrayMethods_Reduce) {
    // Setup test array
    auto setupResult = engine_->executeScript(sessionId_, "var arr = [1, 2, 3, 4];").get();
    ASSERT_TRUE(setupResult.isSuccess());

    // Test reduce with sum
    auto sumResult =
        engine_->executeScript(sessionId_, "var sum = arr.reduce(function(acc, val) { return acc + val; }, 0); sum")
            .get();
    ASSERT_TRUE(sumResult.isSuccess()) << "reduce sum execution failed";
    EXPECT_EQ(sumResult.getValue<double>(), 10.0) << "reduce should sum 1+2+3+4=10";

    // Test reduce with product
    auto productResult =
        engine_
            ->executeScript(sessionId_,
                            "var product = arr.reduce(function(acc, val) { return acc * val; }, 1); product")
            .get();
    ASSERT_TRUE(productResult.isSuccess()) << "reduce product execution failed";
    EXPECT_EQ(productResult.getValue<double>(), 24.0) << "reduce should multiply 1*2*3*4=24";

    // Test reduce without initial value
    auto noInitResult =
        engine_->executeScript(sessionId_, "var result = arr.reduce(function(acc, val) { return acc + val; }); result")
            .get();
    ASSERT_TRUE(noInitResult.isSuccess()) << "reduce without initial value should work";
    EXPECT_EQ(noInitResult.getValue<double>(), 10.0) << "reduce without init should still sum correctly";
}

TEST_F(ECMAScriptComplianceTest, W3C_ArrayMethods_Find) {
    // Setup test array
    auto setupResult = engine_->executeScript(sessionId_, "var arr = [1, 5, 10, 15, 20];").get();
    ASSERT_TRUE(setupResult.isSuccess());

    // Test find with match
    auto foundResult =
        engine_->executeScript(sessionId_, "var found = arr.find(function(val) { return val > 12; }); found").get();
    ASSERT_TRUE(foundResult.isSuccess()) << "find execution failed";
    EXPECT_EQ(foundResult.getValue<double>(), 15.0) << "find should return first element > 12";

    // Test find with no match
    auto notFoundResult =
        engine_
            ->executeScript(sessionId_, "var notFound = arr.find(function(val) { return val > 100; }); typeof notFound")
            .get();
    ASSERT_TRUE(notFoundResult.isSuccess());
    EXPECT_EQ(notFoundResult.getValue<std::string>(), "undefined") << "find should return undefined when no match";
}

TEST_F(ECMAScriptComplianceTest, W3C_ArrayMethods_Some) {
    // Setup test array
    auto setupResult = engine_->executeScript(sessionId_, "var arr = [1, 2, 3, 4, 5];").get();
    ASSERT_TRUE(setupResult.isSuccess());

    // Test some with match
    auto hasSomeResult =
        engine_->executeScript(sessionId_, "var hasSome = arr.some(function(val) { return val > 3; }); hasSome").get();
    ASSERT_TRUE(hasSomeResult.isSuccess());
    EXPECT_TRUE(hasSomeResult.getValue<bool>()) << "some should return true when at least one element > 3";

    // Test some with no match
    auto hasNoneResult =
        engine_->executeScript(sessionId_, "var hasNone = arr.some(function(val) { return val > 10; }); hasNone").get();
    ASSERT_TRUE(hasNoneResult.isSuccess());
    EXPECT_FALSE(hasNoneResult.getValue<bool>()) << "some should return false when no elements > 10";
}

TEST_F(ECMAScriptComplianceTest, W3C_ArrayMethods_Every) {
    // Setup test array
    auto setupResult = engine_->executeScript(sessionId_, "var arr = [2, 4, 6, 8];").get();
    ASSERT_TRUE(setupResult.isSuccess());

    // Test every with all match
    auto allEvenResult =
        engine_->executeScript(sessionId_, "var allEven = arr.every(function(val) { return val % 2 === 0; }); allEven")
            .get();
    ASSERT_TRUE(allEvenResult.isSuccess());
    EXPECT_TRUE(allEvenResult.getValue<bool>()) << "every should return true when all elements are even";

    // Test every with one non-match
    auto setupMixedResult = engine_->executeScript(sessionId_, "var mixed = [2, 4, 5, 8];").get();
    ASSERT_TRUE(setupMixedResult.isSuccess());

    auto notAllEvenResult =
        engine_
            ->executeScript(sessionId_,
                            "var notAllEven = mixed.every(function(val) { return val % 2 === 0; }); notAllEven")
            .get();
    ASSERT_TRUE(notAllEvenResult.isSuccess());
    EXPECT_FALSE(notAllEvenResult.getValue<bool>()) << "every should return false when one element is odd";
}

// ============================================================================
// W3C SCXML B.1: Type Coercion (P0 - Critical)
// ============================================================================

TEST_F(ECMAScriptComplianceTest, W3C_TypeCoercion_StringConcatenation) {
    // String + Number -> String concatenation
    auto strNumResult = engine_->evaluateExpression(sessionId_, "'5' + 3").get();
    ASSERT_TRUE(strNumResult.isSuccess());
    EXPECT_EQ(strNumResult.getValue<std::string>(), "53") << "'5' + 3 should concatenate as string";

    // Number + String -> String concatenation
    auto numStrResult = engine_->evaluateExpression(sessionId_, "3 + '5'").get();
    ASSERT_TRUE(numStrResult.isSuccess());
    EXPECT_EQ(numStrResult.getValue<std::string>(), "35") << "3 + '5' should concatenate as string";

    // String + Boolean -> String concatenation
    auto strBoolResult = engine_->evaluateExpression(sessionId_, "'value: ' + true").get();
    ASSERT_TRUE(strBoolResult.isSuccess());
    EXPECT_EQ(strBoolResult.getValue<std::string>(), "value: true") << "String + boolean should concatenate";
}

TEST_F(ECMAScriptComplianceTest, W3C_TypeCoercion_NumericOperations) {
    // String - Number -> Numeric subtraction
    auto subResult = engine_->evaluateExpression(sessionId_, "'5' - 3").get();
    ASSERT_TRUE(subResult.isSuccess());
    EXPECT_EQ(subResult.getValue<double>(), 2.0) << "'5' - 3 should perform numeric subtraction";

    // String * Number -> Numeric multiplication
    auto mulResult = engine_->evaluateExpression(sessionId_, "'4' * 2").get();
    ASSERT_TRUE(mulResult.isSuccess());
    EXPECT_EQ(mulResult.getValue<double>(), 8.0) << "'4' * 2 should perform numeric multiplication";

    // String / Number -> Numeric division
    auto divResult = engine_->evaluateExpression(sessionId_, "'10' / 2").get();
    ASSERT_TRUE(divResult.isSuccess());
    EXPECT_EQ(divResult.getValue<double>(), 5.0) << "'10' / 2 should perform numeric division";
}

TEST_F(ECMAScriptComplianceTest, W3C_TypeCoercion_BooleanContext) {
    // Truthy values
    auto truthyNumResult = engine_->evaluateExpression(sessionId_, "!!1").get();
    ASSERT_TRUE(truthyNumResult.isSuccess());
    EXPECT_TRUE(truthyNumResult.getValue<bool>()) << "!!1 should be true (truthy)";

    auto truthyStrResult = engine_->evaluateExpression(sessionId_, "!!'hello'").get();
    ASSERT_TRUE(truthyStrResult.isSuccess());
    EXPECT_TRUE(truthyStrResult.getValue<bool>()) << "!!'hello' should be true (truthy)";

    auto truthyObjResult = engine_->evaluateExpression(sessionId_, "!!{}").get();
    ASSERT_TRUE(truthyObjResult.isSuccess());
    EXPECT_TRUE(truthyObjResult.getValue<bool>()) << "!!{} should be true (truthy)";

    auto truthyArrResult = engine_->evaluateExpression(sessionId_, "!![]").get();
    ASSERT_TRUE(truthyArrResult.isSuccess());
    EXPECT_TRUE(truthyArrResult.getValue<bool>()) << "!![] should be true (truthy)";

    // Falsy values
    auto falsyZeroResult = engine_->evaluateExpression(sessionId_, "!!0").get();
    ASSERT_TRUE(falsyZeroResult.isSuccess());
    EXPECT_FALSE(falsyZeroResult.getValue<bool>()) << "!!0 should be false (falsy)";

    auto falsyEmptyStrResult = engine_->evaluateExpression(sessionId_, "!!''").get();
    ASSERT_TRUE(falsyEmptyStrResult.isSuccess());
    EXPECT_FALSE(falsyEmptyStrResult.getValue<bool>()) << "!!'' should be false (falsy)";

    auto falsyNullResult = engine_->evaluateExpression(sessionId_, "!!null").get();
    ASSERT_TRUE(falsyNullResult.isSuccess());
    EXPECT_FALSE(falsyNullResult.getValue<bool>()) << "!!null should be false (falsy)";

    auto falsyUndefinedResult = engine_->evaluateExpression(sessionId_, "!!undefined").get();
    ASSERT_TRUE(falsyUndefinedResult.isSuccess());
    EXPECT_FALSE(falsyUndefinedResult.getValue<bool>()) << "!!undefined should be false (falsy)";
}

TEST_F(ECMAScriptComplianceTest, W3C_TypeCoercion_UnaryPlus) {
    // Unary + converts to number
    auto plusStrResult = engine_->evaluateExpression(sessionId_, "+'42'").get();
    ASSERT_TRUE(plusStrResult.isSuccess());
    EXPECT_EQ(plusStrResult.getValue<double>(), 42.0) << "+'42' should convert to number 42";

    auto plusTrueResult = engine_->evaluateExpression(sessionId_, "+true").get();
    ASSERT_TRUE(plusTrueResult.isSuccess());
    EXPECT_EQ(plusTrueResult.getValue<double>(), 1.0) << "+true should convert to 1";

    auto plusFalseResult = engine_->evaluateExpression(sessionId_, "+false").get();
    ASSERT_TRUE(plusFalseResult.isSuccess());
    EXPECT_EQ(plusFalseResult.getValue<double>(), 0.0) << "+false should convert to 0";

    auto plusNullResult = engine_->evaluateExpression(sessionId_, "+null").get();
    ASSERT_TRUE(plusNullResult.isSuccess());
    EXPECT_EQ(plusNullResult.getValue<double>(), 0.0) << "+null should convert to 0";
}

TEST_F(ECMAScriptComplianceTest, W3C_TypeCoercion_LogicalNegation) {
    // ! operator converts to boolean and negates
    auto notZeroResult = engine_->evaluateExpression(sessionId_, "!0").get();
    ASSERT_TRUE(notZeroResult.isSuccess());
    EXPECT_TRUE(notZeroResult.getValue<bool>()) << "!0 should be true";

    auto notOneResult = engine_->evaluateExpression(sessionId_, "!1").get();
    ASSERT_TRUE(notOneResult.isSuccess());
    EXPECT_FALSE(notOneResult.getValue<bool>()) << "!1 should be false";

    auto notEmptyStrResult = engine_->evaluateExpression(sessionId_, "!''").get();
    ASSERT_TRUE(notEmptyStrResult.isSuccess());
    EXPECT_TRUE(notEmptyStrResult.getValue<bool>()) << "!'' should be true";

    auto notStrResult = engine_->evaluateExpression(sessionId_, "!'hello'").get();
    ASSERT_TRUE(notStrResult.isSuccess());
    EXPECT_FALSE(notStrResult.getValue<bool>()) << "!'hello' should be false";
}

TEST_F(ECMAScriptComplianceTest, W3C_TypeCoercion_ConditionalOperator) {
    // Ternary operator with type coercion in condition
    auto ternaryTruthyResult = engine_->evaluateExpression(sessionId_, "1 ? 'yes' : 'no'").get();
    ASSERT_TRUE(ternaryTruthyResult.isSuccess());
    EXPECT_EQ(ternaryTruthyResult.getValue<std::string>(), "yes") << "1 (truthy) should choose 'yes'";

    auto ternaryFalsyResult = engine_->evaluateExpression(sessionId_, "0 ? 'yes' : 'no'").get();
    ASSERT_TRUE(ternaryFalsyResult.isSuccess());
    EXPECT_EQ(ternaryFalsyResult.getValue<std::string>(), "no") << "0 (falsy) should choose 'no'";

    auto ternaryEmptyStrResult = engine_->evaluateExpression(sessionId_, "'' ? 'yes' : 'no'").get();
    ASSERT_TRUE(ternaryEmptyStrResult.isSuccess());
    EXPECT_EQ(ternaryEmptyStrResult.getValue<std::string>(), "no") << "'' (falsy) should choose 'no'";

    // Ternary with numeric result
    auto ternaryNumResult = engine_->evaluateExpression(sessionId_, "5 > 3 ? 10 : 20").get();
    ASSERT_TRUE(ternaryNumResult.isSuccess());
    EXPECT_EQ(ternaryNumResult.getValue<double>(), 10.0) << "5 > 3 should choose 10";
}

// ============================================================================
// W3C SCXML B.1: Object Static Methods (P0 - Critical)
// ============================================================================

TEST_F(ECMAScriptComplianceTest, W3C_ObjectMethods_Keys) {
    // Setup test object
    auto setupResult = engine_->executeScript(sessionId_, "var obj = {a: 1, b: 2, c: 3};").get();
    ASSERT_TRUE(setupResult.isSuccess());

    // Test Object.keys()
    auto keysResult = engine_->executeScript(sessionId_, "var keys = Object.keys(obj); keys").get();
    ASSERT_TRUE(keysResult.isSuccess()) << "Object.keys() execution failed";
    EXPECT_TRUE(keysResult.isArray()) << "Object.keys() should return array";

    // Verify keys array length
    auto lengthResult = engine_->evaluateExpression(sessionId_, "keys.length").get();
    ASSERT_TRUE(lengthResult.isSuccess());
    EXPECT_EQ(lengthResult.getValue<double>(), 3.0) << "Should have 3 keys";

    // Verify keys contain expected properties
    auto hasAResult = engine_->evaluateExpression(sessionId_, "keys.indexOf('a') >= 0").get();
    ASSERT_TRUE(hasAResult.isSuccess());
    EXPECT_TRUE(hasAResult.getValue<bool>()) << "Keys should contain 'a'";

    auto hasBResult = engine_->evaluateExpression(sessionId_, "keys.indexOf('b') >= 0").get();
    ASSERT_TRUE(hasBResult.isSuccess());
    EXPECT_TRUE(hasBResult.getValue<bool>()) << "Keys should contain 'b'";

    auto hasCResult = engine_->evaluateExpression(sessionId_, "keys.indexOf('c') >= 0").get();
    ASSERT_TRUE(hasCResult.isSuccess());
    EXPECT_TRUE(hasCResult.getValue<bool>()) << "Keys should contain 'c'";
}

TEST_F(ECMAScriptComplianceTest, W3C_ObjectMethods_Values) {
    // Setup test object
    auto setupResult = engine_->executeScript(sessionId_, "var obj = {a: 10, b: 20, c: 30};").get();
    ASSERT_TRUE(setupResult.isSuccess());

    // Test Object.values()
    auto valuesResult = engine_->executeScript(sessionId_, "var values = Object.values(obj); values").get();
    ASSERT_TRUE(valuesResult.isSuccess()) << "Object.values() execution failed";
    EXPECT_TRUE(valuesResult.isArray()) << "Object.values() should return array";

    // Verify values array length
    auto lengthResult = engine_->evaluateExpression(sessionId_, "values.length").get();
    ASSERT_TRUE(lengthResult.isSuccess());
    EXPECT_EQ(lengthResult.getValue<double>(), 3.0) << "Should have 3 values";

    // Verify values contain expected numbers
    auto has10Result = engine_->evaluateExpression(sessionId_, "values.indexOf(10) >= 0").get();
    ASSERT_TRUE(has10Result.isSuccess());
    EXPECT_TRUE(has10Result.getValue<bool>()) << "Values should contain 10";

    auto has20Result = engine_->evaluateExpression(sessionId_, "values.indexOf(20) >= 0").get();
    ASSERT_TRUE(has20Result.isSuccess());
    EXPECT_TRUE(has20Result.getValue<bool>()) << "Values should contain 20";

    auto has30Result = engine_->evaluateExpression(sessionId_, "values.indexOf(30) >= 0").get();
    ASSERT_TRUE(has30Result.isSuccess());
    EXPECT_TRUE(has30Result.getValue<bool>()) << "Values should contain 30";
}

TEST_F(ECMAScriptComplianceTest, W3C_ObjectMethods_Entries) {
    // Setup test object
    auto setupResult = engine_->executeScript(sessionId_, "var obj = {x: 100, y: 200};").get();
    ASSERT_TRUE(setupResult.isSuccess());

    // Test Object.entries()
    auto entriesResult = engine_->executeScript(sessionId_, "var entries = Object.entries(obj); entries").get();
    ASSERT_TRUE(entriesResult.isSuccess()) << "Object.entries() execution failed";
    EXPECT_TRUE(entriesResult.isArray()) << "Object.entries() should return array";

    // Verify entries array length
    auto lengthResult = engine_->evaluateExpression(sessionId_, "entries.length").get();
    ASSERT_TRUE(lengthResult.isSuccess());
    EXPECT_EQ(lengthResult.getValue<double>(), 2.0) << "Should have 2 entries";

    // Verify first entry structure [key, value]
    auto firstKeyResult = engine_->evaluateExpression(sessionId_, "entries[0][0]").get();
    ASSERT_TRUE(firstKeyResult.isSuccess());
    EXPECT_EQ(firstKeyResult.getValue<std::string>(), "x") << "First entry key should be 'x'";

    auto firstValueResult = engine_->evaluateExpression(sessionId_, "entries[0][1]").get();
    ASSERT_TRUE(firstValueResult.isSuccess());
    EXPECT_EQ(firstValueResult.getValue<double>(), 100.0) << "First entry value should be 100";

    // Verify second entry structure
    auto secondKeyResult = engine_->evaluateExpression(sessionId_, "entries[1][0]").get();
    ASSERT_TRUE(secondKeyResult.isSuccess());
    EXPECT_EQ(secondKeyResult.getValue<std::string>(), "y") << "Second entry key should be 'y'";

    auto secondValueResult = engine_->evaluateExpression(sessionId_, "entries[1][1]").get();
    ASSERT_TRUE(secondValueResult.isSuccess());
    EXPECT_EQ(secondValueResult.getValue<double>(), 200.0) << "Second entry value should be 200";
}

// ============================================================================
// W3C SCXML Error Handling (P1 - High Priority)
// ============================================================================

TEST_F(ECMAScriptComplianceTest, W3C_ErrorHandling_TryCatchBasic) {
    // Test basic try-catch error recovery
    auto tryCatchResult = engine_
                              ->executeScript(sessionId_, R"(
        var result = 'success';
        try {
            throw new Error('test error');
            result = 'should not reach';
        } catch (e) {
            result = 'caught: ' + e.message;
        }
        result;
    )")
                              .get();
    ASSERT_TRUE(tryCatchResult.isSuccess()) << "try-catch execution should succeed";
    EXPECT_EQ(tryCatchResult.getValue<std::string>(), "caught: test error") << "Should catch and handle error";
}

TEST_F(ECMAScriptComplianceTest, W3C_ErrorHandling_TryCatchFinally) {
    // Test try-catch-finally execution order
    auto finallyResult = engine_
                             ->executeScript(sessionId_, R"(
        var log = '';
        try {
            log += 'try';
            throw new Error('error');
        } catch (e) {
            log += '-catch';
        } finally {
            log += '-finally';
        }
        log;
    )")
                             .get();
    ASSERT_TRUE(finallyResult.isSuccess()) << "try-catch-finally should execute";
    EXPECT_EQ(finallyResult.getValue<std::string>(), "try-catch-finally") << "finally block must execute";
}

TEST_F(ECMAScriptComplianceTest, W3C_ErrorHandling_FinallyWithReturn) {
    // Test finally block execution even with return in try
    auto finallyReturnResult = engine_
                                   ->executeScript(sessionId_, R"(
        var executed = false;
        function testFinally() {
            try {
                return 'from try';
            } finally {
                executed = true;
            }
        }
        testFinally();
        executed;
    )")
                                   .get();
    ASSERT_TRUE(finallyReturnResult.isSuccess()) << "finally with return should work";
    EXPECT_TRUE(finallyReturnResult.getValue<bool>()) << "finally must execute even with return in try";
}

TEST_F(ECMAScriptComplianceTest, W3C_ErrorHandling_NestedTryCatch) {
    // Test nested try-catch blocks
    auto nestedResult = engine_
                            ->executeScript(sessionId_, R"(
        var result = '';
        try {
            try {
                throw new Error('inner');
            } catch (inner) {
                result = 'inner: ' + inner.message;
                throw new Error('outer');
            }
        } catch (outer) {
            result += ', outer: ' + outer.message;
        }
        result;
    )")
                            .get();
    ASSERT_TRUE(nestedResult.isSuccess()) << "Nested try-catch should work";
    EXPECT_EQ(nestedResult.getValue<std::string>(), "inner: inner, outer: outer") << "Both errors should be caught";
}

TEST_F(ECMAScriptComplianceTest, W3C_ErrorHandling_ThrowCustomObject) {
    // Test throwing custom error objects
    auto customErrorResult = engine_
                                 ->executeScript(sessionId_, R"(
        var caughtError = null;
        try {
            throw {code: 'CUSTOM_ERROR', value: 42};
        } catch (e) {
            caughtError = e;
        }
        caughtError.code + ':' + caughtError.value;
    )")
                                 .get();
    ASSERT_TRUE(customErrorResult.isSuccess()) << "Custom error object should work";
    EXPECT_EQ(customErrorResult.getValue<std::string>(), "CUSTOM_ERROR:42")
        << "Custom error properties should be accessible";
}

TEST_F(ECMAScriptComplianceTest, W3C_ErrorHandling_ErrorTypes) {
    // Test different error types (ReferenceError, TypeError, etc.)

    // ReferenceError
    auto refErrorResult = engine_
                              ->executeScript(sessionId_, R"(
        var errorType = '';
        try {
            nonExistentVariable;
        } catch (e) {
            errorType = e.name;
        }
        errorType;
    )")
                              .get();
    ASSERT_TRUE(refErrorResult.isSuccess()) << "ReferenceError handling should work";
    EXPECT_EQ(refErrorResult.getValue<std::string>(), "ReferenceError") << "Should catch ReferenceError";

    // TypeError
    auto typeErrorResult = engine_
                               ->executeScript(sessionId_, R"(
        var errorType = '';
        try {
            null.someProperty;
        } catch (e) {
            errorType = e.name;
        }
        errorType;
    )")
                               .get();
    ASSERT_TRUE(typeErrorResult.isSuccess()) << "TypeError handling should work";
    EXPECT_EQ(typeErrorResult.getValue<std::string>(), "TypeError") << "Should catch TypeError";
}

// ============================================================================
// W3C SCXML Number Edge Cases (P1 - High Priority)
// ============================================================================

TEST_F(ECMAScriptComplianceTest, W3C_Number_Infinity) {
    // Test positive infinity
    auto posInfResult = engine_->evaluateExpression(sessionId_, "1 / 0").get();
    ASSERT_TRUE(posInfResult.isSuccess()) << "Division by zero should produce Infinity";
    EXPECT_EQ(posInfResult.getValue<double>(), std::numeric_limits<double>::infinity()) << "1/0 should be Infinity";

    auto infTypeResult = engine_->evaluateExpression(sessionId_, "typeof (1/0)").get();
    ASSERT_TRUE(infTypeResult.isSuccess());
    EXPECT_EQ(infTypeResult.getValue<std::string>(), "number") << "Infinity should be of type number";

    // Test negative infinity
    auto negInfResult = engine_->evaluateExpression(sessionId_, "-1 / 0").get();
    ASSERT_TRUE(negInfResult.isSuccess());
    EXPECT_EQ(negInfResult.getValue<double>(), -std::numeric_limits<double>::infinity()) << "-1/0 should be -Infinity";

    // Test Infinity constant
    auto infConstResult = engine_->evaluateExpression(sessionId_, "Infinity").get();
    ASSERT_TRUE(infConstResult.isSuccess()) << "Infinity constant should exist";
    EXPECT_EQ(infConstResult.getValue<double>(), std::numeric_limits<double>::infinity());
}

TEST_F(ECMAScriptComplianceTest, W3C_Number_NaN) {
    // Test NaN generation
    auto nanResult = engine_->evaluateExpression(sessionId_, "0 / 0").get();
    ASSERT_TRUE(nanResult.isSuccess()) << "0/0 should produce NaN";
    EXPECT_TRUE(std::isnan(nanResult.getValue<double>())) << "0/0 should be NaN";

    auto nanTypeResult = engine_->evaluateExpression(sessionId_, "typeof (0/0)").get();
    ASSERT_TRUE(nanTypeResult.isSuccess());
    EXPECT_EQ(nanTypeResult.getValue<std::string>(), "number") << "NaN should be of type number";

    // Test NaN constant
    auto nanConstResult = engine_->evaluateExpression(sessionId_, "NaN").get();
    ASSERT_TRUE(nanConstResult.isSuccess()) << "NaN constant should exist";
    EXPECT_TRUE(std::isnan(nanConstResult.getValue<double>())) << "NaN constant should be NaN";

    // Test NaN === NaN (should be false)
    auto nanEqualityResult = engine_->evaluateExpression(sessionId_, "NaN === NaN").get();
    ASSERT_TRUE(nanEqualityResult.isSuccess());
    EXPECT_FALSE(nanEqualityResult.getValue<bool>()) << "NaN === NaN should be false (IEEE 754)";

    // Test NaN !== NaN (should be true)
    auto nanInequalityResult = engine_->evaluateExpression(sessionId_, "NaN !== NaN").get();
    ASSERT_TRUE(nanInequalityResult.isSuccess());
    EXPECT_TRUE(nanInequalityResult.getValue<bool>()) << "NaN !== NaN should be true";
}

TEST_F(ECMAScriptComplianceTest, W3C_Number_IsNaN) {
    // Test Number.isNaN() function
    auto isNaNTrueResult = engine_->evaluateExpression(sessionId_, "Number.isNaN(NaN)").get();
    ASSERT_TRUE(isNaNTrueResult.isSuccess()) << "Number.isNaN(NaN) should work";
    EXPECT_TRUE(isNaNTrueResult.getValue<bool>()) << "Number.isNaN(NaN) should be true";

    auto isNaNFalseResult = engine_->evaluateExpression(sessionId_, "Number.isNaN(42)").get();
    ASSERT_TRUE(isNaNFalseResult.isSuccess());
    EXPECT_FALSE(isNaNFalseResult.getValue<bool>()) << "Number.isNaN(42) should be false";

    auto isNaNStringResult = engine_->evaluateExpression(sessionId_, "Number.isNaN('hello')").get();
    ASSERT_TRUE(isNaNStringResult.isSuccess());
    EXPECT_FALSE(isNaNStringResult.getValue<bool>()) << "Number.isNaN('hello') should be false (strict check)";

    // Test global isNaN() vs Number.isNaN() difference
    auto globalIsNaNResult = engine_->evaluateExpression(sessionId_, "isNaN('hello')").get();
    ASSERT_TRUE(globalIsNaNResult.isSuccess());
    EXPECT_TRUE(globalIsNaNResult.getValue<bool>()) << "isNaN('hello') should be true (coerces to NaN)";
}

TEST_F(ECMAScriptComplianceTest, W3C_Number_IsFinite) {
    // Test Number.isFinite() function
    auto isFiniteNumberResult = engine_->evaluateExpression(sessionId_, "Number.isFinite(42)").get();
    ASSERT_TRUE(isFiniteNumberResult.isSuccess()) << "Number.isFinite(42) should work";
    EXPECT_TRUE(isFiniteNumberResult.getValue<bool>()) << "Number.isFinite(42) should be true";

    auto isFiniteInfResult = engine_->evaluateExpression(sessionId_, "Number.isFinite(Infinity)").get();
    ASSERT_TRUE(isFiniteInfResult.isSuccess());
    EXPECT_FALSE(isFiniteInfResult.getValue<bool>()) << "Number.isFinite(Infinity) should be false";

    auto isFiniteNaNResult = engine_->evaluateExpression(sessionId_, "Number.isFinite(NaN)").get();
    ASSERT_TRUE(isFiniteNaNResult.isSuccess());
    EXPECT_FALSE(isFiniteNaNResult.getValue<bool>()) << "Number.isFinite(NaN) should be false";

    auto isFiniteNegInfResult = engine_->evaluateExpression(sessionId_, "Number.isFinite(-Infinity)").get();
    ASSERT_TRUE(isFiniteNegInfResult.isSuccess());
    EXPECT_FALSE(isFiniteNegInfResult.getValue<bool>()) << "Number.isFinite(-Infinity) should be false";
}

TEST_F(ECMAScriptComplianceTest, W3C_Number_IsInteger) {
    // Test Number.isInteger() function
    auto isIntegerTrueResult = engine_->evaluateExpression(sessionId_, "Number.isInteger(42)").get();
    ASSERT_TRUE(isIntegerTrueResult.isSuccess()) << "Number.isInteger(42) should work";
    EXPECT_TRUE(isIntegerTrueResult.getValue<bool>()) << "Number.isInteger(42) should be true";

    auto isIntegerFloatResult = engine_->evaluateExpression(sessionId_, "Number.isInteger(42.5)").get();
    ASSERT_TRUE(isIntegerFloatResult.isSuccess());
    EXPECT_FALSE(isIntegerFloatResult.getValue<bool>()) << "Number.isInteger(42.5) should be false";

    auto isIntegerInfResult = engine_->evaluateExpression(sessionId_, "Number.isInteger(Infinity)").get();
    ASSERT_TRUE(isIntegerInfResult.isSuccess());
    EXPECT_FALSE(isIntegerInfResult.getValue<bool>()) << "Number.isInteger(Infinity) should be false";

    auto isIntegerZeroResult = engine_->evaluateExpression(sessionId_, "Number.isInteger(0)").get();
    ASSERT_TRUE(isIntegerZeroResult.isSuccess());
    EXPECT_TRUE(isIntegerZeroResult.getValue<bool>()) << "Number.isInteger(0) should be true";
}

TEST_F(ECMAScriptComplianceTest, W3C_Number_MaxMinValues) {
    // Test Number.MAX_VALUE
    auto maxValueResult = engine_->evaluateExpression(sessionId_, "Number.MAX_VALUE > 0").get();
    ASSERT_TRUE(maxValueResult.isSuccess()) << "Number.MAX_VALUE should exist";
    EXPECT_TRUE(maxValueResult.getValue<bool>()) << "Number.MAX_VALUE should be positive";

    auto maxValueTypeResult = engine_->evaluateExpression(sessionId_, "typeof Number.MAX_VALUE").get();
    ASSERT_TRUE(maxValueTypeResult.isSuccess());
    EXPECT_EQ(maxValueTypeResult.getValue<std::string>(), "number") << "Number.MAX_VALUE should be number";

    // Test Number.MIN_VALUE
    auto minValueResult = engine_->evaluateExpression(sessionId_, "Number.MIN_VALUE > 0").get();
    ASSERT_TRUE(minValueResult.isSuccess()) << "Number.MIN_VALUE should exist";
    EXPECT_TRUE(minValueResult.getValue<bool>()) << "Number.MIN_VALUE should be positive (smallest positive value)";

    // Test beyond MAX_VALUE becomes Infinity
    auto beyondMaxResult = engine_->evaluateExpression(sessionId_, "Number.MAX_VALUE * 2 === Infinity").get();
    ASSERT_TRUE(beyondMaxResult.isSuccess());
    EXPECT_TRUE(beyondMaxResult.getValue<bool>()) << "Beyond MAX_VALUE should become Infinity";
}

TEST_F(ECMAScriptComplianceTest, W3C_Number_ComparisonEdgeCases) {
    // Test 0 === -0 (should be true)
    auto zeroEqualityResult = engine_->evaluateExpression(sessionId_, "0 === -0").get();
    ASSERT_TRUE(zeroEqualityResult.isSuccess());
    EXPECT_TRUE(zeroEqualityResult.getValue<bool>()) << "0 === -0 should be true";

    // Test null == undefined (should be true)
    auto nullUndefinedEqualResult = engine_->evaluateExpression(sessionId_, "null == undefined").get();
    ASSERT_TRUE(nullUndefinedEqualResult.isSuccess());
    EXPECT_TRUE(nullUndefinedEqualResult.getValue<bool>()) << "null == undefined should be true";

    // Test null === undefined (should be false)
    auto nullUndefinedStrictResult = engine_->evaluateExpression(sessionId_, "null === undefined").get();
    ASSERT_TRUE(nullUndefinedStrictResult.isSuccess());
    EXPECT_FALSE(nullUndefinedStrictResult.getValue<bool>()) << "null === undefined should be false";

    // Test Infinity comparisons
    auto infCompareResult = engine_->evaluateExpression(sessionId_, "Infinity > Number.MAX_VALUE").get();
    ASSERT_TRUE(infCompareResult.isSuccess());
    EXPECT_TRUE(infCompareResult.getValue<bool>()) << "Infinity should be greater than MAX_VALUE";
}
