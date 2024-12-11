setup:
	./scripts/setup.sh

build-epoch:
	./scripts/cairo_compile.sh cairo/src/epoch_update.cairo

build-committee:
	./scripts/cairo_compile.sh cairo/src/committee_update.cairo

run-epoch:
	./scripts/cairo_run.sh

run-committee:
	./scripts/cairo_run.sh --committee

run-epoch-pie:
	./scripts/cairo_run.sh --pie

committee-run-pie:
	./scripts/cairo_run.sh --committee --pie

test:
	./cairo/tests/committee.sh
	./cairo/tests/epoch.sh

get-program-hash:
	@make build-main
	@make build-committee
	@echo "EpochProgramHash:"
	@cairo-hash-program --program cairo/build/main.json
	@echo "CommitteeProgramHash:"
	@cairo-hash-program --program cairo/build/committee_update.json
