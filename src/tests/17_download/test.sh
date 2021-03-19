#!/bin/sh

rm -Rf testrepo
git init testrepo
cp foo.c testrepo
git -C testrepo add .
git -C testrepo commit -m ...

. ../test-common.sh

cleanup

${LAZE} build && echo TEST_OK

cleanup
rm -Rf testrepo
