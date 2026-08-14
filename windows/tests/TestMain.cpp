// Test main for the Wave 4a behavior suites (WA/WB manifests,
// docs/testing/behavior/windows-appstate.md + rhythmcore-windows.md).
//
// `AppState` derives from `winrt::implements`, so constructing it requires
// an apartment — initialised exactly once here, before any test runs.
// Catch2's default main is replaced with a custom one for that reason.

#include "pch.h"

#include <catch_amalgamated.hpp>

int main(int argc, char** argv) {
    winrt::init_apartment(winrt::apartment_type::single_threaded);
    return Catch::Session().run(argc, argv);
}
