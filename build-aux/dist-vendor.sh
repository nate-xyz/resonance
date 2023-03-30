#!/bin/sh

# SPDX-FileCopyrightText: 2019 Christopher Davis <brainblasted@disroot.org>
# SPDX-License-Identifier: GPL-3.0-or-later

export SOURCE_ROOT="$1"
export DIST="$2"

cd "$SOURCE_ROOT"
mkdir "$DIST"/.cargo
cargo vendor | sed 's/^directory = ".*"/directory = "vendor"/g' > $DIST/.cargo/config
# Move vendor into dist tarball directory
mv vendor "$DIST"
