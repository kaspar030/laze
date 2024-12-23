#!/bin/sh

. ../test-common.sh

cleanup
build -j4 -v -c
clean_temp_files

diff_build_dir

echo TEST_OK

cleanup
