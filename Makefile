.PHONY: build agent test clean package install-local ready backup

build:
	cargo build

agent:
	$(MAKE) -C native/agent artifacts

test:
	cargo test

package:
	./scripts/package.sh

ready:
	./scripts/release-guard.sh
	cargo fmt --check
	cargo clippy --all-targets --all-features -- -D warnings
	cargo test
	cargo build
	$(MAKE) -C native/agent artifacts
	./scripts/smoke-fixture.sh
	./scripts/package.sh

install-local:
	cargo build --release
	$(MAKE) -C native/agent artifacts
	./scripts/install.sh

backup:
	./scripts/backup.sh

clean:
	cargo clean
	$(MAKE) -C native/agent clean
	rm -rf dist
