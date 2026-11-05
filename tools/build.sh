#!/bin/bash
# Checks and packages the mod in WSL under ~/horizon-build.
#   tools/build.sh check [lines]    cargo check of the game with the mod applied
#   tools/build.sh package          dist/deepbore_quarry.hmod and dist/deepbore_quarry_native.hnmod
set -u
B=~/horizon-build
SRC=$(cd "$(dirname "$0")/.." && pwd)
ID=deepbore_quarry_native
export RUSTUP_HOME=$B/rustup CARGO_HOME=$B/cargo PATH=$B/cargo/bin:$B/uv:$B/tools/bin:$PATH
export UV_PYTHON_INSTALL_DIR=$B/python UV_CACHE_DIR=$B/uv-cache
export NATIVE_NO_DEFAULT_FEATURES=1 CARGO_BUILD_JOBS=4
cd "$B/horizon-mods" || exit 1
mkdir -p "native/$ID" "$SRC/dist"
rsync -a --delete --exclude target --exclude content --exclude dist --exclude docs --exclude tools --exclude .git "$SRC/" "native/$ID/"
PY="$(uv python find 3.12)"
case "${1:-check}" in
  check)
    "$PY" native/tools/native.py check "$ID" 2>&1 | grep -vE '^\s*(Compiling|Checking|Downloaded|Downloading|Updating|Blocking|Locking|Adding) ' | tail -n "${2:-120}"
    echo "exit: ${PIPESTATUS[0]}"
    ;;
  package)
    "$PY" native/tools/native.py package "$ID" 2>&1 | tail -n 5
    cp "native/target/$ID-"*.hnmod "$SRC/dist/$ID.hnmod"
    rm -rf "$B/content"
    cp -r "$SRC/content" "$B/content"
    (cd "$B/content" && cargo horizon-mod build 2>&1 | tail -n 4 && cp target/horizon-mod/*.hmod "$SRC/dist/deepbore_quarry.hmod")
    ls -la "$SRC/dist"
    ;;
esac
