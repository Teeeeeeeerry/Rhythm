# L1 前置重构：Package.swift 拆出 RhythmTheme library target

> 状态：**已落地**（#65 起，v0.5.64 现状）。本文保留为改动记录与数据驱动闭环说明；
> 现行 `macos/Package.swift` 见下方"当前形态"，与本文最初的建议稿有两处差异：
> `Rhythm` 的 `exclude` 不再含 `Views/Theme.swift`（文件已移走），
> 且多出 `AppStateTests` 一个 testTarget（Wave 2 引入）。

SwiftPM 禁止测试 target `import` executable target —— `swift test` 无法直接测
`Rhythm` 可执行目标。因此把品牌主题代码拆为独立 library target：

## 改动

1. 新建 `macos/RhythmTheme/` 目录，**移动** `Theme.swift`（原 `macos/Rhythm/Views/Theme.swift`）
   到该目录，现路径 `macos/RhythmTheme/Theme.swift`
   （仅此一个文件；Theme.swift 只依赖 SwiftUI/AppKit，无 RhythmCore 依赖）。
2. `Theme.swift` 中 `isDark(_:)` 由 `private` 改为 `internal`（去掉 private 即可，
   默认就是 internal），使 `RhythmThemeTests` 可测 isDark 矩阵。
3. 新 `macos/Package.swift`（建议稿；当前形态见文末）：

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

L1 测试当前在 `testing/l1/macos/`（PaletteSeed.swift + 五组测试）。两种挂法：

- **拷贝挂（推荐）**：把 `testing/l1/macos/` 下全部 `.swift` 拷贝到
  `macos/Tests/RhythmThemeTests/`（SwiftPM 测试目录约定），Package.swift 用默认
  `path: "Tests/RhythmThemeTests"`。`gen-palette.py --emit-swift-seed` 重新生成后
  覆盖拷贝即可。
- **直接挂**：testTarget 的 `path` 指向 `../testing/l1/macos`，exclude 留空。
  缺点：SwiftPM 会在 `testing/` 下产生 `.build` 缓存目录，且该目录被包进构建扫描。

CI 脚本（testing/ci/ci.yml）按拷贝挂编写。

## 验证

```bash
cd macos && swift build            # Rhythm/RhythmTheme 均编译通过
swift test                         # RhythmThemeTests 五组全绿
```

## 数据驱动闭环

加/改 token 后：

```bash
python3 scripts/gen-palette.py --emit-swift-seed   # 刷新 PaletteSeed.swift
swift test                         # 新 token 自动获得 RGB/对比度/互异全套断言
```

## 当前形态（v0.5.64 的 macos/Package.swift）

```swift
targets: [
    .systemLibrary(name: "RhythmCore", path: "Rhythm/Bridge"),
    .target(name: "RhythmTheme", path: "RhythmTheme"),
    .executableTarget(
        name: "Rhythm",
        dependencies: ["RhythmCore", "RhythmTheme"],
        path: "Rhythm",
        exclude: ["Bridge", "Resources"],
        linkerSettings: [
            .linkedLibrary("rhythm_core"),
            .unsafeFlags(["-L", "../target/release"]),
        ]
    ),
    .testTarget(name: "RhythmThemeTests", dependencies: ["RhythmTheme"], path: "Tests/RhythmThemeTests"),
    .testTarget(name: "AppStateTests", dependencies: ["Rhythm"], path: "Tests/AppStateTests"),
]
```

## 不变量

- 运行时零改动：`Rhythm` 仍链接同一份代码，仅编译单元划分变化。
- `RhythmTheme` 不得依赖 `Rhythm`（单向依赖），否则循环依赖编译失败。
- F6（isDark 未知 appearance 决策）**已落地**：`isDark(_:)` 用 `bestMatch` 归一到
  darkAqua/aqua/HC 双档，`ThemeIsDarkMatrixTests.testUnknownAppearanceFallsBackToFirstMatch`
  钉住该行为；后续再改 fallback 需同步这条测试。
