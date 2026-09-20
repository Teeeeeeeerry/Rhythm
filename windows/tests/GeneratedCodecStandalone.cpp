// 门的第二条（#369）：生成的编解码器由 RhythmGeneratedCodec 这一个目标编译。
//
// 契约校验（testing/l0/check-ffi-contract.py）比对的是文本，看不见「生成器产不出
// 可编译代码」这一类问题；本翻译单元把生成物单独编译一次——没有预编译头、前面也没有
// 任何模型定义（#413），所以生成物少声明一个包含、或对某个字段类型分派错了，都在这里
// 立刻红。反面证据见 GeneratedCodecTypeError.cpp。
#include "Bridge/GeneratedCodec.h"
