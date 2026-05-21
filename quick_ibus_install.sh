#!/bin/bash

set -euo pipefail

make data-build
make data-check
make ibus-install

rm /home/linak/.cache/khmerime/*.bin

ibus restart
