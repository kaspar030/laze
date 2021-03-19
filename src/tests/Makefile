TESTS = $(dir $(wildcard */test.sh))

.PHONY: test

test:
	for testdir in $(TESTS); do \
		(cd $${testdir}; echo "- testing in $${testdir}"; ./test.sh; ) || exit 1 ; \
		done
