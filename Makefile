# Monmouth SVM ExEx - Development Makefile

.PHONY: help build test clean check fmt clippy doc install-deps run-impl1 run-impl2

# Default target
help:
	@echo "Monmouth SVM ExEx - AI Agent Execution Environment"
	@echo ""
	@echo "Available targets:"
	@echo "  build         - Build all implementations"
	@echo "  test          - Run all tests"
	@echo "  check         - Check code without building"
	@echo "  fmt           - Format code"
	@echo "  clippy        - Run clippy lints"
	@echo "  clean         - Clean build artifacts"
	@echo "  doc           - Generate documentation"
	@echo "  install-deps  - Install system dependencies"
	@echo "  run-impl1     - Run implementation 1 (Enhanced SVM ExEx)"
	@echo "  run-impl2     - Run implementation 2 (Basic SVM ExEx)"
	@echo ""

# Development targets
build:
	cargo build --all-bins

build-release:
	cargo build --release --all-bins

test:
	cargo test --all

check:
	cargo check --all-bins

fmt:
	cargo fmt --all

clippy:
	cargo clippy --all-bins -- -D warnings

clean:
	cargo clean

doc:
	cargo doc --open --all

# Installation
install-deps:
	@echo "Installing system dependencies..."
	@if command -v apt-get > /dev/null; then \
		sudo apt-get update && sudo apt-get install -y build-essential clang lld; \
	elif command -v brew > /dev/null; then \
		brew install llvm; \
	else \
		echo "Please install build tools manually"; \
	fi

# Running implementations
run-impl1:
	@echo "Running Enhanced SVM ExEx (Implementation 1)..."
	cargo run --bin implementation1

run-impl2:
	@echo "Running Basic SVM ExEx (Implementation 2)..."
	cargo run --bin implementation2

# Development workflow
dev-check: fmt clippy check test
	@echo "Development checks completed successfully!"

# AI Agent specific targets
test-agents:
	cargo test --features ai-agents

build-minimal:
	cargo build --no-default-features --features minimal

# Monitoring and profiling
profile:
	cargo build --release --bin implementation1
	valgrind --tool=callgrind target/release/implementation1

benchmark:
	cargo bench --features testing

# Git hooks
pre-commit: fmt clippy check
	@echo "Pre-commit checks passed!"

# Quick development cycle
quick: fmt check
	@echo "Quick development check completed!"