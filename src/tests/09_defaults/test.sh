#!/bin/sh

. ../test-common.sh

cleanup
build
clean_temp_files

diff_build_dir

echo TEST_OK

cleanup
