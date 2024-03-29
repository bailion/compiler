export RUSTFLAGS="-C instrument-coverage"

if command -v cargo-nextest &> /dev/null
then
    cargo nextest run --workspace || { echo 'tests failed' ; exit 1; }
else
    cargo test --workspace || { echo 'tests failed' ; exit 1; }
fi
