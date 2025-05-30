# Makefile for microhttp-rs

# Default target
.PHONY: help
help:
	@echo "Available commands:"
	@echo "  make test         - Run all tests"
	@echo "  make test-verbose - Run all tests with verbose output"
	@echo "  make build        - Build the project"
	@echo "  make clean        - Clean the project"
	@echo "  make examples     - Build all examples"
	@echo "  make doc          - Generate documentation"
	@echo "  make lint         - Run the linter (clippy)"
	@echo "  make lint-fix     - Run the linter and fix issues where possible"

# Run all tests
.PHONY: test
test:
	@echo "Running all tests..."
	@cargo test

# Run all tests with verbose output
.PHONY: test-verbose
test-verbose:
	@echo "Running all tests with verbose output..."
	@cargo test -- --nocapture

# Build the project
.PHONY: build
build:
	@echo "Building the project..."
	@cargo build

# Clean the project
.PHONY: clean
clean:
	@echo "Cleaning the project..."
	@cargo clean

# Build all examples
.PHONY: examples
examples:
	@echo "Building all examples..."
	@cargo build --examples

# Generate documentation
.PHONY: doc
doc:
	@echo "Generating documentation..."
	@cargo doc --no-deps

# Run the linter (clippy)
.PHONY: lint
lint:
	@echo "Running linter..."
	@cargo clippy --all-targets -- \
		-D warnings \
		-D dead_code \
		-D unused_imports \
		-D unused_variables \
		-D unused_assignments \
		-D missing_docs \
		-D unsafe_code \
		-D clippy::all

# Run the linter and fix issues where possible
.PHONY: lint-fix
lint-fix:
	@echo "Running linter and fixing issues..."
	@cargo clippy --fix --allow-dirty --all-targets -- \
		-D warnings \
		-D dead_code \
		-D unused_imports \
		-D unused_variables \
		-D unused_assignments \
		-D missing_docs \
		-D unsafe_code \
		-D clippy::all
