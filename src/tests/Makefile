TESTS = $(wildcard */test.sh)

export LAZE ?= cargo --quiet run

.PHONY: $(TESTS)

all: $(TESTS)

$(TESTS):
	@testdir=$(dir $@); echo "- testing in $${testdir}"; cd $${testdir}; ./test.sh
