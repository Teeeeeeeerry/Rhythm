import SwiftUI

struct PlayerBarView: View {
    @EnvironmentObject var appState: AppState

    var body: some View {
        VStack(spacing: 0) {
            Divider()
            HStack(spacing: 12) {
                trackInfo
                Spacer()
                playbackControls
                Spacer()
                HStack(spacing: 8) {
                    Image(systemName: "speaker.fill")
                        .font(.caption)
                        .foregroundStyle(.secondary)
                    Slider(value: $appState.volume, in: 0...1) { _ in
                        appState.player.setVolume(Float(appState.volume))
                    }
                    .frame(width: 80)
                }
                Text(formattedTime)
                    .font(.caption)
                    .foregroundStyle(.secondary)
                    .monospacedDigit()
                    .frame(width: 80, alignment: .trailing)
            }
            .padding(.horizontal, 16)
            .padding(.vertical, 8)
            .frame(height: 56)

            ProgressView(
                value: appState.duration > 0 ? appState.position / appState.duration : 0
            )
            .progressViewStyle(.linear)
            .tint(.accentColor)
            .padding(.horizontal, 16)
            .padding(.bottom, 4)
        }
        .background(.bar)
    }

    var trackInfo: some View {
        HStack(spacing: 8) {
            coverArt
            VStack(alignment: .leading, spacing: 1) {
                Text(appState.currentTrack?.title ?? L10n.notPlaying)
                    .font(.system(size: 12, weight: .medium))
                    .lineLimit(1)
                if let artist = appState.currentTrack?.artist {
                    Text(artist)
                        .font(.system(size: 11))
                        .foregroundStyle(.secondary)
                        .lineLimit(1)
                }
            }
            .frame(width: 140, alignment: .leading)
        }
    }

    var coverArt: some View {
        Group {
            if let artPath = appState.currentTrack?.artworkPath,
               let nsImage = NSImage(contentsOfFile: artPath) {
                Image(nsImage: nsImage)
                    .resizable()
                    .aspectRatio(contentMode: .fill)
                    .frame(width: 36, height: 36)
                    .cornerRadius(4)
            } else {
                RoundedRectangle(cornerRadius: 4)
                    .fill(.quaternary)
                    .frame(width: 36, height: 36)
                    .overlay(
                        Image(systemName: "music.note")
                            .font(.caption2)
                            .foregroundStyle(.secondary)
                    )
            }
        }
    }

    var playbackControls: some View {
        HStack(spacing: 16) {
            Button(action: { appState.playPrevious() }) {
                Image(systemName: "backward.fill")
            }
            .buttonStyle(.plain)
            .disabled(!appState.isPlaying)

            Button(action: { appState.togglePlayPause() }) {
                Image(systemName: appState.isPlaying ? "pause.fill" : "play.fill")
                    .font(.system(size: 22))
            }
            .buttonStyle(.plain)
            .keyboardShortcut(.space, modifiers: [])

            Button(action: { appState.playNext() }) {
                Image(systemName: "forward.fill")
            }
            .buttonStyle(.plain)
            .disabled(!appState.isPlaying)

            Button(action: { appState.cyclePlayMode() }) {
                Image(systemName: appState.playMode.icon)
                    .font(.caption)
                    .foregroundStyle(appState.playMode == .sequential ? AnyShapeStyle(.secondary) : AnyShapeStyle(.tint))
            }
            .buttonStyle(.plain)
            .help(appState.playMode.label)
        }
    }

    var formattedTime: String {
        let pos = appState.position
        let dur = appState.duration
        let pm = Int(pos) / 60
        let ps = Int(pos) % 60
        let dm = Int(dur) / 60
        let ds = Int(dur) % 60
        return String(format: "%d:%02d / %d:%02d", pm, ps, dm, ds)
    }
}
