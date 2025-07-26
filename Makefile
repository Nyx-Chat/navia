.PHONY: setup dev test build-android fmt clean help

# Default target
all: setup

## Help
help:
	@echo "Navia Development Commands:"
	@echo "  make setup        - Initial setup (install hooks, check tools)"
	@echo "  make dev         - Start development (setup + check)"
	@echo "  make test        - Run all tests"
	@echo "  make build-android - Build for Android (all architectures)"
	@echo "  make fmt         - Format all code"
	@echo "  make clean       - Clean build artifacts"

## Initial setup for new developers
setup:
	@echo "🚀 Setting up Navia development environment..."
	@cd rust/navia-core && cargo check
	@rusty-hook init
	@echo "✅ Setup complete! Git hooks installed."

## Start development
dev: setup
	@cd rust/navia-core && cargo check

## Run tests
test:
	@cd rust/navia-core && cargo test

## Build for Android
build-android:
	@cd scripts && ./build-for-android.sh --package

## Format code
fmt:
	@cd rust/navia-core && cargo fmt

## Clean build artifacts
clean:
	@cd rust && cargo clean
	@rm -rf android/build
	@echo "✅ Clean complete!"