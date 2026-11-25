.PHONY: clean clean-cov clean-all size

# Remove artefatos de compilação (debug + release), mantém coverage
clean:
	cargo clean

# Remove apenas artefatos de coverage (llvm-cov)
clean-cov:
	cargo clean --target-dir target/llvm-cov-target

# Remove tudo: compilação, coverage e logs de QA
clean-all: clean clean-cov
	rm -f target/qa-*.log

# Mostra quanto cada subpasta do target ocupa
size:
	@du -sh target/*/ 2>/dev/null | sort -rh || echo "target/ não existe"
	@echo "---"
	@du -sh target/ 2>/dev/null || echo "target/ não existe"
