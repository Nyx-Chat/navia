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
- `DidComInterface.unpack` now tells a frame that can never be unpacked apart from a failure a later attempt can get past. A malformed frame (a wrong signature included), one addressed to keys this store does not hold, one with an illegal argument, or one in unsupported or incompatible crypto still throws `DidCommException.UnpackingException` (Swift `DidCommError.UnpackingError`). A store, I/O or DID-resolution failure now throws `DidCommException.DatabaseException` (Swift `DidCommError.DatabaseError`) instead, so a consumer acknowledges only the former to the mediator and leaves the latter for redelivery. The kind is read by type from the navia-didcomm error that navia-messaging wraps. navia-didcomm 1.3.0 still reports some faults of the frame itself as `InvalidState`: truncated JSON in the envelope or protected header (an empty frame included), because serde_json's `Eof` maps to `InvalidState`; a wrong skid; an anoncrypt/authcrypt recipient mismatch; a JWS signature kid that does not match. It reports a sender kid missing from its DID document as `DIDUrlNotFound`. These surface as `DatabaseException` too and fail the same way on every redelivery, so a consumer should still cap redeliveries per `delivery_id`. `DidCommError` keeps its variants, and FFI signatures are unchanged.
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