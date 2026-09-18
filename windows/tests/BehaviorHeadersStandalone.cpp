// #329/#342: compile check -- the behaviour library's headers, included on
// their own, pull in no C++/WinRT projection at all: no WinUI / Windows App
// SDK (#329) and, since the theme lookup moved to the shell (#342), no
// Windows SDK projection either. Every C++/WinRT header includes winrt/base.h,
// whose guard is WINRT_BASE_H. WinRT belongs to the XAML shell's prefix
// (Rhythm/pch.h), never to BehaviorPch.h.
#include "BehaviorPch.h"
#include "AppState.h"
#include "L10n.h"
#include "Bridge/GeneratedCodec.h"
#include "Bridge/MessageSpec.h"
#include "Bridge/RhythmCore.h"

#if defined(WINRT_BASE_H)
#error "A C++/WinRT header leaked into the behaviour library (#329/#342)."
#endif
