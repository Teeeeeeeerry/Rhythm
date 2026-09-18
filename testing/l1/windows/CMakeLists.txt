cmake_minimum_required(VERSION 3.20)
project(RhythmColorTests LANGUAGES CXX)

set(CMAKE_CXX_STANDARD 20)
set(CMAKE_CXX_STANDARD_REQUIRED ON)

# L1 Windows 测试：独立测试 exe，直接编译真实 RhythmCore.h（#122 解除桩）。
# RhythmCore.h 引入 pch.h（WinUI 头），故需与主工程相同的 WindowsAppSDK：
# 与主工程共用同一个依赖模块（固定版本下载 + 生成投影头，#386），不各有一套前提。

include(${CMAKE_CURRENT_SOURCE_DIR}/../../../windows/cmake/RhythmWindowsDeps.cmake)

add_executable(rhythm_color_tests source_color_test.cpp)

target_include_directories(rhythm_color_tests PRIVATE
    ${CMAKE_CURRENT_SOURCE_DIR}/../../../windows
    ${CMAKE_CURRENT_SOURCE_DIR}/../../../windows/Rhythm
    ${CMAKE_CURRENT_SOURCE_DIR}/../../../windows/Rhythm/Bridge
)

# 源文件含中文宽字面量；强制 UTF-8（主工程同款设置）。
if(MSVC)
    target_compile_options(rhythm_color_tests PRIVATE /utf-8)
endif()

# <Windows.h> 的 min/max 宏与标准库冲突（#408，主工程同款定义）。
target_compile_definitions(rhythm_color_tests PRIVATE NOMINMAX)

# RhythmCore.h -> L10n.h -> Bridge/MessageSpec.h 需要 <nlohmann/json.hpp>（#427）。
target_link_libraries(rhythm_color_tests PRIVATE Microsoft.WindowsAppSDK nlohmann_json::nlohmann_json)

enable_testing()
add_test(NAME rhythm_color_tests COMMAND rhythm_color_tests)
