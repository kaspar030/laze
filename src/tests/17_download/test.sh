#!/bin/sh

rm -Rf testrepo
git init testrepo
cp foo.c testrepo
git -C testrepo add .
git -C testrepo commit -m ...

. ../test-common.sh

cleanup

laze b
echo TEST_OK

cleanup
rm -Rf testrepo
