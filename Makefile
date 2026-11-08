.PHONY: all build release run test fmt lint check clean install

all: build

build:
	cargo build

release:
	cargo build --release

run:
	cargo run

test:
	cargo test

fmt:
	cargo fmt

lint:
	cargo clippy --all-targets -- -D warnings

check:
	cargo fmt --check
	$(MAKE) lint
	cargo test

clean:
	cargo clean

install:
	cargo install --path .
