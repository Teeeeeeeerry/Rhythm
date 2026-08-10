# L1 前置重构：Package.swift 拆出 RhythmTheme library target

SwiftPM 禁止测试 target `import` executable target —— `swift test` 无法直接测
`Rhythm` 可执行目标。因此把品牌主题代码拆为独立 library target：

## 改动

1. 新建 `macos/RhythmTheme/` 目录，**移动** `macos/Rhythm/Views/Theme.swift` 到该目录
   （仅此一个文件；Theme.swift 只依赖 SwiftUI/AppKit，无 RhythmCore 依赖）。
2. `Theme.swift` 中 `isDark(_:)` 由 `private` 改为 `internal`（去掉 private 即可，
   默认就是 internal），使 `RhythmThemeTests` 可测 isDark 矩阵。
3. 新 `macos/Package.swift`：

```swift
// swift-tools-version: 5.9
import PackageDescription

let package = Package(
    name: "Rhythm",
    platforms: [.macOS(.v14)],
    targets: [
        .systemLibrary(
            name: "RhythmCore",
            path: "Rhythm/Bridge"
        ),
        .target(
            name: "RhythmTheme",
            path: "RhythmTheme"
        ),
        .executableTarget(
            name: "Rhythm",
            dependencies: ["RhythmCore", "RhythmTheme"],
            path: "Rhythm",
            exclude: ["Bridge", "Resources", "Views/Theme.swift"],
            linkerSettings: [
                .linkedLibrary("rhythm_core"),
                .unsafeFlags(["-L", "../target/release"]),
            ]
        ),
        .testTarget(
            name: "RhythmThemeTests",
            dependencies: ["RhythmTheme"],
            path: "Tests/RhythmThemeTests"   // 见下方「测试文件位置」
        ),
    ]
)
```

## 测试文件位置

L1 测试当前在 `docs/testing/l1/macos/`（PaletteSeed.swift + 五组测试）。两种挂法：

- **拷贝挂（推荐）**：把 `docs/testing/l1/macos/` 下全部 `.swift` 拷贝到
  `macos/Tests/RhythmThemeTests/`（SwiftPM 测试目录约定），Package.swift 用默认
  `path: "Tests/RhythmThemeTests"`。`sync-palette.py --emit-swift-seed` 重新生成后
  覆盖拷贝即可。
- **直接挂**：testTarget 的 `path` 指向 `../docs/testing/l1/macos`，exclude 留空。
  缺点：SwiftPM 会在 `docs/` 下产生 `.build` 缓存目录，且 docs 目录被包进构建扫描。

CI 脚本（docs/testing/ci/ci.yml）按拷贝挂编写。

## 验证

```bash
cd macos && swift build            # Rhythm/RhythmTheme 均编译通过
swift test                         # RhythmThemeTests 五组全绿
```

## 数据驱动闭环

加/改 token 后：

```bash
python3 docs/testing/sync-palette.py --emit-swift-seed   # 刷新 PaletteSeed.swift
swift test                         # 新 token 自动获得 RGB/对比度/互异全套断言
```

## 不变量

- 运行时零改动：`Rhythm` 仍链接同一份代码，仅编译单元划分变化。
- `RhythmTheme` 不得依赖 `Rhythm`（单向依赖），否则循环依赖编译失败。
- F6（isDark 未知 appearance 决策）落地时同步更新 `ThemeIsDarkMatrixTests.swift`
  的 `testUnknownAppearanceFallsBackToFirstMatch`。
