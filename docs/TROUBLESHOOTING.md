# Navia Troubleshooting Guide

This guide helps you diagnose and resolve common issues when integrating Navia into mobile applications.

## Table of Contents

- [Installation Issues](#installation-issues)
- [Runtime Errors](#runtime-errors)
- [Performance Problems](#performance-problems)
- [Platform-Specific Issues](#platform-specific-issues)
- [Debugging Techniques](#debugging-techniques)
- [Common Error Messages](#common-error-messages)

## Installation Issues

### Android: "Couldn't find navia_core.so"

**Symptoms:**
```
java.lang.UnsatisfiedLinkError: Couldn't find "libnavia_core.so"
```

**Causes:**
- Missing native libraries for device architecture
- Incorrect AAR packaging
- ProGuard stripping native libraries

**Solutions:**

1. Check included architectures:
```bash
unzip -l app.apk | grep libnavia_core.so
```

2. Ensure all architectures are built:
```bash
./build-for-android.sh --package
```

3. Update ProGuard rules:
```proguard
-keep class uniffi.** { *; }
-keep class com.sun.jna.** { *; }
-keep class * implements com.sun.jna.** { *; }
```

4. Force specific ABI in build.gradle:
```kotlin
android {
    defaultConfig {
        ndk {
            abiFilters 'arm64-v8a', 'armeabi-v7a', 'x86_64'
        }
    }
}
```

### "Failed to resolve: com.nyx:navia:x.x.x"

**Causes:**
- GitHub authentication not configured
- Network connectivity issues
- Package not published

**Solutions:**

1. Check authentication in `local.properties`:
```properties
gpr.user=your-github-username
gpr.token=ghp_xxxxxxxxxxxxxxxxxxxx
```

2. Verify token permissions:
- Needs `read:packages` scope
- Must have access to the repository

3. Test direct download:
```bash
curl -H "Authorization: token YOUR_TOKEN" \
  https://maven.pkg.github.com/Nyx-Chat/navia/com/nyx/navia/1.5.0/navia-1.5.0.aar
```

## Runtime Errors

### Database Initialization Failures

#### "Not initialized"

**Error:**
```
DidCommError.GeneralError: Not initialized
```

**Cause:** Attempting operations before calling `open()`

**Solution:**
```kotlin
// Always initialize first
val interface = DidComInterface("")
interface.open(dbPath, seed) // Required!

// Now you can use other methods
val did = interface.generateDid(uri, keys)
```

#### "Seed must be at least 32 bytes"

**Error:**
```
DidCommError.ValidationError: Seed must be at least 32 bytes
```

**Solution:**
```kotlin
// Bad: Too short
val seed = "short".toByteArray().map { it.toUByte() }

// Good: Exactly 32 bytes
val seed = List(32) { Random.nextInt(256).toUByte() }

// Good: Derive from secure source
val seed = deriveFromKeystore().take(32).map { it.toUByte() }
```

#### Database Lock Errors

**Error:**
```
DidCommError.DatabaseError: database is locked
```

**Causes:**
- Multiple DidComInterface instances
- Database not properly closed
- Concurrent access without synchronization

**Solutions:**

1. Use singleton pattern:
```kotlin
object NaviaManager {
    val interface by lazy {
        DidComInterface("").also { navia ->
            runBlocking {
                navia.open(getDatabasePath(), getSecureSeed())
            }
        }
    }
}
```

2. Ensure proper cleanup:
```kotlin
override fun onDestroy() {
    super.onDestroy()
    // Database auto-closes, but clear error logs
    lifecycleScope.launch {
        interface.clearErrorLogs()
    }
}
```

### Message Packing/Unpacking Errors

#### "Recipient 'to' cannot be empty"

**Error:**
```
DidCommError.ValidationError: Recipient 'to' cannot be empty
```

**Solution:**
```kotlin
// Bad: Empty recipient
val packed = interface.pack(message, myDid, "")

// Good: Valid recipient DID
val packed = interface.pack(message, myDid, recipientDid)
```

#### "Invalid message format: expected JWE"

**Error:**
```
DidCommError.UnpackingError: Invalid message format: expected JWE with dot separators
```

**Causes:**
- Attempting to unpack plaintext
- Corrupted message
- Wrong message format

**Solution:**
```kotlin
// Validate before unpacking
fun isEncryptedMessage(message: String): Boolean {
    return message.count { it == '.' } >= 4 // JWE has 5 parts
}

if (isEncryptedMessage(received)) {
    val unpacked = interface.unpack(received)
} else {
    // Handle plaintext or wrong format
}
```

#### "No suitable key found for decryption"

**Causes:**
- Message encrypted for different recipient
- Database doesn't contain required keys
- Using wrong database/seed

**Solution:**
```kotlin
// Ensure you're using the correct identity
val myDid = interface.get("identity", "current_did")
if (myDid.isEmpty()) {
    // Generate new identity
    val newDid = interface.generateDid(serviceEndpoint, emptyList())
    interface.insert("identity", "current_did", newDid)
}
```

## Performance Problems

### Slow Operations

#### Slow Database Operations

**Symptoms:**
- Insert/get operations take > 100ms
- UI freezes during operations

**Solutions:**

1. Use batch operations:
```kotlin
// Slow: Individual inserts
contacts.forEach { contact ->
    interface.insert("contacts", contact.did, contact.toJson())
}

// Fast: Batch insert
val batch = contacts.map { contact ->
    KeyValue(contact.did, contact.toJson(), null)
}
interface.insertBatch("contacts", batch)
```

2. Run on IO dispatcher:
```kotlin
viewModelScope.launch(Dispatchers.IO) {
    val result = interface.get("data", "key")
    withContext(Dispatchers.Main) {
        updateUI(result)
    }
}
```

3. Optimize database location:
```kotlin
// Use internal storage for better performance
val dbPath = context.filesDir.resolve("navia/navia.db")
    .apply { parentFile?.mkdirs() }
    .absolutePath
```

#### Memory Issues

**Symptoms:**
- OutOfMemoryError with large messages
- App crashes when processing many messages

**Solutions:**

1. Limit message size:
```kotlin
const val MAX_MESSAGE_SIZE = 64 * 1024 // 64KB

fun validateMessageSize(body: String) {
    require(body.toByteArray().size <= MAX_MESSAGE_SIZE) {
        "Message too large"
    }
}
```

2. Process messages in chunks:
```kotlin
fun processLargeMessageBatch(messages: List<String>) {
    messages.chunked(100).forEach { chunk ->
        processBatch(chunk)
        System.gc() // Hint to collect garbage
    }
}
```

## Platform-Specific Issues

### Android

#### "Bad notification for startForeground"

**Cause:** Background DIDComm operations without proper foreground service

**Solution:**
```kotlin
class DIDCommService : Service() {
    override fun onCreate() {
        super.onCreate()
        
        val notification = NotificationCompat.Builder(this, CHANNEL_ID)
            .setContentTitle("Secure Messaging")
            .setContentText("Processing messages...")
            .setSmallIcon(R.drawable.ic_notification)
            .build()
            
        startForeground(NOTIFICATION_ID, notification)
    }
}
```

#### Android 15+ Compatibility

**Issue:** 16KB page size requirement

**Solution:** Already handled in Navia v1.0.8+, but ensure:
```kotlin
android {
    defaultConfig {
        minSdk = 24 // Minimum supported
        targetSdk = 34 // Latest tested
    }
}
```

### iOS (Future)

#### Keychain Access Issues

**Solution pattern:**
```swift
// Store seed in Keychain
let seed = generateSecureSeed()
KeychainWrapper.standard.set(seed, forKey: "NaviaSeed")
```

## Debugging Techniques

### 1. Enable Verbose Logging

```kotlin
// In debug builds
if (BuildConfig.DEBUG) {
    // Log all operations
    interface.interceptor = { operation, params ->
        Log.d("Navia", "$operation: $params")
    }
}
```

### 2. Export Error Logs

```kotlin
suspend fun debugSystemState() {
    val isHealthy = interface.isHealthy()
    val errorLogs = interface.exportErrorLogs()
    
    Log.d("NaviaDebug", """
        System healthy: $isHealthy
        Error logs: $errorLogs
    """.trimIndent())
    
    // Send to crash reporting
    FirebaseCrashlytics.getInstance().log(errorLogs)
}
```

### 3. Test with In-Memory Database

```kotlin
@Test
fun debugOperation() = runTest {
    val interface = DidComInterface("")
    interface.open(":memory:", testSeed) // In-memory for testing
    
    // Reproduce issue...
}
```

### 4. Message Inspection

```kotlin
fun debugMessage(encrypted: String) {
    // Check structure
    val parts = encrypted.split(".")
    Log.d("Debug", "JWE parts: ${parts.size}")
    
    // Try to decode header (first part)
    try {
        val header = String(Base64.decode(parts[0], Base64.URL_SAFE))
        Log.d("Debug", "Header: $header")
    } catch (e: Exception) {
        Log.e("Debug", "Invalid JWE header", e)
    }
}
```

## Common Error Messages

### Quick Reference

| Error | Cause | Solution |
|-------|-------|----------|
| "Not initialized" | `open()` not called | Initialize before use |
| "Seed must be at least 32 bytes" | Short seed | Use 32-byte seed |
| "database is locked" | Multiple instances | Use singleton |
| "Invalid service endpoint" | Malformed URI | Use valid HTTPS URL |
| "Recipient 'to' cannot be empty" | Missing recipient | Provide recipient DID |
| "No suitable key found" | Wrong recipient | Check DIDs match |
| "Message too large" | >1MB message | Reduce size or use references |

### Error Recovery Patterns

```kotlin
class ErrorHandler {
    suspend fun <T> withRetry(
        times: Int = 3,
        delay: Long = 1000,
        block: suspend () -> T
    ): T {
        repeat(times - 1) {
            try {
                return block()
            } catch (e: DidCommError.DatabaseError) {
                delay(delay * (it + 1))
            }
        }
        return block() // Last attempt
    }
    
    fun handleError(error: DidCommError): String {
        return when (error) {
            is DidCommError.ValidationError -> {
                "Please check your input: ${error.message}"
            }
            is DidCommError.DatabaseError -> {
                "Storage error. Please try again."
            }
            is DidCommError.PackingError -> {
                "Failed to encrypt message. Check recipient address."
            }
            is DidCommError.UnpackingError -> {
                "Failed to decrypt message. It may be corrupted."
            }
            is DidCommError.GeneralError -> {
                "An error occurred: ${error.message}"
            }
        }
    }
}
```

## Getting Help

If you're still experiencing issues:

1. **Check logs**: Use `interface.exportErrorLogs()`
2. **Minimal reproduction**: Create a simple test case
3. **System info**: Include Android version, device, Navia version
4. **Report issue**: https://github.com/Nyx-Chat/navia/issues

### Debug Information Template

```kotlin
fun gatherDebugInfo(): String {
    return """
        Navia Version: 1.5.0
        Android Version: ${Build.VERSION.RELEASE}
        Device: ${Build.MANUFACTURER} ${Build.MODEL}
        ABI: ${Build.SUPPORTED_ABIS.joinToString()}
        
        Error Logs:
        ${interface.exportErrorLogs()}
    """.trimIndent()
}
```