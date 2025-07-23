# Error Handling Patterns for Mobile Apps

This guide provides comprehensive error handling strategies for Navia integration in mobile applications.

## Table of Contents

- [Error Types Overview](#error-types-overview)
- [Android Error Handling](#android-error-handling)
- [iOS Error Handling](#ios-error-handling)
- [Error Recovery Strategies](#error-recovery-strategies)
- [User Experience Guidelines](#user-experience-guidelines)
- [Logging and Monitoring](#logging-and-monitoring)

## Error Types Overview

### Navia Error Hierarchy

```
DidCommError
├── ValidationError     // Invalid inputs, parameters
├── DatabaseError       // Storage layer issues
├── PackingError        // Encryption failures
├── UnpackingError      // Decryption failures
└── GeneralError        // Other errors
```

### Error Characteristics

| Error Type | Recoverable | User Action Required | Retry Strategy |
|------------|-------------|---------------------|----------------|
| ValidationError | No | Yes - Fix input | Don't retry |
| DatabaseError | Sometimes | No | Retry with backoff |
| PackingError | Sometimes | Yes - Check recipient | Retry once |
| UnpackingError | No | No | Don't retry |
| GeneralError | Depends | Depends | Case-by-case |

## Android Error Handling

### 1. Basic Error Handling Pattern

```kotlin
sealed class NaviaResult<out T> {
    data class Success<T>(val data: T) : NaviaResult<T>()
    data class Error(val error: DidCommError, val isRecoverable: Boolean) : NaviaResult<Nothing>()
}

class NaviaRepository {
    suspend fun generateDid(endpoint: String): NaviaResult<String> {
        return try {
            val did = interface.generateDid(endpoint, emptyList())
            NaviaResult.Success(did)
        } catch (e: DidCommError) {
            NaviaResult.Error(e, isRecoverable = false)
        }
    }
}
```

### 2. ViewModel Error Handling

```kotlin
class MessagingViewModel : ViewModel() {
    private val _uiState = MutableStateFlow(UiState())
    val uiState = _uiState.asStateFlow()
    
    fun sendMessage(content: String, recipientDid: String) {
        viewModelScope.launch {
            _uiState.update { it.copy(isLoading = true, error = null) }
            
            when (val result = repository.sendMessage(content, recipientDid)) {
                is NaviaResult.Success -> {
                    _uiState.update { 
                        it.copy(
                            isLoading = false,
                            lastMessageSent = true
                        )
                    }
                }
                is NaviaResult.Error -> {
                    handleError(result.error, result.isRecoverable)
                }
            }
        }
    }
    
    private fun handleError(error: DidCommError, isRecoverable: Boolean) {
        val errorMessage = when (error) {
            is DidCommError.ValidationError -> {
                "Invalid recipient address. Please check and try again."
            }
            is DidCommError.DatabaseError -> {
                if (isRecoverable) {
                    // Retry automatically
                    retryLastOperation()
                    return
                }
                "Storage error. Please restart the app."
            }
            is DidCommError.PackingError -> {
                "Cannot encrypt message. The recipient may not be available."
            }
            is DidCommError.UnpackingError -> {
                "Cannot decrypt message. It may be corrupted or not for you."
            }
            is DidCommError.GeneralError -> {
                "Something went wrong. Please try again."
            }
        }
        
        _uiState.update { 
            it.copy(
                isLoading = false,
                error = UserError(errorMessage, isRecoverable)
            )
        }
    }
}
```

### 3. Retry with Exponential Backoff

```kotlin
class RetryableOperation<T>(
    private val maxAttempts: Int = 3,
    private val initialDelay: Long = 1000,
    private val maxDelay: Long = 10000,
    private val factor: Double = 2.0
) {
    suspend fun execute(
        operation: suspend () -> T
    ): Result<T> {
        var currentDelay = initialDelay
        var lastException: Exception? = null
        
        repeat(maxAttempts) { attempt ->
            try {
                return Result.success(operation())
            } catch (e: DidCommError.DatabaseError) {
                lastException = e
                
                if (attempt < maxAttempts - 1) {
                    delay(currentDelay)
                    currentDelay = (currentDelay * factor).toLong()
                        .coerceAtMost(maxDelay)
                }
            } catch (e: Exception) {
                // Non-retryable error
                return Result.failure(e)
            }
        }
        
        return Result.failure(lastException ?: Exception("Unknown error"))
    }
}

// Usage
val retryable = RetryableOperation<String>()
val result = retryable.execute {
    interface.pack(message, from, to)
}
```

### 4. Circuit Breaker Pattern

```kotlin
class CircuitBreaker(
    private val failureThreshold: Int = 5,
    private val resetTimeout: Long = 60000 // 1 minute
) {
    private var failureCount = AtomicInteger(0)
    private var lastFailureTime = AtomicLong(0)
    private var state = AtomicReference(State.CLOSED)
    
    enum class State { CLOSED, OPEN, HALF_OPEN }
    
    suspend fun <T> execute(operation: suspend () -> T): T {
        when (state.get()) {
            State.OPEN -> {
                if (System.currentTimeMillis() - lastFailureTime.get() > resetTimeout) {
                    state.set(State.HALF_OPEN)
                } else {
                    throw CircuitBreakerOpenException()
                }
            }
            State.HALF_OPEN -> {
                return try {
                    val result = operation()
                    reset()
                    result
                } catch (e: Exception) {
                    fail()
                    throw e
                }
            }
            State.CLOSED -> {
                return try {
                    operation()
                } catch (e: Exception) {
                    fail()
                    throw e
                }
            }
        }
    }
    
    private fun fail() {
        lastFailureTime.set(System.currentTimeMillis())
        if (failureCount.incrementAndGet() >= failureThreshold) {
            state.set(State.OPEN)
        }
    }
    
    private fun reset() {
        failureCount.set(0)
        state.set(State.CLOSED)
    }
}
```

### 5. Comprehensive Error Handler

```kotlin
class NaviaErrorHandler(
    private val context: Context,
    private val logger: Logger,
    private val analytics: Analytics
) {
    fun handle(
        error: DidCommError,
        operation: String,
        canRetry: Boolean = true
    ): ErrorAction {
        // Log for debugging
        logger.error("Navia error in $operation", error)
        
        // Track in analytics
        analytics.trackError(
            category = "navia",
            action = operation,
            label = error.javaClass.simpleName
        )
        
        // Determine action
        return when (error) {
            is DidCommError.ValidationError -> {
                ErrorAction.ShowError(
                    message = getValidationMessage(error),
                    actionLabel = "Fix",
                    canDismiss = true
                )
            }
            
            is DidCommError.DatabaseError -> {
                if (canRetry) {
                    ErrorAction.Retry(
                        delay = 1000,
                        showLoading = true
                    )
                } else {
                    ErrorAction.Fatal(
                        message = "Database error. Please restart the app.",
                        requiresRestart = true
                    )
                }
            }
            
            is DidCommError.PackingError -> {
                ErrorAction.ShowError(
                    message = "Cannot send message to this recipient.",
                    actionLabel = "Choose Another",
                    canDismiss = true
                )
            }
            
            is DidCommError.UnpackingError -> {
                ErrorAction.Silent(
                    logMessage = "Failed to decrypt: ${error.message}"
                )
            }
            
            is DidCommError.GeneralError -> {
                ErrorAction.ShowError(
                    message = "Something went wrong. Please try again.",
                    actionLabel = "Retry",
                    canDismiss = true
                )
            }
        }
    }
    
    private fun getValidationMessage(error: DidCommError.ValidationError): String {
        return when {
            error.message.contains("seed") -> {
                "Security initialization failed. Please reinstall the app."
            }
            error.message.contains("endpoint") -> {
                "Invalid server address. Please check your settings."
            }
            error.message.contains("DID") -> {
                "Invalid identity format. Please contact support."
            }
            else -> {
                "Invalid input. Please check and try again."
            }
        }
    }
}

sealed class ErrorAction {
    data class ShowError(
        val message: String,
        val actionLabel: String?,
        val canDismiss: Boolean
    ) : ErrorAction()
    
    data class Retry(
        val delay: Long,
        val showLoading: Boolean
    ) : ErrorAction()
    
    data class Silent(
        val logMessage: String
    ) : ErrorAction()
    
    data class Fatal(
        val message: String,
        val requiresRestart: Boolean
    ) : ErrorAction()
}
```

## iOS Error Handling

### 1. Swift Error Wrapper

```swift
enum NaviaError: LocalizedError {
    case validation(String)
    case database(String)
    case packing(String)
    case unpacking(String)
    case general(String)
    
    init(from didCommError: DidCommError) {
        switch didCommError {
        case .validationError(let message):
            self = .validation(message)
        case .databaseError(let message):
            self = .database(message)
        case .packingError(let message):
            self = .packing(message)
        case .unpackingError(let message):
            self = .unpacking(message)
        case .generalError(let message):
            self = .general(message)
        }
    }
    
    var errorDescription: String? {
        switch self {
        case .validation(let msg):
            return "Invalid input: \(msg)"
        case .database:
            return "Storage error. Please try again."
        case .packing:
            return "Failed to send message."
        case .unpacking:
            return "Failed to receive message."
        case .general(let msg):
            return msg
        }
    }
    
    var isRecoverable: Bool {
        switch self {
        case .database, .general:
            return true
        case .validation, .packing, .unpacking:
            return false
        }
    }
}
```

### 2. Async/Await Error Handling

```swift
@MainActor
class MessagingViewModel: ObservableObject {
    @Published var state = MessagingState()
    
    func sendMessage(_ content: String, to recipient: String) async {
        state.isLoading = true
        state.error = nil
        
        do {
            let encrypted = try await withRetry {
                try await navia.pack(
                    message: createMessage(content),
                    from: myDid,
                    to: recipient
                )
            }
            
            await sendViaNetwork(encrypted)
            state.lastMessageSent = true
        } catch {
            handle(error)
        }
        
        state.isLoading = false
    }
    
    private func handle(_ error: Error) {
        if let naviaError = error as? DidCommError {
            let wrapped = NaviaError(from: naviaError)
            state.error = wrapped
            
            if wrapped.isRecoverable {
                scheduleRetry()
            }
        } else {
            state.error = .general(error.localizedDescription)
        }
    }
}
```

## Error Recovery Strategies

### 1. Graceful Degradation

```kotlin
class MessageService {
    suspend fun sendMessage(message: Message): Result<Unit> {
        return try {
            // Try encrypted first
            val encrypted = interface.pack(
                message.toDidComm(),
                myDid,
                message.recipientDid
            )
            networkClient.send(encrypted)
            Result.success(Unit)
        } catch (e: DidCommError.PackingError) {
            // Fall back to unencrypted if both parties agree
            if (allowUnencrypted && message.allowUnencrypted) {
                networkClient.sendPlaintext(message)
                Result.success(Unit)
            } else {
                Result.failure(e)
            }
        }
    }
}
```

### 2. State Restoration

```kotlin
class StateManager {
    private val pendingOperations = mutableListOf<PendingOperation>()
    
    suspend fun executeWithRecovery(
        operation: suspend () -> Unit
    ) {
        val pending = PendingOperation(
            id = UUID.randomUUID().toString(),
            timestamp = System.currentTimeMillis(),
            operation = operation
        )
        
        pendingOperations.add(pending)
        
        try {
            operation()
            pendingOperations.remove(pending)
            clearPersistedOperation(pending.id)
        } catch (e: Exception) {
            persistOperation(pending)
            throw e
        }
    }
    
    suspend fun recoverPendingOperations() {
        val persisted = loadPersistedOperations()
        
        persisted.forEach { operation ->
            try {
                operation.operation()
                clearPersistedOperation(operation.id)
            } catch (e: Exception) {
                // Log and continue with next
                logger.error("Failed to recover operation ${operation.id}", e)
            }
        }
    }
}
```

### 3. Automatic Cleanup

```kotlin
class NaviaMaintenanceService(
    private val interface: DidComInterface
) {
    suspend fun performMaintenance() {
        try {
            // Check health
            if (!interface.isHealthy()) {
                recoverUnhealthyState()
            }
            
            // Clean old error logs
            val errorLogs = interface.exportErrorLogs()
            if (errorLogs.length > 10000) {
                interface.clearErrorLogs()
            }
            
            // Compact database
            compactStorageIfNeeded()
            
        } catch (e: Exception) {
            logger.error("Maintenance failed", e)
        }
    }
    
    private suspend fun recoverUnhealthyState() {
        // Export current state
        val backup = exportCriticalData()
        
        // Reinitialize
        interface.open(getDatabasePath(), getSecureSeed())
        
        // Restore critical data
        restoreCriticalData(backup)
    }
}
```

## User Experience Guidelines

### 1. Error Message Localization

```kotlin
object ErrorMessages {
    fun get(error: DidCommError, locale: Locale): String {
        val key = when (error) {
            is DidCommError.ValidationError -> "error.validation"
            is DidCommError.DatabaseError -> "error.database"
            is DidCommError.PackingError -> "error.packing"
            is DidCommError.UnpackingError -> "error.unpacking"
            is DidCommError.GeneralError -> "error.general"
        }
        
        return ResourceBundle
            .getBundle("errors", locale)
            .getString(key)
    }
}
```

### 2. User-Friendly Error Display

```kotlin
@Composable
fun ErrorSnackbar(
    error: UserError,
    onDismiss: () -> Unit,
    onAction: (() -> Unit)? = null
) {
    Snackbar(
        modifier = Modifier.padding(8.dp),
        action = {
            error.actionLabel?.let { label ->
                TextButton(onClick = { onAction?.invoke() }) {
                    Text(label)
                }
            }
        },
        dismissAction = if (error.canDismiss) {
            {
                IconButton(onClick = onDismiss) {
                    Icon(Icons.Default.Close, "Dismiss")
                }
            }
        } else null
    ) {
        Row {
            Icon(
                imageVector = when (error.severity) {
                    ErrorSeverity.INFO -> Icons.Default.Info
                    ErrorSeverity.WARNING -> Icons.Default.Warning
                    ErrorSeverity.ERROR -> Icons.Default.Error
                },
                contentDescription = null,
                tint = when (error.severity) {
                    ErrorSeverity.INFO -> Color.Blue
                    ErrorSeverity.WARNING -> Color.Yellow
                    ErrorSeverity.ERROR -> Color.Red
                }
            )
            Spacer(modifier = Modifier.width(8.dp))
            Text(error.message)
        }
    }
}
```

### 3. Progressive Error Handling

```kotlin
class ProgressiveErrorHandler {
    private val errorCounts = mutableMapOf<String, Int>()
    
    fun handleProgressively(
        error: DidCommError,
        operation: String
    ): ErrorResponse {
        val count = errorCounts.getOrDefault(operation, 0) + 1
        errorCounts[operation] = count
        
        return when {
            count == 1 -> {
                // First time - silent retry
                ErrorResponse.SilentRetry
            }
            count <= 3 -> {
                // Show brief message
                ErrorResponse.ShowBrief(
                    "Having trouble. Retrying..."
                )
            }
            count <= 5 -> {
                // Show detailed error
                ErrorResponse.ShowDetailed(
                    title = "Connection Problem",
                    message = getDetailedMessage(error),
                    actions = listOf("Retry", "Cancel")
                )
            }
            else -> {
                // Give up
                ErrorResponse.Fatal(
                    "Unable to complete operation. Please try again later."
                )
            }
        }
    }
}
```

## Logging and Monitoring

### 1. Structured Logging

```kotlin
class NaviaLogger {
    fun logError(
        operation: String,
        error: DidCommError,
        context: Map<String, Any> = emptyMap()
    ) {
        val logEntry = LogEntry(
            timestamp = Instant.now(),
            level = LogLevel.ERROR,
            operation = operation,
            errorType = error::class.simpleName,
            errorMessage = when (error) {
                is DidCommError.ValidationError -> error.message
                is DidCommError.DatabaseError -> error.message
                is DidCommError.PackingError -> error.message
                is DidCommError.UnpackingError -> error.message
                is DidCommError.GeneralError -> error.message
            },
            context = context + mapOf(
                "app_version" to BuildConfig.VERSION_NAME,
                "os_version" to Build.VERSION.RELEASE,
                "device" to "${Build.MANUFACTURER} ${Build.MODEL}"
            )
        )
        
        // Local logging
        Log.e("Navia", logEntry.toJson())
        
        // Remote logging
        remoteLogger.log(logEntry)
    }
}
```

### 2. Error Metrics

```kotlin
class ErrorMetrics {
    private val metrics = ConcurrentHashMap<String, ErrorMetric>()
    
    fun record(error: DidCommError, operation: String) {
        val key = "${error::class.simpleName}:$operation"
        
        metrics.compute(key) { _, existing ->
            (existing ?: ErrorMetric()).apply {
                count++
                lastOccurred = System.currentTimeMillis()
                
                if (count == 1) {
                    firstOccurred = lastOccurred
                }
            }
        }
    }
    
    fun getReport(): ErrorReport {
        return ErrorReport(
            totalErrors = metrics.values.sumOf { it.count },
            errorsByType = metrics.entries
                .groupBy { it.key.substringBefore(":") }
                .mapValues { it.value.sumOf { e -> e.value.count } },
            mostFrequent = metrics.maxByOrNull { it.value.count },
            timeRange = TimeRange(
                start = metrics.values.minOfOrNull { it.firstOccurred },
                end = metrics.values.maxOfOrNull { it.lastOccurred }
            )
        )
    }
}
```

### 3. Crash Reporting Integration

```kotlin
class CrashReporter {
    fun reportNonFatal(error: DidCommError, context: String) {
        FirebaseCrashlytics.getInstance().apply {
            // Set custom keys
            setCustomKey("error_type", error::class.simpleName ?: "Unknown")
            setCustomKey("operation", context)
            setCustomKey("navia_version", BuildConfig.NAVIA_VERSION)
            
            // Log the error
            recordException(
                NaviaException(error, context)
            )
            
            // Add breadcrumb
            log("Navia error in $context: ${error.message}")
        }
    }
}

class NaviaException(
    val error: DidCommError,
    val context: String
) : Exception("${error::class.simpleName} in $context: ${error.message}")
```

## Best Practices Summary

1. **Always handle all error types** - Don't use catch-all handlers
2. **Provide actionable messages** - Tell users what they can do
3. **Log appropriately** - Debug info for developers, clear messages for users
4. **Implement retry strategies** - But know when to stop
5. **Monitor error patterns** - Detect systemic issues early
6. **Test error paths** - Error handling code needs testing too
7. **Graceful degradation** - Partial functionality is better than none
8. **User empathy** - Errors are frustrating; be helpful, not technical