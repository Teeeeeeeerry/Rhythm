# FFI 层行为清单

- 模块：`rust-core/src/ffi/mod.rs`（C ABI 导出：opaque 句柄 + JSON 交换 + 空指针防御 + 错误码契约）
- #175 起：队列直通导出（`rhythm_queue_*`）已删除——队列状态由协调器持有（见 `coordinator.md`）；`ffi_behavior.rs` 相应测试移除，FF-11/FF-18/FF-19/FF-21 归档
- #176 起：`rhythm_resolve_url` 返回结构化结果（成功载荷 + 分类错误一次返回），不再用全局错误槽
- #179 起：状态与模式以具名枚举跨 seam（事件 state 字符串、PlayMode 契约值 0-3 双端锁定）；UI 层无状态/模式魔法整数
- #180 起：数据契约单一声明 `contracts/ffi-contract.json`，`scripts/gen-ffi-bindings.py` 生成双端编解码绑定（`macos/Rhythm/Models/GeneratedCodec.swift`、`windows/Rhythm/Bridge/GeneratedCodec.h`）；Windows 桥的 Track 编解码已迁移到生成绑定，macOS 以产物一致性测试锁定
- #181 起：旧直通导出收缩——`rhythm_player_*`（15 个，被协调器取代）与进程级全局错误槽 `rhythm_last_error` 删除；`classify_url`/`install_ytdlp` 改结构化结果；FF-07~10 归档，双端 Player 包装与测试删除
- #234 起：M3U8 导入新增「解析并入库」导出 `rhythm_import_m3u8_into_library`，返回具名结果 `M3u8ImportOutcome`（契约声明 `m3u8_import_outcome`，双端绑定由生成器产出）；旧的纯解析导出保持可用
- #237 起：资料库导入结果具名结构 `ImportOutcome`（`imported`/`unsupported`/`failed` 三计数分开）进入契约声明 `import_outcome`，双端绑定由生成器产出；expand 阶段与旧魔数导出并存，双端行为未变
- #239 起：三条导入路径的新导出落地（`rhythm_library_import_directory`/`_single_file`/`_paths`），返回同一结果形状；旧魔数导出仍并存，双端尚未切换
- #244 起：旧魔数导入导出 `rhythm_library_import` 与 `rhythm_library_import_file` 删除，资料库导入路径的魔数返回码清零；FF-03/FF-04 归档
- 历史回归：`#21`（解析失败只有 null、无原因）
- 测试途径：`cargo test` 集成测试（现有 `player_ffi.rs` 模式扩展）；library/queue/resolver 部分链接真实现 + 临时库；无需接缝。

## 主路径（P0 — 合并门槛）

| 编号 | 行为 | 断言 |
|---|---|---|
| FF-01 | library open/close 往返 | 合法路径 → 非空句柄；close 后不泄漏崩溃；close(null) 安全 |
| FF-02 | library open 失败 | 不可写路径 → null |
| FF-03 | ~~`rhythm_library_import`~~ | 已删除（#244）：导入路径不再有魔数返回码，覆盖归 FF-24 |
| FF-04 | ~~`rhythm_library_import_file` 返回值契约~~ | 已删除（#244）：同上，成功/不支持/失败三种结局改由 FF-24 的具名计数覆盖 |
| FF-24 | 三条导入路径的具名结果（#239） | `import_directory`/`import_single_file`/`import_paths` 返回同一形状 `{"imported":N,"unsupported":N,"failed":N}`；批量聚合来自核心；非法批量载荷 → 三项全 0；空句柄 → null |
| FF-05 | 字符串内存契约 | `get_all_tracks` 等返回 JSON 需 `rhythm_free_string` 释放；空指针入参 → null/-1 安全默认 |
| FF-06 | `rhythm_library_add_track` JSON 往返 | 合法 JSON → 带 DB id 的 JSON；非法 JSON → null |
| FF-12 | resolve 结构化结果（#176） | 一次返回 `{"ok":true,"resolved":{...}}` 或 `{"ok":false,"error_kind":"...","error_message":"..."}`；不再返回 null、不再读全局错误槽 |
| FF-13 | `rhythm_classify_url` | 结构化结果：成功 `{"ok":true,"source_type":...}`；失败 `{"ok":false,"error_kind":...}`（#181，无全局错误槽） |
| FF-14 | M3U8 FFI | export 成功 0/失败 -1；import 成功 JSON/失败 null |
| FF-23 | M3U8 解析并入库（#234） | 返回具名结果 JSON `{"imported":N,"failed":M}`；不可读列表与空句柄 → null |
| FF-25 | 消息规格导出（#227） | `rhythm_message_playback_failure(kind, detail, language)` 返回 `{"segments":[…]}`：键段带 `key`/`params`，字面量段带 `text`；空指针与未知分类 → 泛化标题单段 |
| FF-15 | `rhythm_free_string` | 空指针安全 |
| FF-16 | metadata FFI | `rhythm_metadata_extract`/`scan`/`extract_artwork`：成功 JSON/路径，失败 null |

## 边界情况（P1）

| 编号 | 行为 | 断言 |
|---|---|---|
| FF-17 | 空字符串路径入参 | 不崩溃（现状：open("") 触发 SQLite 临时库语义 → 非空句柄可正常 close，`ff17_empty_path_inputs_do_not_crash` 锁定"不崩溃"为断言核心） |
| FF-18 | 各错误码函数空指针 | 全部返回安全默认（-1/0/null/false） |

## 错误路径（P2）

| 编号 | 行为 | 断言 |
|---|---|---|
| FF-22 | `rhythm_library_remove_track` 不存在的 id（#98） | 返回 -1（空库、已删 id 均验证）；真实 id 返回 0 |

## 红测登记

| 编号 | 缺陷 | issue | 状态 |
|---|---|---|---|
| FF-04 | `rhythm_library_import_file` 从不返回 0：不支持格式映射为 -1，Swift 侧"不支持的音频格式"分支死代码 | [#79](https://github.com/Teeeeeeerry/Rhythm/issues/79) | 已修复（#108）；导出本身已随 #244 删除，语义由具名计数承载 |
