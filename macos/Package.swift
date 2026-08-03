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
        .executableTarget(
            name: "Rhythm",
            dependencies: ["RhythmCore"],
            path: "Rhythm",
            exclude: ["Bridge", "Resources"],
            linkerSettings: [
                .linkedLibrary("rhythm_core"),
                .unsafeFlags(["-L", "../target/release"]),
            ]
        ),
    ]
)
