#pragma once

#include <chrono>
#include <fstream>
#include <sstream>
#include <string>

namespace SCE::W3C::AotTests {

/**
 * @brief Base interface for AOT engine tests
 *
 * All AOT tests inherit from this interface and implement the run() method.
 * Tests are automatically registered via REGISTER_AOT_TEST macro.
 */
class AotTestBase {
public:
    virtual ~AotTestBase() = default;

    /**
     * @brief Load test description from metadata.txt (Single Source of Truth)
     * @param testId Test number (e.g., 144, 175)
     * @return Formatted description "W3C SCXML X.Y: description text" or "... (Manual)"
     *
     * Reads from resources/{testId}/metadata.txt and extracts:
     * - specnum: W3C SCXML specification section
     * - description: Full W3C specification text
     * - manual: Manual test flag (adds " (Manual)" suffix)
     *
     * This implements metadata.txt as the Single Source of Truth for test descriptions,
     * ensuring 100% consistency between Interpreter and AOT engines.
     */
    static std::string loadMetadataDescription(int testId) {
        std::ostringstream pathStream;
        pathStream << "resources/" << testId << "/metadata.txt";
        std::string metadataPath = pathStream.str();

        std::ifstream file(metadataPath);
        if (!file.is_open()) {
            // Fallback if metadata.txt not found
            std::ostringstream fallback;
            fallback << "Test " << testId << " (metadata.txt not found at: " << metadataPath << ")";
            return fallback.str();
        }

        std::string specnum;
        std::string description;
        bool isManual = false;
        std::string line;

        while (std::getline(file, line)) {
            if (line.find("specnum:") == 0) {
                specnum = line.substr(9);  // Skip "specnum: "
            } else if (line.find("description:") == 0) {
                description = line.substr(13);  // Skip "description: "
            } else if (line.find("manual:") == 0) {
                std::string manualValue = line.substr(8);  // Skip "manual: "
                isManual = (manualValue == "True");
            }
        }

        if (!specnum.empty() && !description.empty()) {
            std::ostringstream result;
            result << "W3C SCXML " << specnum << ": " << description;
            if (isManual) {
                result << " (Manual)";
            }
            return result.str();
        } else if (!description.empty()) {
            if (isManual) {
                return description + " (Manual)";
            }
            return description;
        } else {
            std::ostringstream fallback;
            fallback << "Test " << testId;
            return fallback.str();
        }
    }

    /**
     * @brief Execute the AOT test
     * @return true if test passed, false otherwise
     */
    virtual bool run() = 0;

    /**
     * @brief Get test ID
     * @return Test number (e.g., 144, 147)
     */
    virtual int getTestId() const = 0;

    /**
     * @brief Get test description
     * @return Human-readable test description
     */
    virtual const char *getDescription() const = 0;

    /**
     * @brief Get timeout duration for this test
     * @return Timeout in seconds (default: 2 seconds)
     */
    virtual std::chrono::seconds getTimeout() const {
        return std::chrono::seconds(2);
    }

    /**
     * @brief Check if test requires event scheduler polling
     * @return true if test uses delayed send/invoke
     */
    virtual bool needsSchedulerPolling() const {
        return false;
    }

    /**
     * @brief Get test type: pure_static, static_hybrid, or interpreter_fallback
     * @return Test type string for XML reporting
     */
    virtual const char *getTestType() const {
        return "pure_static";  // Default for most tests
    }
};

}  // namespace SCE::W3C::AotTests
