#include "pch.h"
#include "SidebarView.xaml.h"

namespace winrt::Rhythm::Views::implementation {

SidebarView::SidebarView() {
    InitializeComponent();
    sidebarList().SelectedIndex(0);
}

void SidebarView::OnSelectionChanged(IInspectable const&, SelectionChangedEventArgs const&) {
    // Handled by MainWindow via NavigationView
}

} // namespace winrt::Rhythm::Views::implementation
