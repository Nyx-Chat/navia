// swift-tools-version: 6.0

// Simulator smoke test of Navia through SwiftPM: the root package's static
// navia_coreFFI.xcframework and its Swift bindings, linked the way an app
// links them. SPMSmoke is a Swift 6 language mode consumer of the Swift 5 mode
// Navia target. Build the XCFramework first, then run the test from this
// directory, and link it against the device slice (no device or signing):
//
//   scripts/build-xcframework.sh
//   xcodebuild test -scheme SPMSmoke -destination 'platform=iOS Simulator,name=<iPhone>'
//   xcodebuild build-for-testing -scheme SPMSmoke -destination 'generic/platform=iOS' CODE_SIGNING_ALLOWED=NO

import PackageDescription

let package = Package(
    name: "SPMSmoke",
    platforms: [
        .iOS(.v17),
    ],
    products: [
        .library(name: "SPMSmoke", targets: ["SPMSmoke"]),
    ],
    dependencies: [
        .package(name: "Navia", path: "../.."),
    ],
    targets: [
        .target(
            name: "SPMSmoke",
            dependencies: [
                .product(name: "Navia", package: "Navia"),
            ]
        ),
        .testTarget(
            name: "SPMSmokeTests",
            dependencies: ["SPMSmoke"]
        ),
    ]
)
