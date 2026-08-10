import SwiftUI
#if SWIFT_PACKAGE
import RhythmTheme
#endif

struct PlayerBarView: View {
    @EnvironmentObject var appState: AppState

    var body: some View {
        VStack(spacing: 0) {
            Rectangle()
                .fill(.rhythmBorder)
                .frame(height: 1)
            HStack(spacing: 12) {
                trackInfo
                Spacer()
                playbackControls
                Spacer()
                HStack(spacing: 8) {
                    Image(systemName: "speaker.fill")
                        .font(.caption)
                        .foregroundStyle(.rhythmTextSecondary)
                    Slider(value: $appState.volume, in: 0...1) { _ in
                        appState.player.setVolume(Float(appState.volume))
                    }
                    .frame(width: 80)
                }
                Text(formattedTime)
                    .font(.caption)
                    .foregroundStyle(.rhythmTextSecondary)
                    .monospacedDigit()
                    .frame(width: 80, alignment: .trailing)
            }
            .padding(.horizontal, 16)
            .padding(.vertical, 8)
            .frame(height: 56)

            urlBar

            ProgressView(
                value: appState.duration > 0 ? appState.position / appState.duration : 0
            )
            .progressViewStyle(.linear)
            .tint(.rhythmAccent)
            .padding(.horizontal, 16)
            .padding(.bottom, 4)
        }
        .background(.rhythmSurface)
        .alert(
            L10n.urlErrorTitle,
            isPresented: Binding(
                get: { appState.urlError != nil },
                set: { if !$0 { appState.urlError = nil } }
            )
        ) {
            Button(L10n.ok, role: .cancel) {}
        } message: {
            Text(appState.urlError ?? "")
        }
    }

    /// URL input bar: paste a link and hit enter (or the play button) to
    /// resolve and play it.
    var urlBar: some View {
        HStack(spacing: 6) {
            Image(systemName: "link")
                .font(.caption)
                .foregroundStyle(.rhythmTextSecondary)
            TextField(L10n.urlPlaceholder, text: $appState.urlInput)
                .textFieldStyle(.plain)
                .font(.system(size: 11))
                .onSubmit { appState.resolveAndPlay(appState.urlInput) }
            if appState.isResolvingURL {
                // A fresh install downloads yt-dlp on the first link, which
                // takes long enough that a bare spinner reads as a hang.
                if !appState.urlStatus.isEmpty {
                    Text(appState.urlStatus)
                        .font(.system(size: 10))
                        .foregroundStyle(.rhythmTextSecondary)
                        .lineLimit(1)
                        .fixedSize()
                }
                ProgressView()
                    .controlSize(.small)
                    .help(appState.urlStatus.isEmpty ? L10n.urlResolving : appState.urlStatus)
            } else {
                Button(action: { appState.resolveAndPlay(appState.urlInput) }) {
                    Image(systemName: "arrow.up.circle.fill")
                        .font(.system(size: 14))
                }
                .buttonStyle(.plain)
                .disabled(appState.urlInput.trimmingCharacters(in: .whitespaces).isEmpty)
                .help(L10n.urlPlay)
            }
        }
        .padding(.horizontal, 16)
        .padding(.vertical, 6)
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
                        .foregroundStyle(.rhythmTextSecondary)
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
                    .fill(.rhythmElevated)
                    .frame(width: 36, height: 36)
                    .overlay(
                        Image(systemName: "music.note")
                            .font(.caption2)
                            .foregroundStyle(.rhythmTextSecondary)
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

            Button(action: { appState.stop() }) {
                Image(systemName: "stop.fill")
            }
            .buttonStyle(.plain)
            .disabled(!appState.isPlaying)

            Button(action: { appState.playNext() }) {
                Image(systemName: "forward.fill")
            }
            .buttonStyle(.plain)
            .disabled(!appState.isPlaying)

            Button(action: { appState.cyclePlayMode() }) {
                Image(systemName: appState.playMode.icon)
                    .font(.caption)
                    .foregroundStyle(appState.playMode == .sequential ? AnyShapeStyle(.rhythmTextSecondary) : AnyShapeStyle(.rhythmAccent))
            }
            .buttonStyle(.plain)
            .help(appState.playMode.label)
        }
    }

    var formattedTime: String {
        // Resolving + connecting + prebuffering can take a while on a link;
        // showing 0:00 / 0:00 for all of it reads as a dead player (#23).
        if appState.isBuffering { return L10n.buffering }
        let pos = appState.position
        let dur = appState.duration
        let pm = Int(pos) / 60
        let ps = Int(pos) % 60
        let dm = Int(dur) / 60
        let ds = Int(dur) % 60
        return String(format: "%d:%02d / %d:%02d", pm, ps, dm, ds)
    }
}
