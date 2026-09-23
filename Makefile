BINARY := emberwing-bot
RELEASE_DIR := target/release
DEBUG_DIR := target/debug

.PHONY: all build run release clean check fmt

# Default
all: build

# Debug build
build:
	cargo build

# Run in debug mode
run:
	cargo run

# Optimized release build
release:
	cargo build --release

# Run release binary directly (faster startup)
start: release
	./$(RELEASE_DIR)/$(BINARY)

# Check errors without producing binary
check:
	cargo check

# Format code
fmt:
	cargo fmt

# Clean build artifacts
clean:
	cargo clean

# Show binary size after release build
size: release
	@ls -lh $(RELEASE_DIR)/$(BINARY) | awk '{print "Binary size:", $$5}'