#!/bin/sh

. ../test-common.sh

cleanup
build -j4 -v -c
clean_temp_files

diff -r build build_expected

echo TEST_OK

cleanup
