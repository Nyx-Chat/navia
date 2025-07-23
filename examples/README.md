# Navia Examples

This directory contains runnable examples demonstrating how to use the Navia DIDComm library.

## Running Examples

From the `rust/navia-core` directory:

```bash
# List all examples
cargo run --example

# Run specific examples
cargo run --example message_flow
cargo run --example storage_operations
cargo run --example error_handling
cargo run --example storage_traits
cargo run --example complete_api
```

## Examples Overview

### Basic Examples

#### message_flow
Complete DIDComm message encryption/decryption flow showing:
- DID generation
- Message packing (encryption)
- Message unpacking (decryption)

#### storage_operations
Key-value storage patterns demonstrating:
- Individual insert/get operations
- Batch operations for performance
- Update and remove operations

### Advanced Examples

#### error_handling
Comprehensive error handling patterns:
- Handling each error type
- Validation errors
- Recovery strategies

#### storage_traits
Storage API usage patterns:
- Working with categories
- Batch operations
- Storage best practices

#### complete_api
Full API demonstration covering:
- Every method in DidComInterface
- Health checks
- Error log management
- Complete workflow examples

## Key Concepts Demonstrated

1. **Initialization**: All examples show proper initialization with secure seeds
2. **Async Operations**: Proper use of async/await with Tokio runtime
3. **Error Handling**: Each example includes error handling
4. **Best Practices**: Examples follow production-ready patterns

## Notes

- Examples use fixed seeds for simplicity. In production, use secure random generation
- All examples use temporary databases that are cleaned up automatically
- Examples are designed to be self-contained and runnable independently

For detailed API documentation, see [API Reference](../docs/API.md).