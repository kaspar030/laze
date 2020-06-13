#!/bin/sh

. ../test-common.sh

cleanup
build

diff -r build_expected build

echo TEST_OK

cleanup
