#!/bin/bash

set -euo pipefail

make data-build
make data-check
make ibus-install

ibus restart
