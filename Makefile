RUST_CFG=
BUILD ?= build
LIB ?= $(BUILD)/.timestamp_csv
RUST_PATH ?= -L $(BUILD) -L ./target/deps
RUST_TEST_PATH ?= -L $(BUILD) -L ./target/deps -L ./target/test/deps

compile: $(LIB)

$(LIB):
	@mkdir -p $(BUILD)
	rustc --opt-level=3 ./src/lib.rs --out-dir $(BUILD)
	@touch $(BUILD)/.timestamp_csv

ctags:
	ctags --recurse --options=ctags.rust --languages=Rust

docs:
	rm -rf doc
	rustdoc -L . --test src/lib.rs
	rustdoc src/lib.rs
	# WTF is rustdoc doing?
	chmod 755 doc
	in-dir doc fix-perms
	rscp ./doc/* gopher:~/www/burntsushi.net/rustdoc/

test: $(BUILD)/test
	RUST_TEST_TASKS=1 RUST_LOG=quickcheck,csv $(BUILD)/test

$(BUILD)/test: $(LIB) src/lib.rs src/test.rs src/bench.rs
	rustc $(RUST_TEST_PATH) --test src/lib.rs -o $(BUILD)/test

test-examples:
	(cd ./examples && ./test)

bench: $(BUILD)/bench
	RUST_TEST_TASKS=1 RUST_LOG=quickcheck,csv $(BUILD)/bench --bench

bench-prof: $(BUILD)/bench
	RUST_TEST_TASKS=1 RUST_LOG=quickcheck,csv valgrind --tool=callgrind $(BUILD)/bench --bench

$(BUILD)/bench: $(LIB) src/lib.rs src/test.rs src/bench.rs
	rustc -g $(RUST_PATH) --opt-level=3 -Z lto --test $(RUST_CFG) src/lib.rs -o $(BUILD)/bench

clean:
	rm -f $(BUILD)/* $(LIB)
	rm -rf target

push:
	git push origin master
	git push github master

