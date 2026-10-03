#pragma once

#include "Models/LibraryHeaderItem.g.h"
#include "Models/AlbumGroupItem.g.h"

namespace winrt::Rhythm::Models::implementation {

/// A section heading of the library list for x:Bind (#503): only copies the
/// title the view state gave the section.
struct LibraryHeaderItem : LibraryHeaderItemT<LibraryHeaderItem> {
    explicit LibraryHeaderItem(std::wstring title) : title_(std::move(title)) {}

    hstring Title() const { return hstring{title_}; }

private:
    std::wstring title_;
};

/// An album group's heading for x:Bind (#503): the title the view state gave
/// the group; the template draws the cover placeholder beside it.
struct AlbumGroupItem : AlbumGroupItemT<AlbumGroupItem> {
    explicit AlbumGroupItem(std::wstring title) : title_(std::move(title)) {}

    hstring Title() const { return hstring{title_}; }

private:
    std::wstring title_;
};

} // namespace winrt::Rhythm::Models::implementation
