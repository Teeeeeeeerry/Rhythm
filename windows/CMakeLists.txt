cmake_minimum_required(VERSION 3.20)
project(Rhythm VERSION 0.5.30 LANGUAGES CXX)

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


# ─── Tests (Wave 4a) ────────────────────────────────────────────────

# Catch2 v3 header-only test target. The main target never built a test
# binary before; this also registers the pre-existing implicit dependency
# on nlohmann/json (RhythmCore.cpp includes <nlohmann/json.hpp> without
# the main target declaring it — tracked here explicitly for tests).
enable_testing()

find_package(nlohmann_json CONFIG REQUIRED)

set(TEST_SOURCES
    Rhythm/pch.h
    Rhythm/pch.cpp
    Rhythm/AppState.h
    Rhythm/AppState.cpp
    Rhythm/Bridge/RhythmCore.h
    Rhythm/Bridge/RhythmCore.cpp
    Rhythm/Models/Track.h
    Rhythm/Models/Track.cpp
    tests/TestMain.cpp
    tests/TestHelpers.h
    tests/vendor/catch_amalgamated.cpp
    tests/AppStateBehaviorTests.cpp
    tests/BridgeBehaviorTests.cpp
)

add_executable(RhythmTests ${TEST_SOURCES})

# catch_amalgamated.cpp ships its own main(); TestMain.cpp provides ours
# (with init_apartment), so tell Catch2 to stand down.
target_compile_definitions(RhythmTests PRIVATE CATCH_AMALGAMATED_CUSTOM_MAIN)

# The test sources contain Chinese literals; force UTF-8 so MSVC reads them
# regardless of the system code page. (The main target has the same latent
# issue — registered, not fixed here.)
if(MSVC)
    target_compile_options(RhythmTests PRIVATE /utf-8)
endif()

target_include_directories(RhythmTests PRIVATE
    ${CMAKE_CURRENT_SOURCE_DIR}/Rhythm
    ${CMAKE_CURRENT_SOURCE_DIR}/tests/vendor
)

target_link_libraries(RhythmTests PRIVATE
    Microsoft.WindowsAppSDK
    nlohmann_json::nlohmann_json
    ${CMAKE_CURRENT_SOURCE_DIR}/../target/release/rhythm_core.dll.lib
)

# Runtime dependencies for the test binary.
add_custom_command(TARGET RhythmTests POST_BUILD
    COMMAND ${CMAKE_COMMAND} -E copy_if_different
        ${CMAKE_CURRENT_SOURCE_DIR}/../target/release/rhythm_core.dll
        $<TARGET_FILE_DIR:RhythmTests>
)

add_test(NAME RhythmTests COMMAND RhythmTests)
