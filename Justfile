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

profile:
    rm -rf ./cargo-flamegraph.trace flamegraph.svg
    # needs cargo-flamegraph: https://github.com/flamegraph-rs/flamegraph
    CARGO_PROFILE_RELEASE_DEBUG=true cargo flamegraph -o flamegraph.svg -- -f examples/interval_tests.rc

install: test
    cargo install --path .

repl:
    cargo run --

clean:
    rm -rf ./target
    rm -rf ./rc
    rm -rf ./cargo-flamegraph.trace
