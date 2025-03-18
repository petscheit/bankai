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
