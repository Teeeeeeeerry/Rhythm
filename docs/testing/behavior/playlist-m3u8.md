# Playlist（M3U8）行为清单

- 模块：`rust-core/src/playlist/mod.rs`（M3U8 导入/导出）
- 历史回归：`#34`（M3U8 导出静默失败，修复后无回归测试）、`#136`（macOS 导入后解析结果被丢弃，从未入库）
- 测试途径：`cargo test` 单元测试 + tempfile；纯文件 IO，无需接缝。`#136` 的入库断言走 macOS `AppStateImportTests.importM3U8Entries`。

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

## 错误路径（P2 — 仅断言"错误被正确上报"）

| 编号 | 行为 | 断言 |
|---|---|---|
| PL-14 | 非法 UTF-8 的 M3U8 文件 | 返回 Err，不 panic |

## 视图层（macOS）

| 编号 | 行为 | 断言 |
|---|---|---|
| PL-15 | `importM3U8Entries` 入库（#136） | local 路径 → `sourceType=local` + `filePath`；http(s) → `direct_url` + `sourceUrl`；全部写入数据库 |
| PL-16 | `importM3U8Entries` 无效条目 | 缺失/空 location → 计入 failed，不写库，弹窗汇总 imported/failed |

## 红测登记

（空。全部绿。）

## 错误路径状态

PL-14（非法 UTF-8）已随 Wave 3a 覆盖（`playlist_m3u8_behavior.rs`），无需顺延。
