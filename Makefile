setup:
	./scripts/setup.sh

venv:
	source venv/bin/activate

build-epoch:
	./scripts/cairo_compile.sh cairo/src/epoch_update.cairo

build-epoch-batch:
	./scripts/cairo_compile.sh cairo/src/epoch_batch.cairo

build-committee:
	./scripts/cairo_compile.sh cairo/src/committee_update.cairo

run-epoch:
	./scripts/cairo_run.sh

run-epoch-batch:
	./scripts/cairo_run.sh --epoch-batch

run-committee:
	./scripts/cairo_run.sh --committee

run-epoch-pie:
	./scripts/cairo_run.sh --pie

committee-run-pie:
	./scripts/cairo_run.sh --committee --pie

test:
	./cairo/tests/committee.sh
	./cairo/tests/epoch.sh

ci-local:
	./scripts/ci_local.sh

get-program-hash:
	@make build-epoch
	@make build-epoch-batch
	@make build-committee
	@echo "EpochProgramHash:"
	@cairo-hash-program --program cairo/build/epoch_update.json
	@echo "EpochBatchProgramHash:"
	@cairo-hash-program --program cairo/build/epoch_batch.json
	@echo "CommitteeProgramHash:"
	@cairo-hash-program --program cairo/build/committee_update.json
