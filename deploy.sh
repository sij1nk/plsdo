#!/usr/bin/sh

set -e

dir=$(CDPATH= cd -- "$(dirname -- "$0")" && pwd)
cd $dir

cargo build -r
cp target/release/plsdo ~/.dotfiles/scripts/ 

echo "Deployment done"
