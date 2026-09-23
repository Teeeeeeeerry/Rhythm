#pragma once

#include "Models/PlaylistItem.g.h"
#include "ViewState.h"

namespace winrt::Rhythm::Models::implementation {

/// A playlist row for x:Bind (#428): wraps a view-state row and only copies
/// its fields into bindings -- what to render is decided by the view state
/// (#320), and the row carries the identifier a click selects.
struct PlaylistItem : PlaylistItemT<PlaylistItem> {
    explicit PlaylistItem(rhythm::view::PlaylistRow row);

    hstring Name() const;
    hstring TrackCountText() const;

    /// The row's playlist identifier (not projected): what the click handler
    /// hands to `AppState::SelectPlaylist`.
    int64_t Id() const { return row_.id; }

private:
    rhythm::view::PlaylistRow row_;
};

} // namespace winrt::Rhythm::Models::implementation
