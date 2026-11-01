import Foundation
@preconcurrency import Navia

/// What one trip through Navia produced: the DID it generated, the message it
/// packed, the frame `packNoForward` returned and what `unpack` made of it.
public struct RoundTrip {
    public let did: String
    public let sent: DidCommMessage
    public let frame: String
    public let received: DidCommMessage
}

/// Opens a Navia store at `databasePath`, generates a did:peer, packs a
/// message from that DID to itself (authcrypt, no mediator forward) and
/// unpacks the frame, the way an app calls Navia through SwiftPM.
///
/// The seed is a fixed test value; an app keeps a random one in the Keychain.
public func roundTrip(databasePath: String) async throws -> RoundTrip {
    let navia = DidComInterface(path: databasePath, mediatorDid: "did:peer:placeholder")
    try await navia.open(path: databasePath, seed: Data(repeating: 42, count: 32))

    let did = try await navia.generateDid(uri: "https://example.com/didcomm", routingKeys: [])

    let sent = DidCommMessage(
        id: UUID().uuidString,
        msgType: "https://didcomm.org/basicmessage/2.0/message",
        body: #"{"content":"Hello from SPMSmoke"}"#,
        from: did,
        to: [did]
    )
    let frame = try await navia.packNoForward(msg: sent, from: did, to: [did])
    let received = try await navia.unpack(msg: frame)

    return RoundTrip(did: did, sent: sent, frame: frame, received: received)
}
