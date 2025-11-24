all:

build:
	cargo build

install-debug: build
	cargo uninstall gha || true
	# pseudo-install
	ln -sf $(PWD)/target/debug/gha $(HOME)/.cargo/bin/gha

install:
	cargo install --path .

