# target: lint - Run the linter.
lint:
	cargo clippy

# target: lint-fix - apply easy lint fixes
lint-fix:
	cargo clippy --fix

# target: fmt - apply cargo fmt
fmt:
	cargo fmt

