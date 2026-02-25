#!/bin/bash
set -euxo pipefail

VERSION=$1

sed -i.bak "/\[package\]/,/version =/ s/version = \".*/version = \"${VERSION}\"/" Cargo.toml
rm Cargo.toml.bak 2> /dev/null

sed -i ".bak" "s@install.sh | sh -s -- v.*@install.sh | sh -s -- v${VERSION}@" README.md
rm README.md.bak 2> /dev/null
