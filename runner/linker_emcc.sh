#!/bin/sh
args="$*"
filtered_args=${args/ERROR_ON_UNDEFINED_SYMBOLS\=1/ERROR_ON_UNDEFINED_SYMBOLS\=0}
emcc $filtered_args