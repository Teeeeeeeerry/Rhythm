import Foundation

struct Track: Identifiable, Codable, Equatable {
    let id: Int64
    var filePath: String?
    var sourceType: String
    var sourceUrl: String?
    var title: String
    var artist: String?
    var album: String?
    var albumArtist: String?
    var trackNumber: Int?
    var discNumber: Int?
    var genre: String?
    var year: Int?
    var duration: Double
    var format: String?
    var bitrate: Int?
    var sampleRate: Int?
    var channels: Int?
    var fileSize: Int64?
    var dateAdded: String?
    var lastPlayed: String?
    var playCount: Int
    var artworkPath: String?
    var isAvailable: Bool

    var durationFormatted: String {
        let m = Int(duration) / 60
        let s = Int(duration) % 60
        return String(format: "%d:%02d", m, s)
    }

    var displayLabel: String {
        title
    }
}

struct PlaylistInfo: Identifiable, Codable {
    let id: Int64
    var name: String
    var description: String?
    var dateCreated: String?
    var dateModified: String?
    var tracks: [Track]
}
