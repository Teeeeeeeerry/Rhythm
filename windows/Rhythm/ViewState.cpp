#include "BehaviorPch.h"
#include "ViewState.h"
#include "L10n.h"

namespace rhythm::view {

namespace {

/// Seconds as m:ss, seconds zero-padded; anything not positive is 0:00.
std::wstring MinutesSeconds(double seconds) {
    const int whole = seconds > 0 ? static_cast<int>(seconds) : 0;
    return std::format(L"{}:{:02}", whole / 60, whole % 60);
}

/// The play mode control's icon, like macOS `PlayMode.icon` (#411).
Icon PlayModeIcon(PlayMode mode) {
    switch (mode) {
        case PlayMode::Shuffle:    return Icon::Shuffle;
        case PlayMode::SingleLoop: return Icon::RepeatOne;
        case PlayMode::ListLoop:   return Icon::RepeatAll;
        case PlayMode::Sequential:
        default:                   return Icon::List;
    }
}

} // namespace

std::vector<TrackRow> LibraryRows(const AppState& state) {
    std::vector<TrackRow> rows;
    rows.reserve(state.Tracks.size());
    for (const auto& track : state.Tracks) {
        rows.push_back(TrackRow{track, track.title});
    }
    return rows;
}

PlayerBar PlayerBarState(const AppState& state) {
    PlayerBar bar;
    if (state.CurrentTrack) {
        bar.title = state.CurrentTrack->title;
        bar.artist = state.CurrentTrack->artist.value_or(L"");
    } else {
        bar.title = L10n::NotPlaying();
    }
    if (state.Duration > 0) {
        bar.progressPercent = std::clamp(state.Position / state.Duration * 100.0, 0.0, 100.0);
    }
    bar.timeText = state.IsBuffering
        ? L10n::Buffering()
        : MinutesSeconds(state.Position) + L" / " + MinutesSeconds(state.Duration);
    bar.playIcon = state.IsPlaying || state.IsBuffering ? Icon::Pause : Icon::Play;
    bar.playModeIcon = PlayModeIcon(state.CurrentMode);
    bar.volumePercent = state.Volume * 100.0;
    return bar;
}

} // namespace rhythm::view
