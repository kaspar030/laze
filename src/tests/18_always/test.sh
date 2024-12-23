#!/bin/sh

. ../test-common.sh

cleanup
echo "building first time"
build
grep -vq "overwrite" build/single_builder/single_app/single_app.elf || exit 1
echo "building second time"
build
clean_temp_files

diff_build_dir

echo TEST_OK

cleanup
