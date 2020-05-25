#!/bin/sh

. ../test-common.sh

cleanup
build

test "$(cat single_app.o)" = "local_var global_var single_app.c"
test "$(cat single_app.elf)" = "$(cat single_app.o)"

echo TEST_OK

cleanup
