#include "BehaviorPch.h"
#include "ViewState.h"

namespace rhythm::view {

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
    if (state.Duration > 0) {
        bar.progressPercent = std::clamp(state.Position / state.Duration * 100.0, 0.0, 100.0);
    }
    return bar;
}

} // namespace rhythm::view
