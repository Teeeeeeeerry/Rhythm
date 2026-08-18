#include "pch.h"
#include "SidebarView.xaml.h"
#include "L10n.h"

namespace winrt::Rhythm::Views::implementation {

SidebarView::SidebarView() {
    InitializeComponent();
    // #141: copy from the language layer.
    sidebarLibrary().Text(rhythm::L10n::LibraryTab());
    sidebarPlaylists().Text(rhythm::L10n::PlaylistsTab());
    sidebarList().SelectedIndex(0);
}

void SidebarView::OnSelectionChanged(IInspectable const&, SelectionChangedEventArgs const&) {
    // Handled by MainWindow via NavigationView
}

} // namespace winrt::Rhythm::Views::implementation
