#!/usr/bin/env python3
"""版本号提升任务（#220 收尾）。

版本号的唯一出处是 Cargo.toml 的 [workspace.package] version。三处文档
（两份 README 的版本行、测试基础设施说明的状态表）保留人工副本——正文里带版本号的
叙述句不适合生成——但「人工维护」不该等于「发布时挨个文件手改」：这里把出处与三处
副本一起推到新值，剩下的两处构建配置本来就是构建期派生（#254/#255）。

副本位置不在这里重复声明，直接取 testing/l0/check-version-drift.py 的清单：
校验读哪几处，这里就写哪几处，两边不可能各漂一次。

用法：
    python3 scripts/tasks.py bump-version            # 末位加一
    python3 scripts/tasks.py bump-version 0.6.0      # 指定版本
"""

from __future__ import annotations

import importlib.util
import re
import sys
from pathlib import Path

_HERE = Path(__file__).resolve().parent
if str(_HERE) not in sys.path:
    sys.path.insert(0, str(_HERE))

import tasklib  # noqa: E402

VERSION_RE = re.compile(r"^\d+\.\d+\.\d+$")

# 依赖锁文件由包管理器同步，不手写（#220 的实现决定）。
LOCK_FILE = "Cargo.lock"
LOCK_SYNC_CMD = ["cargo", "update", "-p", "rhythm-core", "--offline"]

DRIFT_CHECK = Path("testing") / "l0" / "check-version-drift.py"


def load_drift_check(root: Path):
    """加载 L0 版本漂移校验模块，副本清单从它取。"""
    path = root / DRIFT_CHECK
    spec = importlib.util.spec_from_file_location("check_version_drift", path)
    module = importlib.util.module_from_spec(spec)
    assert spec.loader is not None
    sys.path.insert(0, str(path.parent.parent))
    spec.loader.exec_module(module)
    return module


def next_patch(version: str) -> str:
    """末位加一。"""
    major, minor, patch = version.split(".")
    return f"{major}.{minor}.{int(patch) + 1}"


def apply_version(root: Path, new_version: str, *, check=None) -> list[str]:
    """把出处与人工副本推到 new_version，返回改动过的相对路径。

    副本按校验用的同一组正则定位，只替换匹配到的那个版本值——正文里其他数字
    （构建号、最低系统版本、示例里的版本）不受影响。

    check 是已加载的校验模块，默认从 root 取；测试拿真实仓库的清单去改夹具树。
    """
    check = check or load_drift_check(root)
    changed: list[str] = []

    for rel, pattern in [(check.SOURCE_FILE, check.SOURCE_RE),
                         *((rel, pat) for rel, _, pat in check.COPIES
                           if rel != LOCK_FILE)]:
        path = root / rel
        if not path.exists():
            raise tasklib.StepFailed([rel], 1, f"版本副本缺失: {rel}")
        text = path.read_text(encoding="utf-8")
        m = pattern.search(text)
        if not m:
            raise tasklib.StepFailed([rel], 1, f"未在 {rel} 里找到版本字段")
        if m.group(1) == new_version:
            continue
        path.write_text(text[:m.start(1)] + new_version + text[m.end(1):],
                        encoding="utf-8")
        changed.append(rel)
    return changed


def bump_version(argv: list[str] | None = None) -> int:
    """提升版本号。不带参数是末位加一，带参数是指定版本。"""
    argv = list(argv or [])
    if len(argv) > 1 or (argv and argv[0].startswith("-")):
        print(f"未知参数: {' '.join(argv)}（bump-version 只接受一个版本号）",
              file=sys.stderr)
        return tasklib.USAGE_ERROR
    root = tasklib.repo_root()
    try:
        current = tasklib.workspace_version(root)
        new_version = argv[0] if argv else next_patch(current)
        if not VERSION_RE.match(new_version):
            print(f"版本号格式非法: {new_version}（形如 0.5.140）", file=sys.stderr)
            return tasklib.USAGE_ERROR
        changed = apply_version(root, new_version)
        print(f"==> 版本号 {current} -> {new_version}")
        for rel in changed:
            print(f"    {rel}")
        print(f"==> 同步依赖锁文件 {LOCK_FILE}")
        tasklib.run_checked(LOCK_SYNC_CMD, cwd=root, echo=False)
    except tasklib.StepFailed as exc:
        print(f"版本提升失败：{exc}", file=sys.stderr)
        return 1
    print("==> 校验")
    return tasklib.run([sys.executable, str(root / DRIFT_CHECK)], cwd=root)
