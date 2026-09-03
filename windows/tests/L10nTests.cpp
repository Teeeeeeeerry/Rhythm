// LK-01~06：Windows 文案层（manifest: docs/testing/behavior/l10n-keys.md）。
// 零接缝：直接测 L10n 字符串，用 SetOverrideLanguage 固定中/英分支，
// 不依赖系统语言；结束恢复原值。
//
// 这些测试在本机（macOS）不可运行——提交后在 Windows 上 `ctest` 验证。

#include "pch.h"
#include "L10n.h"

#include <catch_amalgamated.hpp>

using namespace rhythm;

namespace {

/// Pin the language for a scope, restoring the previous override after.
struct LanguageScope {
    std::wstring previous = L10n::OverrideLanguage();

    explicit LanguageScope(const wchar_t* code) {
        L10n::SetOverrideLanguage(code);
    }

    ~LanguageScope() {
        L10n::SetOverrideLanguage(previous);
    }
};

} // namespace

// ─── WA-26 基础文案中/英分支 ────────────────────────────────────────

TEST_CASE("LK-01 L10n static strings differ per language") {
    {
        LanguageScope zh(L"zh-CN");
        REQUIRE(L10n::LibraryTab() == L"资料库");
        REQUIRE(L10n::PlaylistsTab() == L"播放列表");
        REQUIRE(L10n::SearchPlaceholder() == L"搜索...");
        REQUIRE(L10n::ByArtistAlbum() == L"按艺人/专辑");
        REQUIRE(L10n::ByLetter() == L"按首字母");
        REQUIRE(L10n::NotPlaying() == L"未在播放");
        REQUIRE(L10n::PlayUrl() == L"播放链接");
        REQUIRE(L10n::NewPlaylist() == L"新建播放列表");
        REQUIRE(L10n::Create() == L"创建");
        REQUIRE(L10n::Cancel() == L"取消");
        REQUIRE(L10n::UrlErrorTitle() == L"无法播放链接");
        REQUIRE(L10n::Ok() == L"确定");
    }
    {
        LanguageScope en(L"en");
        REQUIRE(L10n::LibraryTab() == L"Library");
        REQUIRE(L10n::PlaylistsTab() == L"Playlists");
        REQUIRE(L10n::SearchPlaceholder() == L"Search...");
        REQUIRE(L10n::ByArtistAlbum() == L"By Artist/Album");
        REQUIRE(L10n::ByLetter() == L"A–Z");
        REQUIRE(L10n::NotPlaying() == L"Not Playing");
        REQUIRE(L10n::PlayUrl() == L"Play URL");
        REQUIRE(L10n::NewPlaylist() == L"New Playlist");
        REQUIRE(L10n::Create() == L"Create");
        REQUIRE(L10n::Cancel() == L"Cancel");
        REQUIRE(L10n::UrlErrorTitle() == L"Cannot Play URL");
        REQUIRE(L10n::Ok() == L"OK");
    }
}

// ─── WA-26 导入反馈 ─────────────────────────────────────────────────

TEST_CASE("LK-02 L10n imported-tracks count copy") {
    {
        LanguageScope zh(L"zh");
        REQUIRE(L10n::ImportedTracks(2) == L"已导入 2 首歌曲");
        REQUIRE(L10n::ImportedTracks(1) == L"已导入 1 首歌曲");
    }
    {
        LanguageScope en(L"en");
        REQUIRE(L10n::ImportedTracks(2) == L"Imported 2 tracks.");
        REQUIRE(L10n::ImportedTracks(1) == L"Imported 1 track.");
    }
}

// ─── WA-26 解析/播放失败分类（#120 英文分支）────────────────────────

TEST_CASE("LK-03 L10n playback failure classification has English branches") {
    {
        LanguageScope en(L"en");
        auto expired = L10n::PlaybackFailed(L"expired", L"detail");
        REQUIRE(expired.find(L"expired") != std::wstring::npos);
        REQUIRE(expired.find(L"past") != std::wstring::npos);

        auto rejected = L10n::PlaybackFailed(L"cdn_rejected", L"detail");
        REQUIRE(rejected.find(L"network") != std::wstring::npos);
        // The truthful advice must not be the "re-paste" one.
        REQUIRE(rejected.find(L"past") == std::wstring::npos);

        auto generic = L10n::PlaybackFailed(L"other", L"detail");
        REQUIRE(generic.find(L"Playback failed") != std::wstring::npos);

        // #226: the kind crossing the seam is the core's own value.
        REQUIRE(L10n::UrlErrorText(L"expired", L"detail") == expired);
        REQUIRE(L10n::UrlErrorText(L"", L"detail") == generic);
    }
    {
        LanguageScope zh(L"zh");
        auto expired = L10n::PlaybackFailed(L"expired", L"detail");
        REQUIRE(expired.find(L"重新粘贴") != std::wstring::npos);
        auto rejected = L10n::PlaybackFailed(L"cdn_rejected", L"detail");
        REQUIRE(rejected.find(L"换网络") != std::wstring::npos);
    }
}

TEST_CASE("LK-04 L10n resolve errors: Chinese headline, English raw detail") {
    {
        LanguageScope zh(L"zh");
        auto msg = L10n::UrlResolveError(L"timeout", L"engine detail");
        REQUIRE(msg.find(L"解析超时") != std::wstring::npos);
        REQUIRE(msg.find(L"engine detail") != std::wstring::npos);
    }
    {
        LanguageScope en(L"en");
        // English users get the engine message verbatim (macOS parity).
        REQUIRE(L10n::UrlResolveError(L"timeout", L"engine detail") == L"engine detail");
    }
}

TEST_CASE("LK-05 L10n resolver provisioning status") {
    {
        LanguageScope en(L"en");
        REQUIRE(L10n::ResolverStatusText(L"checking", 0, 0) == L"Preparing resolver…");
        REQUIRE(L10n::ResolverStatusText(L"failed", 0, 0) == L"Resolver install failed");
        REQUIRE(L10n::ResolverStatusText(L"downloading", 1048576, 2097152) ==
                L"Downloading resolver 1.0 / 2.0 MB");
        REQUIRE(L10n::ResolverStatusText(L"downloading", 1048576, 0) ==
                L"Downloading resolver 1.0 MB");
        REQUIRE(L10n::ResolverStatusText(L"idle", 0, 0).empty());
    }
    {
        LanguageScope zh(L"zh");
        REQUIRE(L10n::ResolverStatusText(L"checking", 0, 0) == L"正在准备解析组件…");
        REQUIRE(L10n::ResolverStatusText(L"downloading", 1048576, 2097152) ==
                L"正在下载解析组件 1.0 / 2.0 MB");
    }
}

// ─── LK-09 适配层模板填充（#228）────────────────────────────────────

TEST_CASE("LK-09 adapter fills placeholders and renders a core message spec") {
    LanguageScope zh(L"zh");

    // 按参数填占位符。
    REQUIRE(L10n::Fill(L"a {x} b {x} c", {{L"x", L"1"}}) == L"a 1 b 1 c");
    // 未知键回退空串（键表缺失由 L0 校验拦截，本层不猜文案）。
    REQUIRE(std::wstring(L10n::Key("no_such_key")).empty());

    // 按键取模板 + 字面量原样拼接：这是本层剩下的全部职责。
    std::vector<rhythm::MessageSegment> spec;
    rhythm::MessageSegment headline;
    headline.isKey = true;
    headline.key = "playback_failed_expired";
    spec.push_back(headline);
    rhythm::MessageSegment tail;
    tail.text = L"|tail";
    spec.push_back(tail);
    REQUIRE(L10n::RenderMessageSpec(spec) ==
            std::wstring(L10n::Key("playback_failed_expired")) + L"|tail");
}

// ─── WA-26 来源徽标与托盘 ───────────────────────────────────────────

TEST_CASE("LK-06 L10n source tags and tray copy") {
    {
        LanguageScope zh(L"zh");
        REQUIRE(L10n::SourceTag(L"local") == L"本地");
        REQUIRE(L10n::SourceTag(L"bilibili") == L"B站");
        REQUIRE(L10n::SourceTag(L"direct_url") == L"链接");
        REQUIRE(L10n::SourceTag(L"youtube") == L"YT");
        REQUIRE(L10n::TrayPlayPause() == L"播放 / 暂停");
        REQUIRE(L10n::TrayShowWindow() == L"显示主窗口");
        REQUIRE(L10n::TrayQuit() == L"退出 Rhythm");
    }
    {
        LanguageScope en(L"en");
        REQUIRE(L10n::SourceTag(L"local") == L"Local");
        REQUIRE(L10n::SourceTag(L"bilibili") == L"Bili");
        REQUIRE(L10n::SourceTag(L"direct_url") == L"Link");
        REQUIRE(L10n::SourceTag(L"youtube") == L"YT");
        REQUIRE(L10n::TrayPlayPause() == L"Play / Pause");
        REQUIRE(L10n::TrayShowWindow() == L"Show Window");
        REQUIRE(L10n::TrayQuit() == L"Quit Rhythm");
    }
}
