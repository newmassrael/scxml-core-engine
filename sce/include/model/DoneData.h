// SPDX-License-Identifier: LGPL-2.1-or-later WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2025 newmassrael

#pragma once

#include <memory>
#include <string>
#include <utility>
#include <vector>

namespace SCE {

class IDataModelItem;

/**
 * @brief Class storing information for <donedata> element (§scxml-5.5).
 *
 * Represents the inline `<content>` semantic as a kind-tagged value:
 *
 *   - None       — `<content>` absent.
 *   - Expression — `<content expr="X"/>`; X MUST be evaluated against the
 *                  datamodel at runtime (requires a script engine).
 *   - Literal    — `<content>inline text</content>`; per §scxml-5.5 the
 *                  children are used **as the content value** — no
 *                  evaluation, no script engine required.
 *
 * Collapsing the two into a single string was a spec misreading: the
 * literal path was silently routed through `evaluateExpression` which
 * forced a script engine even when the document only needed to emit a
 * fixed verdict payload.
 */
class DoneData {
public:
    enum class ContentKind { None, Expression, Literal };

    DoneData() = default;
    ~DoneData() = default;

    /// §scxml-5.5: `<content expr="X"/>` — evaluate X against the datamodel.
    void setContentExpression(const std::string &expr) {
        content_ = expr;
        contentKind_ = ContentKind::Expression;
    }

    /// §scxml-5.5: `<content>inline text</content>` — children are the value.
    void setContentLiteral(const std::string &literal) {
        content_ = literal;
        contentKind_ = ContentKind::Literal;
    }

    /// Clear content (used by XOR-resolution in `DoneDataParser`).
    void clearContent() {
        content_.clear();
        contentKind_ = ContentKind::None;
    }

    /// Raw payload (expression source or literal text, depending on kind).
    const std::string &getContent() const {
        return content_;
    }

    ContentKind getContentKind() const {
        return contentKind_;
    }

    bool hasContent() const {
        return contentKind_ != ContentKind::None;
    }

    void addParam(const std::string &name, const std::string &location) {
        params_.push_back(std::make_pair(name, location));
    }

    const std::vector<std::pair<std::string, std::string>> &getParams() const {
        return params_;
    }

    bool isEmpty() const {
        return contentKind_ == ContentKind::None && params_.empty();
    }

    void clearParams() {
        params_.clear();
    }

private:
    std::string content_;
    std::vector<std::pair<std::string, std::string>> params_;
    ContentKind contentKind_ = ContentKind::None;
};

}  // namespace SCE
