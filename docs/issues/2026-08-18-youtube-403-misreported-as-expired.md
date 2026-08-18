# Issue: YouTube 播放报"链接可能已过期"，但 URL 完全有效——403 误报 + 缓存不失效导致重贴无效

- **日期**: 2026-08-18
- **来源**: 用户错误报告（播放失败，HTTP 403 Forbidden on googlevideo.com/videoplayback）
- **状态**: 已修复（#120；App 侧诊断与恢复已实装，根因仍在外网侧）
- **涉及平台**: macOS（Windows 同构，`AppState.cpp` 镜像相同逻辑）

---

## 现象

播放 YouTube 曲目失败，弹出：

> 播放失败。链接可能已过期，重新粘贴一次试试。
>
> 详细信息：
> Network error: GET `https://rr2---sn-55goxu-hxas.googlevideo.com/videoplayback?...` failed: HTTP 403 Forbidden

报错 URL 关键参数：`mt=1787020361`、`expire=1787042504`、`ip=138.25.4.51`、`itag=140`、`c=ANDROID_VR`、`clen=4296823`、`dur=265.450`、`rqh=1`、`gir=yes`。

## 证据：URL 没有过期

对报错 URL 的签名参数解码（本机时区 AEST，UTC+10）：

| 参数 | 值 | 含义 |
|---|---|---|
| `mt` | 1787020361 → **2026-08-18 12:32:41 AEST** | URL 签发时间 |
| `expire` | 1787042504 → **2026-08-18 18:41:44 AEST** | URL 硬过期时间（有效期 6 小时 9 分） |
| `ip` | 138.25.4.51 | 签发时绑定的客户端 IP |

失败发生在 ~12:41（资料库 `last_played` 与 DB WAL 时间戳均落在 12:41–12:45），即 **URL 签发后仅约 9 分钟**，距过期还有 6 小时。**"链接已过期"不是事实。**

## 复现与重放验证（本机实测）

1. **原 URL 重放**：在有效期内（12:50，expire 未到）用 5 种请求模式请求原 URL，全部 `HTTP 403`、0 字节响应体（`Server: gvs 1.0`）：
   - 裸 GET（无任何头）、`Range: bytes=0-`、yt-dlp 全套头（Chrome UA/Accept/Accept-Language/Sec-Fetch-Mode）、`User-Agent: Rhythm/0.1`（App 兜底 UA）、无 Range 带 yt-dlp UA。
2. **全新 URL 重放**：用 App 完全相同的 yt-dlp 参数（`-f bestaudio[acodec^=mp4a]/...` + `--no-check-certificates` 等）现场解析同一视频，得到签发仅 60 秒的新 URL（`ip=` 仍为 138.25.4.51），同样 5 种模式**全部 403**。
3. **yt-dlp 自身**：`yt-dlp -f 140 --download-sections "*0:00-0:30"` → ffmpeg 拉流 **同样 403**。
4. **当前公网 IP**：`api.ipify.org` / `api64.ipify.org` 均返回 **138.25.4.51**，与 URL 内嵌 `ip=` 一致 → 排除 IP 不匹配。
5. **出口边缘**：`rr1---sn-55goxu-hxas.googlevideo.com` 解析为 **203.13.161.76 = `cache.google.com`**——**ISP（TPG）网络内托管的 Google Global Cache 节点**。系统解析器、8.8.8.8、1.1.1.1 结果一致，此网络无其他边缘可选。
6. youtube.com API 流量正常（解析成功、能签发 URL），仅媒体 CDN 拒绝。

**结论：403 与该 App 的请求方式无关，也与 URL 是否过期无关——是网络侧（ISP 的 GGC 节点 / YouTube 对该出口 IP 的拒绝）对"仍然有效"的媒体 URL 持续返回 403。** 用户侧缓解：换网络 / VPN、等待 ISP 修复；重贴链接无效（见下）。

## App 侧问题（可修，本次事故暴露）

### P1. 错误一律误报为"链接已过期"，且建议的操作必然无效

- `macos/Rhythm/Models/L10n.swift:34-40`：`playbackFailed(detail:)` 对**所有**播放失败固定显示"链接可能已过期，重新粘贴一次试试"，不看错误内容。
- `macos/Rhythm/AppState.swift:508-514`：引擎 `state == 4`（Error）时无条件套用该文案，`detail` 只是原始网络错误串。
- `rust-core/src/audio/http_stream.rs:191-197`：非 2xx 一律转成 `RhythmError::Network("GET … failed: HTTP 403")`，**不区分 403 与 5xx/DNS/TLS，也不携带 URL 的 expire/mt**——上层无从判断"真过期"还是"CDN 拒绝"。

后果：用户看到的是错误归因；真实原因是网络侧 403 时，重贴链接**不可能**解决（见 P2）。

### P2. 解析缓存不失效：重贴链接在 1 小时内必然拿到同一个坏 URL

- `rust-core/src/resolver/mod.rs:14-21`：`RESOLVED_CACHE` 按**页面 URL** 缓存解析结果（含带签名的 CDN `stream_url`），TTL = 1 小时（`CACHE_TTL`，`mod.rs:21`）。
- `mod.rs:669-678`：命中缓存直接返回，**播放 403 不会淘汰对应条目**。
- 播放路径 `rust-core/src/audio/mod.rs:151-183`（`play_url` → `resolve_url`）与重贴路径 `resolveAndImport`（Swift → `rhythm_resolve_url` → `resolve_url`）走**同一个缓存**。

因此：12:32:41 签发 URL 失败后，直到 13:32:41 之前，**无论重播还是"重新粘贴一次"，拿到的都是同一个 403 的 CDN URL**——错误弹窗给出的唯一建议在 TTL 内结构性无效。本次报告中用户 12:32 与 12:41 两次尝试（`last_played` 12:41:15）拿到的就是同一份签名 URL（`mt` 相同），与缓存命中行为完全吻合。

### P3. 无任何恢复策略

- 播放 403 后不重试、不重解析、不淘汰缓存。哪怕 403 是瞬时抖动，用户也只能等缓存过期或重启 App（内存缓存重启即清）。

## 建议修复

1. **结构化错误**：`RhythmError::Network` 携带 HTTP 状态码；核心在 403 时顺带解析 URL 的 `expire`/`mt`，输出"真过期 / CDN 拒绝 / 其他"分类。Swift 按分类给文案：
   - 真过期（`expire` 已过）→ 现有"链接可能已过期"；
   - 未过期 403 → "YouTube 拒绝了当前网络（可能与 ISP/VPN 有关），换网络或稍后再试"，不再建议重贴。
2. **缓存失效 + 一次性重试**：播放 403 时（a）淘汰该页面 URL 的缓存条目，并（b）**绕过缓存重新解析一次**再试；仍失败才报错。这同时覆盖"瞬时 403"与"缓存串味"两类情况。
3. **记录诊断信息**：resolver 日志追加播放侧 403（含 `expire`/`mt`/`ip`），便于区分网络侧故障与真过期。

## 复现步骤

1. 处于被 YouTube CDN 拒绝的网络出口（本次为 TPG 网络内的 GGC 节点故障期间）。
2. 粘贴任意 YouTube 链接 → 解析成功（API 正常）→ 播放 → 立即 403 弹窗。
3. 重贴同一链接 → 1 小时内必现同一 403（缓存命中）。
4. 手动验证（任意时刻可重复）：对报错 URL 裸 `curl -I`，`HTTP 403`。

## 附：验证命令与现场数据（已脱敏）

```bash
# 解码 URL 时间戳
date -r 1787020361   # mt（签发）→ 2026-08-18 12:32:41 AEST
date -r 1787042504   # expire   → 2026-08-18 18:41:44 AEST
# 出口边缘
dig +short rr1---sn-55goxu-hxas.googlevideo.com   # 203.13.161.76 = cache.google.com（ISP GGC）
# 任意模式重放均 403
curl -sS -o /dev/null -w "%{http_code}" "<报错URL>"   # 403
curl -sS -o /dev/null -w "%{http_code}" -H "Range: bytes=0-" "<报错URL>"   # 403
```

## 关联

- `CONTEXT.md` 坑条目："URL 曲目存页面 URL 不存 CDN 链接"（设计如此，但需配合 P2 的失效机制）。
- 播放失败文案入口：`L10n.playbackFailed`；错误通道：`AppState.updatePlaybackProgress`（`state == 4`）。
