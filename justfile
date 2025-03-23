list:
    just --list

run $CONFIG:
    cargo build
    sudo -E RUST_LOG=debug target/debug/rota {{CONFIG}}
