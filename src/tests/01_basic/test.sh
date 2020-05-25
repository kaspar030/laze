#!/bin/sh

. ../test-common.sh

cleanup

build

test "$(cat single_app.o)" = "$(cat single_app.c)"
test "$(cat single_app.elf)" = "$(cat single_app.c)"

echo TEST_OK

cleanup
