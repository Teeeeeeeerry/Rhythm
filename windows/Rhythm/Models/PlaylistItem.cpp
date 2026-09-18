#include "pch.h"
#include "Models/PlaylistItem.h"
#if __has_include("Models/PlaylistItem.g.cpp")
#include "Models/PlaylistItem.g.cpp"
#endif

namespace winrt::Rhythm::Models::implementation {

PlaylistItem::PlaylistItem(rhythm::Playlist playlist) : playlist_(std::move(playlist)) {}

hstring PlaylistItem::Name() const { return hstring{playlist_.name}; }

hstring PlaylistItem::TrackCountText() const {
    return winrt::to_hstring(static_cast<uint64_t>(playlist_.tracks.size()));
}

} // namespace winrt::Rhythm::Models::implementation
