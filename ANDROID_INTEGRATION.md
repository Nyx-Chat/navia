# Android Integration Guide

This guide walks you through integrating Navia into your Android application using Kotlin.

## Prerequisites

- Android Studio Arctic Fox or later
- Minimum SDK: API 24 (Android 7.0)
- Target SDK: API 34 (Android 14) or later
- Kotlin 1.8.0 or later
- Gradle 8.0 or later

## Installation

### 1. Add Navia Dependency

Add the Navia library to your module's `build.gradle.kts`:

```kotlin
dependencies {
    implementation("com.navia:navia-android:1.1.2")
}
```

### 2. Configure ProGuard/R8

Add the following rules to your `proguard-rules.pro`:

```proguard
# Keep Navia classes
-keep class com.navia.** { *; }
-keep class uniffi.navia_core.** { *; }

# Keep native methods
-keepclasseswithmembernames class * {
    native <methods>;
}
```

### 3. Add Required Permissions

In your `AndroidManifest.xml`:

```xml
<!-- For network operations (DIDComm messaging) -->
<uses-permission android:name="android.permission.INTERNET" />

<!-- For secure storage (optional, for additional security) -->
<uses-permission android:name="android.permission.USE_BIOMETRIC" />
```

## Basic Setup

### 1. Initialize Navia

Create a singleton instance in your Application class:

```kotlin
class MyApplication : Application() {
    companion object {
        lateinit var navia: DidComInterface
            private set
    }

    override fun onCreate() {
        super.onCreate()
        
        // Initialize Navia
        navia = DidComInterface("")
        
        // Open database with secure seed
        lifecycleScope.launch {
            try {
                val seed = generateSecureSeed()
                val dbPath = getDatabasePath("navia.db").absolutePath
                navia.open(dbPath, seed)
            } catch (e: DidCommError) {
                Log.e("Navia", "Failed to initialize", e)
            }
        }
    }
    
    private fun generateSecureSeed(): List<UByte> {
        // Generate secure 32-byte seed using Android Keystore
        val keyAlias = "NaviaSeed"
        val keyGenerator = KeyGenerator.getInstance(
            KeyProperties.KEY_ALGORITHM_AES,
            "AndroidKeyStore"
        )
        
        val keyGenParams = KeyGenParameterSpec.Builder(
            keyAlias,
            KeyProperties.PURPOSE_ENCRYPT or KeyProperties.PURPOSE_DECRYPT
        )
            .setBlockModes(KeyProperties.BLOCK_MODE_GCM)
            .setEncryptionPaddings(KeyProperties.ENCRYPTION_PADDING_NONE)
            .setKeySize(256)
            .build()
            
        keyGenerator.init(keyGenParams)
        val secretKey = keyGenerator.generateKey()
        
        // Use the key to derive a seed
        val cipher = Cipher.getInstance("AES/GCM/NoPadding")
        cipher.init(Cipher.ENCRYPT_MODE, secretKey)
        val iv = cipher.iv
        val seedData = "navia_seed_v1".toByteArray()
        val encrypted = cipher.doFinal(seedData)
        
        // Combine IV and encrypted data to create 32-byte seed
        return (iv + encrypted).take(32).map { it.toUByte() }
    }
}
```

### 2. Generate DID

```kotlin
class ProfileViewModel : ViewModel() {
    private val navia = MyApplication.navia
    
    suspend fun createIdentity(): String {
        return try {
            val serviceEndpoint = "https://example.com/didcomm"
            val routingKeys = emptyList<String>()
            
            val did = navia.generateDid(serviceEndpoint, routingKeys)
            
            // Store DID in preferences
            saveUserDid(did)
            
            did
        } catch (e: DidCommError.ValidationError) {
            throw IllegalArgumentException("Invalid service endpoint: ${e.message}")
        }
    }
}
```

### 3. Send Encrypted Message

```kotlin
class MessagingViewModel : ViewModel() {
    private val navia = MyApplication.navia
    
    suspend fun sendMessage(
        recipientDid: String,
        content: String
    ): String {
        val myDid = getUserDid() ?: throw IllegalStateException("No identity")
        
        val message = DIDCommMessage(
            id = UUID.randomUUID().toString(),
            msgType = "https://didcomm.org/basicmessage/2.0/message",
            body = """{"content": "$content", "sent_time": "${Instant.now()}"}""",
            from = myDid,
            to = listOf(recipientDid)
        )
        
        return try {
            navia.pack(message, myDid, recipientDid)
        } catch (e: DidCommError.PackingError) {
            throw IOException("Failed to encrypt message: ${e.message}")
        }
    }
}
```

### 4. Receive and Decrypt Message

```kotlin
class MessageReceiver {
    private val navia = MyApplication.navia
    
    suspend fun processIncomingMessage(encryptedMessage: String): DIDCommMessage {
        return try {
            val decrypted = navia.unpack(encryptedMessage)
            
            // Verify sender if needed
            decrypted.from?.let { senderDid ->
                if (!isTrustedSender(senderDid)) {
                    throw SecurityException("Untrusted sender")
                }
            }
            
            // Process the message
            handleMessage(decrypted)
            
            decrypted
        } catch (e: DidCommError.UnpackingError) {
            throw SecurityException("Failed to decrypt: ${e.message}")
        }
    }
    
    private fun handleMessage(message: DIDCommMessage) {
        when (message.msgType) {
            "https://didcomm.org/basicmessage/2.0/message" -> {
                // Handle basic message
                val content = JSONObject(message.body).getString("content")
                displayMessage(content)
            }
            else -> {
                Log.w("MessageReceiver", "Unknown message type: ${message.msgType}")
            }
        }
    }
}
```

## Advanced Features

### 1. Batch Storage Operations

```kotlin
class ContactsRepository {
    private val navia = MyApplication.navia
    
    suspend fun importContacts(contacts: List<Contact>) {
        val keyValues = contacts.map { contact ->
            KeyValue(
                key = contact.did,
                value = Json.encodeToString(contact),
                metadata = "imported:${Instant.now()}"
            )
        }
        
        navia.insertBatch("contacts", keyValues)
    }
    
    suspend fun getContacts(dids: List<String>): List<Contact> {
        val results = navia.getBatch("contacts", dids)
        
        return results.mapNotNull { kv ->
            try {
                Json.decodeFromString<Contact>(kv.value)
            } catch (e: Exception) {
                Log.e("Contacts", "Failed to parse contact: ${kv.key}")
                null
            }
        }
    }
}
```

### 2. Error Handling

```kotlin
class NaviaErrorHandler {
    fun handleError(error: DidCommError): String {
        return when (error) {
            is DidCommError.ValidationError -> {
                "Invalid input: ${error.message}"
            }
            is DidCommError.DatabaseError -> {
                "Storage error. Please try again."
            }
            is DidCommError.PackingError -> {
                "Failed to encrypt message. Check recipient DID."
            }
            is DidCommError.UnpackingError -> {
                "Failed to decrypt message. Message may be corrupted."
            }
            is DidCommError.GeneralError -> {
                "An error occurred: ${error.message}"
            }
        }
    }
}
```

### 3. Health Monitoring

```kotlin
class HealthMonitor(private val scope: CoroutineScope) {
    private val navia = MyApplication.navia
    
    fun startMonitoring() {
        scope.launch {
            while (isActive) {
                try {
                    val isHealthy = navia.isHealthy()
                    if (!isHealthy) {
                        handleUnhealthyState()
                    }
                } catch (e: Exception) {
                    Log.e("HealthMonitor", "Health check failed", e)
                }
                
                delay(30_000) // Check every 30 seconds
            }
        }
    }
    
    private suspend fun handleUnhealthyState() {
        // Export error logs
        val errorLogs = navia.exportErrorLogs()
        Log.e("HealthMonitor", "System unhealthy. Logs: $errorLogs")
        
        // Attempt recovery
        reinitializeNavia()
    }
}
```

## Best Practices

### 1. Secure Seed Management

Always use Android Keystore for seed generation and storage:

```kotlin
object SecureSeedManager {
    private const val KEY_ALIAS = "NaviaSeedKey"
    
    fun getOrCreateSeed(): List<UByte> {
        val keyStore = KeyStore.getInstance("AndroidKeyStore").apply { load(null) }
        
        if (!keyStore.containsAlias(KEY_ALIAS)) {
            generateNewKey()
        }
        
        return deriveSeedFromKey()
    }
    
    private fun generateNewKey() {
        val keyGenerator = KeyGenerator.getInstance(
            KeyProperties.KEY_ALGORITHM_AES,
            "AndroidKeyStore"
        )
        
        val spec = KeyGenParameterSpec.Builder(
            KEY_ALIAS,
            KeyProperties.PURPOSE_ENCRYPT or KeyProperties.PURPOSE_DECRYPT
        )
            .setBlockModes(KeyProperties.BLOCK_MODE_GCM)
            .setEncryptionPaddings(KeyProperties.ENCRYPTION_PADDING_NONE)
            .setUserAuthenticationRequired(true)
            .setUserAuthenticationValidityDurationSeconds(300)
            .build()
            
        keyGenerator.init(spec)
        keyGenerator.generateKey()
    }
}
```

### 2. Lifecycle Management

Properly manage Navia lifecycle with Android components:

```kotlin
class MainActivity : AppCompatActivity() {
    override fun onDestroy() {
        super.onDestroy()
        
        if (isFinishing) {
            // Clear sensitive data
            lifecycleScope.launch {
                MyApplication.navia.clearErrorLogs()
            }
        }
    }
}
```

### 3. Background Processing

Use WorkManager for background DIDComm operations:

```kotlin
class MessageSyncWorker(
    context: Context,
    params: WorkerParameters
) : CoroutineWorker(context, params) {
    
    override suspend fun doWork(): Result {
        return try {
            val navia = MyApplication.navia
            
            // Check for pending messages
            val pendingMessages = getPendingMessages()
            
            // Process each message
            pendingMessages.forEach { encrypted ->
                val decrypted = navia.unpack(encrypted)
                processMessage(decrypted)
            }
            
            Result.success()
        } catch (e: Exception) {
            Log.e("MessageSync", "Sync failed", e)
            Result.retry()
        }
    }
}
```

## Performance Optimization

### 1. Use Batch Operations

```kotlin
// Bad: Multiple individual operations
messages.forEach { msg ->
    navia.insert("messages", msg.id, msg.toJson())
}

// Good: Single batch operation
val batch = messages.map { msg ->
    KeyValue(msg.id, msg.toJson(), null)
}
navia.insertBatch("messages", batch)
```

### 2. Cache DIDs

```kotlin
object DidCache {
    private val cache = LruCache<String, String>(100)
    
    suspend fun getDid(endpoint: String): String {
        return cache.get(endpoint) ?: run {
            val did = MyApplication.navia.generateDid(endpoint, emptyList())
            cache.put(endpoint, did)
            did
        }
    }
}
```

### 3. Optimize Database Path

Place the database in the app's private directory:

```kotlin
val dbPath = context.filesDir.resolve("navia/navia.db").apply {
    parentFile?.mkdirs()
}.absolutePath
```

## Troubleshooting

### Common Issues

1. **Database locked errors**
   - Ensure only one instance of DidComInterface exists
   - Use a singleton pattern

2. **Memory issues with large messages**
   - Implement streaming for large payloads
   - Set appropriate message size limits

3. **Slow performance**
   - Use batch operations
   - Enable R8 optimization
   - Profile with Android Studio

### Debug Logging

Enable detailed logging in debug builds:

```kotlin
if (BuildConfig.DEBUG) {
    // Log all Navia operations
    navia.setLogLevel(LogLevel.DEBUG)
}
```

## Testing

### Unit Tests

```kotlin
@Test
fun testMessageEncryption() = runTest {
    val navia = DidComInterface("")
    val seed = List(32) { 42.toUByte() }
    navia.open(":memory:", seed)
    
    val did = navia.generateDid("https://test.com", emptyList())
    
    val message = DIDCommMessage(
        id = "test-1",
        msgType = "test",
        body = "{}",
        from = did,
        to = listOf(did)
    )
    
    val encrypted = navia.pack(message, did, did)
    assertNotNull(encrypted)
    assertTrue(encrypted.length > 100)
}
```

### Integration Tests

```kotlin
@Test
fun testFullMessageFlow() = runTest {
    val sender = createTestUser("alice")
    val recipient = createTestUser("bob")
    
    val encrypted = sender.navia.pack(
        createMessage("Hello Bob"),
        sender.did,
        recipient.did
    )
    
    val decrypted = recipient.navia.unpack(encrypted)
    assertEquals("Hello Bob", parseMessageContent(decrypted))
}
```

## Migration Guide

If upgrading from an older version:

```kotlin
class NaviaMigration {
    suspend fun migrate(oldVersion: Int, newVersion: Int) {
        when (oldVersion) {
            1 -> migrateV1ToV2()
            2 -> migrateV2ToV3()
        }
    }
    
    private suspend fun migrateV1ToV2() {
        // Export data from old format
        val oldData = exportV1Data()
        
        // Reinitialize with new version
        MyApplication.navia = DidComInterface("")
        
        // Import to new format
        importV2Data(oldData)
    }
}
```

## Resources

- [Navia GitHub Repository](https://github.com/navia/navia)
- [DIDComm Specification](https://identity.foundation/didcomm-messaging/spec/)
- [Android Security Best Practices](https://developer.android.com/topic/security/best-practices)
- [Sample Android App](https://github.com/navia/navia-android-sample)