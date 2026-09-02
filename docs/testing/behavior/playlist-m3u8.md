# Playlist（M3U8）行为清单

- 模块：`rust-core/src/playlist/mod.rs`（M3U8 导入/导出）
- 历史回归：`#34`（M3U8 导出静默失败，修复后无回归测试）、`#136` 与 `#173`（解析结果从未入库——
  同一缺陷分别在 macOS 与 Windows 各修一次，是入库策略没有单一出处的必然结果，#217 把策略下沉到本模块）
- 测试途径：`cargo test` 单元测试 + tempfile；解析段纯文件 IO，入库段用真实 SQLite 临时库，均无需接缝。
  `#136`/`#173` 的入库断言自 #233 起归属核心（`playlist_m3u8_behavior.rs` PL-17–24）。

## 主路径（P0 — 合并门槛）

| 编号 | 行为 | 断言 |
|---|---|---|
| PL-01 | `export_m3u8` 格式 | 首行 `#EXTM3U`；每曲 `#EXTINF:{整数秒},{artist} - {title}` + 位置行 |
| PL-02 | `export_m3u8` 位置行 | local → `file_path`；URL → `source_url`（断言并入 `pl01_export_m3u8_format_and_locations`） |
| PL-03 | `export_m3u8` 缺 artist | 回退 "Unknown Artist" |
| PL-04 | `import_m3u8` 标准解析 | 返回具名字段 `M3u8Entry{title, artist, location}` 列表，顺序保真（#177） |
| PL-05 | `import_m3u8` 头行与空行 | `#EXTM3U` 与空白行被忽略 |
| PL-06 | `import_m3u8` EXTINF 无 ` - ` 分隔 | title = 逗号后全文，artist = None |
| PL-07 | `import_m3u8` 位置行无前置 EXTINF | title 回退为文件 stem |
| PL-08 | `import_m3u8` 其他 `#` 注释行 | 忽略 |
| PL-09 | `export_m3u8` 写入失败（#34） | 路径不可写 → 返回 Err（不静默、不 panic） |

## 边界情况（P1）

| 编号 | 行为 | 断言 |
|---|---|---|
| PL-10 | 空曲目列表导出 | 仅 `#EXTM3U` 头 |
| PL-11 | `import_m3u8` 文件不存在 | 返回 Err |
| PL-12 | 导出→导入往返 | title/artist/location 保真 |
| PL-13 | 小数/负 duration 导出 | EXTINF 取整数秒（`as i64` 截断），不 panic |
| PL-24 | `import_m3u8_into_library` 文件不存在 | 返回 Err（解析失败即整体失败，不产出计数） |

## 入库策略（P0 — 解析并入库入口，#233）

`import_m3u8_into_library` 一次完成解析与入库，返回具名结果 `M3u8ImportOutcome{imported, failed}`。
入库判定、位置类型识别、标题回退三条规则只在本模块决定，不跨接缝暴露。

| 编号 | 行为 | 断言 |
|---|---|---|
| PL-17 | 全部成功 | 每条写入数据库，`imported` = 条目数，`failed` = 0 |
| PL-18 | 位置为空 | 计入 `failed`，不写库 |
| PL-19 | 混合来源 | `http(s)` → `direct_url` + `source_url`；其余 → `local` + `file_path` |
| PL-20 | 标题缺失 | 回退占位名 `Unknown` |
| PL-21 | 空列表 | `imported` = 0，`failed` = 0，库中无新增 |
| PL-22 | 全部失败 | `imported` = 0，`failed` = 条目数 |
| PL-23 | 部分失败 | 成功与失败分项计数各自正确 |

## 错误路径（P2 — 仅断言"错误被正确上报"）

| 编号 | 行为 | 断言 |
|---|---|---|
| PL-14 | 非法 UTF-8 的 M3U8 文件 | 返回 Err，不 panic |

## 视图层（macOS，#235 起只剩结果渲染）

入库策略已归属核心（PL-17 至 PL-24），macOS 侧只做三步：调用核心入口、按结果选提示语、从数据库重载列表。

| 编号 | 行为 | 断言 |
|---|---|---|
| PL-15 | `AppState.importM3U8` 结果渲染 | 具名结果 `{imported, failed}` → 全成 `importedTracks`、有失败 `importSomeFailed`；调用后从数据库重载列表 |
| PL-16 | `AppState.importM3U8` 列表不可读 | 核心返回 null → 不弹提示、列表不变 |

## 红测登记

（空。全部绿。）

## 错误路径状态

PL-14（非法 UTF-8）已随 Wave 3a 覆盖（`playlist_m3u8_behavior.rs`），无需顺延。
