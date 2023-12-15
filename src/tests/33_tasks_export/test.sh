#!/bin/sh

. ../test-common.sh

cleanup

${LAZE} build foobar

echo TEST_OK

cleanup
