#!/bin/sh

. ../test-common.sh

cleanup
build -i build/insights.json -G
clean_temp_files

diff -r build build_expected

echo TEST_OK

cleanup
