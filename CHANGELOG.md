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

## [1.3.3] - 2026-09-28

### Fixed
- `unpack` keeps a JSON-string body as plain text again; regression since 1.3.2. A body such as `hello` reaches Kotlin/Swift as `hello`, not `"hello"`, so plain text survives a pack/unpack round trip byte for byte. Object and array bodies arrive as compact JSON text as before; number, bool and null bodies arrive as their JSON text (`42`, `true`, `null`). The body rule is documented in `rust/navia-core/src/ffi/conversions.rs`. (#78)

### Changed
- `DidComInterface.unpack` now tells a frame that can never be unpacked apart from a failure a later attempt can get past. A malformed frame (a wrong signature included), one addressed to keys this store does not hold, one with an illegal argument, or one in unsupported or incompatible crypto still throws `DidCommException.UnpackingException` (Swift `DidCommError.UnpackingError`). A store, I/O or DID-resolution failure now throws `DidCommException.DatabaseException` (Swift `DidCommError.DatabaseError`) instead, so a consumer acknowledges only the former to the mediator and leaves the latter for redelivery. The kind is read by type from the navia-didcomm error that navia-messaging wraps. navia-didcomm 1.3.0 still reports some faults of the frame itself as `InvalidState`: truncated JSON in the envelope or protected header (an empty frame included), because serde_json's `Eof` maps to `InvalidState`; a wrong skid; an anoncrypt/authcrypt recipient mismatch; a JWS signature kid that does not match. It reports a sender kid missing from its DID document as `DIDUrlNotFound`. These surface as `DatabaseException` too and fail the same way on every redelivery, so a consumer should still cap redeliveries per `delivery_id`. `DidCommError` keeps its variants, and FFI signatures are unchanged. (#79)
- CI pins the Rust toolchain to 1.98.1 through the `setup-rust-env` composite (the target cache key carries the version), moves to the Node 24 action majors (`actions/checkout@v5`, `actions/cache@v5`, `actions/setup-java@v5`, `actions/upload-artifact@v6`, `actions/download-artifact@v7`) and to `android-actions/setup-android@v4`, fails the Android packaging job when either `libnavia_core.so` is missing, and runs the full Rust and Android matrix for a change under `.github/`. (#78, #80)
- `rust/Cargo.lock` moves the transitive crates that have a fixed release on the pinned line by exact version (bytes 1.11.1, crossbeam-epoch 0.9.20, h2 0.4.16, rustls 0.23.45, rustls-webpki 0.103.15, time 0.3.47) so `cargo audit` in Quality Checks passes; the four advisories with no fix under `ssi 0.12` / `reqwest 0.11` (rustls-webpki 0.101, h2 0.3) are listed in `rust/.cargo/audit.toml` with the reason. (#82)

### Compatibility
- Patch release: FFI signatures are unchanged, `DidCommError` keeps its variants, and the DIDComm wire format is unchanged. navia-server and nyx-org-gateway need no update; 1.3.2 and 1.3.3 clients interoperate (a 1.3.2 receiver keeps showing string bodies with JSON quotes until it updates).
- Behaviour change on unpack: a store, I/O or DID-resolution failure now throws `DidCommException.DatabaseException` (Swift `DidCommError.DatabaseError`) where 1.3.2 threw `UnpackingException`. A consumer that acknowledges every unpack failure to the mediator keeps working; one that wants redelivery for transient faults can now tell them apart, and should still cap redeliveries per `delivery_id` for the frame faults navia-didcomm 1.3.0 reports as `InvalidState`.
- Messages a 1.3.2 client already stored keep their literal quotes; the fix applies to messages unpacked from 1.3.3 on.

## [1.1.11] - 2024-07-27

### Changed
- Updated version for 16KB alignment support release

## [1.1.10] - Previous releases...