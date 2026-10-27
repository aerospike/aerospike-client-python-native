# Go parameters
hosts ?= ""
host ?= localhost
port ?= 3000
user ?= ""
pass ?= ""
ns ?= "test"

.PHONY: build test test-unit test-int test-rust install clean stubs local-cargo git-cargo lint
all: lint dev build test install clean

local-cargo:
	@if [ -f Cargo.toml.local ]; then \
		cp Cargo.toml Cargo.toml.git.bak; \
		cp Cargo.toml.local Cargo.toml; \
		echo "Switched to local Cargo.toml (points to local rust client)"; \
		echo "Original Cargo.toml backed up to Cargo.toml.git.bak"; \
	else \
		echo "Cargo.toml.local not found!"; \
		exit 1; \
	fi

git-cargo:
	@if [ -f Cargo.toml.git.bak ]; then \
		cp Cargo.toml.git.bak Cargo.toml; \
		rm Cargo.toml.git.bak; \
		echo "Switched to git Cargo.toml (points to tls branch)"; \
	else \
		git checkout Cargo.toml; \
		echo "Switched to git Cargo.toml (points to tls branch)"; \
	fi

stubs:
	# Generate type stubs and organize them as a Python package
	# stub_gen.rs will automatically move _native.pyi to the correct location
	# Suppress warnings from dependencies (aerospike-core) to keep output clean
	# Uses --no-default-features + --features tls so TlsConfig stubs are generated
	# (extension-module is excluded; only tls is needed for complete stubs)
	# A generator build failure must fail this target: the previous pipeline form
	# swallowed it and post-processed the stale file instead.
	@[ -f aerospike.env ] && . ./aerospike.env; \
	RUSTFLAGS="-A warnings" cargo run --no-default-features --features tls --bin stub_gen > stub_gen.log 2>&1 \
		|| { cat stub_gen.log; rm -f stub_gen.log; exit 1; }; \
	grep -v "warning:.*aerospike-core" stub_gen.log || true; rm -f stub_gen.log
	# Post-process stubs to fix issues pyo3_stub_gen can't handle automatically
	@if [ -f python/aerospike_native/__init__.pyi ]; then \
		python python/postprocess_stubs.py python/aerospike_native/__init__.pyi; \
	fi
	@if [ -f python/aerospike_native/_native.pyi ]; then \
		python python/postprocess_stubs.py python/aerospike_native/_native.pyi; \
	fi
	@echo "Generated stubs in python/aerospike_native/"

lint:
	ruff check python/
	cargo clippy --locked --all-targets -- -D warnings

dev:
	# Show resolved Rust core (at a glance)
	@echo "Rust core: $$(cargo tree -p aerospike-core --depth 0 2>/dev/null | head -1)"
	# Generate a temp wheel & install it as a Python module in local virtual environment
	maturin develop

test:
	# Clear any stale pytest/bytecode cache that might have incorrect imports
	@python/clean_caches.sh || true
	source aerospike.env 2>/dev/null || source aerospike.env.example && python -m pytest python/tests

test-unit:
	@python/clean_caches.sh || true
	python -m pytest python/tests/unit

test-int:
	@python/clean_caches.sh || true
	source aerospike.env 2>/dev/null || source aerospike.env.example && python -m pytest python/tests/integration

test-rust:
	# extension-module leaves Python symbols unresolved, so a test binary
	# built with the default features cannot link; this links libpython.
	cargo test --locked --no-default-features --features tls --lib

dev-test: dev stubs test

build:
	# Generate distributable Python wheel binary and put it in the target folder
	maturin build -r

bench:
	python -m benchmarks.benchmark -k 100000 -z 32 -w I -c 100000 -d 120 --warmup 0 --cooldown 0
	python -m benchmarks.benchmark -k 100000 -z 32 -w RU,50 -d 10

bench-quick:
	python -m benchmarks.benchmark -k 1000 -z 4 -w RU,50 -d 5 --warmup 0 --cooldown 0

clean:
	cargo clean
