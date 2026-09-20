// CR：契约编解码往返（manifest: docs/testing/behavior/ffi.md「契约编解码往返」，#365/#366）。
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

/// The object's declared fields whose type passes `keep`.
template <typename Keep>
std::set<std::string> FieldsWhere(const std::string& object, Keep keep) {
    std::set<std::string> fields;
    for (const auto& [field, type] : Contract()[object].items()) {
        if (keep(type.get<std::string>())) fields.insert(field);
    }
    return fields;
}

std::set<std::string> DeclaredFields(const std::string& object) {
    return FieldsWhere(object, [](const std::string&) { return true; });
}

/// Declared optional (`?`, #366).
std::set<std::string> OptionalFields(const std::string& object) {
    return FieldsWhere(object, [](const std::string& type) { return type.ends_with('?'); });
}

/// Declared as optional integers (`i32?` / `i64?`, #366).
std::set<std::string> IntegerOptionalFields(const std::string& object) {
    return FieldsWhere(object, [](const std::string& type) { return type == "i32?" || type == "i64?"; });
}

/// `object` with each of `fields` set to an explicit null (#366).
json WithNulls(json object, const std::set<std::string>& fields) {
    for (const auto& field : fields) object[field] = nullptr;
    return object;
}

std::set<std::string> Keys(const json& object) {
    std::set<std::string> keys;
    for (const auto& [key, value] : object.items()) keys.insert(key);
    return keys;
}

std::wstring Widen(const char* ascii) {
    return std::wstring(ascii, ascii + std::strlen(ascii));
}

/// Gives every member of a model a value -- optionals too, unless told to
/// leave them empty (#366) -- and records the JSON the encoder must write for
/// it. No value is what decoding a missing field yields, and numbers differ
/// per field, so a dropped field or two fields of one type trading places
/// cannot round-trip by accident.
struct Filler {
    json& expected;
    int& serial;
    bool withOptionals = true;

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
        if (!withOptionals) return;
        T value{};
        (*this)(key, value);
        member = std::move(value);
    }
    /// A field holding another contract object.
    template <typename Model>
        requires requires(Model& model, Filler& filler) { generated::ForEachField(model, filler); }
    void operator()(const char* key, Model& member) {
        json nested = json::object();
        Filler inner{nested, serial, withOptionals};
        generated::ForEachField(member, inner);
        expected[key] = nested;
    }
    /// A field holding a list of contract objects (#367). One element is
    /// enough to prove the element codec is the contract's own: an empty
    /// list would round-trip through any encoder.
    template <typename Model>
        requires requires(Model& model, Filler& filler) { generated::ForEachField(model, filler); }
    void operator()(const char* key, std::vector<Model>& member) {
        json nested = json::object();
        Model element{};
        Filler inner{nested, serial, withOptionals};
        generated::ForEachField(element, inner);
        member.push_back(std::move(element));
        expected[key] = json::array({nested});
    }
};

/// A model with only its required fields set, and the JSON that encodes it.
template <typename Model>
Model RequiredOnly(json& expected) {
    Model object{};
    expected = json::object();
    int serial = 0;
    Filler filler{.expected = expected, .serial = serial, .withOptionals = false};
    generated::ForEachField(object, filler);
    return object;
}

/// Reads every integer optional member of a model, keyed by contract field.
struct IntegerOptionals {
    std::map<std::string, std::optional<int64_t>> values;

    void operator()(const char* key, std::optional<int32_t>& member) {
        values[key] = member ? std::optional<int64_t>(*member) : std::nullopt;
    }
    void operator()(const char* key, std::optional<int64_t>& member) { values[key] = member; }
    template <typename T>
    void operator()(const char*, T&) {}

    std::set<std::string> Fields() const {
        std::set<std::string> fields;
        for (const auto& [field, value] : values) fields.insert(field);
        return fields;
    }
};

/// Sets every integer optional member of a model to an explicit 0.
struct ZeroIntegerOptionals {
    void operator()(const char*, std::optional<int32_t>& member) { member = 0; }
    void operator()(const char*, std::optional<int64_t>& member) { member = 0; }
    template <typename T>
    void operator()(const char*, T&) {}
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
        Filler filler{expected, serial};
        generated::ForEachField(object, filler);
        REQUIRE(Keys(expected) == DeclaredFields(key));

        // Encode: every field under its own key, number kinds included.
        auto encoded = toJson(object);
        REQUIRE(encoded.dump() == expected.dump());

        // Decode: the same object back, field by field.
        REQUIRE(fromJson(encoded) == object);
    });
}

// ─── CR-03 缺省的可选字段往返后仍缺省（#366）─────────────────────────

TEST_CASE("CR-03 optional fields left empty stay absent through a round trip") {
    size_t optionals = 0;
    generated::ForEachContractObject([&](const char* key, auto fromJson, auto toJson) {
        INFO(key);
        json expected;
        auto object = RequiredOnly<decltype(fromJson(json{}))>(expected);

        // Encode: an empty optional is left out, not written as null, "" or 0.
        auto encoded = toJson(object);
        for (const auto& field : OptionalFields(key)) {
            INFO(field);
            REQUIRE_FALSE(encoded.contains(field));
            ++optionals;
        }
        REQUIRE(encoded.dump() == expected.dump());

        // Decode: still empty.
        REQUIRE(fromJson(encoded) == object);
    });
    REQUIRE(optionals > 0);
}

// ─── CR-04 显式空值往返后只会变成缺省（#366）─────────────────────────

TEST_CASE("CR-04 an explicit null decodes like an absent field and re-encodes as absent") {
    size_t nulledFields = 0;
    generated::ForEachContractObject([&](const char* key, auto fromJson, auto toJson) {
        INFO(key);
        json expected;
        auto object = RequiredOnly<decltype(fromJson(json{}))>(expected);

        const auto optionals = OptionalFields(key);
        nulledFields += optionals.size();

        // null is "no value": the same object as leaving the field out...
        auto decoded = fromJson(WithNulls(expected, optionals));
        REQUIRE(decoded == object);
        // ...and it comes back absent, never as null or as some default.
        REQUIRE(toJson(decoded).dump() == expected.dump());
    });
    REQUIRE(nulledFields > 0);
}

// ─── CR-05 整型可选字段的缺省形态（#366）─────────────────────────────

TEST_CASE("CR-05 integer optionals: absent and null stay empty, an explicit 0 stays 0") {
    size_t checked = 0;
    generated::ForEachContractObject([&](const char* key, auto fromJson, auto toJson) {
        const auto integers = IntegerOptionalFields(key);
        if (integers.empty()) return;
        INFO(key);
        json expected;
        auto object = RequiredOnly<decltype(fromJson(json{}))>(expected);

        // The model's integer optionals are exactly the contract's i32?/i64? fields.
        IntegerOptionals declared;
        generated::ForEachField(object, declared);
        REQUIRE(declared.Fields() == integers);

        // Absent and null both decode to empty -- not 0 -- and stay out when encoded.
        for (const json& input : {expected, WithNulls(expected, integers)}) {
            INFO(input.dump());
            auto decoded = fromJson(input);
            IntegerOptionals read;
            generated::ForEachField(decoded, read);
            for (const auto& [field, value] : read.values) {
                INFO(field);
                REQUIRE_FALSE(value.has_value());
            }
            auto encoded = toJson(decoded);
            for (const auto& field : integers) REQUIRE_FALSE(encoded.contains(field));
        }

        // 0 is a value like any other: written, and read back as 0.
        ZeroIntegerOptionals zero;
        generated::ForEachField(object, zero);
        auto encoded = toJson(object);
        for (const auto& field : integers) {
            INFO(field);
            REQUIRE(encoded.contains(field));
            REQUIRE(encoded[field] == 0);
        }
        auto decoded = fromJson(encoded);
        IntegerOptionals read;
        generated::ForEachField(decoded, read);
        for (const auto& [field, value] : read.values) {
            INFO(field);
            REQUIRE(value == int64_t{0});
        }
        checked += integers.size();
    });
    REQUIRE(checked > 0);
}
