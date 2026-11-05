// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
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
 *   - InlineText — `<content>children</content>`; the children take the
 *                  one reading in-line content has: the value of the
 *                  expression they are when the data model reads them as
 *                  one, the XML as written when they open with `<`, and
 *                  their space-normalized text otherwise. The same reading
 *                  a `<send>`'s in-line `<content>` gets, and the one the
 *                  code generator's `DoneDataContent::InlineText` names.
 *                  Under the `null` data model, which has no expressions,
 *                  the first step reads none
 *                  (`DataModelInitHelper::nullDataModelExpressionReading`),
 *                  so no script engine is asked.
 *
 * Reading in-line children as an Expression was a spec misreading:
 * `<content>hello world</content>` could only fail to evaluate, where the
 * generated engines carry the string. A separate verbatim reading under the
 * `null` data model was the same misreading the other way: the text went
 * unnormalized where every generated machine normalizes it.
 */
class DoneData {
public:
    enum class ContentKind { None, Expression, InlineText };

    DoneData() = default;
    ~DoneData() = default;

    /// §scxml-5.5: `<content expr="X"/>` — evaluate X against the datamodel.
    void setContentExpression(const std::string &expr) {
        content_ = expr;
        contentKind_ = ContentKind::Expression;
        contentUnderNull_ = false;
    }

    /// `<content>children</content>` — read at run time as in-line content is
    /// read (see `ContentKind::InlineText`), with the expression step answered
    /// by the `null` data model when `underNullDatamodel` is true.
    void setContentInlineText(const std::string &text, bool underNullDatamodel) {
        content_ = text;
        contentKind_ = ContentKind::InlineText;
        contentUnderNull_ = underNullDatamodel;
    }

    /// Whether in-line children are read under the `null` data model.
    bool isContentUnderNullDatamodel() const {
        return contentUnderNull_;
    }

    /// Clear content (used by XOR-resolution in `DoneDataParser`).
    void clearContent() {
        content_.clear();
        contentKind_ = ContentKind::None;
        contentUnderNull_ = false;
    }

    /// Raw payload (expression source or in-line children, depending on
    /// kind).
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
    bool contentUnderNull_ = false;
};

}  // namespace SCE
