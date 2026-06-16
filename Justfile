test: basic-test integration-test

basic-test:
    cargo fmt
    cargo clippy
    cargo test --quiet

integration-test: build-dev
    bats tests.bats

build-dev:
    cargo build --profile dev

build:
    cargo build --release
    cp target/release/rc .

timeit: build
    hyperfine './rc -f examples/interval_tests.rc'

profile:
    rm -rf ./perf.data* ./cargo-flamegraph.trace flamegraph.svg
    # needs cargo-flamegraph: https://github.com/flamegraph-rs/flamegraph
    cargo flamegraph --profile dev -o flamegraph.svg -- -f examples/interval_tests.rc

install: test
    cargo install --path .

repl:
    cargo run --

clean:
    rm -rf ./target
    rm -rf ./rc
    rm -rf ./cargo-flamegraph.trace
