#include "pch.h"
#include "Models/PlaylistItem.h"
#if __has_include("Models/PlaylistItem.g.cpp")
#include "Models/PlaylistItem.g.cpp"
#endif

namespace winrt::Rhythm::Models::implementation {

PlaylistItem::PlaylistItem(rhythm::view::PlaylistRow row) : row_(std::move(row)) {}

hstring PlaylistItem::Name() const { return hstring{row_.name}; }

hstring PlaylistItem::TrackCountText() const { return hstring{row_.trackCountText}; }

} // namespace winrt::Rhythm::Models::implementation
