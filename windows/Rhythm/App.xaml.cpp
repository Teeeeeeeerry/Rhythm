#include "pch.h"
#include "App.xaml.h"
#include "MainWindow.xaml.h"

namespace winrt::Rhythm::implementation {

App::App() {
    InitializeComponent();
}

void App::OnLaunched(LaunchActivatedEventArgs const&) {
    window = make<MainWindow>();
    window.Activate();
}

} // namespace winrt::Rhythm::implementation
