// WB-01–19：Windows RhythmCore（Bridge 封装层）行为清单（manifest:
// docs/testing/behavior/rhythmcore-windows.md）。零接缝：真 rhythm_core DLL
//（WB-05/06/07/09/10/14 经 FFI 往返），纯函数直测（WB-12/13）。WB-01 的时长文案与 WB-02/03/04/20 的来源徽标
// 已迁入视图状态（ViewStateTests.cpp VS-18～VS-25，#337/#338）。
//
// 这些测试在本机（macOS）不可运行——提交后在 Windows 上 `ctest` 验证。

#include "BehaviorPch.h"
#include "Bridge/RhythmCore.h"
#include "Bridge/GeneratedCodec.h"

#include <catch_amalgamated.hpp>
#include "TestHelpers.h"

using namespace rhythm;
using namespace rhythm_tests;

// ─── WB-21 ExportM3U8（导出走生成的编码器，#428）──────────────────────

TEST_CASE("WB-21 ExportM3U8 writes the tracks and reports failure") {
    TempDir dir;
    Track t = makeLocalTrack(L"C:\\music\\wb21 曲目.mp3", L"WB21 标题");
    auto out = dir.path / L"list.m3u8";

    REQUIRE(ExportM3U8(out.wstring(), {t}));
    std::ifstream in(out, std::ios::binary);
    std::string text((std::istreambuf_iterator<char>(in)), std::istreambuf_iterator<char>());
    REQUIRE(text.find("#EXTM3U") != std::string::npos);
    REQUIRE(text.find(WideToUtf8ForTest(L"wb21 曲目.mp3")) != std::string::npos);

    REQUIRE_FALSE(ExportM3U8((dir.path / L"missing" / L"list.m3u8").wstring(), {t}));
}

// ─── WB-22 文件选择面板的扩展名都是核心收的格式（#242/#327）──────────

TEST_CASE("WB-22 every picker extension is a format the core imports") {
    TempDir dir;
    Library lib(dir.dbPath());

    // The core is the gate; the picker list only shapes the dialog. A file
    // the picker offers must never come back "unsupported" -- unreadable
    // (these are not audio) is the expected outcome.
    for (const std::wstring& ext : kAudioFileTypes) {
        INFO(WideToUtf8ForTest(ext));
        auto file = dir.path / (L"x" + ext);
        std::ofstream(file) << "not audio";
        auto outcome = lib.ImportFile(file.wstring());
        REQUIRE(outcome.has_value());
        REQUIRE(outcome->unsupported == 0);
    }

    // Control: a format the core refuses is reported as unsupported.
    auto txt = dir.path / L"x.txt";
    std::ofstream(txt) << "not audio";
    REQUIRE(lib.ImportFile(txt.wstring())->unsupported == 1);
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
    REQUIRE_FALSE(lib.ImportDirectory(L"C:\\x").has_value());
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


// ─── WB-17/18/19 协调器绑定资料库（#416）─────────────────────────────

namespace {
int64_t playCountOf(Library& lib, int64_t id) {
    for (const auto& t : lib.AllTracks())
        if (t.id == id) return t.playCount;
    return -1;
}
}

TEST_CASE("WB-17 Coordinator bound to a real library records a play on start") {
    TempDir dir;
    Library lib((dir.path / L"lib.db").wstring());
    auto wav = writeWavAt(dir.path, L"a.wav");
    auto saved = lib.AddTrack(makeLocalTrack(wav.wstring(), L"A"));
    REQUIRE(saved.id >= 0);

    Coordinator coord;
    coord.SetLibrary(&lib);
    auto result = coord.Start(saved, {saved}, 0);
    if (result.ok) {
        REQUIRE(playCountOf(lib, saved.id) == 1);
    } else {
        // No audio device: the core's classified error, never swallowed.
        REQUIRE(result.errorKind == L"playback_failed");
        REQUIRE(playCountOf(lib, saved.id) == 0);
    }
    coord.Stop();
}

TEST_CASE("WB-18 Coordinator syncs an empty queue safely") {
    TempDir dir;
    Library lib((dir.path / L"lib.db").wstring());
    auto wav = writeWavAt(dir.path, L"a.wav");
    auto saved = lib.AddTrack(makeLocalTrack(wav.wstring(), L"A"));

    Coordinator withEmpty;
    withEmpty.SetLibrary(&lib);
    withEmpty.SyncQueue({});
    auto a = withEmpty.Start(saved, {}, 0);
    withEmpty.Stop();

    Coordinator withQueue;
    withQueue.SetLibrary(&lib);
    auto b = withQueue.Start(saved, {saved}, 0);
    withQueue.Stop();

    REQUIRE(a.ok == b.ok);
    REQUIRE(a.errorKind == b.errorKind);
}

TEST_CASE("WB-19 Coordinator bound to a failed-open library stays safe") {
    TempDir dir;
    Library lib(dir.path.wstring()); // a directory is not a database path

    Coordinator coord;
    coord.SetLibrary(&lib);
    auto track = makeLocalTrack((dir.path / L"missing.wav").wstring(), L"M");
    track.id = 1;
    auto result = coord.Start(track, {track}, 0);
    REQUIRE_FALSE(result.ok);
    coord.Next();
    coord.Previous();
    coord.TogglePlayPause();
    coord.SyncQueue({track});
    coord.Stop();
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

// ─── WB-23/24 解析结果由生成物解码（#362）───────────────────────────

/// The core's raw resolve payload, freed on the way out.
static std::string RawResolvePayload(const char* url) {
    char* raw = rhythm_resolve_url(url);
    REQUIRE(raw != nullptr);
    std::string payload(raw);
    rhythm_free_string(raw);
    return payload;
}

TEST_CASE("WB-23 the generated resolve-result decoder agrees with ResolveURL field by field") {
    for (const char* url : {"https://example.com/wb23-song.mp3", "not a url"}) {
        INFO(url);
        auto decoded = generated::ResolveResultFromJson(nlohmann::json::parse(RawResolvePayload(url)));
        auto outcome = Resolver::ResolveURL(Utf8ToWide(url));

        REQUIRE(decoded.ok == outcome.ok);
        if (outcome.ok) {
            REQUIRE(decoded.resolved.has_value());
            REQUIRE(decoded.resolved->title == outcome.track.title);
            REQUIRE(decoded.resolved->artist == outcome.track.artist);
            REQUIRE(decoded.resolved->duration == outcome.track.duration);
            REQUIRE(decoded.resolved->sourceType == outcome.track.sourceType);
        } else {
            REQUIRE_FALSE(decoded.resolved.has_value());
            REQUIRE(decoded.errorKind == outcome.errorKind);
            REQUIRE(decoded.errorMessage == outcome.errorMessage);
        }
    }
}

TEST_CASE("WB-24 the generated resolve-result decoder reads every contract field") {
    auto decoded = generated::ResolveResultFromJson(nlohmann::json::parse(R"({
        "ok": true,
        "resolved": {
            "title": "标题", "artist": "艺人",
            "stream_url": "https://cdn.example.com/a.m4a", "duration": 12.5,
            "source_type": "bilibili", "thumbnail_url": "https://img.example.com/a.jpg",
            "http_headers": {"Referer": "https://www.bilibili.com/video/x"}
        },
        "error_kind": "timeout",
        "error_message": "slow"
    })"));

    REQUIRE(decoded.ok);
    REQUIRE(decoded.resolved.has_value());
    const auto& r = *decoded.resolved;
    REQUIRE(r.title == L"标题");
    REQUIRE(r.artist == L"艺人");
    REQUIRE(r.streamUrl == L"https://cdn.example.com/a.m4a");
    REQUIRE(r.duration == 12.5);
    REQUIRE(r.sourceType == L"bilibili");
    REQUIRE(r.thumbnailUrl == L"https://img.example.com/a.jpg");
    REQUIRE(r.httpHeaders == std::map<std::wstring, std::wstring>{
        {L"Referer", L"https://www.bilibili.com/video/x"}});
    REQUIRE(decoded.errorKind == L"timeout");
    REQUIRE(decoded.errorMessage == L"slow");
}

// ─── WB-27 解析出的曲目形状（#364）──────────────────────────────────

TEST_CASE("WB-27 a resolved URL becomes an unsaved, available track that keeps the page URL") {
    auto outcome = Resolver::ResolveURL(L"https://example.com/wb27.mp3");
    REQUIRE(outcome.ok);

    // What the core resolved, plus the pasted page URL -- nothing else.
    Track expected;
    expected.id = -1;  // not in the library yet
    expected.sourceType = L"direct_url";
    expected.sourceUrl = L"https://example.com/wb27.mp3";
    expected.title = L"wb27.mp3";
    expected.duration = 0.0;
    REQUIRE(outcome.track.id == -1);
    REQUIRE(outcome.track.isAvailable);
    REQUIRE(outcome.track == expected);
}

// ─── WB-25/26 协调器结果由生成物解码（#363）─────────────────────────

/// One call on a fresh raw core coordinator; returns its result payload.
template <typename Call>
static std::string RawCoordinatorPayload(Call call) {
    RhythmCoordinator* ptr = rhythm_coordinator_create();
    REQUIRE(ptr != nullptr);
    char* raw = call(ptr);
    REQUIRE(raw != nullptr);
    std::string payload(raw);
    rhythm_free_string(raw);
    rhythm_coordinator_stop(ptr);
    rhythm_coordinator_destroy(ptr);
    return payload;
}

/// Every field of the two results, the whole current track included.
static void RequireSameCoordinatorResult(const CoordinatorResult& decoded,
                                         const CoordinatorResult& wrapped) {
    REQUIRE(decoded.ok == wrapped.ok);
    REQUIRE(decoded.errorKind == wrapped.errorKind);
    REQUIRE(decoded.errorMessage == wrapped.errorMessage);
    REQUIRE(decoded.currentTrack == wrapped.currentTrack);
    REQUIRE(decoded.playbackActive == wrapped.playbackActive);
}

TEST_CASE("WB-25 the generated coordinator-result decoder agrees with Coordinator field by field") {
    TempDir dir;
    auto wav = writeWavAt(dir.path, L"wb25.wav");
    Track nowhere = makeLocalTrack(L"", L"No Location");   // no_playable_location
    Track missing = makeLocalTrack((dir.path / L"missing.wav").wstring(), L"Missing");
    Track real = makeLocalTrack(wav.wstring(), L"Real");   // ok, or playback_failed without a device

    for (const Track& track : {nowhere, missing, real}) {
        INFO(WideToUtf8ForTest(track.title));
        auto trackJson = generated::TrackToJson(track).dump();
        auto queueJson = nlohmann::json::array({generated::TrackToJson(track)}).dump();
        auto decoded = generated::CoordinatorResultFromJson(nlohmann::json::parse(
            RawCoordinatorPayload([&](RhythmCoordinator* c) {
                return rhythm_coordinator_start(c, nullptr, trackJson.c_str(), queueJson.c_str(), 0);
            })));

        Coordinator coord;
        auto wrapped = coord.Start(track, {track}, 0);
        coord.Stop();

        RequireSameCoordinatorResult(decoded, wrapped);
    }

    // Idle transport: ok results with no track.
    using RawCall = char* (*)(RhythmCoordinator*, RhythmLibrary*);
    using WrappedCall = CoordinatorResult (Coordinator::*)();
    const std::pair<RawCall, WrappedCall> idleCalls[] = {
        {rhythm_coordinator_toggle_play_pause, &Coordinator::TogglePlayPause},
        {rhythm_coordinator_next, &Coordinator::Next},
        {rhythm_coordinator_previous, &Coordinator::Previous},
    };
    for (const auto& [raw, wrapped] : idleCalls) {
        auto decoded = generated::CoordinatorResultFromJson(nlohmann::json::parse(
            RawCoordinatorPayload([raw](RhythmCoordinator* c) { return raw(c, nullptr); })));
        Coordinator idle;
        RequireSameCoordinatorResult(decoded, (idle.*wrapped)());
    }
}

TEST_CASE("WB-26 the generated coordinator-result decoder reads every contract field") {
    auto decoded = generated::CoordinatorResultFromJson(nlohmann::json::parse(R"({
        "ok": false,
        "error_kind": "playback_failed",
        "error_message": "设备不可用",
        "current_track": {"id": 42, "source_type": "local", "file_path": "C:/m/a.wav", "title": "曲目"},
        "playback_active": true
    })"));

    REQUIRE_FALSE(decoded.ok);
    REQUIRE(decoded.errorKind == L"playback_failed");
    REQUIRE(decoded.errorMessage == L"设备不可用");
    REQUIRE(decoded.currentTrack.has_value());
    REQUIRE(decoded.currentTrack->id == 42);
    REQUIRE(decoded.currentTrack->title == L"曲目");
    REQUIRE(decoded.currentTrack->filePath == L"C:/m/a.wav");
    REQUIRE(decoded.playbackActive);

    // A success carries no error pair: both stay empty rather than "".
    auto success = generated::CoordinatorResultFromJson(
        nlohmann::json::parse(R"({"ok": true, "playback_active": false})"));
    REQUIRE(success.ok);
    REQUIRE_FALSE(success.errorKind.has_value());
    REQUIRE_FALSE(success.errorMessage.has_value());
    REQUIRE_FALSE(success.currentTrack.has_value());
    REQUIRE_FALSE(success.playbackActive);
}

// ─── WB-12/13 ResolverStatus ────────────────────────────────────────

TEST_CASE("WB-12 StatusText renders every phase") {
    // Status copy follows the UI language: pin both branches (#418).
    auto text = [](const wchar_t* phase, int64_t received = 0, int64_t total = 0) {
        ResolverStatus s;
        s.phase = phase;
        s.received = received;
        s.total = total;
        return Resolver::StatusText(s);
    };
    const int64_t mb = 1048576;
    {
        LanguageScope zh(L"zh");
        REQUIRE(text(L"checking") == L"正在准备解析组件…");
        REQUIRE(text(L"verifying") == L"正在校验解析组件…");
        REQUIRE(text(L"updating") == L"正在更新解析组件…");
        REQUIRE(text(L"failed") == L"解析组件安装失败");
        REQUIRE(text(L"downloading", mb, 4 * mb) == L"正在下载解析组件 1.0 / 4.0 MB");
        REQUIRE(text(L"downloading", mb, 0) == L"正在下载解析组件 1.0 MB");
        REQUIRE(text(L"idle") == L"");
        REQUIRE(text(L"something_unknown") == L"");
    }
    {
        LanguageScope en(L"en");
        REQUIRE(text(L"checking") == L"Preparing resolver…");
        REQUIRE(text(L"verifying") == L"Verifying resolver…");
        REQUIRE(text(L"updating") == L"Updating resolver…");
        REQUIRE(text(L"failed") == L"Resolver install failed");
        REQUIRE(text(L"downloading", mb, 4 * mb) == L"Downloading resolver 1.0 / 4.0 MB");
        REQUIRE(text(L"downloading", mb, 0) == L"Downloading resolver 1.0 MB");
        REQUIRE(text(L"idle") == L"");
        REQUIRE(text(L"something_unknown") == L"");
    }
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
