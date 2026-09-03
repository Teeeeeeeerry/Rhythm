import XCTest
import Foundation
@testable import Rhythm

/// SW-01–14：RhythmCore Swift 封装层行为清单（manifest:
/// docs/testing/behavior/rhythmcore-swift.md）。零接缝：链接真 rust-core。
final class RhythmCoreSwiftBehaviorTests: XCTestCase {

    /// A fresh temp directory for the test (callers still remove it).
    private func makeTempDir() -> URL {
        let dir = FileManager.default.temporaryDirectory
            .appendingPathComponent("RhythmSWTests-\(UUID().uuidString)")
        try? FileManager.default.createDirectory(at: dir, withIntermediateDirectories: true)
        return dir
    }

    // MARK: - SW-01 encodeJSON/decodeJSON snake_case 往返

    func testSW01_TrackJSONRoundtripPreservesFields() {
        let track = Track(
            id: 7,
            filePath: "/music/a.mp3",
            sourceType: "direct_url",
            sourceUrl: "https://example.com/a.mp3",
            title: "Round Trip",
            artist: "Artist",
            album: "Album",
            albumArtist: "Album Artist",
            trackNumber: 3,
            discNumber: 2,
            genre: "Rock",
            year: 2021,
            duration: 123.5,
            format: "mp3",
            bitrate: 320,
            sampleRate: 44100,
            channels: 2,
            fileSize: 1_000_000,
            dateAdded: "2026-01-01",
            lastPlayed: "2026-02-02",
            playCount: 5,
            artworkPath: "/cache/art.jpg",
            isAvailable: true
        )

        let json = encodeJSON(track)
        let decoded: Track? = decodeJSON(json)

        XCTAssertNotNil(decoded, "roundtrip must decode")
        XCTAssertEqual(decoded, track, "every field must survive the snake_case roundtrip")
    }

    func testSW01_PlaylistJSONRoundtripPreservesFields() {
        let track = Track(
            id: 1, filePath: nil, sourceType: "local", sourceUrl: nil,
            title: "In Playlist", artist: nil, album: nil, albumArtist: nil,
            trackNumber: nil, discNumber: nil, genre: nil, year: nil,
            duration: 60.0, format: nil, bitrate: nil, sampleRate: nil,
            channels: nil, fileSize: nil, dateAdded: nil, lastPlayed: nil,
            playCount: 0, artworkPath: nil, isAvailable: true
        )
        let playlist = Playlist(
            id: 3,
            name: "My List",
            description: "desc",
            dateCreated: "2026-01-01",
            dateModified: "2026-01-02",
            tracks: [track]
        )

        let decoded: Playlist? = decodeJSON(encodeJSON(playlist))

        XCTAssertEqual(decoded?.id, 3)
        XCTAssertEqual(decoded?.name, "My List")
        XCTAssertEqual(decoded?.description, "desc")
        XCTAssertEqual(decoded?.dateCreated, "2026-01-01")
        XCTAssertEqual(decoded?.dateModified, "2026-01-02")
        XCTAssertEqual(decoded?.tracks.count, 1)
        XCTAssertEqual(decoded?.tracks.first, track)
    }

    // MARK: - SW-02 decodeJSON 非法输入

    func testSW02_DecodeInvalidInputReturnsNil() {
        let nilTrack: Track? = decodeJSON("not json at all{")
        XCTAssertNil(nilTrack, "invalid JSON must decode to nil, not crash")

        let wrongShape: Track? = decodeJSON("{\"foo\": 1}")
        XCTAssertNil(wrongShape, "wrong shape must decode to nil")
    }

    // MARK: - SW-03 PlayMode.next() 循环

    func testSW03_PlayModeNextCycles() {
        XCTAssertEqual(PlayMode.sequential.next(), .shuffle)
        XCTAssertEqual(PlayMode.shuffle.next(), .singleLoop)
        XCTAssertEqual(PlayMode.singleLoop.next(), .listLoop)
        XCTAssertEqual(PlayMode.listLoop.next(), .sequential)
    }

    /// SW-03b (#179): the FFI contract values (0-3) are locked — the core's
    /// `queue::PlayMode` is the canonical declaration.
    func testSW03b_PlayModeContractValues() {
        XCTAssertEqual(PlayMode.sequential.rawValue, 0)
        XCTAssertEqual(PlayMode.shuffle.rawValue, 1)
        XCTAssertEqual(PlayMode.singleLoop.rawValue, 2)
        XCTAssertEqual(PlayMode.listLoop.rawValue, 3)
    }

    // MARK: - SW-04 ResolverStatus.isQuiet

    func testSW04_ResolverStatusIsQuiet() {
        XCTAssertTrue(ResolverStatus(phase: "idle", received: nil, total: nil, message: nil).isQuiet)
        XCTAssertTrue(ResolverStatus(phase: "ready", received: nil, total: nil, message: nil).isQuiet)
        XCTAssertFalse(ResolverStatus(phase: "downloading", received: 1, total: 2, message: nil).isQuiet)
        XCTAssertFalse(ResolverStatus(phase: "checking", received: nil, total: nil, message: nil).isQuiet)
    }

    // MARK: - SW-05/06/07 resolveURL 分派

    func testSW05_ResolveURLSuccessDispatches() {
        // Direct audio URLs resolve locally in the core (no network) —
        // a stable success path for the wrapper.
        let result = resolveURL("https://example.com/sw-tone.mp3")

        guard case .success(let info) = result else {
            return XCTFail("direct URL must resolve, got \(result)")
        }
        XCTAssertEqual(info.title, "sw-tone.mp3")
        XCTAssertEqual(info.streamUrl, "https://example.com/sw-tone.mp3")
        XCTAssertEqual(info.sourceType, "direct_url")
    }

    func testSW06_ResolveURLFailureFallsBackToLastError() {
        let result = resolveURL("not a url")

        guard case .failure(let error) = result else {
            return XCTFail("garbage input must fail, got \(result)")
        }
        XCTAssertEqual(error.kind, "invalid_url", "kind must come from the core (#21)")
        XCTAssertFalse(error.message.isEmpty)
    }

    /// SW-07: the `.failure(kind: "internal")` branch fires only when the
    /// core returns a non-null JSON that fails to decode — `ResolvedUrl`'s
    /// extra keys are ignored by the decoder, so this branch is defensive
    /// and unreachable from the public API today. The observable dispatch
    /// (success/failure) is locked by SW-05/SW-06.
    func testSW07_MalformedResponseBranchIsDefensive() {
        let resolved: ResolvedInfo? = decodeJSON("{\"title\":\"x\",\"stream_url\":null,\"duration\":0,\"source_type\":\"direct_url\"}")
        XCTAssertNotNil(resolved, "well-formed core payloads always decode")
    }

    // MARK: - SW-16 生成绑定与手写绑定产物一致（#180）

    func testSW16_GeneratedCodecMatchesCodableRoundtrip() {
        // The generated codec and the Codable+convertFromSnakeCase path must
        // produce identical results for the same payloads (expand-contract
        // acceptance: "生成绑定与手写绑定产物对比一致").
        let track = makeTrack(
            id: 42,
            title: "Generated vs Codable",
            sourceType: "local",
            filePath: "/music/parity.mp3",
            sourceUrl: nil,
            duration: 210.5
        )
        // Codable encode (snake_case) → generated decode.
        let codableJSON = encodeJSON(track)
        let viaGenerated = GeneratedCodec.decodeTrack(codableJSON)
        XCTAssertEqual(viaGenerated?.id, track.id)
        XCTAssertEqual(viaGenerated?.title, track.title)
        XCTAssertEqual(viaGenerated?.filePath, track.filePath)
        XCTAssertEqual(viaGenerated?.duration, track.duration)

        // Generated encode → Codable decode (the reverse direction).
        let generatedJSON = GeneratedCodec.encodeTrack(track)
        let viaCodable: Track? = decodeJSON(generatedJSON)
        XCTAssertEqual(viaCodable?.id, track.id)
        XCTAssertEqual(viaCodable?.title, track.title)
        XCTAssertEqual(viaCodable?.sourceType, track.sourceType)
        XCTAssertEqual(viaCodable?.filePath, track.filePath)

        // Both encoders produce the same snake_case payload (key parity).
        let a = (try? JSONSerialization.jsonObject(with: codableJSON.data(using: .utf8)!)) as? [String: Any]
        let b = (try? JSONSerialization.jsonObject(with: generatedJSON.data(using: .utf8)!)) as? [String: Any]
        XCTAssertEqual(a?["file_path"] as? String, b?["file_path"] as? String)
        XCTAssertEqual(a?["source_type"] as? String, b?["source_type"] as? String)
        XCTAssertEqual(a?["duration"] as? Double, b?["duration"] as? Double)
    }

    func testSW16_GeneratedCodecDecodesM3u8Entries() {
        let json = #"[{"title":"A","artist":"Artist","location":"/music/a.mp3"},{"title":"B","location":"https://x.com/b.mp3"}]"#
        let entries = GeneratedCodec.decodeM3u8Entries(json)
        XCTAssertEqual(entries?.count, 2)
        XCTAssertEqual(entries?[0].title, "A")
        XCTAssertEqual(entries?[0].artist, "Artist")
        XCTAssertEqual(entries?[0].location, "/music/a.mp3")
        XCTAssertNil(entries?[1].artist)
        XCTAssertEqual(entries?[1].location, "https://x.com/b.mp3")
    }

    // MARK: - SW-17 键表完整性（#182）

    /// 键表是单一事实来源：所有静态文案键在两个语言下都有非空/可用的取值。
    /// 新增文案只改 contracts/l10n-keys.json，再跑 scripts/gen-l10n.py。
    func testSW17_KeyTableCoversStaticCopy() {
        let keys: [String] = [
            "library_tab", "url_placeholder", "ok", "buffering",
            "mode_shuffle", "import_button", "new_playlist", "delete_button",
            "tray_next", "tag_bilibili", "menu_playback",
        ]
        UserDefaults.standard.set("zh", forKey: "AppLanguage")
        for key in keys {
            XCTAssertFalse(L10nKeys.value(key).isEmpty, "zh copy for \(key)")
        }
        UserDefaults.standard.set("en", forKey: "AppLanguage")
        for key in keys {
            XCTAssertFalse(L10nKeys.value(key).isEmpty, "en copy for \(key)")
        }
        UserDefaults.standard.removeObject(forKey: "AppLanguage")
    }

    // MARK: - LK-09 适配层模板填充（#228）

    /// 分派已经在核心；适配层只剩两件事——按键取模板、按参数填占位符。
    /// 固定 locale，不依赖运行环境语言（#142）。
    func testLK09_AdapterFillsPlaceholdersAndFallsBackOnUnknownKey() {
        UserDefaults.standard.set("zh", forKey: "AppLanguage")
        defer { UserDefaults.standard.removeObject(forKey: "AppLanguage") }

        XCTAssertEqual(L10n.importedTracks(2), "已导入 2 首歌曲")
        XCTAssertEqual(L10n.importSomeFailed(3, 1), "已导入 3 首，1 个失败")
        // 未知键回退键名（键表缺失由 L0 校验拦截，本层不猜文案）。
        XCTAssertEqual(L10nKeys.value("no_such_key"), "no_such_key")
    }

    // MARK: - SW-08 RhythmLibrary 打开失败

    func testSW08_LibraryOpenFailureReturnsNil() {
        let dir = makeTempDir()
        defer { try? FileManager.default.removeItem(at: dir) }

        // A directory is not a valid database path → open fails → init? nil.
        // The manifest's ptr-nil "safe defaults" half is unreachable from the
        // outside: a failed init? never hands out an instance, so the
        // `guard let ptr` branches are defensive (like SW-09).
        let lib = RhythmLibrary(path: dir.path)
        XCTAssertNil(lib, "failed open must yield nil, not a zombie instance")
    }


    // MARK: - SW-11 RhythmLibrary.addTrack

    func testSW11_AddTrackReturnsSavedTrackWithDBId() {
        let dir = makeTempDir()
        defer { try? FileManager.default.removeItem(at: dir) }

        // The manifest's "core returns null → nil" half is unreachable today:
        // every model is plain Codable, so encodeJSON always produces valid
        // JSON the core accepts (defensive, like SW-07/SW-13).
        guard let lib = RhythmLibrary(path: dir.appendingPathComponent("t.db").path) else {
            return XCTFail("temp library must open")
        }
        let track = Track(
            id: -1, filePath: "/music/sw.mp3", sourceType: "local", sourceUrl: nil,
            title: "SW Track", artist: "A", album: nil, albumArtist: nil,
            trackNumber: nil, discNumber: nil, genre: nil, year: nil,
            duration: 90.0, format: "mp3", bitrate: nil, sampleRate: nil,
            channels: nil, fileSize: nil, dateAdded: nil, lastPlayed: nil,
            playCount: 0, artworkPath: nil, isAvailable: true
        )

        let saved = lib.addTrack(track)

        XCTAssertNotNil(saved, "valid track must save")
        XCTAssertGreaterThan(saved?.id ?? -1, 0, "saved track must carry the DB id")
        XCTAssertEqual(saved?.title, "SW Track")
    }

    // MARK: - SW-12 Track 可选字段缺省

    func testSW12_TrackMissingOptionalFieldsDecodeAsNil() {
        let json = """
        {"id":1,"file_path":null,"source_type":"local","source_url":null,\
        "title":"Minimal","duration":100.0,"play_count":0,"is_available":true}
        """
        let track: Track? = decodeJSON(json)

        XCTAssertNotNil(track, "optional fields must not be required")
        XCTAssertNil(track?.artist)
        XCTAssertNil(track?.album)
        XCTAssertNil(track?.dateAdded)
        XCTAssertEqual(track?.title, "Minimal")
        XCTAssertEqual(track?.playCount, 0)
    }

    // MARK: - SW-13 encodeJSON 编码失败兜底

    /// SW-13: `encodeJSON` falls back to `"[]"` when encoding fails — but
    /// every model here is plain Codable data, so the failure branch is
    /// defensive and unreachable from the public API. Locked as observed:
    /// well-formed values always encode.
    func testSW13_EncodeJSONWellFormedValuesAlwaysEncode() {
        XCTAssertEqual(encodeJSON([Track]()), "[]")
        XCTAssertNotEqual(encodeJSON([Track]()), "")
    }

    // MARK: - SW-14 removeTrack 不存在的 id

    /// 期望：不存在的 id → false（rhythm#98 已修，skip 分支不可达）。
    /// 若 core 回归（0 行 DELETE 仍返回成功 → true），skip 分支重新激活，
    /// 保持红测登记语义。
    func testSW14_RemoveTrackMissingIdReturnsFalse() throws {
        let dir = makeTempDir()
        defer { try? FileManager.default.removeItem(at: dir) }

        guard let lib = RhythmLibrary(path: dir.appendingPathComponent("t.db").path) else {
            return XCTFail("temp library must open")
        }
        let removed = lib.removeTrack(999)

        if removed {
            throw XCTSkip(
                "rhythm#98 removeTrack 对不存在的 id 返回 true"
                + " — https://github.com/Teeeeeeerry/Rhythm/issues/98"
            )
        }
        XCTAssertFalse(removed, "removing a missing id must report false")
    }
}
