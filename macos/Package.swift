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
            exclude: ["Bridge", "Resources"],
            linkerSettings: [
                .linkedLibrary("rhythm_core"),
                .unsafeFlags(["-L", "../target/release"]),
            ]
        ),
        .testTarget(
            name: "RhythmThemeTests",
            dependencies: ["RhythmTheme"],
            path: "Tests/RhythmThemeTests"
        ),
    ]
)
