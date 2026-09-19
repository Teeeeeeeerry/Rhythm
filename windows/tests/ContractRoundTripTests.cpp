// CR：契约编解码往返（manifest: docs/testing/behavior/ffi.md「契约编解码往返」，#365）。
//
// 用例里不写对象名也不写字段名：对象经生成物的 ForEachContractObject 遍历，字段经
// ForEachField 逐个赋值；契约文件本身在运行时读入，作为「声明了哪些对象、哪些字段」的
// 对照。契约新增一个对象或字段，重新生成后本组自动把它纳入。

#include "BehaviorPch.h"
#include "Bridge/GeneratedCodec.h"

#include <catch_amalgamated.hpp>

#include <cstring>
#include <fstream>
#include <set>

using nlohmann::json;
using namespace rhythm;

namespace {

/// contracts/ffi-contract.json (path injected by the build).
const json& Contract() {
    static const json contract = [] {
        std::ifstream in(RHYTHM_FFI_CONTRACT);
        REQUIRE(in.good());
        return json::parse(in);
    }();
    return contract;
}

bool IsObjectKey(const std::string& key) {
    return key != "version" && key != "doc" && key != "enums";
}

std::set<std::string> DeclaredObjects() {
    std::set<std::string> objects;
    for (const auto& [key, value] : Contract().items()) {
        if (IsObjectKey(key)) objects.insert(key);
    }
    return objects;
}

std::set<std::string> DeclaredFields(const std::string& object) {
    std::set<std::string> fields;
    for (const auto& [field, type] : Contract()[object].items()) fields.insert(field);
    return fields;
}

std::set<std::string> Keys(const json& object) {
    std::set<std::string> keys;
    for (const auto& [key, value] : object.items()) keys.insert(key);
    return keys;
}

std::wstring Widen(const char* ascii) {
    return std::wstring(ascii, ascii + std::strlen(ascii));
}

/// Gives every member of a model a value, optionals included, and records the
/// JSON the encoder must write for it. No value is what decoding a missing
/// field yields, and numbers differ per field, so a dropped field or two
/// fields of one type trading places cannot round-trip by accident.
struct FullFiller {
    json& expected;
    int& serial;

    void operator()(const char* key, std::wstring& member) {
        member = Widen(key) + L"-值";
        expected[key] = std::string(key) + "-值";
    }
    void operator()(const char* key, int32_t& member) {
        member = 100 + serial++;
        expected[key] = member;
    }
    void operator()(const char* key, int64_t& member) {
        member = (int64_t{1} << 40) + serial++;  // does not fit in 32 bits
        expected[key] = member;
    }
    void operator()(const char* key, double& member) {
        member = 0.5 + serial++;
        expected[key] = member;
    }
    void operator()(const char* key, bool& member) {
        member = true;  // a missing bool decodes to false
        expected[key] = true;
    }
    void operator()(const char* key, std::map<std::wstring, std::wstring>& member) {
        member = {{L"Referer", L"https://example.com/页面"}, {L"User-Agent", L"Rhythm"}};
        expected[key] = {{"Referer", "https://example.com/页面"}, {"User-Agent", "Rhythm"}};
    }
    template <typename T>
    void operator()(const char* key, std::optional<T>& member) {
        T value{};
        (*this)(key, value);
        member = std::move(value);
    }
    /// A field holding another contract object.
    template <typename Model>
        requires requires(Model& model, FullFiller& filler) { generated::ForEachField(model, filler); }
    void operator()(const char* key, Model& member) {
        json nested = json::object();
        FullFiller inner{nested, serial};
        generated::ForEachField(member, inner);
        expected[key] = nested;
    }
};

} // namespace

// ─── CR-01 契约对象与生成的编解码一一对应（#365）──────────────────────

TEST_CASE("CR-01 every contract object has a generated codec, and nothing else does") {
    std::set<std::string> visited;
    generated::ForEachContractObject([&](const char* key, auto, auto) { visited.insert(key); });

    REQUIRE_FALSE(visited.empty());
    REQUIRE(visited == DeclaredObjects());
}

// ─── CR-02 全字段有值的往返（#365）──────────────────────────────────

TEST_CASE("CR-02 every contract object round-trips with every field set, optionals included") {
    generated::ForEachContractObject([&](const char* key, auto fromJson, auto toJson) {
        INFO(key);
        using Model = decltype(fromJson(json{}));

        // Construct the object member by member.
        Model object{};
        json expected = json::object();
        int serial = 0;
        FullFiller filler{expected, serial};
        generated::ForEachField(object, filler);
        REQUIRE(Keys(expected) == DeclaredFields(key));

        // Encode: every field under its own key, number kinds included.
        auto encoded = toJson(object);
        REQUIRE(encoded.dump() == expected.dump());

        // Decode: the same object back, field by field.
        REQUIRE(fromJson(encoded) == object);
    });
}
