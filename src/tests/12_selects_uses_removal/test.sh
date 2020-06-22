#!/bin/sh

. ../test-common.sh

cleanup
build

diff -r build build_expected

echo TEST_OK

cleanup
