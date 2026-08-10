import SwiftUI

struct ArtistAlbumView: View {
    @EnvironmentObject var appState: AppState

    var body: some View {
        let grouped = groupByArtistAlbum()
        if grouped.isEmpty {
            Text("无内容").foregroundStyle(.rhythmTextSecondary)
                .frame(maxWidth: .infinity, maxHeight: .infinity)
        } else {
            List {
                ForEach(grouped, id: \.0) { artist, albums in
                    Section(artist) {
                        ForEach(albums, id: \.0) { album, tracks in
                            AlbumRow(album: album, tracks: tracks)
                        }
                    }
                }
            }
            .listStyle(.inset)
        }
    }

    private func groupByArtistAlbum() -> [(String, [(String, [Track])])] {
        var artists: [String: [String: [Track]]] = [:]
        for track in appState.tracks {
            let artist = track.artist ?? "Unknown Artist"
            let album = track.album ?? "Unknown Album"
            artists[artist, default: [:]][album, default: []].append(track)
        }
        return artists
            .map { (artist, albums) in
                (artist, albums.map { ($0.key, $0.value.sorted { ($0.discNumber ?? 0, $0.trackNumber ?? 0) < ($1.discNumber ?? 0, $1.trackNumber ?? 0) }) }
                    .sorted { $0.0 < $1.0 })
            }
            .sorted { $0.0 < $1.0 }
    }
}

struct AlbumRow: View {
    let album: String
    let tracks: [Track]
    @EnvironmentObject var appState: AppState

    var body: some View {
        HStack(spacing: 8) {
            // Album artwork thumbnail
            if let artPath = tracks.first(where: { $0.artworkPath != nil })?.artworkPath,
               let nsImage = NSImage(contentsOfFile: artPath) {
                Image(nsImage: nsImage)
                    .resizable()
                    .aspectRatio(contentMode: .fill)
                    .frame(width: 48, height: 48)
                    .cornerRadius(4)
            } else {
                RoundedRectangle(cornerRadius: 4)
                    .fill(.rhythmElevated)
                    .frame(width: 48, height: 48)
                    .overlay(
                        Image(systemName: "music.note.list")
                            .font(.caption)
                            .foregroundStyle(.rhythmTextSecondary)
                    )
            }

            VStack(alignment: .leading, spacing: 2) {
                Text(album)
                    .font(.headline)
                ForEach(tracks) { track in
                    TrackRowView(track: track)
                }
            }
        }
        .padding(.vertical, 2)
    }
}

struct TrackRowView: View {
    let track: Track
    @EnvironmentObject var appState: AppState

    var body: some View {
        HStack {
            VStack(alignment: .leading, spacing: 1) {
                Text(track.title)
                    .foregroundStyle(.rhythmTextPrimary)
                    .lineLimit(1)
                if let artist = track.artist {
                    Text(artist)
                        .font(.caption)
                        .foregroundStyle(.rhythmTextSecondary)
                }
            }
            Spacer()
            SourceTagView(sourceType: track.sourceType)
            Text(track.durationFormatted)
                .font(.caption)
                .foregroundStyle(.rhythmTextSecondary)
                .monospacedDigit()
        }
        .padding(.vertical, 1)
        .contentShape(Rectangle())
        .onTapGesture { appState.selectedTrackID = track.id }
        .onTapGesture(count: 2) { appState.playTrack(track) }
        .contextMenu {
            Button(L10n.play) { appState.playTrack(track) }
            Divider()
            Menu(L10n.addToPlaylist) {
                ForEach(appState.playlists) { pl in
                    Button(pl.name) {
                        if let id = pl.id {
                            appState.library?.addToPlaylist(playlistId: id, trackId: track.id)
                        }
                    }
                }
            }
            Divider()
            Button(L10n.deleteFromLibrary, role: .destructive) {
                appState.requestDeleteTrack(track)
            }
        }
    }
}

struct SourceTagView: View {
    let sourceType: String

    var color: Color {
        switch sourceType {
        case "local": .blue
        case "youtube": .red
        case "bilibili": .pink
        case "direct_url": .green
        default: .gray
        }
    }

    var label: String {
        switch sourceType {
        case "local": "本地"
        case "youtube": "YT"
        case "bilibili": "B站"
        case "direct_url": "链接"
        default: ""
        }
    }

    var body: some View {
        Text(label)
            .font(.caption2)
            .padding(.horizontal, 4)
            .padding(.vertical, 1)
            .background(color.opacity(0.15))
            .foregroundColor(color)
            .clipShape(RoundedRectangle(cornerRadius: 3))
    }
}
