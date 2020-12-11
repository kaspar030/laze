#!/bin/sh

. ../test-common.sh

cleanup
build
clean_temp_files

diff -r build_expected build

echo TEST_OK

cleanup
