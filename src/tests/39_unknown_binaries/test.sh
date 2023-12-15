#!/bin/sh

. ../test-common.sh

cleanup
build -a foo,bar

echo TEST_OK

cleanup
