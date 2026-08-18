// WB-01–16：Windows RhythmCore（Bridge 封装层）行为清单（manifest:
// docs/testing/behavior/rhythmcore-windows.md）。零接缝：真 rhythm_core DLL
//（WB-05/06/07/09/10/14 经 FFI 往返），纯函数直测（WB-01–04/12/13）。
//
// 这些测试在本机（macOS）不可运行——提交后在 Windows 上 `ctest` 验证。

#include "pch.h"
#include "Bridge/RhythmCore.h"

#include <catch_amalgamated.hpp>
#include "TestHelpers.h"

using namespace rhythm;
using namespace rhythm_tests;

// ─── WB-01 DurationFormatted ────────────────────────────────────────

TEST_CASE("WB-01 DurationFormatted renders m:ss with zero padding") {
    Track t;
    t.duration = 65.0;
    REQUIRE(t.DurationFormatted() == L"1:05");
    t.duration = 5.0;
    REQUIRE(t.DurationFormatted() == L"0:05");
    t.duration = 0.0;
    REQUIRE(t.DurationFormatted() == L"0:00");
}

// ─── WB-02/03 SourceTag / SourceColor ───────────────────────────────

TEST_CASE("WB-02 SourceTag maps every source type") {
    Track t;
    t.sourceType = L"local";
    REQUIRE(t.SourceTag() == L"本地");
    t.sourceType = L"youtube";
    REQUIRE(t.SourceTag() == L"YT");
    t.sourceType = L"bilibili";
    REQUIRE(t.SourceTag() == L"B站");
    t.sourceType = L"direct_url";
    REQUIRE(t.SourceTag() == L"链接");
    t.sourceType = L"something_else";
    REQUIRE(t.SourceTag() == L"");
}

TEST_CASE("WB-03 SourceColor maps every source type in both themes (#121)") {
    Track t;
    t.sourceType = L"local";
    REQUIRE(t.SourceColor(t.sourceType, true) == L"#8ABCD0");
    REQUIRE(t.SourceColor(t.sourceType, false) == L"#3A7A8C");
    t.sourceType = L"youtube";
    REQUIRE(t.SourceColor(t.sourceType, true) == L"#D49573");
    REQUIRE(t.SourceColor(t.sourceType, false) == L"#8B4A28");
    t.sourceType = L"bilibili";
    REQUIRE(t.SourceColor(t.sourceType, true) == L"#C88DA8");
    REQUIRE(t.SourceColor(t.sourceType, false) == L"#8C4D68");
    t.sourceType = L"direct_url";
    REQUIRE(t.SourceColor(t.sourceType, true) == L"#8CB89A");
    REQUIRE(t.SourceColor(t.sourceType, false) == L"#4C785A");
    // F4: unknown sources fall back to the teal text colour, never "Gray".
    t.sourceType = L"nope";
    REQUIRE(t.SourceColor(t.sourceType, true) == L"#ABC8D4");
    REQUIRE(t.SourceColor(t.sourceType, false) == L"#0D464D");
    REQUIRE(t.SourceColor(t.sourceType, true) != L"Gray");
    REQUIRE(t.SourceColor(t.sourceType, false) != L"Gray");
}

// ─── WB-04 SourceBackgroundBrush（需要 apartment，main 已 init）───────

TEST_CASE("WB-04 SourceBackgroundBrush carries 15% alpha of the source color") {
    Track t;
    t.sourceType = L"local";
    auto brush = t.SourceBackgroundBrush();
    auto color = brush.Color();
    REQUIRE(color.A == 38);
    REQUIRE(color.R == 0x8A);
    REQUIRE(color.G == 0xBC);
    REQUIRE(color.B == 0xD0);

    Track unknown;
    unknown.sourceType = L"weird";
    auto fallback = unknown.SourceBackgroundBrush().Color();
    REQUIRE(fallback.A == 38);
    REQUIRE(fallback.R == 128);
    REQUIRE(fallback.G == 128);
    REQUIRE(fallback.B == 128);
}

// ─── WB-05 JsonToTrack/TrackToJson 往返（经 AddTrack 黑盒）───────────

TEST_CASE("WB-05 AddTrack roundtrip preserves every field") {
    TempDir dir;
    Library lib(dir.dbPath());

    Track t;
    t.id = -1;
    t.filePath = L"C:\\music\\中文 曲目.mp3";
    t.sourceType = L"local";
    t.sourceUrl = L"https://example.com/ignored";
    t.title = L"中文标题";
    t.artist = L"艺术家";
    t.album = L"专辑";
    t.albumArtist = L"专辑艺术家";
    t.trackNumber = 3;
    t.discNumber = 2;
    t.genre = L"摇滚";
    t.year = 2021;
    t.duration = 123.5;
    t.format = L"mp3";
    t.bitrate = 320;
    t.sampleRate = 44100;
    t.channels = 2;
    t.fileSize = 1'000'000;
    t.playCount = 7;
    t.isAvailable = false;

    auto saved = lib.AddTrack(t);

    REQUIRE(saved.id > 0);
    REQUIRE(saved.filePath == L"C:\\music\\中文 曲目.mp3");
    REQUIRE(saved.sourceType == L"local");
    REQUIRE(saved.title == L"中文标题");
    REQUIRE(saved.artist == L"艺术家");
    REQUIRE(saved.album == L"专辑");
    REQUIRE(saved.trackNumber == 3);
    REQUIRE(saved.discNumber == 2);
    // #101: JsonToTrack now parses every core field.
    REQUIRE(saved.albumArtist == L"专辑艺术家");
    REQUIRE(saved.genre == L"摇滚");
    REQUIRE(saved.year == 2021);
    REQUIRE(saved.duration == 123.5);
    REQUIRE(saved.format == L"mp3");
    REQUIRE(saved.bitrate == 320);
    REQUIRE(saved.sampleRate == 44100);
    REQUIRE(saved.channels == 2);
    REQUIRE(saved.fileSize == 1'000'000);
    REQUIRE(saved.dateAdded.has_value()); // the DB stamps the insert time
    REQUIRE_FALSE(saved.lastPlayed.has_value()); // fresh insert → NULL
    REQUIRE(saved.playCount == 0); // the DB resets the count on insert
    REQUIRE(saved.isAvailable == false);
}

TEST_CASE("WB-05 missing optional fields roundtrip as nullopt") {
    TempDir dir;
    Library lib(dir.dbPath());

    Track t;
    t.id = -1;
    t.sourceType = L"local";
    t.title = L"Bare";

    auto saved = lib.AddTrack(t);

    REQUIRE_FALSE(saved.filePath.has_value());
    REQUIRE_FALSE(saved.sourceUrl.has_value());
    REQUIRE_FALSE(saved.artist.has_value());
    REQUIRE_FALSE(saved.album.has_value());
    REQUIRE_FALSE(saved.albumArtist.has_value());
    REQUIRE_FALSE(saved.trackNumber.has_value());
    REQUIRE_FALSE(saved.genre.has_value());
    REQUIRE_FALSE(saved.format.has_value());
    REQUIRE_FALSE(saved.fileSize.has_value());
    REQUIRE_FALSE(saved.lastPlayed.has_value());
    REQUIRE_FALSE(saved.artworkPath.has_value());
}

// ─── WB-06 UTF-8/UTF-16 转换往返（经 ResolveURL 黑盒）────────────────

TEST_CASE("WB-06 UTF roundtrip survives Chinese and emoji titles") {
    // Direct URLs resolve locally in the core: the title is the URL's file
    // name, which has to survive Utf8ToWide/WideToUtf8 in both directions.
    auto outcome = Resolver::ResolveURL(L"https://example.com/中文标题.mp3");
    REQUIRE(outcome.ok);
    REQUIRE(outcome.track.title == L"中文标题.mp3");

    auto emoji = Resolver::ResolveURL(L"https://example.com/\U0001F3B5.mp3");
    REQUIRE(emoji.ok);
    REQUIRE(emoji.track.title == L"\U0001F3B5.mp3");

    // Empty input: the conversion helpers handle it, the resolver reports
    // the failure kind instead of crashing.
    auto empty = Resolver::ResolveURL(L"");
    REQUIRE_FALSE(empty.ok);
    REQUIRE(empty.errorKind == L"invalid_url");
}

// ─── WB-07 Library 空指针防御 ───────────────────────────────────────

TEST_CASE("WB-07 Library with failed open returns safe defaults") {
    TempDir dir;
    Library lib(dir.path.wstring()); // a directory is not a database path

    REQUIRE(lib.AllTracks().empty());
    REQUIRE(lib.AllPlaylists().empty());
    REQUIRE(lib.Search(L"x").empty());
    REQUIRE(lib.ImportDirectory(L"C:\\x") == -1);
    REQUIRE(lib.CreatePlaylist(L"p") == -1);

    auto original = makeUrlTrack(L"https://example.com/x.mp3", L"Keep Me");
    auto saved = lib.AddTrack(original);
    REQUIRE(saved.id == original.id); // returned as-is
    REQUIRE(lib.RemoveTrack(1) == false);

    lib.RecordPlay(1);   // must not crash
    lib.VerifyFiles();   // must not crash
    lib.AddToPlaylist(1, 2);
    lib.RemoveFromPlaylist(1, 2);
    lib.DeletePlaylist(1);
}

// ─── WB-08 Player 空指针防御 ────────────────────────────────────────

TEST_CASE("WB-08 Player defaults are observable and safe") {
    // A failed create() cannot be constructed from the outside (the
    // constructor always calls rhythm_player_create), so the ptr-null
    // guard branches are defensive — lock the fresh-player defaults.
    Player player;
    REQUIRE(player.State() == 0);
    REQUIRE(player.Position() == 0.0);
    REQUIRE(player.Duration() == 0.0);
    REQUIRE(player.ErrorMessage().empty());
    REQUIRE(player.ErrorKind().empty()); // no failure → no classification (#120)

    player.Pause();   // must not crash on a stopped player
    player.Resume();
    player.Stop();
    player.SetVolume(0.5f);
    REQUIRE(player.Volume() == Catch::Approx(0.5f));
}

// ─── WB-09/10 ResolveURL 分派 ───────────────────────────────────────

TEST_CASE("WB-09 ResolveURL success keeps the page URL") {
    auto outcome = Resolver::ResolveURL(L"https://example.com/wb09-song.mp3");

    REQUIRE(outcome.ok);
    REQUIRE(outcome.track.title == L"wb09-song.mp3");
    // The source URL is the page URL the user pasted — never the resolved
    // CDN link (which carries an expiring deadline).
    REQUIRE(outcome.track.sourceUrl == L"https://example.com/wb09-song.mp3");
    REQUIRE(outcome.track.sourceType == L"direct_url");
}

TEST_CASE("WB-10 ResolveURL failure surfaces the core's reason (#21)") {
    auto outcome = Resolver::ResolveURL(L"not a url");

    REQUIRE_FALSE(outcome.ok);
    REQUIRE(outcome.errorKind == L"invalid_url");
    REQUIRE_FALSE(outcome.errorMessage.empty());
}

// ─── WB-11 LastResolveFailure 兜底 ──────────────────────────────────

/// The "no payload / malformed JSON" fallbacks are unreachable from the
/// public API: the core always records a well-formed `{kind, message}` JSON
/// before returning null, and successes clear it. Locked as observed: every
/// failure carries the core's own kind and message.
TEST_CASE("WB-11 failures always carry the core's kind and message") {
    auto first = Resolver::ResolveURL(L"garbage input one");
    auto second = Resolver::ResolveURL(L"garbage input two");

    REQUIRE_FALSE(first.ok);
    REQUIRE(first.errorKind == L"invalid_url");
    REQUIRE_FALSE(first.errorMessage.empty());
    REQUIRE(second.errorKind == L"invalid_url");
}

// ─── WB-12/13 ResolverStatus ────────────────────────────────────────

TEST_CASE("WB-12 StatusText renders every phase") {
    ResolverStatus s;
    s.phase = L"checking";
    REQUIRE(Resolver::StatusText(s) == L"正在准备解析组件…");
    s.phase = L"verifying";
    REQUIRE(Resolver::StatusText(s) == L"正在校验解析组件…");
    s.phase = L"updating";
    REQUIRE(Resolver::StatusText(s) == L"正在更新解析组件…");
    s.phase = L"failed";
    REQUIRE(Resolver::StatusText(s) == L"解析组件安装失败");

    s.phase = L"downloading";
    s.received = 1048576;  // 1 MB
    s.total = 4194304;     // 4 MB
    REQUIRE(Resolver::StatusText(s) == L"正在下载解析组件 1.0 / 4.0 MB");

    s.total = 0;
    REQUIRE(Resolver::StatusText(s) == L"正在下载解析组件 1.0 MB");

    s.phase = L"idle";
    REQUIRE(Resolver::StatusText(s) == L"");
    s.phase = L"something_unknown";
    REQUIRE(Resolver::StatusText(s) == L"");
}

TEST_CASE("WB-13 ResolverStatus IsQuiet") {
    ResolverStatus s;
    s.phase = L"idle";
    REQUIRE(s.IsQuiet());
    s.phase = L"ready";
    REQUIRE(s.IsQuiet());
    s.phase = L"downloading";
    REQUIRE_FALSE(s.IsQuiet());
    s.phase = L"failed";
    REQUIRE_FALSE(s.IsQuiet());
}

// ─── WB-14 ClassifyURL ──────────────────────────────────────────────

TEST_CASE("WB-14 ClassifyURL returns the source type string") {
    REQUIRE(Resolver::ClassifyURL(L"https://www.youtube.com/watch?v=abc") == L"youtube");
    REQUIRE(Resolver::ClassifyURL(L"https://www.bilibili.com/video/BV1xx") == L"bilibili");
    REQUIRE(Resolver::ClassifyURL(L"https://example.com/song.mp3") == L"direct_url");
    REQUIRE(Resolver::ClassifyURL(L"garbage") == L"");
}

// ─── WB-15/16 边界 ──────────────────────────────────────────────────

/// The "malformed JSON" branch is unreachable from the public API: the core
/// serializes its own payloads and every field decodes. Locked as observed:
/// core payloads always decode (covered by WB-09/WB-10).
TEST_CASE("WB-15 core payloads always decode") {
    auto outcome = Resolver::ResolveURL(L"https://example.com/wb15.mp3");
    REQUIRE(outcome.ok);
    REQUIRE_FALSE(outcome.track.title.empty());
}

/// `ParseTrackList` is a file-local helper exercised through `AllTracks`;
/// the null-pointer input branch is unreachable (the FFI returns "[]" on an
/// empty library, never null). Locked as observed: an empty library parses
/// to an empty list.
TEST_CASE("WB-16 empty track list parses to an empty vector") {
    TempDir dir;
    Library lib(dir.dbPath());
    REQUIRE(lib.AllTracks().empty());
}
