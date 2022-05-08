# Prerequisites 'rustup component add llvm-tools-preview' and 'cargo install grcov'

build_with_cov:
	RUSTFLAGS="-Cinstrument-coverage" cargo build

coverage: build_with_cov
	RUSTFLAGS="-Cinstrument-coverage" LLVM_PROFILE_FILE="STS1-%p-%m.profraw" cargo test
	grcov . -s . --binary-path ./target/debug/ -t html --branch --ignore-not-existing -o ./target/debug/coverage/
	firefox ./target/debug/coverage/index.html&

clean:
	rm -rf *.profraw