#pragma once

#include "Models/PlaylistItem.g.h"
#include "Bridge/RhythmCore.h"

namespace winrt::Rhythm::Models::implementation {

/// A playlist row for x:Bind (#428): wraps the behaviour-library Playlist.
struct PlaylistItem : PlaylistItemT<PlaylistItem> {
    explicit PlaylistItem(rhythm::Playlist playlist);

    hstring Name() const;
    hstring TrackCountText() const;

    /// The wrapped model (not projected).
    rhythm::Playlist const& Model() const { return playlist_; }

private:
    rhythm::Playlist playlist_;
};

} // namespace winrt::Rhythm::Models::implementation
