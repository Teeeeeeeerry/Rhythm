// 门的第二条的反面证据（#369）：人为注入一个类型错误，构建必须失败。
//
// 这里复刻的就是 #323 记录的那种缺陷——生成器对可选字段一律走「转成宽字符串」，
// 于是把整型字段交给一个只接受 const std::wstring& 的函数。它由 EXCLUDE_FROM_ALL
// 的目标 RhythmGeneratedCodecTypeError 承载，默认构建不碰；ctest 用例
// GeneratedCodecTypeErrorFailsTheBuild 去构建它并断言构建失败。
// 这个文件一旦编译得过，说明门没有牙齿。
#include "Bridge/GeneratedCodec.h"

namespace {

nlohmann::json EncodeYearLikeTheBrokenGenerator(const rhythm::Track& track) {
    nlohmann::json j;
    // int32_t -> const std::wstring&：语言层面没有可行转换。
    j["year"] = rhythm::WideToUtf8(*track.year);
    return j;
}

} // namespace
