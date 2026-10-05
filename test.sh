#!/bin/bash
# Make the script stop if error occurs
set -e

CWD=$(pwd)
cargo nextest run --no-default-features
cargo build --release
cargo nextest run --release
cargo nextest run --release --all-features

cd simusa-py && maturin develop
cd $CWD

target/release/simusa-cmd config_example/config_2.yaml --no-gui
target/release/simusa-cmd config_example/config_scenario.yaml --no-gui

cd examples && ./run_examples.sh