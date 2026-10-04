import SwiftUI
#if SWIFT_PACKAGE
import RhythmTheme
#endif

struct ArtistAlbumView: View {
    @EnvironmentObject var appState: AppState

    var body: some View {
        let sections = appState.artistAlbumSections
        if sections.isEmpty {
            Text(L10n.libraryEmpty).foregroundStyle(.rhythmTextSecondary)
                .frame(maxWidth: .infinity, maxHeight: .infinity)
        } else {
            List {
                ForEach(sections) { section in
                    Section(section.name) {
                        ForEach(section.albums) { entry in
                            AlbumRow(album: entry.name, tracks: entry.tracks)
                        }
                    }
                }
            }
            .listStyle(.inset)
        }
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
        .background(
            appState.selectedTrackID == track.id
                ? AnyShapeStyle(.rhythmAccent.opacity(0.12))
                : AnyShapeStyle(.clear)
        )
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
        case "local": .rhythmSourceLocal
        case "youtube": .rhythmSourceYoutube
        case "bilibili": .rhythmSourceBilibili
        case "direct_url": .rhythmSourceUrl
        default: .rhythmTextTertiary
        }
    }

    var label: String {
        switch sourceType {
        case "local": L10n.tagLocal
        case "youtube": L10n.tagYoutube
        case "bilibili": L10n.tagBilibili
        case "direct_url": L10n.tagLink
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
