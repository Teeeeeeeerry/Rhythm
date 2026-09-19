// TF：测试夹具自身的行为（manifest: docs/testing/behavior/windows-appstate.md「测试设施」，#455）。
// 夹具出错时，红灯落在与被测行为无关的用例上——这里把夹具的约定单独锁住。

#include "BehaviorPch.h"
#include "AppState.h"

#include <catch_amalgamated.hpp>
#include "TestHelpers.h"

using namespace rhythm;
using namespace rhythm_tests;

// ─── TF-01 临时目录每个实例唯一（#455）──────────────────────────────

TEST_CASE("TF-01 TempDir instances made back to back get distinct, empty directories") {
    // Constructed within one GetTickCount64 tick (~15.6 ms): the old
    // pid + tick name handed both the same directory.
    TempDir a;
    TempDir b;

    REQUIRE(a.path != b.path);
    REQUIRE(fs::is_directory(a.path));
    REQUIRE(fs::is_directory(b.path));
    REQUIRE(fs::is_empty(a.path));
    REQUIRE(fs::is_empty(b.path));
}

// ─── TF-02 目录先于库声明时清理干净（#455）──────────────────────────

TEST_CASE("TF-02 a TempDir declared before the AppState using it is removed at scope end") {
    fs::path leftover;
    {
        TempDir dir;
        AppState state;
        state.OpenDatabase(dir.dbPath());
        writeWavAt(dir.path, L"tone.wav");
        state.Library->ImportDirectory(dir.path.wstring());
        leftover = dir.path;
    }
    // AppState is destroyed first and closes the database, so nothing inside
    // the directory is still open when it is removed.
    REQUIRE_FALSE(fs::exists(leftover));
}
