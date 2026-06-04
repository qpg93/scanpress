#!/bin/bash
set -e

usage() {
  echo "Usage: $0 {release|debug} [--clean]"
  echo ""
  echo "  release    Build with optimizations (for production)"
  echo "  debug      Build without optimizations (for development)"
  echo "  --clean    Remove target/ after build (disables incremental compilation)"
  exit 1
}

if [ -z "${1:-}" ]; then
  usage
fi

MODE="$1"

if [ "$MODE" = "--clean" ]; then
  usage
fi

CLEAN=false
if [ "${2:-}" = "--clean" ]; then
  CLEAN=true
elif [ -n "${2:-}" ]; then
  usage
fi

case "$MODE" in
  release)
    cargo build --release
    cp target/release/scanpress .
    ;;
  debug)
    cargo build
    cp target/debug/scanpress .
    ;;
  *)
    usage
    ;;
esac

if [ "$CLEAN" = true ]; then
  rm -rf target/
fi
