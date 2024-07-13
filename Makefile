# Prerequisites 'rustup component add llvm-tools-preview' and 'cargo install grcov'

build_with_cov:
	RUSTFLAGS="-Cinstrument-coverage" cargo build --release --features mock

coverage: build_with_cov
	LLVM_PROFILE_FILE="STS1-%p-%m.profraw" cargo test --features mock --release
	grcov . -s . --binary-path ./target/release/ -t html --branch --ignore-not-existing -o ./target/release/coverage/
	firefox ./target/release/coverage/index.html&

sw_test:
	cargo build --release -Fmock && RUST_LOG=info cargo test --release -Fmock

packs:
	cargo test build_pack --features rpi

clean:
	rm -rf *.profraw
	rm -f ThreadId* updatepin
	rm -f data/*
	rm -rf archives/*
	rm -rf tests/tmp
	rm -f tests/*.pack

rebuild_student_archive:
	rm -f tests/student_program.zip
	cd tests/test_data; zip -r ../student_program.zip *
