#include "common/Logger.h"
#include "parsing/IXMLParser.h"
#include "parsing/PugiXMLParser.h"

namespace RSM {

std::shared_ptr<IXMLParser> IXMLParser::create() {
    LOG_DEBUG("Creating PugiXMLParser (unified for all platforms)");
    return std::make_shared<PugiXMLParser>();
}

}  // namespace RSM
