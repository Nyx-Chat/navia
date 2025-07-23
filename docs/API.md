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
)
```

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
- **Returns**: Decrypted message
- **Throws**: `DidCommError.UnpackingError` if decryption fails

**Example:**
```kotlin
val message = interface.unpack(encryptedMessage)
println("Received: ${message.body}")
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
        
        // 2. Validate sender
        requireNotNull(message.from) { "Anonymous messages not allowed" }
        require(isKnownContact(message.from)) { "Unknown sender" }
        
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