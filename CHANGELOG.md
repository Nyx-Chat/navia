# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

### Added
- Full support for Android 15+ 16KB page size requirements
- Automatic 16KB alignment verification during build process
- Post-build alignment fix tools for native libraries
- CI/CD validation for 16KB alignment on all releases

### Changed
- Updated minimum Android NDK requirement from r25b to r27c for 16KB support
- Removed cargo-ndk from build process as it interferes with alignment flags
- Updated CI/CD workflows to use NDK r27c and verify alignment
- Enhanced build scripts with automatic alignment fixes

### Fixed
- Native libraries now properly align all LOAD segments to 16KB boundaries
- Resolved Google Play Store rejection for Android 15+ compatibility

## [1.1.11] - 2024-07-27

### Changed
- Updated version for 16KB alignment support release

## [1.1.10] - Previous releases...