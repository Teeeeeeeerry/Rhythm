# The Windows behaviour library, declared once (#330).
#
# Everything that is not XAML shell: app state, the core bridge and the L10n
# layer (ADR-0004, #428). Its source list, the usage requirements every
# consumer needs (include paths, NOMINMAX, /utf-8) and the Rust core it links
# live here and nowhere else:
#   - windows/CMakeLists.txt links it into the test host and writes the props
#     file the MSBuild app imports (values derived from this target);
#   - testing/l1/windows links it for the colour tests.
# Adding a behaviour source means adding one line to BEHAVIOR_SOURCES.
#
# Include once per project: include(<repo>/windows/cmake/RhythmBehavior.cmake)

include_guard(GLOBAL)
include(${CMAKE_CURRENT_LIST_DIR}/RhythmWindowsDeps.cmake)

set(RHYTHM_WINDOWS_DIR "${RHYTHM_REPO_ROOT}/windows")

# The Rust core: a DLL behind its import library, built by `cargo build
# --release` into the workspace target/ directory (task_build.core_artifact_dir).
# Consumers copy it next to their executables through $<TARGET_RUNTIME_DLLS>.
if(NOT TARGET Rhythm::Core)
    add_library(Rhythm::Core SHARED IMPORTED GLOBAL)
    set_target_properties(Rhythm::Core PROPERTIES
        IMPORTED_LOCATION "${RHYTHM_REPO_ROOT}/target/release/rhythm_core.dll"
        IMPORTED_IMPLIB "${RHYTHM_REPO_ROOT}/target/release/rhythm_core.dll.lib")
endif()

set(BEHAVIOR_SOURCES
    Rhythm/BehaviorPch.h
    Rhythm/L10n.h
    Rhythm/L10nAccessors.h
    Rhythm/AppState.h
    Rhythm/AppState.cpp
    Rhythm/Bridge/GeneratedCodec.h
    Rhythm/Bridge/L10nKeys.h
    Rhythm/Bridge/MessageSpec.h
    Rhythm/Bridge/RhythmCore.h
    Rhythm/Bridge/RhythmCore.cpp
    Rhythm/Bridge/rhythm_core.h
)
list(TRANSFORM BEHAVIOR_SOURCES PREPEND "${RHYTHM_WINDOWS_DIR}/")

add_library(RhythmBehavior STATIC ${BEHAVIOR_SOURCES})

# Bridge/ holds the core's C header, included as <rhythm_core.h> (#408).
target_include_directories(RhythmBehavior PUBLIC
    ${RHYTHM_WINDOWS_DIR}/Rhythm
    ${RHYTHM_WINDOWS_DIR}/Rhythm/Bridge
)

# <Windows.h> defines min/max macros that break std::min/max and Catch2 (#408).
target_compile_definitions(RhythmBehavior PUBLIC NOMINMAX)

# The L10n layer holds the remaining Chinese literals (localized fallback);
# force UTF-8 so MSVC reads them regardless of the system code page.
if(MSVC)
    target_compile_options(RhythmBehavior PUBLIC /utf-8)
endif()

# L10n.h includes Bridge/MessageSpec.h, which parses the core's message spec
# JSON. The C++/WinRT projection, not the Windows App SDK: neither the library
# nor the test hosts link the Windows App Runtime (#325).
target_link_libraries(RhythmBehavior PUBLIC
    Rhythm::CppWinRT
    nlohmann_json::nlohmann_json
    Rhythm::Core
)
