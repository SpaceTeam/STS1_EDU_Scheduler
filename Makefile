upload: build_target
	rsync -avzh ./target/armv7-unknown-linux-gnueabihf/release/STS1_EDU_Scheduler flatsat:edu/STS1_EDU_Scheduler --rsync-path='wsl rsync'
	ssh flatsat 'scp edu/STS1_EDU_Scheduler edu:./scheduler/STS1_EDU_Scheduler'

build_target:
	cargo build --release --target=armv7-unknown-linux-gnueabihf

upload_test:
	rsync -avzh ./tests/integration_tests flatsat:edu/ --rsync-path='wsl rsync'

remote_test: upload_test
	ssh flatsat 'cd edu/integration_tests && python test.py'

sw_test:
	cargo test --features mock

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



# Prerequisites 'rustup component add llvm-tools-preview' and 'cargo install grcov'
build_with_cov:
	RUSTFLAGS="-Cinstrument-coverage" cargo build

coverage: build_with_cov
	RUSTFLAGS="-Cinstrument-coverage" LLVM_PROFILE_FILE="STS1-%p-%m.profraw" cargo test --features mock
	grcov . -s . --binary-path ./target/debug/ -t html --branch --ignore-not-existing -o ./target/debug/coverage/
	firefox ./target/debug/coverage/index.html&