#!/bin/sh

. ../test-common.sh

cleanup
build
clean_temp_files

diff -r build build_expected

echo TEST_OK

cleanup
