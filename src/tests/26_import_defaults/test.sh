#!/bin/sh

. ../test-common.sh

cleanup
build

${LAZE} build run | tail -n 1 | grep -s "^Hello Laze!$" && echo TEST_OK

cleanup
