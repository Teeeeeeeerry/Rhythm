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

} // namespace rhythm::view
