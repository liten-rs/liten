lint: doc
	cargo clippy --all-features
doc:
	cargo test --doc
	RUSTDOCFLAGS="--cfg docsrs" cargo +nightly doc --no-deps --all-features

lio-cbuild:
	cargo rustc -p lio --crate-type dylib --features unstable_ffi --release
	cbindgen --crate lio --output lio/include/lio.h --cpp-compat &> /dev/null
	echo "lio: built c api at: $(pwd)/target/release/liblio.(dylib|so|dll)"

lio-test:
	cargo nextest r --release -p lio --features high
	./lio/tests/ffi/test.sh
	./lio/tests/nix-build/test.sh

check-flags:
	cargo hack check --feature-powerset --lib --tests
