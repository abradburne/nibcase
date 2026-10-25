.PHONY: check
check:
	cargo fmt --all --check
	cargo clippy --workspace --all-targets --locked -- -D warnings
	cargo test --workspace --locked
	cargo clippy -p nibcase --no-default-features --features desktop,dslot --all-targets --locked -- -D warnings
	cargo clippy -p nibcase --no-default-features --features stock,dslot --all-targets --locked -- -D warnings
