if command -v cargo-nextest &> /dev/null
then
    cargo nextest run --workspace
else
    cargo test --workspace
fi

echo "------------------------------------------------------------------------"
echo "TESTING COMPILED PROGRAMS"
(cd tests/oracle && cargo run)
