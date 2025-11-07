#include "common/UrlEncodingHelper.h"

#include <cctype>
#include <iomanip>
#include <sstream>

namespace SCE {

std::string UrlEncodingHelper::urlEncode(const std::string &str) {
    std::ostringstream escaped;
    escaped.fill('0');
    escaped << std::hex;

    for (char c : str) {
        // RFC 3986: Unreserved characters don't need encoding
        // unreserved = ALPHA / DIGIT / "-" / "." / "_" / "~"
        if (std::isalnum(static_cast<unsigned char>(c)) || c == '-' || c == '_' || c == '.' || c == '~') {
            escaped << c;
        } else {
            // Percent-encode as %XX
            escaped << std::uppercase;
            escaped << '%' << std::setw(2) << static_cast<int>(static_cast<unsigned char>(c));
            escaped << std::nouppercase;
        }
    }

    return escaped.str();
}

}  // namespace SCE
