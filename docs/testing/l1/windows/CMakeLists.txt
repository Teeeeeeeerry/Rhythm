cmake_minimum_required(VERSION 3.20)
project(RhythmColorTests LANGUAGES CXX)

set(CMAKE_CXX_STANDARD 20)
set(CMAKE_CXX_STANDARD_REQUIRED ON)

# L1 Windows 测试：独立测试 exe（不依赖主工程链接链）。
# 注：source_color_test.cpp 按 F1 修复后的目标签名编写，F1 未落地前
# 编译失败属预期验收信号（勿注释掉断言绕过）。

# F1 修复后启用：include 主工程的头文件目录以拿到真实签名
# target_include_directories(rhythm_color_tests PRIVATE ${CMAKE_CURRENT_SOURCE_DIR}/../../Rhythm)

add_executable(rhythm_color_tests source_color_test.cpp)

enable_testing()
add_test(NAME rhythm_color_tests COMMAND rhythm_color_tests)
