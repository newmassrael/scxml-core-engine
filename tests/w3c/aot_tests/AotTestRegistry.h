#pragma once

#include "AotTestBase.h"
#include <functional>
#include <map>
#include <memory>
#include <string>

namespace RSM::W3C::AotTests {

/**
 * @brief Singleton registry for AOT tests
 *
 * Manages automatic registration and creation of AOT test instances.
 * Tests register themselves via REGISTER_AOT_TEST macro during static initialization.
 */
class AotTestRegistry {
public:
    using TestFactory = std::function<std::unique_ptr<AotTestBase>()>;

    /**
     * @brief Get singleton instance
     */
    static AotTestRegistry &instance() {
        static AotTestRegistry registry;
        return registry;
    }

    /**
     * @brief Register a test factory (string variant)
     * @param testId Test ID string (e.g., "144", "403a", "403b")
     * @param factory Function that creates test instance
     */
    void registerTest(const std::string &testId, TestFactory factory) {
        tests_[testId] = std::move(factory);
    }

    /**
     * @brief Register a test factory (int variant for backward compatibility)
     * @param testId Test number
     * @param factory Function that creates test instance
     */
    void registerTest(int testId, TestFactory factory) {
        registerTest(std::to_string(testId), std::move(factory));
    }

    /**
     * @brief Create test instance by ID (string variant)
     * @param testId Test ID string (e.g., "144", "403a", "403b")
     * @return Unique pointer to test instance, or nullptr if not found
     */
    std::unique_ptr<AotTestBase> createTest(const std::string &testId) const {
        auto it = tests_.find(testId);
        return it != tests_.end() ? it->second() : nullptr;
    }

    /**
     * @brief Create test instance by ID (int variant for backward compatibility)
     * @param testId Test number
     * @return Unique pointer to test instance, or nullptr if not found
     */
    std::unique_ptr<AotTestBase> createTest(int testId) const {
        return createTest(std::to_string(testId));
    }

    /**
     * @brief Check if test is registered (string variant)
     * @param testId Test ID string
     * @return true if test exists in registry
     */
    bool hasTest(const std::string &testId) const {
        return tests_.find(testId) != tests_.end();
    }

    /**
     * @brief Check if test is registered (int variant)
     * @param testId Test number
     * @return true if test exists in registry
     */
    bool hasTest(int testId) const {
        return hasTest(std::to_string(testId));
    }

    /**
     * @brief Get all registered test IDs
     * @return Vector of test ID strings
     */
    std::vector<std::string> getAllTestIds() const {
        std::vector<std::string> ids;
        ids.reserve(tests_.size());
        for (const auto &[id, _] : tests_) {
            ids.push_back(id);
        }
        return ids;
    }

private:
    AotTestRegistry() = default;
    std::map<std::string, TestFactory> tests_;
};

/**
 * @brief Automatic test registration helper
 *
 * Usage in test file:
 * @code
 * struct Test144 : public AotTestBase {
 *     static constexpr int TEST_ID = 144;
 *     static constexpr const char* DESCRIPTION = "Event queue ordering";
 *     // ... implement interface
 * };
 * REGISTER_AOT_TEST(Test144);
 *
 * // For variant tests (403a, 403b, 403c):
 * struct Test403a : public AotTestBase {
 *     static constexpr int TEST_ID = 403;
 *     // ... implement interface
 * };
 * inline static AotTestRegistrar<Test403a> registrar_Test403a("403a");
 * @endcode
 */
template <typename TestClass> struct AotTestRegistrar {
    // Default constructor: use TestClass::TEST_ID (int or convertible to int)
    AotTestRegistrar() {
        AotTestRegistry::instance().registerTest(TestClass::TEST_ID, []() { return std::make_unique<TestClass>(); });
    }

    // String variant constructor: for variant tests like "403a", "403b", "403c"
    explicit AotTestRegistrar(const std::string &testId) {
        AotTestRegistry::instance().registerTest(testId, []() { return std::make_unique<TestClass>(); });
    }
};

}  // namespace RSM::W3C::AotTests

/**
 * @brief Macro to auto-register a AOT test
 *
 * Creates static instance that registers test during program initialization.
 * Must be used at global namespace scope.
 */
#define REGISTER_AOT_TEST(TestClass)                                                                                   \
    namespace {                                                                                                        \
    static ::RSM::W3C::AotTests::AotTestRegistrar<TestClass> registrar_##TestClass{};                                  \
    }
