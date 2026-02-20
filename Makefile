.PHONY: setup check test fmt clippy build release

# Install git hooks and build
setup:
	cp scripts/pre-commit .git/hooks/pre-commit
	chmod +x .git/hooks/pre-commit
	cargo build
	@echo "Setup complete. Pre-commit hook installed."

# Run all checks (same as pre-commit)
check: fmt clippy test

fmt:
	cargo fmt --check

clippy:
	cargo clippy -- -D warnings

test:
	cargo test

build:
	cargo build

release:
	cargo build --release
