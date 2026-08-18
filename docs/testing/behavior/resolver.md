# Resolver 行为清单

- 模块：`rust-core/src/resolver/mod.rs`（URL 分类与解析、yt-dlp 子进程、缓存、失败分类、日志）+ `resolver/install.rs`（yt-dlp 自动安装）
- 历史回归：`#19`、`#22`（自动安装）、`#26`（解析成功却播不出）、`#23`（m4s 误交 yt-dlp）
- 测试途径（R3-Q2=A 已定）：
  1. 纯函数直测（已有 28 单测的扩展）：`classify_url`、`parse_hh_mm_ss`、`extract_stream`、`classify_ytdlp_stderr`、`summarize_stderr`、`prune_cache`、`format_utc` 等。
  2. **stub 可执行脚本**（假 yt-dlp）：fixtures 目录放一个脚本，解析参数、按 URL 吐出预置 JSON 或按场景报错/超时；测试设 `RHYTHM_YTDLP_PATH` 指向它，测 `resolve_url` 的"进程调用→输出解析→缓存→失败落地"全链路，不碰网络。
  3. 缓存为全局 `LazyLock`——测试间用不同 URL 前缀隔离（`unique()` 生成每测试唯一 URL）；`YTDLP_PATH` 路径缓存同样全局，路径失效类场景（RS-14/RS-21）放在独立测试二进制 `resolver_path_failure.rs`（独立进程、缓存从空开始）。
  4. `run_with_timeout` 用短超时直测（已有先例）。

## 主路径（P0 — 合并门槛）

| 编号 | 行为 | 断言 |
|---|---|---|
| RS-01 | `resolve_url` 空串/非 http | `InvalidUrl`，不调子进程 |
| RS-02 | `classify_url` YouTube 变体 | watch/youtu.be/shorts/embed/music./m. 各变体（含带参数）→ YouTube（"无协议"变体锁定现状：classify 要求 http(s) 前缀，无协议 → `InvalidUrl`，见 `rs02_classify_youtube_variants`） |
| RS-03 | `classify_url` Bilibili 变体 | BV 号视频页、b23.tv 短链 → Bilibili |
| RS-04 | `classify_url` 直链（#23） | 音频扩展 + 可选 query → DirectUrl；m4s DASH 段 → DirectUrl（不误交 yt-dlp） |
| RS-05 | `classify_url` 未识别 http 链接 | 回退 YouTube（yt-dlp 兜底） |
| RS-06 | 直链 resolve | 不走 yt-dlp；标题 = 文件名（去 query）；stream_url 原样；duration 0.0 |
| RS-07 | yt-dlp 输出字段提取（stub） | title fallback 链（title→fulltitle→alt_title→Unknown）；artist fallback 链（uploader→channel→artist→creator→uploader_id）；duration 数字/数字串/duration_string 三形态 |
| RS-08 | stream URL 提取优先级（stub） | `url` 字段 → `requested_formats` 首个 → `formats`（音频优先、兜底任意）→ `manifest_url`；全无 → `NoAudioStream` |
| RS-09 | headers 提取（stub） | 顶层 `http_headers` 与 format 级 fallback（Bilibili Referer 场景） |
| RS-10 | 缓存命中 | 同 URL 二次 resolve 不重复调子进程；TTL 内返回克隆 |
| RS-11 | 缓存容量 | 超 256 条目驱逐最旧（`prune_cache` 直测）；过期条目被剔除 |
| RS-12 | 失败 stderr 分类 | outdated/unavailable/network 各关键词 → 对应 kind；未知 → Internal（已有单测 RS-38–41 覆盖；e2e 经 stub 触发 outdated/unavailable/network/未知 全链） |
| RS-13 | 失败写日志 | resolver.log 追加含 kind/url/message；超 512KB 轮转；日志 IO 失败不影响解析结果 |
| RS-14 | yt-dlp spawn 失败 | 遗忘缓存路径（下次重发现）+ `YtDlpMissing` |
| RS-15 | 超时 | 子进程超时被杀 + `Timeout`（已有单测 RS-35 覆盖短超时；e2e 不重复——`YTDLP_TIMEOUT` 60s 不可注入，集成等待不可接受） |
| RS-16 | outdated 且受管副本 | 自动 update + retry 一次；update 失败保留原错误（**顺延**：触发需受管副本 + 网络下载，stub 无法覆盖 update_now；非受管路径已由 `rs20_non_managed_outdated_reports_with_upgrade_advice` 锁定） |
| RS-17 | `resolved_to_track` | `source_url` 存页面 URL（非 CDN）；artwork_path = thumbnail_url |
| RS-18 | yt-dlp 空输出 | 非空 stderr → 按分类失败；全空 → `Unavailable` |
| RS-19 | 非法 JSON 输出 | `Internal`，不 panic |

## 边界情况（P1）

| 编号 | 行为 | 断言 |
|---|---|---|
| RS-20 | 非受管副本 outdated | 不自动 update，直接报错带升级建议 |
| RS-21 | `RHYTHM_NO_AUTO_INSTALL` | 无 yt-dlp 时 → `YtDlpMissing`（不触发安装） |
| RS-22 | `RHYTHM_YTDLP_PATH` 覆盖 | 优先于一切发现路径（已有单测 RS-33/34 覆盖；e2e 全部场景经该覆盖注入 stub，即为持续验证） |
| RS-23 | 直链 URL 含百分号编码 | 期望：标题按 URL 百分号编码规则正确解码（e2e `rs23_direct_url_title_decodes_percent_encoding`；纯函数单测 `test_urlencoding_if_needed` 覆盖中文/空格/畸形转义/非法 UTF-8 回退） |
| RS-24 | 缓存 TTL 边界 | `prune_cache` 直测：恰好 1 小时的条目被剔除（`< TTL` 严格边界），1 秒内保留（`test_prune_cache_evicts_oldest_and_expired`，rust-core/src/resolver/mod.rs tests） |
| RS-44 | 播放侧 403 缓存失效（#120） | `evict_resolution` 删除页面 URL 条目 → 下次 resolve 走真实解析（`test_evict_resolution_drops_poisoned_entry`） |
| RS-45 | 绕过缓存重解析（#120） | `resolve_url_fresh` 不读缓存、成功后仍写缓存（`test_resolve_url_fresh_bypasses_cache`） |
| RS-46 | 播放侧 403 诊断日志（#120） | `log_playback_http` 向 resolver.log 追加分类/status/expire/mt/ip，best-effort |

## 错误路径（P2 — 仅断言"错误被正确上报"）

| 编号 | 行为 | 断言 |
|---|---|---|
| RS-25 | stderr 极长输出 | `summarize_stderr` 截断至 4 行/600 字符，含省略号（已有单测 RS-43/44 覆盖，无需顺延） |

## 红测登记

| 编号 | 缺陷 | issue | 状态 |
|---|---|---|---|
| RS-23 | 直链标题不解码百分号编码（`urlencoding_if_needed` 为 no-op） | [#80](https://github.com/Teeeeeeerry/Rhythm/issues/80) | 已修复（红测解禁转绿） |

## 已有测试行为对照（完整性要求：每条已有测试的行为均列入清单）

| 编号 | 行为（已有测试） | 出处 |
|---|---|---|
| RS-26 | `classify_url` YouTube 五变体：watch/youtu.be/shorts/music./embed | `test_classify_youtube` |
| RS-27 | `classify_url` Bilibili 三变体：www./m./b23.tv | `test_classify_bilibili` |
| RS-28 | `classify_url` 直链：mp3 / flac+query / opus | `test_classify_direct_audio` |
| RS-29 | `classify_url` 拒绝非 http：`not-a-url`、`ftp://` | `test_classify_rejects_non_http` |
| RS-30 | `resolve_url` 空白串 / `file://` → `InvalidUrl` | `test_resolve_rejects_empty_and_non_http` |
| RS-31 | `parse_hh_mm_ss`：45/3:45/1:02:30/0:05 → 秒数；空串/abc → None | `test_parse_hh_mm_ss` |
| RS-32 | 候选路径含 GUI 不可达前缀：/opt/homebrew、/usr/local（mac）；WindowsApps/chocolatey（win） | `test_candidate_paths_include_gui_unreachable_prefixes` |
| RS-33 | `RHYTHM_YTDLP_PATH` 覆盖为探针第一项 | `test_env_override_is_probed_first` |
| RS-34 | env override 纯空白值 → 视为未设 | `test_env_override_ignores_blank_value` |
| RS-35 | `run_with_timeout` 超时杀进程且及时返回 | `test_run_with_timeout_kills_slow_process` |
| RS-36 | `run_with_timeout` 大 stdout（200KB）不死锁不截断 | `test_run_with_timeout_captures_large_stdout` |
| RS-37 | `run_with_timeout` 二进制不存在 → `RunError::Spawn` | `test_run_with_timeout_reports_spawn_failure` |
| RS-38 | stderr 分类：outdated 关键词（nsig extraction / unable to extract player） | `test_classify_stderr_outdated` |
| RS-39 | stderr 分类：unavailable 关键词（private video / bot 验证 / 地区限制） | `test_classify_stderr_unavailable` |
| RS-40 | stderr 分类：network 关键词（download webpage / certificate） | `test_classify_stderr_network` |
| RS-41 | stderr 未知 → `Internal` | `test_classify_stderr_unknown_is_internal` |
| RS-42 | `ytdlp_missing_error`：kind=YtDlpMissing、消息含 env override 与 brew 建议 | `test_missing_binary_message_mentions_env_override` |
| RS-43 | `summarize_stderr` 保留尾部 4 行 | `test_summarize_stderr_keeps_tail` |
| RS-44 | `summarize_stderr` 超长行截断 ≤601 字符并加省略号 | `test_summarize_stderr_truncates_long_lines` |
| RS-45 | `extract_stream` 顶层 headers 随 url 返回（Bilibili Referer 形状） | `test_extract_stream_takes_top_level_headers` |
| RS-46 | `extract_stream` format 级 headers 优先于顶层 | `test_extract_stream_prefers_format_headers` |
| RS-47 | `extract_stream` format 无 headers 时回退顶层 | `test_extract_stream_falls_back_to_top_level_headers` |
| RS-48 | `extract_stream` 无 headers → 空 map 而非错误 | `test_extract_stream_without_headers_is_empty_not_error` |
| RS-49 | `extract_stream` 忽略非字符串 header 值 | `test_extract_stream_ignores_non_string_header_values` |
| RS-50 | m4s DASH 段 → DirectUrl（不交 yt-dlp，#23） | `test_dash_segment_is_a_direct_url_not_a_ytdlp_target` |
| RS-51 | `ResolveFailure → RhythmError` 映射：InvalidUrl→InvalidInput、Timeout/Network→Network、其余→Resolution | `test_failure_maps_to_rhythm_error` |
| RS-52 | `ResolveFailure` 序列化 kind 为 snake_case | `test_failure_serializes_snake_case_kind` |
| RS-53 | `format_utc`：epoch/常规日期/闰日 | `test_format_utc` |
| RS-54 | `parse_checksum` 按资产名匹配 / 缺席 → None | `test_parse_checksum_picks_matching_asset` |
| RS-55 | `parse_checksum` 二进制模式前缀 `*` | `test_parse_checksum_handles_binary_mode_prefix` |
| RS-56 | `parse_checksum` 畸形 hash 拒绝 | `test_parse_checksum_rejects_malformed_hash` |
| RS-57 | `asset_name()` 平台匹配（macos→yt-dlp_macos、windows→yt-dlp.exe） | `test_asset_matches_platform` |
| RS-58 | 受管路径位于 Application Support/Rhythm/bin | `test_managed_path_is_under_app_data` |
| RS-59 | `RHYTHM_NO_AUTO_INSTALL`：设 1 禁用、设 0/未设启用 | `test_auto_install_opt_out` |
| RS-60 | `InstallStatus` 序列化带 phase tag；Idle 精确为 `{"phase":"idle"}` | `test_status_serializes_with_phase_tag` |
| RS-61 | `hex` 编码 | `test_hex_encoding` |
