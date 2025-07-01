.PHONY: help setup-pre-commit lint fmt test clean contracts-build contracts-clean

# Default target
help: ## Show this help message
	@echo "Available targets:"
	@echo ""
	@echo "Development:"
	@grep -E '^[a-zA-Z_-]+:.*?## .*$$' $(MAKEFILE_LIST) | grep -E "(setup-pre-commit|lint|fmt|test)" | sort | awk 'BEGIN {FS = ":.*?## "}; {printf "  \033[36m%-20s\033[0m %s\n", $$1, $$2}'
	@echo ""
	@echo "Build & Clean:"
	@grep -E '^[a-zA-Z_-]+:.*?## .*$$' $(MAKEFILE_LIST) | grep -E "(clean|contracts)" | sort | awk 'BEGIN {FS = ":.*?## "}; {printf "  \033[36m%-20s\033[0m %s\n", $$1, $$2}'
	@echo ""
	@echo "Individual tools (advanced):"
	@echo "  \033[36m./bin/lint --help\033[0m        Show lint options (--rust, --cairo, --prettier, --all, --check-only)"
	@echo "  \033[36m./bin/fmt --help\033[0m         Show format options (--rust, --cairo, --prettier, --all)"
	@echo "  \033[36m./bin/rust-lint\033[0m          Rust formatting & linting"
	@echo "  \033[36m./bin/cairo-lint\033[0m         Cairo formatting"
	@echo "  \033[36m./bin/prettier-lint\033[0m      Prettier formatting"
	@echo "  \033[36m./bin/test\033[0m               Run all tests including examples"

setup-pre-commit: ## Set up pre-commit hooks
	@./bin/setup-pre-commit

lint: ## Run all linting checks (same as CI)
	@./bin/lint --all --check-only

fmt: ## Format all code
	@./bin/fmt --all

test: ## Run all tests
	@./bin/test

clean: ## Clean build artifacts
	@echo "🧹 Cleaning build artifacts..."
	@cargo clean
	@cd contracts && scarb clean

# Contract-specific targets
contracts-build: ## Build contracts
	@echo "🏗️  Building contracts..."
	@make -C contracts generate_artifacts

contracts-clean: ## Clean contract artifacts
	@cd contracts && scarb clean 