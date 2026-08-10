#!/usr/bin/env python3
"""L3 Windows: 主题切换冒烟 + 正式用例脚本（零依赖，直接调 WinAppDriver REST）。

WinAppDriver 是 REST 服务（默认 http://127.0.0.1:4723），stdlib urllib 即可驱动。

用法：
    python3 docs/testing/l3/windows/theme_switch.py --smoke
    python3 docs/testing/l3/windows/theme_switch.py --dark --app path/to/Rhythm.exe
    python3 docs/testing/l3/windows/theme_switch.py --light --app ...
    （所有模式支持 --log PATH，默认 docs/testing/logs/theme_switch.log）

输出截图到 out/，断言窗口中心像素（dark surface ≈ #011F26 / light ≈ #FFFFFF）。
若 WinAppDriver 对 WinUI 3 attach 失败，按 winappdriver.md §4 降级。

先决条件：WinAppDriver.exe 已在 127.0.0.1:4723 运行；管理员权限（改注册表）。
"""

from __future__ import annotations

import argparse
import base64
import io
import json
import subprocess
import sys
import time
import urllib.request
from pathlib import Path

# 复用 L2 的零依赖 PNG 解码器做像素断言；palette_lib 提供统一测试日志
sys.path.insert(0, str(Path(__file__).resolve().parent.parent.parent / "l2" / "windows"))
sys.path.insert(0, str(Path(__file__).resolve().parent.parent.parent))
from compare_screenshots import PNG  # noqa: E402
from palette_lib import default_log_path, open_log  # noqa: E402

BASE = "http://127.0.0.1:4723"

# WinUI 3 主题注册表键（AppsUseLightTheme=0 dark / 1 light）
THEME_KEY = (r"HKCU\Software\Microsoft\Windows\CurrentVersion\Themes\Personalize",
             "/v", "AppsUseLightTheme", "/t", "REG_DWORD")


def http(method: str, path: str, payload: dict | None = None) -> dict | None:
    url = BASE + path
    data = json.dumps(payload).encode() if payload else None
    req = urllib.request.Request(url, data=data, method=method,
                                 headers={"Content-Type": "application/json"})
    try:
        with urllib.request.urlopen(req, timeout=30) as resp:
            return json.loads(resp.read().decode())
    except Exception as e:  # noqa: BLE001 — 冒烟阶段错误即报告
        print(f"HTTP {method} {path} 失败: {e}")
        return None


class Session:
    def __init__(self, app_path: str | None = None):
        caps = {"capabilities": {"app": app_path or ""}}  # 空 app = attach 前台
        r = http("POST", "/session", caps)
        if not r or "sessionId" not in (r.get("value") or {}):
            # WinAppDriver 1.2 返回值形态：value 即 session 元组
            r2 = http("POST", "/session", {"desiredCapabilities": {"app": app_path or ""}})
            if not r2:
                raise RuntimeError("无法创建 WinAppDriver 会话（WinAppDriver 未启动？）")
            self.id = r2.get("sessionId")
        else:
            self.id = r["value"]["sessionId"]

    def req(self, method: str, path: str, payload=None):
        return http(method, f"/session/{self.id}{path}", payload)

    def screenshot(self, out: Path) -> None:
        r = self.req("GET", "/screenshot")
        if not r:
            raise RuntimeError("截图接口失败")
        png = base64.b64decode(r["value"])
        out.write_bytes(png)

    def elements(self, strategy: str = "xpath", value: str = "//*") -> list:
        r = self.req("POST", "/elements",
                     {"using": strategy, "value": value})
        return (r or {}).get("value", [])

    def close(self):
        self.req("DELETE", "")


def set_system_theme(dark: bool) -> None:
    value = "0" if dark else "1"
    subprocess.run(["reg", "add", *THEME_KEY, "/d", value, "/f"],
                   check=True, capture_output=True)
    # 系统广播异步生效：等待窗口重绘（正式用例改为轮询截图稳定）
    time.sleep(2.0)


def center_pixel(png_path: Path) -> tuple[int, int, int]:
    img = PNG(png_path.read_bytes())
    x, y = img.width // 2, img.height // 2
    return img.pixel(x, y)[:3]


def main() -> int:
    ap = argparse.ArgumentParser()
    ap.add_argument("--smoke", action="store_true", help="冒烟：attach + 元素树 + 截图")
    ap.add_argument("--dark", action="store_true")
    ap.add_argument("--light", action="store_true")
    ap.add_argument("--app", type=str, default=None, help="Rhythm.exe 路径")
    ap.add_argument("--out", type=Path, default=Path("out"))
    ap.add_argument("--log", type=Path, default=None,
                    help="日志文件（默认 docs/testing/logs/theme_switch.log，覆盖写入）")
    args = ap.parse_args()

    open_log(args.log or default_log_path("theme_switch"))

    out = args.out
    out.mkdir(parents=True, exist_ok=True)

    if args.smoke:
        try:
            s = Session(args.app)
        except RuntimeError as e:
            print(f"FAIL — {e}；降级路径见 winappdriver.md §4")
            return 1
        els = s.elements()
        print(f"元素树: {len(els)} 个控件")
        shot = out / "smoke.png"
        s.screenshot(shot)
        img = PNG(shot.read_bytes())
        print(f"截图: {img.width}x{img.height} → {shot}")
        s.close()
        ok = len(els) > 0 and img.width > 0
        print("SMOKE PASS" if ok else "SMOKE FAIL（进入降级）")
        return 0 if ok else 1

    if not (args.dark or args.light) or not args.app:
        print("正式用例需 --dark/--light + --app；或 --smoke 冒烟")
        return 2

    set_system_theme(dark=args.dark)
    s = Session(args.app)
    time.sleep(3.0)  # 应用启动 + 主题重载
    shot = out / ("dark.png" if args.dark else "light.png")
    s.screenshot(shot)
    s.close()

    r, g, b = center_pixel(shot)
    if args.dark:
        ok = r < 40 and g < 60 and b < 70          # ≈ #011F26 (1,31,38)
    else:
        ok = r > 230 and g > 230 and b > 230       # ≈ #FFFFFF
    status = "PASS" if ok else "FAIL"
    print(f"[{status}] 中心像素 ({r},{g},{b}) → {shot}")
    return 0 if ok else 1


if __name__ == "__main__":
    sys.exit(main())
