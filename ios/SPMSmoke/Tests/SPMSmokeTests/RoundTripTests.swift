import Foundation
import SPMSmoke
import Testing

// The time limit turns a hung FFI call into a failure instead of a CI job that
// runs until its timeout.
@Test("open, generateDid, pack and unpack through navia_coreFFI.xcframework", .timeLimit(.minutes(1)))
func openGenerateDidPackUnpack() async throws {
    let directory = FileManager.default.temporaryDirectory
        .appendingPathComponent("SPMSmoke-\(UUID().uuidString)", isDirectory: true)
    try FileManager.default.createDirectory(at: directory, withIntermediateDirectories: true)
    defer { try? FileManager.default.removeItem(at: directory) }

    let trip = try await roundTrip(databasePath: directory.appendingPathComponent("navia.db").path)

    #expect(trip.did.hasPrefix("did:peer:"))
    #expect(trip.frame.contains("ciphertext"))
    #expect(!trip.frame.contains("Hello from SPMSmoke"), "the frame carries the body encrypted")

    #expect(trip.received.id == trip.sent.id)
    #expect(trip.received.msgType == trip.sent.msgType)
    #expect(trip.received.body == trip.sent.body)
    #expect(trip.received.from == trip.did)
    #expect(trip.received.to == [trip.did])
    #expect(trip.received.authenticated, "authcrypt from a DID in this store proves its sender")
    #expect(trip.received.encryptedFromKid?.hasPrefix(trip.did + "#") == true)
    #expect(!trip.received.anonymousSender)
}
