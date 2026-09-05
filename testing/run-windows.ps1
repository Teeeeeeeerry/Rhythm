# Rhythm Windows 测试一键入口：L1 ctest + L2 截屏比对 + L3 WinAppDriver 冒烟。
# 每步输出 tee 到 testing/logs/，日志文件名见各步。
#
# 用法（仓库根）：
#     powershell -File testing/run-windows.ps1           # L1 + L2
#     powershell -File testing/run-windows.ps1 -Smoke    # 追加 L3 冒烟

param([switch]$Smoke)
$ErrorActionPreference = "Continue"

$Root = Split-Path -Parent $PSScriptRoot
Set-Location $Root
$LogDir = Join-Path $Root "testing/logs"
New-Item -ItemType Directory -Force $LogDir | Out-Null

Write-Host "===== Rhythm Windows 测试 $(Get-Date) ====="

Write-Host "----- L1: cmake 构建 + ctest（l1-windows-cmake.log / l1-windows-ctest.log）-----"
cmake -S testing/l1/windows -B build 2>&1 | Tee-Object -FilePath (Join-Path $LogDir "l1-windows-cmake.log")
if ($LASTEXITCODE -ne 0) { exit $LASTEXITCODE }
cmake --build build 2>&1 | Tee-Object -FilePath (Join-Path $LogDir "l1-windows-cmake.log") -Append
if ($LASTEXITCODE -ne 0) { exit $LASTEXITCODE }
ctest --test-dir build --output-on-failure 2>&1 | Tee-Object -FilePath (Join-Path $LogDir "l1-windows-ctest.log")
if ($LASTEXITCODE -ne 0) { Write-Host "! ctest 失败（$LASTEXITCODE），继续 L2" }

Write-Host "----- L1b: 应用工程行为测试（windows/CMakeLists.txt RhythmTests，l1-windows-rhythmtests.log）-----"
cmake -S windows -B build-rhythm-tests 2>&1 | Tee-Object -FilePath (Join-Path $LogDir "l1-windows-rhythmtests.log")
if ($LASTEXITCODE -ne 0) { Write-Host "! RhythmTests cmake 失败（$LASTEXITCODE）"; exit $LASTEXITCODE }
cmake --build build-rhythm-tests --target RhythmTests --config Release 2>&1 | Tee-Object -FilePath (Join-Path $LogDir "l1-windows-rhythmtests.log") -Append
if ($LASTEXITCODE -ne 0) { Write-Host "! RhythmTests 构建失败（$LASTEXITCODE）"; exit $LASTEXITCODE }
ctest --test-dir build-rhythm-tests --output-on-failure 2>&1 | Tee-Object -FilePath (Join-Path $LogDir "l1-windows-rhythmtests.log") -Append
if ($LASTEXITCODE -ne 0) { Write-Host "! RhythmTests ctest 失败（$LASTEXITCODE）"; exit $LASTEXITCODE }

Write-Host "----- L2: 截屏宿主构建 + golden 像素比对（l2-windows-capture.log / l2-windows-compare.log）-----"
cmake -S testing/l2/windows -B build-capture 2>&1 | Tee-Object -FilePath (Join-Path $LogDir "l2-windows-capture.log")
if ($LASTEXITCODE -ne 0) { exit $LASTEXITCODE }
cmake --build build-capture 2>&1 | Tee-Object -FilePath (Join-Path $LogDir "l2-windows-capture.log") -Append
if ($LASTEXITCODE -ne 0) { exit $LASTEXITCODE }

$CaptureExe = "build-capture\Release\capture_views.exe"
if (-not (Test-Path $CaptureExe)) {
    $CaptureExe = "build-capture\Debug\capture_views.exe"
}
if (Test-Path $CaptureExe) {
    & $CaptureExe build/artifacts 2>&1 | Tee-Object -FilePath (Join-Path $LogDir "l2-windows-capture.log") -Append
} else {
    Write-Host "! 未找到 capture_views.exe，跳过截屏"
}
python3 testing/l2/windows/compare_screenshots.py `
    --actual build/artifacts --golden testing/l2/windows/golden 2>&1 `
    | Tee-Object -FilePath (Join-Path $LogDir "l2-windows-compare.log")
if ($LASTEXITCODE -ne 0) { Write-Host "! 像素比对失败（$LASTEXITCODE），详情见日志" }

if ($Smoke) {
    Write-Host "----- L3: WinAppDriver 冒烟（l3-windows-smoke.log）-----"
    python3 testing/l3/windows/theme_switch.py --smoke --app build/Release/Rhythm.exe 2>&1 `
        | Tee-Object -FilePath (Join-Path $LogDir "l3-windows-smoke.log")
    if ($LASTEXITCODE -ne 0) { Write-Host "! 冒烟失败，降级路径见 l3/windows/winappdriver.md" }
}

Write-Host ""
Write-Host "===== 全部日志见 $LogDir/ ====="
Get-ChildItem $LogDir | Select-Object -ExpandProperty Name
