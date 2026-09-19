// CR：契约编解码往返（manifest: docs/testing/behavior/ffi.md「契约编解码往返」，#365）。
//
// 用例里不写对象名：哪些对象、各有哪些字段，运行时读契约文件本身；编解码经生成物的
// ForEachContractObject 取得。契约新增一个对象，重新生成后本组自动把它纳入。

#include "BehaviorPch.h"
#include "Bridge/GeneratedCodec.h"

#include <catch_amalgamated.hpp>

#include <fstream>
#include <functional>
#include <map>
#include <set>

using nlohmann::json;
using namespace rhythm;

namespace {

/// contracts/ffi-contract.json, the oracle for which objects exist and what
/// their fields are (path injected by the build).
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

json FullSample(const std::string& object);

/// A value of the declared type that decoding a missing field would never
/// produce, so a field the codec drops cannot round-trip by accident.
json FullValue(const std::string& field, const std::string& declared) {
    std::string type = declared;
    if (!type.empty() && type.back() == '?') type.pop_back();

    if (type == "string") return field + "-值";           // unique per field, non-ASCII
    if (type == "i32") return 7;
    if (type == "i64") return int64_t{1} << 40;           // does not fit in 32 bits
    if (type == "f64") return 12.5;
    if (type == "bool") return true;                      // a missing bool decodes to false
    if (type == "map") return json{{"Referer", "https://example.com/页面"}, {"User-Agent", "Rhythm"}};
    const json& enums = Contract()["enums"];
    if (enums.contains(type)) return enums[type].back();  // the first value is the missing default
    if (Contract().contains(type) && IsObjectKey(type)) return FullSample(type);
    FAIL("unknown contract type " << declared << " for " << field);
    return nullptr;
}

/// An object with every declared field set, optional ones included.
json FullSample(const std::string& object) {
    json sample = json::object();
    for (const auto& [field, type] : Contract()[object].items()) {
        sample[field] = FullValue(field, type.get<std::string>());
    }
    return sample;
}

/// One generated codec, type-erased so every object runs the same checks.
struct Codec {
    /// ToJson(FromJson(j)).
    std::function<json(const json&)> reencode;
    /// With o = FromJson(j): FromJson(ToJson(o)) == o, field by field.
    std::function<bool(const json&)> survivesRoundTrip;
};

std::map<std::string, Codec> GeneratedCodecs() {
    std::map<std::string, Codec> codecs;
    generated::ForEachContractObject([&](const char* key, auto fromJson, auto toJson) {
        codecs[key] = Codec{
            [=](const json& j) { return toJson(fromJson(j)); },
            [=](const json& j) {
                auto object = fromJson(j);
                return fromJson(toJson(object)) == object;
            },
        };
    });
    return codecs;
}

} // namespace

// ─── CR-01 契约对象与生成的编解码一一对应（#365）──────────────────────

TEST_CASE("CR-01 every contract object has a generated codec, and nothing else does") {
    std::set<std::string> declared;
    for (const auto& [key, value] : Contract().items()) {
        if (IsObjectKey(key)) declared.insert(key);
    }
    std::set<std::string> generatedKeys;
    for (const auto& [key, codec] : GeneratedCodecs()) generatedKeys.insert(key);

    REQUIRE_FALSE(declared.empty());
    REQUIRE(generatedKeys == declared);
}

// ─── CR-02 全字段有值的往返（#365）──────────────────────────────────

TEST_CASE("CR-02 every contract object round-trips with every field set, optionals included") {
    for (const auto& [key, codec] : GeneratedCodecs()) {
        INFO(key);
        auto full = FullSample(key);
        // Every declared field is decoded and encoded back unchanged...
        REQUIRE(codec.reencode(full) == full);
        // ...and encoding then decoding the object gives the same object.
        REQUIRE(codec.survivesRoundTrip(full));
    }
}
