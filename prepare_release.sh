#!/bin/env bash

set -e


POSITIONAL_ARGS=()
while [[ $# -gt 0 ]]; do
  case $1 in
    --hotfix)
      USE_VERSION="$2"
      shift # past argument
      shift # past value
      ;;
    -*|--*)
      echo "Unknown option $1"
      exit 1
      ;;
    *)
      POSITIONAL_ARGS+=("$1") # save positional arg
      shift # past argument
      ;;
  esac
done

if [ -n "$USE_VERSION" ]; then
    CURRENT_RELEASE="$USE_VERSION"
else
    CURRENT_RELEASE=$(git branch --show-current | sed 's/release\/v//')
fi

echo "Preparing release $CURRENT_RELEASE"

if [ -z "$CURRENT_RELEASE" ]; then
    echo "Could not determine current release from git branch"
    exit 1
fi

CARGO_RELEASE_VERSION=$(cat Cargo.toml | grep '^version =' | head -n 1 | sed 's/version = "//' | sed 's/"//')
if [ "$CARGO_RELEASE_VERSION" != "$CURRENT_RELEASE" ]; then
    echo "Cargo version ($CARGO_RELEASE_VERSION) does not match release version ($CURRENT_RELEASE)"
    exit 1
fi

CHANGELOG_VERSION_LINE=$(cat CHANGELOGS.md | grep "$CURRENT_RELEASE" | head -n 1)
if [ -z "$CHANGELOG_VERSION_LINE" ]; then
    echo "Could not find changelog entry for version $CURRENT_RELEASE"
    exit 1
fi

# Check that when subcrate import other workspace crates, they use the same version as the workspace version
DIRS=$(find . -name Cargo.toml -not -path "./target/*" | xargs dirname | grep "^./simusa-" | sed 's/^\.\///')
DEPS="simusa $DIRS"
for dir in $DIRS; do
    for dep in $DEPS; do
        if [ "$dir" != "$dep" ]; then
            # Check if dir/Cargo.toml contains a dependency on dep with the correct version
            DEP_VERSION=$(cat "$dir/Cargo.toml" | grep "^$dep" | head -n 1 | sed 's/.*version = "//' | sed 's/".*//')
            if [ -n "$DEP_VERSION" ] && [ "$DEP_VERSION" != "$CURRENT_RELEASE" ]; then
                echo "Error: $dir depends on $dep with version $DEP_VERSION, but current release is $CURRENT_RELEASE"
                echo "File $dir/Cargo.toml"
                exit 1
            fi
        fi
    done
done

cargo fmt --all
cargo clippy --all-targets -- -D warnings
cargo clippy -p simusa --features monothreaded -- -D warnings
cargo build --release
./generate_config_doc.py
RUSTFLAGS="-D warnings" cargo doc --no-deps --release
target/release/simusa-tools --generate-schema config.schema.json

./test.sh