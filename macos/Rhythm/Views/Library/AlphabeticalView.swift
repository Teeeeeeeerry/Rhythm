import SwiftUI

struct AlphabeticalView: View {
    @EnvironmentObject var appState: AppState

    var body: some View {
        let sections = groupByFirstLetter()
        if sections.isEmpty {
            Text("无内容").foregroundStyle(.rhythmTextSecondary)
                .frame(maxWidth: .infinity, maxHeight: .infinity)
        } else {
            ScrollViewReader { proxy in
                HStack(alignment: .top, spacing: 0) {
                    List {
                        ForEach(sections, id: \.0) { letter, tracks in
                            Section(letter) {
                                ForEach(tracks) { track in
                                    TrackRowView(track: track)
                                }
                            }
                            .id(letter)
                        }
                    }
                    .listStyle(.inset)

                    VStack(spacing: 0) {
                        ForEach(sections, id: \.0) { letter, _ in
                            Button(letter) {
                                withAnimation { proxy.scrollTo(letter, anchor: .top) }
                            }
                            .buttonStyle(.plain)
                            .font(.caption2)
                            .frame(width: 20, height: 16)
                        }
                    }
                    .padding(.trailing, 4)
                }
            }
        }
    }

    private func groupByFirstLetter() -> [(String, [Track])] {
        var groups: [String: [Track]] = [:]
        for track in appState.tracks {
            let key = String(track.title.prefix(1)).uppercased()
            let letter = key.first?.isLetter == true ? key : "#"
            groups[letter, default: []].append(track)
        }
        return groups
            .map { ($0.key, $0.value.sorted { $0.title.localizedCaseInsensitiveCompare($1.title) == .orderedAscending }) }
            .sorted { $0.0 < $1.0 }
    }
}
