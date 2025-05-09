#!/bin/sh

. ../test-common.sh

cleanup

build -DANY=first echo-all 1 2 3 4

echo TEST_OK

cleanup
