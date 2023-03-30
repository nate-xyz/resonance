#!/usr/bin/env bash

meson compile -C _builddir --verbose && \
RUST_LOG=debug meson devenv -C _builddir ./src/resonance; exit;