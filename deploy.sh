#!/usr/bin/sh

set -e

dir=$(CDPATH= cd -- "$(dirname -- "$0")" && pwd)
cd $dir

cargo build -r

rm "$(which plsdo)"
cp target/release/plsdo ~/.scripts/

echo "Deployment done"
