// #329: compile check -- the behaviour library's headers, included on their
// own, pull in no WinUI / Windows App SDK projection. C++/WinRT defines one
// guard per namespace (WINRT_<Namespace>_0_H); any of these being defined
// means a WinUI header leaked into behaviour code. Those headers belong to
// the XAML shell's prefix (Rhythm/pch.h), never to BehaviorPch.h.
#include "BehaviorPch.h"
#include "AppState.h"
#include "L10n.h"
#include "Bridge/GeneratedCodec.h"
#include "Bridge/MessageSpec.h"
#include "Bridge/RhythmCore.h"

#if defined(WINRT_Microsoft_UI_0_H) || defined(WINRT_Microsoft_UI_Xaml_0_H) ||             \
    defined(WINRT_Microsoft_UI_Composition_0_H) || defined(WINRT_Microsoft_UI_Dispatching_0_H) || \
    defined(WINRT_Microsoft_UI_Windowing_0_H)
#error "A WinUI / Windows App SDK header leaked into the behaviour library (#329)."
#endif
