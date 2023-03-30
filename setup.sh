#!/usr/bin/env bash

sudo rm -R _builddir && \
meson setup _builddir && \
sh run.sh 