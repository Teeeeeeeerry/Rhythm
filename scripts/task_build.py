#!/usr/bin/env python3
"""构建任务的实现（#221 编排层收敛）。

产物落点在这里是单一约定，两个平台同构：

    核心（Rust workspace）  <仓库根>/target/[<目标三元组>/]release/
    应用（可分发产物）      <仓库根>/build/

迁移前四个脚本各有一套约定，其中两个还从成员目录的相对路径取产物（那是工作区
布局，产物在仓库根，必然落空，#222/#223）。取用点只剩 core_artifact_dir 一处。
"""

from __future__ import annotations

import shutil
import sys
from pathlib import Path

_HERE = Path(__file__).resolve().parent
if str(_HERE) not in sys.path:
    sys.path.insert(0, str(_HERE))

import tasklib  # noqa: E402

CORE_PACKAGE = "rhythm-core"
BUILD_DIR = "build"


def core_artifact_dir(root: Path, *, target: str | None = None,
                      profile: str = "release") -> Path:
    """核心构建产物目录（工作区布局：在仓库根的 target/ 下）。"""
    base = root / "target"
    return (base / target / profile) if target else (base / profile)


def build_core(root: Path, *, target: str | None = None) -> None:
    """构建 Rust 核心（release）。失败即抛 StepFailed。

    同一平台内「只构建核心」与「构建完整应用」必须用同一组参数，
    因此这里是两者唯一的构建调用处。
    """
    cmd = ["cargo", "build", "--release", "-p", CORE_PACKAGE]
    if target:
        cmd += ["--target", target]
    tasklib.run_checked(cmd, cwd=root)


# ---------------------------------------------------------------------------
# macOS 应用包（#261）
# ---------------------------------------------------------------------------

MACOS_EXECUTABLE = "Rhythm"
MACOS_BUNDLE = "Rhythm.app"
CORE_DYLIB = "librhythm_core.dylib"
BUNDLED_DYLIB_REF = f"@executable_path/../Frameworks/{CORE_DYLIB}"


def _capture(cmd: list[str], cwd: Path | None = None) -> str:
    """取命令的标准输出（供 otool 之类的查询用）。"""
    import subprocess

    return subprocess.run(
        [str(c) for c in cmd], cwd=str(cwd) if cwd else None,
        check=True, capture_output=True, text=True,
    ).stdout


def assemble_macos_bundle(root: Path) -> Path:
    """组装 build/Rhythm.app，返回应用包路径。

    动态库引用改写与临时签名都保留：Swift 可执行文件按构建树里的绝对路径链接
    dylib，不改写的话包里的那份从未被用到，target/ 一清应用就打不开；
    install_name_tool 会让既有签名失效，所以临时签名必须排在它之后。
    """
    bundle = root / BUILD_DIR / MACOS_BUNDLE
    contents = bundle / "Contents"
    for sub in ("MacOS", "Resources", "Frameworks"):
        (contents / sub).mkdir(parents=True, exist_ok=True)

    executable = contents / "MacOS" / MACOS_EXECUTABLE
    shutil.copy2(root / "macos" / ".build" / "release" / MACOS_EXECUTABLE, executable)

    plist = contents / "Info.plist"
    template = (root / "macos" / "Rhythm" / "Resources" / "Info.plist").read_text(
        encoding="utf-8")
    # Xcode 构建变量占位符在 SwiftPM 下不会被展开，直接写成真实可执行文件名
    plist.write_text(template.replace("$(EXECUTABLE_NAME)", MACOS_EXECUTABLE),
                     encoding="utf-8")

    bundled_dylib = contents / "Frameworks" / CORE_DYLIB
    shutil.copy2(core_artifact_dir(root) / CORE_DYLIB, bundled_dylib)

    current_ref = next(
        (line.split()[0] for line in _capture(["otool", "-L", str(executable)]).splitlines()
         if CORE_DYLIB in line),
        None,
    )
    if current_ref:
        tasklib.run_checked(
            ["install_name_tool", "-change", current_ref, BUNDLED_DYLIB_REF,
             str(executable)], echo=False)
    tasklib.run_checked(
        ["install_name_tool", "-id", BUNDLED_DYLIB_REF, str(bundled_dylib)], echo=False)

    # 临时签名，让应用包在任意 Mac 上都能启动（必须在 install_name_tool 之后）
    tasklib.run(["codesign", "--force", "--deep", "--sign", "-", str(bundle)],
                echo=False)
    return bundle


def assert_no_build_tree_reference(root: Path, bundle: Path) -> None:
    """应用包不得再引用构建树路径——否则它只在这台机器上能跑。"""
    linked = _capture(["otool", "-L", str(bundle / "Contents" / "MacOS" / MACOS_EXECUTABLE)])
    if str(root / "target") in linked:
        raise tasklib.StepFailed(["otool", "-L"], 1, "应用包仍引用构建树路径")


def build_macos(argv: list[str] | None = None) -> int:
    """构建 macOS 应用包。任一步失败即非零退出。"""
    if argv:
        print(f"未知参数: {' '.join(argv)}（build-macos 不接受参数）", file=sys.stderr)
        return 2
    root = tasklib.repo_root()
    try:
        print("==> 构建 Rust 核心")
        build_core(root)
        print("==> 构建 macOS 应用")
        tasklib.run_checked(["swift", "build", "-c", "release"], cwd=root / "macos")
        print("==> 组装应用包")
        bundle = assemble_macos_bundle(root)
        assert_no_build_tree_reference(root, bundle)
    except tasklib.StepFailed as exc:
        print(f"构建失败：{exc}", file=sys.stderr)
        return 1
    print(f"==> 应用包：{bundle}")
    print(f"    运行：open {bundle}")
    return 0
