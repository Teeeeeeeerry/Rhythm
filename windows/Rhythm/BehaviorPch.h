#pragma once

// Common prefix of the behaviour library (#329): the standard library and
// Win32 only. No WinUI / Windows App SDK header belongs here or in any
// behaviour source -- the XAML shell's prefix (pch.h) adds those on top, and
// tests/BehaviorHeadersStandalone.cpp fails the build if one leaks in.

#include <Windows.h>

#include <string>
#include <vector>
#include <memory>
#include <optional>
#include <algorithm>
#include <format>
#include <functional>
#include <atomic>
