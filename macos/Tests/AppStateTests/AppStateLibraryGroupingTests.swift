import XCTest
import Foundation
@testable import Rhythm

/// AS-45（#518）：按艺人/专辑分组里缺失的艺人/专辑归入「未知」组，
/// 组名取键表 `unknown_artist` / `unknown_album`，并按当前语言的文案与
/// 其他名字一起排序（与 Windows VS-50 一致）。
final class AppStateLibraryGroupingTests: AppStatePlaybackTestCase {

    private func track(_ title: String, artist: String?, album: String?) -> Track {
        var track = makeTrack(title: title)
        track.artist = artist
        track.album = album
        return track
    }

    private func sections(locale: String) -> [ArtistSection] {
        UserDefaults.standard.set(locale, forKey: "AppLanguage")
        defer { UserDefaults.standard.removeObject(forKey: "AppLanguage") }
        return appState.artistAlbumSections
    }

    private func useLibraryWithMissingTags() {
        appState.tracks = [
            track("No Tags", artist: nil, album: nil),
            track("Zed Song", artist: "Zed", album: "Zulu"),
            track("Abba Single", artist: "Abba", album: nil),
            track("Abba Hit", artist: "Abba", album: "Gold"),
        ]
    }

    func testMissingArtistAndAlbum_UseChineseLabels_AndSortAfterLatinArtists() {
        useLibraryWithMissingTags()

        let result = sections(locale: "zh")

        XCTAssertEqual(result.map(\.name), ["Abba", "Zed", "未知艺人"])
        XCTAssertEqual(result[0].albums.map(\.name), ["Gold", "未知专辑"])
        XCTAssertEqual(result[2].albums.map(\.name), ["未知专辑"])
        XCTAssertEqual(result[2].albums[0].tracks.map(\.title), ["No Tags"])
    }

    func testMissingArtistAndAlbum_UseEnglishLabels_AndSortAmongArtists() {
        useLibraryWithMissingTags()

        let result = sections(locale: "en")

        XCTAssertEqual(result.map(\.name), ["Abba", "Unknown Artist", "Zed"])
        XCTAssertEqual(result[0].albums.map(\.name), ["Gold", "Unknown Album"])
        XCTAssertEqual(result[1].albums.map(\.name), ["Unknown Album"])
    }

    func testUnknownAlbumUnderTwoArtists_StaysTwoGroups() {
        appState.tracks = [
            track("One", artist: "Abba", album: nil),
            track("Two", artist: nil, album: nil),
        ]

        let albums = sections(locale: "zh").flatMap(\.albums)

        XCTAssertEqual(albums.count, 2)
        XCTAssertEqual(Set(albums.map(\.id)).count, 2, "专辑组 id 跨艺人不得碰撞（#66）")
    }
}
