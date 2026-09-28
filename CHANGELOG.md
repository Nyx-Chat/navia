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
- `DIDCommMessage` gains four sender authentication fields that `DidComInterface.unpack` fills from navia-didcomm's unpack metadata: `authenticated`, `encrypted_from_kid` (Kotlin/Swift `encryptedFromKid`), `sign_from` (`signFrom`) and `anonymous_sender` (`anonymousSender`). `authenticated` is `true` only when the plaintext `from` is set, navia-didcomm authenticated the frame (authcrypt with a resolved sender key, or a verified signature), every sender key it used belongs to the `from` DID, and the frame is tied to the recipient: it is authcrypt-encrypted, or, when a signature is the only proof, every recipient key of its encrypted envelope belongs to a DID named in the plaintext `to`. A consumer acts on `from` only when it is `true`. The key that proves `from` is `encrypted_from_kid` when it is set and `sign_from` otherwise, so `encrypted_from_kid` can be `null` / `nil` on an authenticated message; a consumer that accepts only authcrypt proof checks `authenticated && encryptedFromKid != null`. The flag says nothing about freshness, so consumers still deduplicate by `id`. The kids are the raw key IDs (`did#fragment`); `anonymous_sender` is `true` for a frame that arrived in an anoncrypt envelope. The fields carry meaning only on a message returned by `unpack`: every other path that creates a `DIDCommMessage` sets `false` / `None`, and `pack` / `pack_no_forward` ignore them. They have UniFFI defaults (`false` / `null` / `nil`), so Kotlin and Swift code that builds a `DidCommMessage` without them keeps compiling; Rust struct literals must name them. This changes a UniFFI record, so the release that ships it is a minor bump to **1.4.0**, made in the separate version-bump PR; consumers rebuild against the 1.4.0 AAR / xcframework bindings. Other FFI signatures are unchanged. The committed Swift bindings in `ios/Navia/Generated/` are regenerated; they also catch up on `mediatorDid`, `packNoForward` and `to: [String]`, which the committed copy had missed.

### Changed
- Updated minimum Android NDK requirement from r25b to r27c for 16KB support
- Removed cargo-ndk from build process as it interferes with alignment flags
- Updated CI/CD workflows to use NDK r27c and verify alignment
- Enhanced build scripts with automatic alignment fixes
- `DidComInterface.unpack` refuses an authcrypt frame whose plaintext `from` names another DID than the DID part of `encrypted_from_kid` (a forged `from`) with `DidCommError::UnpackingError` (`Sender mismatch: ...`; the message names neither DID). The failure is permanent (Kotlin `DidCommException.UnpackingException`, Swift `DidCommError.UnpackingError`): a redelivery fails the same way, so a consumer acknowledges the frame and drops it. The check runs only when `encrypted_from_kid` is set. Unsigned anoncrypt frames carry no sender key and still unpack, with `authenticated = false`; so does a frame that is not authcrypt-encrypted and whose signer kid names another DID than `from`, and a signed frame that is not authcrypt-encrypted and was never encrypted or was re-encrypted to a recipient key whose DID its `to` does not name (a message Alice signed for Mallory that Mallory relays to Bob). navia-didcomm does not compare `to` with the envelope's recipient keys, and it lists every recipient key of the envelope, so every one of them has to belong to a DID in `to`.

### Fixed
- Native libraries now properly align all LOAD segments to 16KB boundaries
- Resolved Google Play Store rejection for Android 15+ compatibility

## [1.1.11] - 2024-07-27

### Changed
- Updated version for 16KB alignment support release

## [1.1.10] - Previous releases...