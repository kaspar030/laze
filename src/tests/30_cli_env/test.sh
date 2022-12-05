#!/bin/sh

. ../test-common.sh

cleanup
build -DVAR1=var1 -DVAR2+=foo -DVAR2+=bar -DVAR3=var3
clean_temp_files

diff -r build build_expected

echo TEST_OK

cleanup
