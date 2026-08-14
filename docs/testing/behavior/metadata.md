# Metadata 行为清单

- 模块：`rust-core/src/metadata/mod.rs` + `metadata/scanner.rs`（标签/属性提取、封面提取、目录扫描）
- 历史回归：无直接记录（本模块零测试是当前缺口本身）
- 夹具策略：测试内生成——用 lofty 写入 ID3v2/MP4 标签生成临时夹具文件（避免二进制入仓）；损坏文件用截断字节构造。夹具生成代码已落地：`rust-core/tests/common/mod.rs` 的 `write_wav`/`write_tagged_wav`（lofty 写 ID3v2 标签）。
- 测试途径：`cargo test` 单元/集成测试，tempfile；无需接缝。

## 主路径（P0 — 合并门槛）

| 编号 | 行为 | 断言 |
|---|---|---|
| MD-01 | `extract_track_info` 完整标签 | title/artist/album/track/disc/genre/year/duration/format/bitrate/sample_rate/channels 全部提取正确（format 字段现状返回标签类型 "id3v2" 而非音频格式 → #96，`md01_extract_track_info_full_tags` 锁定现状值） |
| MD-02 | 标题回退 | 无 title 标签 → 用文件名 stem |
| MD-03 | duration/format 回退 | duration 缺省 0.0；format 缺省用扩展名小写（现状：两兜底分支为防御性代码，lofty 恒有 duration/format、symphonia 恒有 format，公共 API 不可达；`md03_duration_and_format_fallbacks_not_observable` 锁定可观察行为） |
| MD-04 | lofty→symphonia 回退 | lofty 拿不到 title/duration 时 symphonia 兜底（duration 可提取） |
| MD-05 | 文件不存在 | `FileNotFound` |
| MD-06 | `is_supported_audio` | 大小写不敏感；`SUPPORTED_EXTENSIONS` 全接受；未知/无扩展 → false |
| MD-07 | `is_mp4_container` | mp4/m4a/m4b/m4v → true；其余 → false |
| MD-08 | `extract_artwork` 内嵌图 | 写入 cache_dir，文件名 = blake3(数据).扩展名；已存在不重写（幂等）；无图 → None |
| MD-09 | `extract_artwork` 超大图 | >1MB → 跳过返回 None |
| MD-10 | `extract_artwork` 类型判定 | jpeg/jpg → .jpg、png → .png、其他 → .jpg（png → .png 现状失败 → #94，红测 `md10_extract_artwork_mime_png_maps_to_png` 禁用中；jpeg/其他 → .jpg 已绿） |
| MD-11 | `scan_directory` | 递归收集全部支持格式文件，返回 `TrackInfo` 列表 |
| MD-12 | `scan_directory` 跳过隐藏目录 | `.` 开头的目录不进入 |
| MD-13 | `scan_directory` 非目录 | `InvalidInput` |
| MD-14 | `scan_directory` 单文件失败 | 坏文件跳过（log warn），不影响其余文件 |

## 边界情况（P1）

| 编号 | 行为 | 断言 |
|---|---|---|
| MD-15 | 空目录扫描 | 返回空列表 |
| MD-16 | 无标签的音频文件 | 不报错，字段回退（标题=文件名等） |
| MD-17 | 大小写混合扩展名 | `.MP3` 识别为支持格式 |

## 错误路径（P2 — 仅断言"错误被正确上报"）

| 编号 | 行为 | 断言 |
|---|---|---|
| MD-18 | 损坏的音频文件 | `extract_track_info` 返回 Err（Metadata/Decode），不 panic |
| MD-19 | `extract_artwork` 无法读取的文件 | 返回 Err，不 panic |

## 红测登记

| 编号 | 缺陷 | 测试 | 状态 |
|---|---|---|---|
| MD-10 | MIME 判定大小写敏感（`MimeType` Debug 为大写 `Png`），PNG 内嵌图存成 `.jpg` | `md10_extract_artwork_mime_png_maps_to_png`（`#[ignore]`） | #94 待修 |
| MD-01（format 项） | format 字段返回标签类型（"id3v2"）而非音频格式，lofty/symphonia 两路径语义不一致 | `md01_extract_track_info_full_tags`（format 断言锁定现状，附注） | #96 待修 |

## 错误路径状态

MD-18（损坏文件）、MD-19（artwork 无法读取）已随 Wave 3a 覆盖
（`metadata_behavior.rs`），无需顺延。
