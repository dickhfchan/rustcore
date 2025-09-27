# Rustcore Testing Framework Makefile
# Provides convenient commands for testing and development

.PHONY: help test test-all test-quick test-ci test-boot test-smoke test-enhanced test-services test-build clean setup install-deps

# Default target
help:
	@echo "Rustcore Testing Framework"
	@echo "========================="
	@echo ""
	@echo "Available targets:"
	@echo "  help          Show this help message"
	@echo "  test          Run all tests (default)"
	@echo "  test-all      Run comprehensive test suite"
	@echo "  test-quick    Run quick tests only"
	@echo "  test-ci       Run CI/CD tests"
	@echo "  test-boot     Run boot sequence tests"
	@echo "  test-smoke    Run smoke tests"
	@echo "  test-enhanced Run enhanced functionality tests"
	@echo "  test-services Run service integration tests"
	@echo "  test-build    Run build system tests"
	@echo "  clean         Clean build artifacts"
	@echo "  setup         Setup testing environment"
	@echo "  install-deps  Install dependencies"
	@echo ""
	@echo "Examples:"
	@echo "  make test              # Run all tests"
	@echo "  make test-quick        # Quick tests"
	@echo "  make test-ci           # CI tests"
	@echo "  make clean test        # Clean and test"

# Default test target
test: test-all

# Run all tests
test-all:
	@echo "Running comprehensive test suite..."
	./scripts/test.sh --verbose all

# Run quick tests
test-quick:
	@echo "Running quick tests..."
	./scripts/test.sh --quick all

# Run CI tests
test-ci:
	@echo "Running CI tests..."
	./scripts/test.sh --verbose ci

# Run boot tests
test-boot:
	@echo "Running boot tests..."
	./scripts/test.sh boot

# Run smoke tests
test-smoke:
	@echo "Running smoke tests..."
	./scripts/test.sh smoke

# Run enhanced tests
test-enhanced:
	@echo "Running enhanced tests..."
	./scripts/test.sh enhanced

# Run service tests
test-services:
	@echo "Running service tests..."
	./scripts/test.sh services

# Run build tests
test-build:
	@echo "Running build tests..."
	./scripts/test.sh build

# Clean build artifacts
clean:
	@echo "Cleaning build artifacts..."
	cargo clean
	rm -f debug*.log qemu.log

# Setup testing environment
setup:
	@echo "Setting up testing environment..."
	./scripts/setup-testing.sh

# Install dependencies (macOS)
install-deps-mac:
	@echo "Installing dependencies for macOS..."
	brew install qemu rust

# Install dependencies (Ubuntu/Debian)
install-deps-ubuntu:
	@echo "Installing dependencies for Ubuntu/Debian..."
	sudo apt-get update
	sudo apt-get install -y qemu-system-x86 build-essential

# Build release version
build-release:
	@echo "Building release version..."
	cargo +nightly build --release

# Test release build
test-release:
	@echo "Testing release build..."
	./scripts/test.sh --release all

# Run benchmarks
benchmark:
	@echo "Running benchmarks..."
	./scripts/test.sh benchmark

# Generate test coverage
coverage:
	@echo "Generating test coverage..."
	./scripts/test.sh --coverage all

# Check code formatting
fmt:
	@echo "Checking code formatting..."
	cargo +nightly fmt -- --check

# Run clippy
clippy:
	@echo "Running clippy..."
	cargo +nightly clippy --workspace

# Run all checks (format, clippy, tests)
check: fmt clippy test

# Development workflow
dev: clean build-release test-release

# CI workflow
ci: fmt clippy test-ci

# Release workflow
release: clean fmt clippy test-all build-release test-release

# Documentation
docs:
	@echo "Generating documentation..."
	cargo doc --workspace --no-deps

# Show test results
show-results:
	@echo "Recent test results:"
	@if [ -f "docs/qemu_test_results.md" ]; then \
		head -50 docs/qemu_test_results.md; \
	else \
		echo "No test results found. Run 'make test' first."; \
	fi

# Show QEMU logs
show-logs:
	@echo "Recent QEMU logs:"
	@if [ -f "debug.log" ]; then \
		cat debug.log; \
	else \
		echo "No debug logs found."; \
	fi

# Monitor tests in real-time
monitor:
	@echo "Monitoring tests in real-time..."
	@echo "Press Ctrl+C to stop"
	@while true; do \
		clear; \
		echo "=== Rustcore Test Monitor ==="; \
		echo "Time: $$(date)"; \
		echo ""; \
		if [ -f "debug.log" ]; then \
			echo "Latest debug log:"; \
			tail -10 debug.log; \
		else \
			echo "No debug logs found"; \
		fi; \
		echo ""; \
		echo "Press Ctrl+C to stop monitoring"; \
		sleep 2; \
	done
