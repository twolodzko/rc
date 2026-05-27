test: basic-test integration-test

basic-test:
    cargo fmt
    cargo clippy
    cargo test --quiet

integration-test: build
    bats tests.bats

build:
    cargo build --release
    cp target/release/rc .

install: test
    cargo install --path .

repl:
    cargo run --

clean:
    rm -rf ./target
    rm -rf ./rc
