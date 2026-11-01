// swift-tools-version: 6.0

// Navia for iOS: the UniFFI Swift bindings (target Navia) over the static
// navia_coreFFI.xcframework of the Rust library.
//
// The XCFramework is a build output: run scripts/build-xcframework.sh first.
// It builds ios/build/navia_coreFFI.xcframework and regenerates
// ios/Navia/Generated from the same build, so the bindings' API checksums
// match the binary. This manifest is for local path use, such as
// `.package(path: "../navia")` or ios/SPMSmoke; a git dependency on this
// repository finds no XCFramework.

import PackageDescription

let package = Package(
    name: "Navia",
    platforms: [
        // Keep in step with IPHONEOS_DEPLOYMENT_TARGET in scripts/build-xcframework.sh.
        .iOS(.v17),
    ],
    products: [
        .library(name: "Navia", targets: ["Navia"]),
    ],
    targets: [
        .binaryTarget(
            name: "navia_coreFFI",
            path: "ios/build/navia_coreFFI.xcframework"
        ),
        .target(
            name: "Navia",
            dependencies: ["navia_coreFFI"],
            path: "ios/Navia/Generated",
            // The XCFramework carries the header and the module map; these are
            // the committed copies CI compares with each build.
            exclude: ["navia_coreFFI.h", "module.modulemap"],
            // The native-static-libs rustc reports for libnavia_core.a, less
            // libSystem, libc and libm, which every link gets. The archive
            // references no C++, libresolv or SystemConfiguration symbols.
            linkerSettings: [
                .linkedFramework("Security"),
                .linkedFramework("CoreFoundation"),
                .linkedLibrary("iconv"),
            ]
        ),
    ],
    // UniFFI 0.28's generated Swift is not concurrency-clean under the Swift 6
    // language mode. Swift 6 apps import it with `@preconcurrency import Navia`.
    swiftLanguageModes: [.v5]
)
