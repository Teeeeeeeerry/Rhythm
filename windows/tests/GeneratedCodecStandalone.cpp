// #413: compile check -- the generated codec included on its own, with no
// precompiled header or model definitions ahead of it. Building RhythmTests
// fails if GeneratedCodec.h stops declaring what it depends on.
#include "Bridge/GeneratedCodec.h"
