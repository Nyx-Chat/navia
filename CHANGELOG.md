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
- `rust/Cargo.lock` pins navia-messaging 1.1.2 (the manifest is unchanged).
  1.1.2 passes navia-didcomm's error kind through instead of wrapping every
  unpack failure in `InvalidState`, and reports a failed key store as
  `IoError`. The FFI classes are unchanged: a store failure still surfaces as
  `DidCommError::DatabaseError` (Kotlin `DidCommException.DatabaseException`)
  and a bad frame as `UnpackingError`; tests pin both, end to end through the
  FFI with a closed store. Only the message text changes: the navia-messaging
  part of the `DidCommError` message now starts with the passed-through kind
  (`Malformed message: Malformed: ...` where 1.1.1 gave `Malformed message:
  Invalid state: ...`), and a store failure reads `Storage operation failed:
  unpack - IO error: ...`.
- `rust/Cargo.lock` pins navia-didcomm 1.3.1 (the manifest is unchanged).
  1.3.1 labels the faults of a frame itself by cause, where 1.3.0 gave most of
  them `InvalidState`, so `DidComInterface.unpack` now throws
  `DidCommError::UnpackingError` (Kotlin `DidCommException.UnpackingException`,
  Swift `DidCommError.UnpackingError`) for frames it used to report as
  `DatabaseError`, and a consumer acknowledges them on the first delivery
  instead of leaving them for redelivery. Truncated JSON in the frame or its
  protected header (an empty frame included), an anoncrypt envelope that
  carries `apu` or is addressed to other keys than the authcrypt inside it, an
  authcrypt tag longer than 124 bytes, a signature or `from_prior` `alg` that
  does not match the signer's or issuer's key type, and an authcrypt sender
  whose DID document carries a Multikey that cannot be decoded surface as
  `UnpackingError::MalformedMessage` (`Malformed message: Malformed: ...`), as
  does a recipient secret in this store whose multicodec prefix cannot be read.
  A recipient key removed from the store while the frame is unpacked surfaces
  as `UnpackingError::RecipientKeyNotFound` (`No matching recipient key found:
  ...`). An authcrypt sender whose DID document has a verification method of a
  type navia-messaging's resolver does not support, and a signer or
  `from_prior` issuer key of a type navia-didcomm does not support, surface as
  `UnpackingError::DecryptionFailed` (`Decryption failed: Unsupported crypto
  or method: ...`). All of these were `DatabaseError`. A sender DID that does
  not resolve (`DIDNotResolved`) and a sender key its DID document does not
  list (`DIDUrlNotFound`) stay `DatabaseError`, like a store or I/O failure, so
  keep the redelivery cap per frame. navia-didcomm still rejects a re-wrapped
  forward whose outer anoncrypt envelope names other keys than the authcrypt
  inside it, a genuine one included; such a frame now fails on the first
  delivery instead of at the cap. `rust/navia-core/tests/error_scenarios.rs`
  pins truncated JSON, anoncrypt `apu`, the tag limit and the sender-resolution
  kinds (a sender DID that does not resolve, a sender kid its DID document does
  not list, a sender key of an unsupported type and one that cannot be
  decoded) end to end through the FFI; the other relabels rest on
  navia-didcomm's and navia-messaging's own tests and the kind mapping in
  `handler.rs`.
- The `DidComInterface.unpack` doc comment (`interface.rs`) says the redelivery
  cap for a `DatabaseError` is per frame, not per `delivery_id`, as `handler.rs`
  and `docs/API.md` have since 1.4.0, and describes the navia-didcomm 1.3.1
  split above: the `UnpackingError` message prefix for each fault of the frame,
  and what still lands in `DatabaseError` besides a store, I/O or DID
  resolution failure (`DIDNotResolved`, `DIDUrlNotFound` and navia-didcomm's
  own `InvalidState`). The doc comment is part of the method's UniFFI checksum,
  so these doc-only edits move it (to 6160); Kotlin and Swift bindings must
  come from the same build as the library they call, or the first FFI call
  fails with `UniFFI API checksum mismatch`.
- The committed Swift bindings in `ios/Navia/Generated/` are regenerated from
  the source (the 1.4.0 copy carried stale `pack`, `pack_no_forward` and
  `unpack` checksums), and `scripts/build-for-ios.sh` generates them with
  `--no-format`, so the output no longer depends on whether the machine has
  swiftformat.
- The GitHub release notes name the published coordinate `com.nyx:navia` (not
  `com.github.nyx-chat:navia`), add the JNA dependency and the GitHub Packages
  credentials note, drop the Swift Package Manager snippet (the repository has
  no `Package.swift`), name the `Navia.xcframework-<version>.zip` asset and link
  the iOS guide's known problems. `ANDROID_INTEGRATION.md` gains the same JNA
  line and credentials note as `README.md`.
- `docs/IOS_INTEGRATION.md` drops Swift Package Manager, names the release
  asset, lists what the pod's from-source build needs, and describes the known
  problems of both iOS paths (the pod does not link; the XCFramework may fail
  to load at launch). The README's release steps describe the workflow that
  actually publishes, including the iOS `MARKETING_VERSION`.
- CI: the iOS test job picks an available iPhone simulator for the SDK of the
  runner's Xcode instead of a fixed device, and fails when the committed
  `ios/Navia/Generated/` differs from the bindings the build generated. The
  release workflow takes the Android and iOS binaries from the release PR's own
  PR Validation run (the successful run for the PR's head commit) instead of
  the latest successful run of any PR, and fails before it tags or publishes
  anything when that run or its artifacts are missing.

### Fixed
- Native libraries now properly align all LOAD segments to 16KB boundaries
- Resolved Google Play Store rejection for Android 15+ compatibility
- A release could publish another PR's binaries: it downloaded the artifacts of
  whichever PR Validation run had succeeded last, with no filter on branch, PR
  or commit.

### Compatibility
- FFI signatures, `DidCommError` variants and the DIDComm wire format are
  unchanged. The `unpack` checksum changes, so a consumer takes the Kotlin or
  Swift bindings and the library from the same AAR / XCFramework, as usual.
- A consumer that parses `DidCommError` message text sees the new kind prefix
  described above. The navia-messaging 1.1.2 pin changes no exception class.
- Behaviour change on unpack from the navia-didcomm 1.3.1 pin: the frame faults
  listed above throw `UnpackingException` / `UnpackingError` where they threw
  `DatabaseException` / `DatabaseError`, so a consumer that branches on the
  exception class (nyx-android does) acknowledges them on the first delivery.
  Keep the redelivery cap for `DatabaseError`: a sender DID that does not
  resolve and a sender key its DID document does not list still land there.

## [1.4.0] - 2026-09-29

### Added
- `DIDCommMessage` gains four sender authentication fields that
  `DidComInterface.unpack` fills from navia-didcomm's unpack metadata:
  `authenticated`, `encrypted_from_kid` (Kotlin/Swift `encryptedFromKid`),
  `sign_from` (`signFrom`) and `anonymous_sender` (`anonymousSender`).
  `authenticated` is `true` only when the plaintext `from` is set, navia-didcomm
  authenticated the frame (authcrypt with a resolved sender key, or a verified
  signature), every sender key it used belongs to the `from` DID, and the frame
  is tied to the recipient: it is authcrypt-encrypted, or, when a signature is
  the only proof, every recipient key of its encrypted envelope belongs to a DID
  named in the plaintext `to`. A consumer acts on `from` only when it is `true`.
  The key that proves `from` is `encrypted_from_kid` when it is set and
  `sign_from` otherwise, so `encrypted_from_kid` can be `null` / `nil` on an
  authenticated message; a consumer that accepts only authcrypt proof checks
  `authenticated && encryptedFromKid != null`. The flag says nothing about
  freshness, so consumers still deduplicate by `id`. The kids are the raw key
  IDs (`did#fragment`); `anonymous_sender` is `true` for a frame that arrived in
  an anoncrypt envelope. The fields carry meaning only on a message returned by
  `unpack`: every other path that creates a `DIDCommMessage` sets `false` /
  `None`, and `pack` / `pack_no_forward` ignore them. They have UniFFI defaults
  (`false` / `null` / `nil`), so Kotlin and Swift code that builds a
  `DidCommMessage` without them keeps compiling; Rust struct literals must name
  them. This changes a UniFFI record, so this release is a minor bump to **1.4.0**; consumers rebuild
  against the 1.4.0 AAR / xcframework bindings. Other FFI signatures are
  unchanged. The committed Swift bindings in `ios/Navia/Generated/` are
  regenerated; they also catch up on `mediatorDid`, `packNoForward` and `to:
  [String]`, which the committed copy had missed. (#81)

### Changed
- `DidComInterface.unpack` refuses an authcrypt frame whose plaintext `from`
  names another DID than the DID part of `encrypted_from_kid` (a forged `from`)
  with `DidCommError::UnpackingError` (`Sender mismatch: ...`; the message names
  neither DID). The failure is permanent (Kotlin
  `DidCommException.UnpackingException`, Swift `DidCommError.UnpackingError`): a
  redelivery fails the same way, so a consumer acknowledges the frame and drops
  it. The check runs only when `encrypted_from_kid` is set. Unsigned anoncrypt
  frames carry no sender key and still unpack, with `authenticated = false`; so
  does a frame that is not authcrypt-encrypted and whose signer kid names
  another DID than `from`, and a signed frame that is not authcrypt-encrypted
  and was never encrypted or was re-encrypted to a recipient key whose DID its
  `to` does not name (a message Alice signed for Mallory that Mallory relays to
  Bob). navia-didcomm does not compare `to` with the envelope's recipient keys,
  and it lists every recipient key of the envelope, so every one of them has to
  belong to a DID in `to`. (#81)
- The classifier docs (`handler.rs`, `docs/API.md`) describe the redelivery cap a
  consumer applies to a `DatabaseError` as per frame (the stored payload with the
  mediator's `delivery_id` left out), since the mediator mints a new `delivery_id`
  for every delivery and accepts an ACK for any of them (#81).

### Compatibility
- Minor release: the UniFFI record `DIDCommMessage` gains `authenticated`,
  `encryptedFromKid`, `signFrom` and `anonymousSender`, so the generated Kotlin
  and Swift bindings change (the `unpack` method checksum changes as well, since
  its docstring is part of it); consumers rebuild against the 1.4.0 AAR /
  xcframework. The new fields carry UniFFI defaults, so Kotlin and Swift code
  that builds a `DidCommMessage` by name keeps compiling; Rust struct literals
  must name them. Every other FFI signature is unchanged.
- The DIDComm wire format is unchanged; navia-server and nyx-org-gateway need no
  update, and 1.3.3 and 1.4.0 clients interoperate.
- Behaviour change: an authcrypt frame whose plaintext `from` names another DID
  than the DID of `encryptedFromKid` now fails to unpack with
  `UnpackingException` / `UnpackingError` (permanent; acknowledge and drop).
  Genuine Navia traffic is unaffected: navia-didcomm refuses to pack a message
  whose plaintext `from` is not the DID whose key encrypts, so every frame `pack`
  produces already satisfies the check.

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