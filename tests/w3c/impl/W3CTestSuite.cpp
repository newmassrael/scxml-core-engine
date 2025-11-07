#include "../interfaces/ITestSuite.h"
#include <algorithm>
#include <filesystem>
#include <sstream>

namespace SCE::W3C {

/**
 * @brief W3C test suite implementation
 */
class W3CTestSuite : public ITestSuite {
private:
    std::string resourcePath_;

public:
    explicit W3CTestSuite(const std::string &resourcePath = "resources") : resourcePath_(resourcePath) {}

    ~W3CTestSuite() override = default;

    TestSuiteInfo getInfo() const override {
        TestSuiteInfo info;
        info.name = "W3C SCXML Test Suite";
        info.description = "Official W3C SCXML 1.0 Conformance Tests";
        info.resourcePath = resourcePath_;

        // Count available tests
        try {
            auto tests = const_cast<W3CTestSuite *>(this)->discoverTests();
            info.totalTests = tests.size();
        } catch (...) {
            info.totalTests = 0;
        }

        return info;
    }

    std::vector<std::string> discoverTests() override {
        std::vector<std::string> testDirs;

        try {
            for (const auto &entry : std::filesystem::directory_iterator(resourcePath_)) {
                if (entry.is_directory()) {
                    std::string dirName = entry.path().filename().string();

                    // Check if directory name is numeric (W3C test ID)
                    if (isNumericTestDir(dirName)) {
                        // W3C SCXML: Check for variant test files (test403a.txml, test403b.txml, etc.)
                        // Format: testNNNx.txml where NNN is test ID and x is variant suffix (a,b,c,...)
                        std::string dirPath = entry.path().string();
                        std::string metadataPath = getMetadataPath(dirPath);

                        // Metadata must exist for all tests
                        if (!std::filesystem::exists(metadataPath)) {
                            continue;
                        }

                        int testId = extractTestId(dirPath);
                        std::string testPrefix = "test" + std::to_string(testId);

                        // Scan for all testNNN*.txml files
                        std::vector<std::string> txmlFiles;
                        for (const auto &fileEntry : std::filesystem::directory_iterator(dirPath)) {
                            if (fileEntry.is_regular_file() && fileEntry.path().extension() == ".txml") {
                                std::string filename = fileEntry.path().filename().string();
                                if (filename.find(testPrefix) == 0) {
                                    txmlFiles.push_back(filename);
                                }
                            }
                        }

                        // Process each variant
                        for (const auto &txmlFile : txmlFiles) {
                            // Extract variant suffix if present
                            // e.g., "test403a.txml" → variant suffix is "a"
                            // e.g., "test403.txml" → no variant suffix
                            std::string stem = txmlFile.substr(0, txmlFile.find(".txml"));  // "test403a" or "test403"

                            if (stem == testPrefix) {
                                // Base file without variant suffix
                                testDirs.push_back(dirPath);
                            } else if (stem.length() > testPrefix.length()) {
                                // Variant file with suffix
                                std::string variantSuffix = stem.substr(testPrefix.length());  // "a", "b", "c", etc.
                                // Format: "dirPath:variantSuffix" (e.g., "resources/403:a")
                                testDirs.push_back(dirPath + ":" + variantSuffix);
                            }
                        }
                    }
                }
            }
        } catch (const std::exception &e) {
            throw std::runtime_error("Failed to discover W3C tests: " + std::string(e.what()));
        }

        // Sort test directories by test ID and variant
        std::sort(testDirs.begin(), testDirs.end(), [](const std::string &a, const std::string &b) {
            // Extract base test ID and variant suffix
            auto extractIdAndVariant = [](const std::string &path) -> std::pair<int, std::string> {
                size_t colonPos = path.find(':');
                std::string basePath = (colonPos != std::string::npos) ? path.substr(0, colonPos) : path;
                std::string variant = (colonPos != std::string::npos) ? path.substr(colonPos + 1) : "";
                int id = extractTestId(basePath);
                return {id, variant};
            };

            auto [idA, variantA] = extractIdAndVariant(a);
            auto [idB, variantB] = extractIdAndVariant(b);

            // Sort by test ID first, then by variant suffix
            if (idA != idB) {
                return idA < idB;
            }
            return variantA < variantB;
        });

        return testDirs;
    }

    std::string getTXMLPath(const std::string &testDirectory) override {
        // Handle variant format: "dirPath:variant" (e.g., "resources/403:a")
        size_t colonPos = testDirectory.find(':');
        std::string basePath = testDirectory;
        std::string variantSuffix = "";

        if (colonPos != std::string::npos) {
            basePath = testDirectory.substr(0, colonPos);        // "resources/403"
            variantSuffix = testDirectory.substr(colonPos + 1);  // "a"
        }

        int testId = extractTestId(basePath);
        // For variant: "resources/403/test403a.txml"
        // For base:    "resources/403/test403.txml"
        return basePath + "/test" + std::to_string(testId) + variantSuffix + ".txml";
    }

    std::string getMetadataPath(const std::string &testDirectory) override {
        // Handle variant format: "dirPath:variant" (e.g., "resources/403:a")
        // Metadata is shared across variants, so strip variant suffix
        size_t colonPos = testDirectory.find(':');
        std::string basePath = (colonPos != std::string::npos) ? testDirectory.substr(0, colonPos) : testDirectory;
        return basePath + "/metadata.txt";
    }

    std::vector<std::string> filterTests(const std::string &conformanceLevel, const std::string &specSection) override {
        auto allTests = discoverTests();

        if (conformanceLevel.empty() && specSection.empty()) {
            return allTests;
        }

        std::vector<std::string> filteredTests;

        // For now, return all tests
        // TODO: Implement filtering based on metadata parsing
        return allTests;
    }

private:
    bool isNumericTestDir(const std::string &dirName) {
        return !dirName.empty() && std::all_of(dirName.begin(), dirName.end(), ::isdigit);
    }

    static int extractTestId(const std::string &testPath) {
        std::filesystem::path path(testPath);
        std::string dirName = path.filename().string();

        try {
            return std::stoi(dirName);
        } catch (...) {
            return 0;
        }
    }
};

}  // namespace SCE::W3C