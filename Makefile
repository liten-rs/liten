lint: doc
	cargo clippy --all-features
doc:
	RUSTDOCFLAGS="--cfg docsrs --cfg lio_unstable_ffi" cargo +nightly doc --no-deps --all-features

lio-cbuild:
	RUSTFLAGS="--cfg lio_unstable_ffi" cargo rustc -p lio --crate-type dylib --features ffi --release
	cbindgen --crate lio --output lio/include/lio.h --cpp-compat
