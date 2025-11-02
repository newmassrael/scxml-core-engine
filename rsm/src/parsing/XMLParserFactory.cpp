#include "parsing/IXMLParser.h"

#ifdef __EMSCRIPTEN__
#include "parsing/PugiXMLParser.h"
#else
#include "parsing/LibXMLParser.h"
#endif

#include "common/Logger.h"

namespace RSM {

std::shared_ptr<IXMLParser> IXMLParser::create() {
#ifdef __EMSCRIPTEN__
    LOG_DEBUG("Creating PugiXMLParser for WASM build");
    return std::make_shared<PugiXMLParser>();
#else
    LOG_DEBUG("Creating LibXMLParser for native build");
    return std::make_shared<LibXMLParser>();
#endif
}

}  // namespace RSM
