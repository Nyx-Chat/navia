# Navia API Reference

This document provides a comprehensive reference for all Navia APIs, common patterns, and best practices.

## Table of Contents

- [Core Types](#core-types)
- [Main Interface](#main-interface)
- [Error Handling](#error-handling)
- [Common Patterns](#common-patterns)
- [Best Practices](#best-practices)

## Core Types

### DIDCommMessage

The primary message structure for DIDComm v2 communication.

```rust
pub struct DIDCommMessage {
    pub id: String,          // Unique message identifier
    pub msg_type: String,    // Protocol message type URI
    pub body: String,        // Message body (typically JSON)
    pub from: Option<String>, // Sender DID (optional for anonymous)
    pub to: Vec<String>,     // List of recipient DIDs
    // Sender authentication, set by `unpack` only (UniFFI defaults shown)
    pub authenticated: bool,                // default false
    pub encrypted_from_kid: Option<String>, // default None
    pub sign_from: Option<String>,          // default None
    pub anonymous_sender: bool,             // default false
}
```

**Usage Example:**
```kotlin
val message = DIDCommMessage(
    id = UUID.randomUUID().toString(),
    msgType = "https://didcomm.org/basicmessage/2.0/message",
    body = """{"content": "Hello", "sent_time": "${Instant.now()}"}""",
    from = senderDid,
    to = listOf(recipientDid)
    // authenticated, encryptedFromKid, signFrom, anonymousSender keep their defaults
)
```

#### Sender authentication fields

The last four fields describe how the frame `unpack` decrypted was
authenticated. They carry meaning only on a message returned by `unpack`.
Every other path that creates a `DIDCommMessage` (a message you build for
`pack` / `packNoForward`, or any other conversion) sets
`authenticated = false`, both kids to `null` / `nil` and
`anonymousSender = false`, and `pack` / `packNoForward` ignore whatever you
put in them. The fields have UniFFI defaults, so Kotlin and Swift code that
builds a message without them keeps compiling.

| Field (Kotlin / Swift) | Type | Meaning on an unpacked message |
|---|---|---|
| `authenticated` | `Boolean` / `Bool` | `true` when the plaintext `from` is proven and the frame is tied to you: `from` is set, navia-didcomm authenticated the frame (authcrypt with a resolved sender key, or a verified signature), every sender key it used belongs to the `from` DID, and either the frame is authcrypt-encrypted or, when a signature is the only proof, every recipient key of its encrypted envelope belongs to a DID named in `to`. **Act on `from` only when this is `true`.** |
| `encryptedFromKid` | `String?` | Key ID (`did#fragment`) of the authcrypt sender key; `null` for anoncrypt, signed-only or plaintext frames, so it can be `null` on an authenticated message. |
| `signFrom` | `String?` | Key ID of the verified signature; `null` when the frame was not signed. On a frame that is not authcrypt-encrypted, this is the key that proves `from`. `pack` signs with the sender DID, so a Navia frame carries both kids. |
| `anonymousSender` | `Boolean` / `Bool` | `true` when the frame arrived in an anoncrypt envelope (anoncrypt alone, or anoncrypt wrapping authcrypt). It says nothing about authentication; read `authenticated` for that. |

`authenticated` is `false` for:

- an unsigned anoncrypt frame or a plaintext frame (no sender key);
- a frame without `from`, such as navia-messaging's own forward wrapper,
  which a mediator still sees with the sender's `encryptedFromKid`;
- a frame whose signature verifies but whose signer kid names another DID
  than `from`, and that is not authcrypt-encrypted;
- a signed frame that is not authcrypt-encrypted and was either never
  encrypted or encrypted to a recipient key whose DID its `to` does not name.
  A signature stays valid after anyone re-encrypts the signed message, so a
  message Alice signed for Mallory that Mallory re-encrypts to you comes back
  with `from` = Alice, `to` = Mallory, `encryptedFromKid = null` and
  `authenticated = false`. navia-didcomm lists every recipient key of the
  envelope, not only yours, so every one of them has to belong to a DID in
  `to`: a relay that adds its own key next to yours does not pass.

`unpack` refuses an authcrypt frame whose `from` names another DID than
`encryptedFromKid` outright (see [unpack](#unpack)). Authcrypt needs no `to`
check: only the holder of the sender's key can encrypt from that key to
yours.

The key that proves `from` on an authenticated message is `encryptedFromKid`
when it is set and `signFrom` otherwise. Genuine Navia traffic is authcrypt,
so a consumer that wants only authcrypt proof checks
`authenticated && encryptedFromKid != null`. `authenticated` says nothing
about freshness: a genuine frame can arrive twice, so deduplicate by `id`.

### KeyValue

Used for batch storage operations.

```rust
pub struct KeyValue {
    pub key: String,              // Unique identifier
    pub value: String,            // Data to store
    pub metadata: Option<String>, // Optional metadata
}
```

**Usage Example:**
```kotlin
val contact = KeyValue(
    key = "did:peer:alice123",
    value = """{"name": "Alice", "email": "alice@example.com"}""",
    metadata = "verified:2024-01-01"
)
```

### DidCommError

Comprehensive error types for all operations.

```rust
pub enum DidCommError {
    ValidationError { message: String },
    DatabaseError { message: String },
    PackingError { message: String },
    UnpackingError { message: String },
    GeneralError { message: String },
}
```

## Main Interface

### DidComInterface

The primary entry point for all Navia operations.

#### Constructor

```kotlin
val interface = DidComInterface(path: String)
```

- **path**: Initial database path (can be empty string)
- **Returns**: New interface instance

#### open

Initialize the database with a secure seed.

```kotlin
suspend fun open(path: String, seed: List<UByte>)
```

- **path**: Database file path (use `:memory:` for in-memory)
- **seed**: 32-byte seed for key derivation
- **Throws**: `DidCommError.ValidationError` if seed is invalid
- **Throws**: `DidCommError.DatabaseError` if initialization fails

**Example:**
```kotlin
val seed = generateSecure32ByteSeed()
interface.open("/data/navia.db", seed)
```

#### generateDid

Generate a new peer DID with optional service endpoint.

```kotlin
suspend fun generateDid(
    uri: String, 
    routingKeys: List<String>
): String
```

- **uri**: Service endpoint URI for DIDComm
- **routingKeys**: Optional routing keys for message forwarding
- **Returns**: Generated DID string
- **Throws**: `DidCommError.ValidationError` if URI is invalid

**Example:**
```kotlin
val did = interface.generateDid(
    "https://example.com/didcomm",
    emptyList()
)
```

#### pack

Encrypt a message for secure transmission.

```kotlin
suspend fun pack(
    msg: DIDCommMessage,
    from: String,
    to: String
): String
```

- **msg**: Message to encrypt
- **from**: Sender's DID
- **to**: Recipient's DID
- **Returns**: Encrypted message as JWE string
- **Throws**: `DidCommError.PackingError` if encryption fails

**Example:**
```kotlin
val encrypted = interface.pack(message, senderDid, recipientDid)
```

#### unpack

Decrypt a received message.

```kotlin
suspend fun unpack(msg: String): DIDCommMessage
```

- **msg**: Encrypted JWE message
- **Returns**: Decrypted message, with the [sender authentication fields](#sender-authentication-fields) filled in from navia-didcomm's unpack metadata
- **Throws**: `DidCommError.UnpackingError` if the frame can never be unpacked (malformed, not for this store's keys, unsupported crypto), or if an authcrypt frame's plaintext `from` names another DID than the DID part of `encryptedFromKid` (a forged `from`; message `Sender mismatch: ...`, which names neither DID). The sender check is permanent: a redelivery fails the same way, so acknowledge the frame and drop it. It runs only when `encryptedFromKid` is set. An anoncrypt frame carries no authcrypt sender key: unsigned, it comes back with `authenticated = false`; signed, `authenticated` follows the signature and `to` (see [Sender authentication fields](#sender-authentication-fields)). `DidCommError.DatabaseError` if the store or DID resolution failed (retry later). navia-didcomm 1.3.0 still reports some faults of the frame itself as `DatabaseError`: truncated JSON in the envelope or protected header (an empty frame included), a wrong skid, an anoncrypt/authcrypt recipient mismatch, a JWS signature kid that does not match, or a sender kid missing from its DID document. Those fail the same way on every retry, so cap redeliveries per frame (the stored payload with the mediator's `delivery_id` left out; the mediator mints a new `delivery_id` for every delivery) rather than retrying a `DatabaseError` forever.

**Example:**
```kotlin
val message = interface.unpack(encryptedMessage)
println("Received: ${message.body}")

// Act on `from` only when it is proven
val sender = message.from?.takeIf { message.authenticated }
```

#### Storage Operations

##### insert

Store a single key-value pair.

```kotlin
suspend fun insert(
    category: String,
    name: String,
    value: String
)
```

- **category**: Storage category/namespace
- **name**: Unique key within category
- **value**: Data to store

##### get

Retrieve a single value.

```kotlin
suspend fun get(
    category: String,
    name: String
): String
```

- **Returns**: Stored value or empty string if not found

##### update

Update an existing value.

```kotlin
suspend fun update(
    category: String,
    name: String,
    value: String
)
```

##### remove

Delete a key-value pair.

```kotlin
suspend fun remove(
    category: String,
    name: String
)
```

##### insertBatch

Store multiple items efficiently.

```kotlin
suspend fun insertBatch(
    category: String,
    items: List<KeyValue>
)
```

**Example:**
```kotlin
val contacts = listOf(
    KeyValue("alice", """{"name": "Alice"}""", null),
    KeyValue("bob", """{"name": "Bob"}""", "verified")
)
interface.insertBatch("contacts", contacts)
```

##### getBatch

Retrieve multiple items efficiently.

```kotlin
suspend fun getBatch(
    category: String,
    keys: List<String>
): List<KeyValue>
```

- **Returns**: List of KeyValue objects (only found items)

#### Health and Diagnostics

##### isHealthy

Check system health status.

```kotlin
suspend fun isHealthy(): Boolean
```

- **Returns**: true if system is functioning properly

##### exportErrorLogs

Export error logs for debugging.

```kotlin
fun exportErrorLogs(): String
```

- **Returns**: JSON string containing error logs

##### clearErrorLogs

Clear all error logs.

```kotlin
fun clearErrorLogs()
```

## Common Patterns

### 1. Message Exchange Pattern

```kotlin
// Alice sends to Bob
class MessageExchange {
    suspend fun sendMessage(content: String) {
        // 1. Create structured message
        val message = DIDCommMessage(
            id = generateMessageId(),
            msgType = "https://example.org/chat/1.0/message",
            body = createMessageBody(content),
            from = aliceDid,
            to = listOf(bobDid)
        )
        
        // 2. Pack (encrypt)
        val encrypted = interface.pack(message, aliceDid, bobDid)
        
        // 3. Send over transport
        sendOverNetwork(encrypted)
    }
    
    suspend fun receiveMessage(encrypted: String) {
        // 1. Unpack (decrypt)
        val message = interface.unpack(encrypted)
        
        // 2. Validate sender: `from` is proven only when `authenticated`
        val sender = requireNotNull(message.from) { "Anonymous messages not allowed" }
        require(message.authenticated) { "Sender not authenticated" }
        require(isKnownContact(sender)) { "Unknown sender" }
        
        // 3. Process by type
        when (message.msgType) {
            "https://example.org/chat/1.0/message" -> handleChatMessage(message)
            else -> handleUnknownType(message)
        }
    }
}
```

### 2. Connection Management Pattern

```kotlin
class ConnectionManager {
    private val interface = MyApp.navia
    
    suspend fun saveConnection(theirDid: String, metadata: ConnectionMetadata) {
        val data = Json.encodeToString(metadata)
        interface.insert("connections", theirDid, data)
    }
    
    suspend fun loadConnections(): List<Connection> {
        // Get all connection DIDs from another source
        val dids = getStoredConnectionDids()
        
        // Batch load
        val results = interface.getBatch("connections", dids)
        
        return results.map { kv ->
            Connection(
                did = kv.key,
                metadata = Json.decodeFromString(kv.value)
            )
        }
    }
}
```

### 3. Secure Credential Storage Pattern

```kotlin
class CredentialStore {
    suspend fun storeCredential(credential: Credential) {
        // Store credential data
        interface.insert(
            category = "credentials",
            name = credential.id,
            value = credential.toJson()
        )
        
        // Store index for quick lookup
        interface.insert(
            category = "credential_index",
            name = credential.type,
            value = credential.id
        )
    }
    
    suspend fun findCredentialsByType(type: String): List<Credential> {
        // Get credential IDs of this type
        val indexData = interface.get("credential_index", type)
        val credentialIds = indexData.split(",")
        
        // Batch load credentials
        val results = interface.getBatch("credentials", credentialIds)
        
        return results.map { kv ->
            Credential.fromJson(kv.value)
        }
    }
}
```

### 4. Message Threading Pattern

```kotlin
class ThreadedMessaging {
    suspend fun replyToMessage(
        originalMessage: DIDCommMessage,
        replyContent: String
    ) {
        val reply = DIDCommMessage(
            id = UUID.randomUUID().toString(),
            msgType = originalMessage.msgType,
            body = createReplyBody(
                content = replyContent,
                threadId = originalMessage.id,  // Reference original
                inReplyTo = originalMessage.id
            ),
            from = myDid,
            to = listOf(originalMessage.from ?: throw IllegalStateException())
        )
        
        val encrypted = interface.pack(reply, myDid, originalMessage.from!!)
        sendMessage(encrypted)
    }
}
```

## Best Practices

### 1. Seed Management

**DO:**
- Use hardware-backed key storage (Android Keystore, iOS Keychain)
- Generate seeds using secure random sources
- Derive seeds deterministically from user credentials when needed

**DON'T:**
- Store seeds in SharedPreferences or files
- Use predictable seeds
- Log or display seeds

```kotlin
// Good: Hardware-backed seed
fun getSecureSeed(): List<UByte> {
    val keyAlias = "NaviaSeed"
    val keyStore = KeyStore.getInstance("AndroidKeyStore")
    keyStore.load(null)
    
    if (!keyStore.containsAlias(keyAlias)) {
        generateHardwareBackedKey(keyAlias)
    }
    
    return deriveFixedSeedFromKey(keyAlias)
}
```

### 2. Message Size Limits

**DO:**
- Keep message bodies under 64KB
- Use external storage for large payloads
- Compress data when appropriate

**DON'T:**
- Embed base64 images in messages
- Send entire documents inline

```kotlin
// Good: Reference pattern for large data
val message = DIDCommMessage(
    msgType = "https://example.org/data-share/1.0/notification",
    body = """{
        "data_ref": "https://storage.example.com/doc/12345",
        "size_bytes": 1048576,
        "hash": "sha256:abcd..."
    }"""
)
```

### 3. Error Recovery

**DO:**
- Implement retry logic for transient failures
- Log errors for debugging
- Provide user-friendly error messages

**DON'T:**
- Ignore errors
- Retry indefinitely
- Expose internal error details to users

```kotlin
// Good: Retry with backoff
suspend fun reliablePack(
    message: DIDCommMessage,
    from: String,
    to: String,
    maxRetries: Int = 3
): String {
    var lastError: Exception? = null
    
    repeat(maxRetries) { attempt ->
        try {
            return interface.pack(message, from, to)
        } catch (e: DidCommError.DatabaseError) {
            lastError = e
            delay(100L * (attempt + 1)) // Exponential backoff
        }
    }
    
    throw lastError ?: IllegalStateException("Pack failed")
}
```

### 4. Batch Operations

**DO:**
- Use batch operations for multiple items
- Group related operations
- Consider transaction boundaries

**DON'T:**
- Loop individual operations
- Mix unrelated data in batches

```kotlin
// Good: Efficient batch update
suspend fun updateMultipleContacts(updates: Map<String, Contact>) {
    val batch = updates.map { (did, contact) ->
        KeyValue(
            key = did,
            value = contact.toJson(),
            metadata = "updated:${Instant.now()}"
        )
    }
    interface.insertBatch("contacts", batch)
}

// Bad: Inefficient loop
suspend fun updateMultipleContactsBad(updates: Map<String, Contact>) {
    updates.forEach { (did, contact) ->
        interface.update("contacts", did, contact.toJson())
    }
}
```

### 5. Category Naming

**DO:**
- Use consistent, descriptive category names
- Follow a naming convention
- Document category purposes

**DON'T:**
- Use spaces or special characters
- Create too many categories
- Mix different data types in one category

```kotlin
// Good: Clear category structure
object Categories {
    const val CONNECTIONS = "connections"      // DID -> Connection metadata
    const val MESSAGES = "messages"           // Message ID -> Message
    const val CREDENTIALS = "credentials"     // Credential ID -> Credential
    const val SETTINGS = "settings"          // Setting key -> Value
}
```

### 6. Concurrent Access

**DO:**
- Use a single DidComInterface instance
- Implement proper synchronization
- Handle concurrent operations gracefully

**DON'T:**
- Create multiple instances
- Assume operations are atomic
- Ignore threading issues

```kotlin
// Good: Singleton pattern
object NaviaManager {
    private var _interface: DidComInterface? = null
    private val lock = Mutex()
    
    suspend fun getInstance(): DidComInterface {
        return lock.withLock {
            _interface ?: run {
                val new = DidComInterface("")
                new.open(getDatabasePath(), getSecureSeed())
                _interface = new
                new
            }
        }
    }
}
```

### 7. Testing

**DO:**
- Use in-memory database for tests
- Test error conditions
- Verify encryption/decryption roundtrips

```kotlin
@Test
fun testMessageRoundtrip() = runTest {
    val interface = DidComInterface("")
    interface.open(":memory:", List(32) { 42u })
    
    val alice = interface.generateDid("https://alice.test", emptyList())
    val bob = interface.generateDid("https://bob.test", emptyList())
    
    val original = DIDCommMessage(
        id = "test-1",
        msgType = "test/1.0",
        body = """{"test": true}""",
        from = alice,
        to = listOf(bob)
    )
    
    val encrypted = interface.pack(original, alice, bob)
    val decrypted = interface.unpack(encrypted)
    
    assertEquals(original.body, decrypted.body)
    assertEquals(original.from, decrypted.from)
}
```