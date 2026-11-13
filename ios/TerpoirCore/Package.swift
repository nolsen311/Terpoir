// swift-tools-version:5.9
// SPDX-License-Identifier: GPL-3.0-or-later WITH AdditionRef-Terpoir-AppStore-exception
// Copyright (C) 2026 The Terpoir Authors
// See LICENSING.md for the additional permission under GPLv3 section 7.

// The Rust core for iOS: the xcframework and the Swift bindings that
// ios/scripts/build-core.sh builds into this package. No other dependencies.

import PackageDescription

let package = Package(
    name: "TerpoirCore",
    platforms: [.iOS(.v17)],
    products: [
        .library(name: "TerpoirCore", targets: ["TerpoirCore"]),
    ],
    targets: [
        .binaryTarget(name: "TerpoirCoreFFI", path: "TerpoirCoreFFI.xcframework"),
        .target(name: "TerpoirCore", dependencies: ["TerpoirCoreFFI"], path: "Sources/TerpoirCore"),
        .testTarget(name: "TerpoirCoreTests", dependencies: ["TerpoirCore"], path: "Tests/TerpoirCoreTests"),
    ]
)
