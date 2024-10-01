setup:
	./scripts/setup.sh

buildx:
	./scripts/cairo_compile.sh

run:
	./scripts/cairo_run.sh

build_and_run:
	./scripts/cairo_compile.sh
	./scripts/cairo_run.sh

	