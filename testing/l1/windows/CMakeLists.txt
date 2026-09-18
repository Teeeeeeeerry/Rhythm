cmake_minimum_required(VERSION 3.21)
project(RhythmColorTests LANGUAGES CXX)

set(CMAKE_CXX_STANDARD 20)
set(CMAKE_CXX_STANDARD_REQUIRED ON)

# L1 Windows 测试：独立测试 exe，直接测真实曲目模型（#122 解除桩）。
# 链接与主工程同一份行为库声明（windows/cmake/RhythmBehavior.cmake）：包含目录、
# 编译选项与核心产物都从库目标取，不再抄一份（#330）；依赖取自共用的依赖模块（#386），
# 不链接 Windows App SDK 运行时（#325）。

include(${CMAKE_CURRENT_SOURCE_DIR}/../../../windows/cmake/RhythmBehavior.cmake)

add_executable(rhythm_color_tests source_color_test.cpp)

target_link_libraries(rhythm_color_tests PRIVATE RhythmBehavior)

# 运行时依赖（核心 DLL）从链接的目标取，放到可执行文件旁（#330）。
add_custom_command(TARGET rhythm_color_tests POST_BUILD
    COMMAND ${CMAKE_COMMAND} -E copy_if_different
        $<TARGET_RUNTIME_DLLS:rhythm_color_tests> $<TARGET_FILE_DIR:rhythm_color_tests>
    COMMAND_EXPAND_LISTS
)

enable_testing()
add_test(NAME rhythm_color_tests COMMAND rhythm_color_tests)
