# Windows dependencies, declared and fetched in the repository (#386).
#
# find_package(Microsoft.WindowsAppSDK) cannot work: upstream ships the Windows
# App SDK as a NuGet package holding only MSBuild .props/.targets, no CMake
# package config, and vcpkg has no port. nlohmann_json had the same "assume it
# is installed" problem. This module turns both into one reproducible step:
#
#   1. download pinned NuGet packages / release files, verified by SHA-256
#   2. expose nlohmann_json::nlohmann_json, the one import CMake links: the
#      behaviour library and its test hosts use no C++/WinRT (#325/#342)
#   3. write the props file the MSBuild app imports (Windows App SDK, WebView2
#      and the C++/WinRT build tooling; the app generates its own projection)
#
# Everything lands under <repo>/build/windows-deps (the build/ convention; one
# copy shared by the app and the test hosts). Prerequisites that are NOT fetched
# here: MSVC + Windows SDK (Visual Studio 2022 Build Tools) and network access
# on the first configure. Bumping a pin means changing version + hash below.
#
# Include once per project: include(<repo>/windows/cmake/RhythmWindowsDeps.cmake)

include_guard(GLOBAL)

get_filename_component(RHYTHM_REPO_ROOT "${CMAKE_CURRENT_LIST_DIR}/../.." ABSOLUTE)
set(RHYTHM_WINDOWS_DEPS_DIR "${RHYTHM_REPO_ROOT}/build/windows-deps"
    CACHE PATH "Download and generation cache for Windows dependencies (#386)")

# name | version | sha256 of the .nupkg
set(RHYTHM_WASDK_VERSION    "1.6.250602001")
set(RHYTHM_WASDK_SHA256     "a54f9025e44f8222ba225eb3a7755b4f60d774635482325e47717626481d841d")
set(RHYTHM_WEBVIEW2_VERSION "1.0.2651.64")
set(RHYTHM_WEBVIEW2_SHA256  "c7909cdab68a56d1b8eeb1d5b18b0e00122ed812a075a8be785ed48de689eeca")
set(RHYTHM_CPPWINRT_VERSION "2.0.250303.1")
set(RHYTHM_CPPWINRT_SHA256  "955e3051b35db1c00177ed14ab6b7b995c3b32a53fdb6c184ea53b1dba66e439")
set(RHYTHM_JSON_VERSION     "3.12.0")
set(RHYTHM_JSON_SHA256      "aaf127c04cb31c406e5b04a63f1ae89369fccde6d8fa7cdda1ed4f32dfc5de63")

# Download url to dest unless a verified copy is already there. Failures name
# the file, the url and what to do - never a bare "package not found".
function(_rhythm_download url dest sha256)
    if(EXISTS "${dest}")
        file(SHA256 "${dest}" have)
        if(have STREQUAL sha256)
            return()
        endif()
        file(REMOVE "${dest}")
    endif()
    message(STATUS "Rhythm deps: downloading ${url}")
    file(DOWNLOAD "${url}" "${dest}.part" STATUS status TLS_VERIFY ON)
    list(GET status 0 code)
    if(NOT code EQUAL 0)
        list(GET status 1 reason)
        file(REMOVE "${dest}.part")
        message(FATAL_ERROR
            "Rhythm deps: download failed (${reason}): ${url}\n"
            "The first Windows configure needs network access to fetch pinned "
            "dependencies into ${RHYTHM_WINDOWS_DEPS_DIR}. Check the connection "
            "or proxy and re-run configure.")
    endif()
    file(SHA256 "${dest}.part" have)
    if(NOT have STREQUAL sha256)
        file(REMOVE "${dest}.part")
        message(FATAL_ERROR
            "Rhythm deps: SHA-256 mismatch for ${url}\n"
            "  expected ${sha256}\n  got      ${have}\n"
            "The pin in windows/cmake/RhythmWindowsDeps.cmake and the download "
            "disagree; do not bypass - update version and hash together.")
    endif()
    file(RENAME "${dest}.part" "${dest}")
endfunction()

# Fetch and unpack one NuGet package; sets <out_var> to its extracted root.
function(_rhythm_nuget id version sha256 out_var)
    string(TOLOWER "${id}" lower)
    set(root "${RHYTHM_WINDOWS_DEPS_DIR}/${lower}.${version}")
    set(pkg "${RHYTHM_WINDOWS_DEPS_DIR}/${lower}.${version}.nupkg")
    if(NOT EXISTS "${root}/.extracted")
        _rhythm_download(
            "https://api.nuget.org/v3-flatcontainer/${lower}/${version}/${lower}.${version}.nupkg"
            "${pkg}" "${sha256}")
        file(REMOVE_RECURSE "${root}")
        file(ARCHIVE_EXTRACT INPUT "${pkg}" DESTINATION "${root}")
        file(TOUCH "${root}/.extracted")
    endif()
    set(${out_var} "${root}" PARENT_SCOPE)
endfunction()

file(MAKE_DIRECTORY "${RHYTHM_WINDOWS_DEPS_DIR}")

_rhythm_nuget(Microsoft.WindowsAppSDK ${RHYTHM_WASDK_VERSION} ${RHYTHM_WASDK_SHA256} RHYTHM_WASDK_ROOT)
_rhythm_nuget(Microsoft.Web.WebView2 ${RHYTHM_WEBVIEW2_VERSION} ${RHYTHM_WEBVIEW2_SHA256} RHYTHM_WEBVIEW2_ROOT)
_rhythm_nuget(Microsoft.Windows.CppWinRT ${RHYTHM_CPPWINRT_VERSION} ${RHYTHM_CPPWINRT_SHA256} RHYTHM_CPPWINRT_ROOT)

# ---- nlohmann/json (single header) ---------------------------------------
set(RHYTHM_JSON_INCLUDE "${RHYTHM_WINDOWS_DEPS_DIR}/nlohmann-json.${RHYTHM_JSON_VERSION}/include")
_rhythm_download(
    "https://github.com/nlohmann/json/releases/download/v${RHYTHM_JSON_VERSION}/json.hpp"
    "${RHYTHM_JSON_INCLUDE}/nlohmann/json.hpp" "${RHYTHM_JSON_SHA256}")

# ---- Imported targets ------------------------------------------------------
# Nothing CMake builds uses the Windows App SDK or any C++/WinRT projection
# (#325/#342): the test hosts are unpackaged exes that could not activate a
# runtime class anyway (#418), and the app gets both from MSBuild.
if(NOT TARGET nlohmann_json::nlohmann_json)
    add_library(nlohmann_json::nlohmann_json INTERFACE IMPORTED GLOBAL)
    target_include_directories(nlohmann_json::nlohmann_json INTERFACE "${RHYTHM_JSON_INCLUDE}")
endif()

# ---- MSBuild view of the same pins (ADR-0004, #428) -----------------------
# The app is built by MSBuild (XAML markup compilation exists only as MSBuild
# tasks). It imports the packages fetched above through this file instead of
# declaring versions of its own, so the two builds cannot drift apart.
file(CONFIGURE OUTPUT "${RHYTHM_WINDOWS_DEPS_DIR}/RhythmWindowsDeps.props" CONTENT [=[
<?xml version="1.0" encoding="utf-8"?>
<!-- Generated by windows/cmake/RhythmWindowsDeps.cmake - do not edit. -->
<Project xmlns="http://schemas.microsoft.com/developer/msbuild/2003">
  <PropertyGroup>
    <RhythmWindowsAppSDKDir>@RHYTHM_WASDK_ROOT@/</RhythmWindowsAppSDKDir>
    <RhythmWebView2Dir>@RHYTHM_WEBVIEW2_ROOT@/</RhythmWebView2Dir>
    <RhythmCppWinRTDir>@RHYTHM_CPPWINRT_ROOT@/</RhythmCppWinRTDir>
  </PropertyGroup>
</Project>
]=] @ONLY)
