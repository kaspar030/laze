TESTS = $(dir $(wildcard */test.sh))

export LAZE ?= cargo run

.PHONY: test

all: test

test: $(TEST_DEPS)
	for testdir in $(TESTS); do \
		(cd $${testdir}; echo "- testing in $${testdir}"; ./test.sh; ) || exit 1 ; \
		done
