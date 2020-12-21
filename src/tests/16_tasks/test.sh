#!/bin/sh

. ../test-common.sh

cleanup

${LAZE} task echo foo | grep -s "^foo$"
${LAZE} task foobar | grep -s "^foobar$"

echo TEST_OK

cleanup
