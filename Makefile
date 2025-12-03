lint: doc
	cargo clippy --all-features
doc:
	RUSTDOCFLAGS="--cfg docsrs" cargo +nightly doc --no-deps --all-features

lio-cbuild:
	cargo rustc -p lio --crate-type dylib --features ffi --release
	cbindgen --crate lio --output lio/include/lio.h --cpp-compat &> /dev/null
	echo "lio: built c api at: $(pwd)/target/release/liblio.(dylib|so|dll)"

lio-test:
	cargo nextest r --release -p lio --features high
	./lio/tests/ffi/test.sh
	./lio/tests/nix-build/test.sh

check-flags:
	RUSTFLAGS="--cfg lio_unstable_ffi" cargo hack check --feature-powerset --lib --tests
