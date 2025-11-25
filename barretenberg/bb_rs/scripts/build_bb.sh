#!/bin/bash

cd ../cpp || exit
rm -fr .cache
rm -fr build
PRESET=clang20
export DISABLE_AZTEC_VM=0
cmake -G Ninja --preset $PRESET -DCMAKE_BUILD_TYPE=Release
cmake --build --preset $PRESET --target bb
