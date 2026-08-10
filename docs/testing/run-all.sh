#!/usr/bin/env bash
# Rhythm 全量测试一键入口（macOS/Linux）：L0 + L1。
# 每步输出落盘 docs/testing/logs/（Python 脚本自带默认日志；swift test 在此 tee）。
#
# 用法：
#     bash docs/testing/run-all.sh            # 全量
#     bash docs/testing/run-all.sh --l0-only  # 只跑 L0 静态分析
#
# 注意：F1/F2/F4 修复前 L0 三项预期失败（README「当前状态」表），
# 脚本不因预期失败中断，日志文件仍完整写入。

set -uo pipefail

ROOT="$(cd "$(dirname "$0")/../.." && pwd)"
cd "$ROOT"
LOG_DIR="$ROOT/docs/testing/logs"
mkdir -p "$LOG_DIR"

# XCTest 模块只随完整 Xcode 提供（Command Line Tools 的 Swift 没有）：
# 若 xcode-select 指向 CLT 但 Xcode.app 已安装，局部切换到 Xcode 工具链。
if [[ -d /Applications/Xcode.app && "$(xcode-select -p)" == "/Library/Developer/CommandLineTools" ]]; then
  echo "[env] xcode-select 指向 CLT，局部切到 Xcode 工具链（DEVELOPER_DIR）以提供 XCTest"
  export DEVELOPER_DIR=/Applications/Xcode.app
fi

step() { echo; echo "----- $* -----"; }

echo "===== Rhythm 全量测试 $(date '+%F %T') ====="

step "L0-0 sync-palette --check（palette.json 与源码一致性）"
python3 docs/testing/sync-palette.py --check \
    || echo "! palette.json 与源码漂移：先运行 python3 docs/testing/sync-palette.py 刷新后提交"

step "L0 静态分析（5 项，各自写入默认日志）"
for s in docs/testing/l0/check-*.py; do
  echo ">>> $s"
  python3 "$s" || true
done

if [[ "${1:-}" != "--l0-only" ]]; then
  step "拷贝 L1 测试到 SwiftPM 目录（与 CI 一致，保证种子最新）"
  mkdir -p macos/Tests/RhythmThemeTests
  cp docs/testing/l1/macos/*.swift macos/Tests/RhythmThemeTests/

  step "L1 macOS swift test（tee → $LOG_DIR/l1-macos-swift-test.log）"
  (cd macos && swift test 2>&1 | tee "$LOG_DIR/l1-macos-swift-test.log")
  echo "swift test 退出码: ${PIPESTATUS[0]:-?}（tee 不影响真实退出码）"

  step "L1 macOS 内存卫生（ASan，tee → $LOG_DIR/l1-macos-asan.log）"
  (cd macos && swift test --sanitize=address 2>&1 | tee "$LOG_DIR/l1-macos-asan.log")
fi

echo
echo "===== 全部日志见 $LOG_DIR/ ====="
ls -1 "$LOG_DIR"
