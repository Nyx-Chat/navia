# Navia Code Refactoring Plan

## Overview

This document outlines the plan to refactor Navia's codebase to achieve better separation of concerns, improved testability, and cleaner architecture.

## Current Issues

1. **Mixed Responsibilities**: `messaging.rs` contains both UniFFI bindings and core business logic
2. **Tight Coupling**: FFI layer is directly intertwined with DIDComm operations
3. **Testing Difficulties**: Hard to unit test core logic without FFI overhead
4. **Limited Reusability**: Core DIDComm logic can't be used in non-FFI contexts

## Proposed Structure

```
rust/navia-core/src/
├── lib.rs                    # Module declarations, uniffi setup
│
├── ffi/                      # FFI/UniFFI layer (thin wrapper)
│   ├── mod.rs               # Module exports
│   ├── interface.rs         # DidComInterface - UniFFI object
│   ├── types.rs             # DIDCommMessage, KeyValue, DidCommError
│   └── conversions.rs       # Convert between FFI and core types
│
├── core/                     # Core business logic (no UniFFI)
│   ├── mod.rs               
│   ├── didcomm/             # DIDComm protocol implementation
│   │   ├── mod.rs
│   │   ├── handler.rs       # MessageHandler - core logic
│   │   ├── message.rs       # Core message types
│   │   ├── encryption.rs    # Pack/unpack operations
│   │   └── did.rs           # DID generation
│   │
│   └── storage/             # Storage abstraction
│       ├── mod.rs
│       ├── traits.rs        # Storage trait definitions
│       └── kv_store.rs      # Key-value operations
│
├── infrastructure/           # External integrations
│   ├── mod.rs
│   ├── askar/               # Aries Askar implementation
│   │   ├── mod.rs
│   │   ├── database.rs      # AskarDB wrapper
│   │   └── storage.rs       # Storage trait impl
│   │
│   └── runtime/             # Async runtime management
│       ├── mod.rs
│       └── tokio.rs         # Tokio runtime wrapper
│
└── error/                    # Error handling
    ├── mod.rs
    ├── core.rs              # Core domain errors
    └── ffi.rs               # FFI error conversions
```

## Implementation Steps

### Phase 1: Create Directory Structure (Week 1)

1. Create new directories: `ffi/`, `core/`, `infrastructure/`, `error/`
2. Create module files (`mod.rs`) for each directory
3. Update `lib.rs` to declare new modules

### Phase 2: Extract Core Types (Week 1)

1. Create `core/didcomm/message.rs`:
   - Define core `Message` type (without UniFFI)
   - Define core `EncryptedMessage` type
   - Pure Rust types for internal use

2. Create `ffi/types.rs`:
   - Keep existing `DIDCommMessage` with `#[derive(uniffi::Record)]`
   - Keep existing `KeyValue` and `DidCommError`
   - These remain as thin DTOs for FFI

3. Create `ffi/conversions.rs`:
   - Implement conversions between FFI and core types
   - Example: `impl From<ffi::DIDCommMessage> for core::Message`

### Phase 3: Extract Core Logic (Week 2)

1. Create `core/didcomm/handler.rs`:
   ```rust
   pub struct MessageHandler {
       storage: Arc<dyn MessageStorage>,
       crypto: Arc<dyn CryptoService>,
   }
   
   impl MessageHandler {
       pub async fn pack_message(
           &self,
           message: &Message,
           from: &Did,
           to: &Did,
       ) -> Result<EncryptedMessage, CoreError> {
           // Pure business logic, no FFI concerns
       }
   }
   ```

2. Create `core/storage/traits.rs`:
   ```rust
   #[async_trait]
   pub trait MessageStorage {
       async fn insert(&self, category: &str, key: &str, value: &str) -> Result<()>;
       async fn get(&self, category: &str, key: &str) -> Result<Option<String>>;
       // etc.
   }
   ```

3. Move Askar-specific code to `infrastructure/askar/`

### Phase 4: Refactor FFI Layer (Week 2)

1. Update `ffi/interface.rs`:
   ```rust
   #[derive(uniffi::Object)]
   pub struct DidComInterface {
       handler: Arc<MessageHandler>,
       runtime: Arc<Runtime>,
   }
   
   #[uniffi::export]
   impl DidComInterface {
       // Thin wrappers that delegate to core
       pub async fn pack(&self, msg: DIDCommMessage, from: String, to: String) 
           -> Result<String, DidCommError> {
           // Convert types and delegate to handler
       }
   }
   ```

2. Ensure FFI layer only handles:
   - Type conversions
   - Runtime context management
   - Error conversions

### Phase 5: Testing & Documentation (Week 3)

1. Add unit tests for core modules
2. Add integration tests for FFI layer
3. Update documentation
4. Ensure CI passes

## Benefits

1. **Testability**: Core logic can be unit tested without FFI
2. **Reusability**: Core can be used in other Rust projects
3. **Maintainability**: Clear boundaries between layers
4. **Performance**: Easier to optimize each layer
5. **Flexibility**: Can swap storage backends or FFI frameworks

## Migration Strategy

1. **Incremental Refactoring**: Move one component at a time
2. **Maintain Compatibility**: Keep existing API working during refactor
3. **Test Coverage**: Add tests before refactoring each component
4. **Version Control**: Use feature branches for each phase

## Success Criteria

- [ ] All existing tests pass
- [ ] Core logic has >80% test coverage
- [ ] FFI layer is <500 lines of code
- [ ] No UniFFI dependencies in core modules
- [ ] Performance benchmarks show no regression

## Next Steps

1. Review and approve this plan
2. Create feature branch `refactor/clean-architecture`
3. Begin Phase 1 implementation
4. Regular progress reviews

This refactoring will position Navia for long-term maintainability and growth while keeping the API stable for existing users.