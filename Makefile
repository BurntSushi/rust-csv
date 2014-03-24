RUST_CFG=

compile:
	rustc ./src/lib.rs

install:
	cargo-lite install

ctags:
	ctags --recurse --options=ctags.rust --languages=Rust

docs:
	rm -rf doc
	rustdoc src/lib.rs
	# WTF is rustdoc doing?
	chmod 755 doc
	in-dir doc fix-perms
	rscp ./doc/* gopher:~/www/burntsushi.net/rustdoc/

test: test-build
	RUST_TEST_TASKS=1 RUST_LOG=quickcheck,csv ./test

test-build: src/lib.rs
	rustc --test src/lib.rs -o test

test-examples:
	(cd ./examples && ./test)

bench: bench-build
	RUST_TEST_TASKS=1 RUST_LOG=quickcheck,csv ./bench --bench --save-metrics=bench.json

bench-build: src/lib.rs src/bench.rs
	rustc -O --test $(RUST_CFG) src/lib.rs -o bench

test-clean:
	rm -rf ./test

clean: test-clean
	rm -f *.rlib

push:
	git push origin master
	git push github master

