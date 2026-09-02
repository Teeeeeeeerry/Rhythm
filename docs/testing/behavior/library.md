# Library 行为清单

- 模块：`rust-core/src/library/mod.rs`（SQLite 曲库：CRUD、去重、播放列表、FTS 搜索、目录/文件导入）
- 历史回归：`#40`+`#57`（URL 重复入库）、`#54`（Mutex 死锁）、`#55`（后台导入）、`#56`（URL 持久化）、`#66`+`#67`（艺人/专辑分组）
- 测试途径：`cargo test` 集成测试，tempfile 临时库 + 代码生成 WAV 夹具；无需接缝（真实 SQLite 即可测）。

## 主路径（P0 — 合并门槛）

| 编号 | 行为 | 断言 |
|---|---|---|
| LB-01 | `open` 建库 | 父目录自动创建；重复 open 幂等（schema 已存在不报错） |
| LB-02 | `add_track` 新曲目 | 插入并返回带数据库 id 的 `TrackInfo` |
| LB-03 | `add_track` 本地去重 | 同 `file_path` → 更新原行而非插入，返回原 id |
| LB-04 | `add_track` URL 去重（#40/#57） | 同 `source_url`（非 local）→ 更新原行；DB 层部分唯一索引兜底（绕过应用层去重时插入被拒） |
| LB-05 | `get_all_tracks` | 按 title 大小写不敏感排序 |
| LB-06 | ~~`get_tracks_by_artist_album`（#66/#67）~~ | 已删除（#147）：无 FFI/UI 消费方，客户端各自分组，Rust 分组为重复逻辑 |
| LB-07 | `remove_track` | 行删除；FTS 索引同步删除；`playlist_tracks` 级联清理；不存在的 id → `NotFound` 错误（#98） |
| LB-08 | `record_play` | `last_played` 更新、`play_count` +1 |
| LB-09 | `verify_local_files` | 磁盘不存在的 local 曲目标记 `is_available=0` 并返回其 id 列表；存在的保持不变 |
| LB-10 | 播放列表 CRUD | create 返回 id；delete 生效（rename 已删除 #147：无消费方） |
| LB-11 | `add_to_playlist` | 追加到末尾 position；同曲重复添加被忽略（不产生重复行） |
| LB-12 | `remove_from_playlist`/`reorder_playlist_track` | 删除/改序生效，`get_playlist` 按 position 返回（reorder 到已占用 position 时其余行移位、position 无重复 → #95） |
| LB-13 | `get_all_playlists`/`get_playlist` | 元数据完整，tracks 按 position 排序 |
| LB-14 | `search`（FTS） | title/artist/album/genre 命中；rank 排序；上限 100；`*`/`"`/`(`/`)` 被清洗不报错 |
| LB-15 | `import_from_directory` | 递归扫描入库；单曲失败（log warn）不中断整体；返回扫描曲目数（魔数返回码路径，#244 删除） |
| LB-16 | `import_file` | 支持格式 → 1；metadata 与 artwork 提取后入库（魔数返回码路径，#244 删除） |

## 边界情况（P1）

| 编号 | 行为 | 断言 |
|---|---|---|
| LB-17 | URL 曲目无 `source_url` | 跳过 URL 去重直接插入 |
| LB-18 | 曲目同时有 file_path 与 source_url | file_path 去重优先 |
| LB-19 | `search` 空/纯空白查询 | 不 panic，返回结果可空（现状：FTS5 `MATCH ''` 语法错误 → 返回 `Err`，不崩溃；已锁定为现状行为） |
| LB-20 | `import_file` 不存在的文件 | 报错（Unsupported 或 FileNotFound 之一），不 panic |
| LB-21 | `mark_unavailable` | 置 `is_available=0` 落库（`mark_available` 已删除 #147：无消费方） |
| LB-22 | `add_to_playlist` 引用不存在的 playlist/track | 外键约束报错（`foreign_keys=ON`），不静默 |
| LB-23 | 同目录重复 `import_from_directory` | file_path 去重（第二次导入不产生重复行） |

## 错误路径（P2 — 仅断言"错误被正确上报"）

| 编号 | 行为 | 断言 |
|---|---|---|
| LB-24 | 库路径不可写/损坏 | `open` 返回 Err（现状以目录路径代测 `lb24_open_directory_path_errors`：权限变体在 CI 不可靠、损坏库由 SQLite 自身报错兜底） |
| LB-25 | `record_play` 不存在的 id | 影响 0 行，不报错 |
| LB-26 | `import_from_directory` 非目录路径 | `InvalidInput` |

## 导入结果分类（P0 — 具名结果与聚合规则，#238）

三条导入路径共用同一结果形状 `ImportOutcome{imported, unsupported, failed}`；「格式不支持」与
「读写失败」分开计数，多路径批量导入的聚合在核心完成（此前只有 macOS 有，Windows 完全没有）。

| 编号 | 行为 | 断言 |
|---|---|---|
| LB-27 | `import_directory` 全部成功 | 每条入库，`imported` = 扫描数，另两项为 0 |
| LB-28 | `import_directory` 空目录 | 三项全 0，库中无新增 |
| LB-29 | `import_directory` 路径不存在/非目录 | 扫描跑不起来 → `failed` = 1 |
| LB-30 | `import_single_file` 三种结局 | 支持格式 → `imported`；扩展名不支持 → `unsupported`；支持但读不出 → `failed` |
| LB-31 | `import_paths` 部分成功 | 目录与文件混合，成功与失败分项累加，聚合在核心 |
| LB-32 | `import_paths` 全部失败 | `failed` = 路径数，库中无新增 |
| LB-33 | `import_paths` 成功与不支持混合 | `unsupported` 与 `imported` 各自计数，不被合并 |

## 与 library_integration.rs 的关系

遗留套件 `rust-core/tests/library_integration.rs`（#66/#67 修复时引入）与本清单
测试在 URL 去重上有重叠覆盖。遗留套件保持零改动；艺人/专辑分组的 Rust 断言
已随 #147 删除（分组为客户端职责）。新增行为以本清单测试
（`library_behavior.rs`）为准，未来可择机将遗留套件并入。

## 红测登记

（空 — #95 已修复，`lb12_reorder_to_occupied_position_keeps_order` 已转真断言）

## 错误路径状态

LB-24（open 不可写路径）、LB-25（record_play 不存在 id）、LB-26（import 非目录）
已随 Wave 3a 覆盖（`library_behavior.rs`），无需顺延。
