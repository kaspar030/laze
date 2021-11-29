TESTS = $(dir $(wildcard */test.sh))

.PHONY: test test-debug build-debug

all: test

test-debug: export LAZE=$(CURDIR)/../../target/debug/laze
test-debug: test build-debug

ifneq (, $(filter test-debug, $(MAKECMDGOALS)))
  TEST_DEPS += build-debug
endif

build-debug:
	cargo build

test: $(TEST_DEPS)
	for testdir in $(TESTS); do \
		(cd $${testdir}; echo "- testing in $${testdir}"; ./test.sh; ) || exit 1 ; \
		done
