#!/usr/bin/env bash
# Rhythm 全量测试一键入口（macOS/Linux）：L0 + L1。
# 每步输出落盘 testing/logs/（Python 脚本自带默认日志；swift test 在此 tee）。
#
# 用法：
#     bash testing/run-all.sh             # 全量；任一红 → 非零退出（严格模式）
#     bash testing/run-all.sh --l0-only   # 只跑 L0 静态分析
#     bash testing/run-all.sh --allow-expected-failures  # 显式豁免预期失败
# 环境变量 ALLOW_EXPECTED_FAILURES=1 与 --allow-expected-failures 等效。
# 历史注记：F1/F2/F4 修复前 L0 三项预期失败，脚本曾默认容错（导致吞掉真实失败，
# 见 #144）；相关修复早已落地，容错不再默认，仅可显式开启。

set -uo pipefail

ROOT="$(cd "$(dirname "$0")/.." && pwd)"
cd "$ROOT"
LOG_DIR="$ROOT/testing/logs"
mkdir -p "$LOG_DIR"

L0_ONLY=0
ALLOW_EXPECTED_FAILURES="${ALLOW_EXPECTED_FAILURES:-0}"
for arg in "$@"; do
  case "$arg" in
    --l0-only) L0_ONLY=1 ;;
    --allow-expected-failures) ALLOW_EXPECTED_FAILURES=1 ;;
    *) echo "未知参数: ${arg}（支持 --l0-only / --allow-expected-failures）" >&2; exit 2 ;;
  esac
done

FAILED=0  # 失败步数累计；严格模式下任一红 → 最终非零退出

# XCTest 模块只随完整 Xcode 提供（Command Line Tools 的 Swift 没有）：
# 若 xcode-select 指向 CLT 但 Xcode.app 已安装，局部切换到 Xcode 工具链。
if [[ -d /Applications/Xcode.app && "$(xcode-select -p)" == "/Library/Developer/CommandLineTools" ]]; then
  echo "[env] xcode-select 指向 CLT，局部切到 Xcode 工具链（DEVELOPER_DIR）以提供 XCTest"
  export DEVELOPER_DIR=/Applications/Xcode.app
fi

step() { echo; echo "----- $* -----"; }

echo "===== Rhythm 全量测试 $(date '+%F %T') ====="

step "L0-0 sync-palette --check（palette.json 与源码一致性）"
if python3 testing/sync-palette.py --check; then
  echo "ok"
else
  echo "! palette.json 与源码漂移：先运行 python3 testing/sync-palette.py 刷新后提交"
  FAILED=$((FAILED + 1))
fi

step "L0 静态分析（testing/l0/check-*.py，各自写入默认日志）"
# 含版本号漂移校验 check-version-drift.py（#253）：版本号只改 Cargo.toml，
# 其余六处副本漂移在此报红，不必等到发布后才发现。
for s in testing/l0/check-*.py; do
  echo ">>> $s"
  python3 "$s" || FAILED=$((FAILED + 1))
done

step "L0 校验脚本自测（testing/l0/tests/）"
python3 -m unittest discover -s testing/l0/tests 2>&1 | tee "$LOG_DIR/l0-script-tests.log"
rc=${PIPESTATUS[0]}
[[ $rc -eq 0 ]] || FAILED=$((FAILED + 1))

if [[ $L0_ONLY -eq 0 ]]; then
  step "拷贝 L1 测试到 SwiftPM 目录（与 CI 一致，保证种子最新）"
  mkdir -p macos/Tests/RhythmThemeTests
  cp testing/l1/macos/*.swift macos/Tests/RhythmThemeTests/

  step "L1 macOS swift test（tee → $LOG_DIR/l1-macos-swift-test.log）"
  # pipefail 使管道退出码取 swift test（tee 恒 0），子 shell 退出码即真实结果
  (cd macos && swift test 2>&1 | tee "$LOG_DIR/l1-macos-swift-test.log")
  rc=$?
  echo "swift test 退出码: $rc"
  [[ $rc -eq 0 ]] || FAILED=$((FAILED + 1))

  step "L1 macOS 内存卫生（ASan，tee → $LOG_DIR/l1-macos-asan.log）"
  (cd macos && swift test --sanitize=address 2>&1 | tee "$LOG_DIR/l1-macos-asan.log")
  rc=$?
  [[ $rc -eq 0 ]] || FAILED=$((FAILED + 1))
fi

echo
if [[ $FAILED -eq 0 ]]; then
  echo "===== 全部通过 ====="
else
  echo "===== $FAILED 步失败 ====="
fi
echo "全部日志见 $LOG_DIR/"
ls -1 "$LOG_DIR"

if [[ $FAILED -gt 0 && $ALLOW_EXPECTED_FAILURES -eq 0 ]]; then
  echo "存在失败步骤，以非零退出码结束（预期失败请用 --allow-expected-failures 或 ALLOW_EXPECTED_FAILURES=1 显式豁免）" >&2
  exit 1
fi
exit 0
