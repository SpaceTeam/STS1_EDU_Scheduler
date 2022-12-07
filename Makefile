# Prerequisites 'rustup component add llvm-tools-preview' and 'cargo install grcov'

build_with_cov:
	RUSTFLAGS="-Cinstrument-coverage" cargo build

coverage: build_with_cov
	RUSTFLAGS="-Cinstrument-coverage" LLVM_PROFILE_FILE="STS1-%p-%m.profraw" cargo test --features mock
	grcov . -s . --binary-path ./target/debug/ -t html --branch --ignore-not-existing -o ./target/debug/coverage/
	firefox ./target/debug/coverage/index.html&

sw_test:
	cargo test --release --features mock

packs:
	cargo test build_pack --features rpi

clean:
	rm -rf *.profraw
	rm -f ThreadId*
	rm -f data/*
	rm -rf archives/*
	rm -rf tests/tmp
	rm -f tests/*.pack

rebuild_student_archive:
	rm -f tests/student_program.zip
	cd tests/test_data; zip -r ../student_program.zip *