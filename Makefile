setup:
	./scripts/setup.sh

build-main:
	./scripts/cairo_compile.sh cairo/src/main.cairo

build-committee:
	./scripts/cairo_compile.sh cairo/src/committee_update.cairo

run-main:
	./scripts/cairo_run.sh

run-committee:
	./scripts/cairo_run.sh --committee

main-run-pie:
	./scripts/cairo_run.sh --pie

committee-run-pie:
	./scripts/cairo_run.sh --committee --pie

build-main-and-run:
	./scripts/cairo_compile.sh cairo/src/main.cairo
	./scripts/cairo_run.sh

build-committee-and-run:
	./scripts/cairo_compile.sh cairo/src/committee_update.cairo
	./scripts/cairo_run.sh

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
