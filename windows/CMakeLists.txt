cmake_minimum_required(VERSION 3.20)
project(Rhythm VERSION 0.5.29 LANGUAGES CXX)

set(CMAKE_CXX_STANDARD 20)
set(CMAKE_CXX_STANDARD_REQUIRED ON)

# Windows SDK
set(CMAKE_VS_WINDOWS_TARGET_PLATFORM_VERSION "10.0")

# Find WinUI 3 via NuGet or vcpkg
find_package(Microsoft.WindowsAppSDK REQUIRED)

# Source files
set(SOURCES
    Rhythm/pch.h
    Rhythm/pch.cpp
    Rhythm/App.xaml.cpp
    Rhythm/App.xaml.h
    Rhythm/MainWindow.xaml.cpp
    Rhythm/MainWindow.xaml.h
    Rhythm/AppState.h
    Rhythm/AppState.cpp
    Rhythm/Bridge/RhythmCore.h
    Rhythm/Bridge/RhythmCore.cpp
    Rhythm/Models/Track.h
    Rhythm/Models/Track.cpp
    Rhythm/Views/SidebarView.xaml
    Rhythm/Views/SidebarView.xaml.cpp
    Rhythm/Views/SidebarView.xaml.h
    Rhythm/Views/LibraryView.xaml
    Rhythm/Views/LibraryView.xaml.cpp
    Rhythm/Views/LibraryView.xaml.h
    Rhythm/Views/PlaylistListView.xaml
    Rhythm/Views/PlaylistListView.xaml.cpp
    Rhythm/Views/PlaylistListView.xaml.h
    Rhythm/Views/PlaylistDetailView.xaml
    Rhythm/Views/PlaylistDetailView.xaml.cpp
    Rhythm/Views/PlaylistDetailView.xaml.h
    Rhythm/Views/PlayerBarView.xaml
    Rhythm/Views/PlayerBarView.xaml.cpp
    Rhythm/Views/PlayerBarView.xaml.h
    Rhythm/Themes/Colors.xaml
    Rhythm/Views/TrayManager.h
    Rhythm/Views/TrayManager.cpp
)

# Create executable
add_executable(${PROJECT_NAME} WIN32 ${SOURCES})

target_include_directories(${PROJECT_NAME} PRIVATE
    ${CMAKE_CURRENT_SOURCE_DIR}/Rhythm
)

# Link WinUI 3
target_link_libraries(${PROJECT_NAME} PRIVATE
    Microsoft.WindowsAppSDK
)

# Link Rust core DLL (import library)
target_link_libraries(${PROJECT_NAME} PRIVATE
    ${CMAKE_CURRENT_SOURCE_DIR}/../target/release/rhythm_core.dll.lib
)

# Copy runtime dependencies
add_custom_command(TARGET ${PROJECT_NAME} POST_BUILD
    COMMAND ${CMAKE_COMMAND} -E copy_if_different
        ${CMAKE_CURRENT_SOURCE_DIR}/../target/release/rhythm_core.dll
        $<TARGET_FILE_DIR:${PROJECT_NAME}>
)

