# Use a Makefile instead of build.zig for simplicity.
# zig build-exe could alternatively also be invoked manually with those arguments.

# Point this to the Chiptrack's released ct.zig
CT_ZIG=ct.zig

all: $(patsubst %.zig, %.wat, $(wildcard *instruments.zig)) $(CT_ZIG)

%.wasm: %.zig Makefile $(CT_ZIG)
	zig build-exe -target wasm32-freestanding -dynamic -rdynamic --export-table -O ReleaseFast --max-memory=65536 --stack 8192 --mod ct:ct:$(CT_ZIG) --deps ct $<

%.wat: %.wasm Makefile $(CT_ZIG)
	# Needed for instruments that will be published to GitHub Gists.
	wasm2wat -f $< | tee $@

	# Basic smoke test
	wasm-interp $< --host-print --dummy-import-func --run-all-exports

	# We got the function names, now strip the .wasm from debug info.
	wasm-strip $<
