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
	rustc -O --test src/lib.rs -o test

test-clean:
	rm -rf ./test

clean: test-clean
	rm -f *.rlib

push:
	git push origin master
	git push github master

