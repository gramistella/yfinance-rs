# 🧪 Test Runner Justfile
# Run `just` or `just help` to see this help.

set shell := ["bash", "-cu"]
set dotenv-load := true
set export := true
# set quiet := true  # optional: hide all command echoing

# ---- Tunables ---------------------------------------------------------------

FEATURES := 'test-mode'         # cargo features for tests
TEST_THREADS := '1'             # default for live/record (override: just TEST_THREADS=4 live)
FIXDIR := ''                    # default when YF_FIXDIR isn't set in the env

# ---- Helpers ----------------------------------------------------------------

banner MESSAGE:
	@printf "\n\033[1m▶ %s\033[0m\n\n" "{{MESSAGE}}"

vars:
	@echo "FEATURES      = {{FEATURES}}"
	@echo "TEST_THREADS  = {{TEST_THREADS}}"
	@echo "YF_FIXDIR     = ${YF_FIXDIR:-{{FIXDIR}}}"
	@echo "YF_LIVE       = ${YF_LIVE:-}"
	@echo "YF_RECORD     = ${YF_RECORD:-}"

# ---- Recipes ----------------------------------------------------------------

default: help

help:
	@just --list --unsorted

# NOTE on arg parsing:
# - If the first token looks like a test binary name (no leading `--`, no `::`),
#   it's passed as `--test <name>` BEFORE `--`.
# - Everything else goes AFTER `--` to the harness.

# Offline (replay cached fixtures)
test-offline +args='':
	@just banner "Offline tests (cached fixtures)"
	@set -euo pipefail; \
	TARGET_OPT=(); TEST_ARGS=(); \
	if [ -n "{{args}}" ]; then \
		set -- {{args}}; \
		first="${1:-}"; shift || true; \
		if [ -n "$first" ] && [[ "$first" != --* ]] && [[ "$first" != *::* ]]; then \
			TARGET_OPT=(--test "$first"); \
			TEST_ARGS=("$@"); \
		else \
			TEST_ARGS=("$first" "$@"); \
		fi; \
	fi; \
	cargo test --features {{FEATURES}} "${TARGET_OPT[@]}" -- "${TEST_ARGS[@]}"

# Full live sweep (no writes; runs all tests including ignored)
test-live +args='':
	@just banner "Live sweep (no writes, includes ignored)"
	@set -euo pipefail; \
	TARGET_OPT=(); TEST_ARGS=(); \
	if [ -n "{{args}}" ]; then \
		set -- {{args}}; \
		first="${1:-}"; shift || true; \
		if [ -n "$first" ] && [[ "$first" != --* ]] && [[ "$first" != *::* ]]; then \
			TARGET_OPT=(--test "$first"); \
			TEST_ARGS=("$@"); \
		else \
			TEST_ARGS=("$first" "$@"); \
		fi; \
	fi; \
	YF_LIVE=1 cargo test --features {{FEATURES}} "${TARGET_OPT[@]}" -- --include-ignored --test-threads={{TEST_THREADS}} "${TEST_ARGS[@]}"

# Record fixtures (live → cache)
test-record +args='':
	@just banner "Recording fixtures (runs ignored tests)"
	@set -euo pipefail; \
	TARGET_OPT=(); TEST_ARGS=(); \
	if [ -n "{{args}}" ]; then \
		set -- {{args}}; \
		first="${1:-}"; shift || true; \
		if [ -n "$first" ] && [[ "$first" != --* ]] && [[ "$first" != *::* ]]; then \
			TARGET_OPT=(--test "$first"); \
			TEST_ARGS=("$@"); \
		else \
			TEST_ARGS=("$first" "$@"); \
		fi; \
	fi; \
	YF_RECORD=1 cargo test --features {{FEATURES}} "${TARGET_OPT[@]}" -- --ignored --test-threads={{TEST_THREADS}} "${TEST_ARGS[@]}"

# Use a different fixture directory, then replay
test-with-fixdir dir='/tmp/yf-fixtures' +args='':
	@just banner "Recording to {{dir}} then replaying offline"
	@set -euo pipefail; \
	TARGET_OPT=(); TEST_ARGS=(); \
	if [ -n "{{args}}" ]; then \
		set -- {{args}}; \
		first="${1:-}"; shift || true; \
		if [ -n "$first" ] && [[ "$first" != --* ]] && [[ "$first" != *::* ]]; then \
			TARGET_OPT=(--test "$first"); \
			TEST_ARGS=("$@"); \
		else \
			TEST_ARGS=("$first" "$@"); \
		fi; \
	fi; \
	export YF_FIXDIR="{{dir}}"; \
	YF_RECORD=1 cargo test --features {{FEATURES}} "${TARGET_OPT[@]}" -- --ignored --test-threads={{TEST_THREADS}} "${TEST_ARGS[@]}"; \
	cargo test --features {{FEATURES}} "${TARGET_OPT[@]}" -- "${TEST_ARGS[@]}"

# Full test: clear phase markers; only run offline if live/record passes
test-full +args='':
	@just banner "Full test (Phase 1: live/record → Phase 2: offline)"
	@set -euo pipefail; \
	ts() { date '+%Y-%m-%d %H:%M:%S'; }; \
	TARGET_OPT=(); TEST_ARGS=(); \
	if [ -n "{{args}}" ]; then \
		set -- {{args}}; \
		first="${1:-}"; shift || true; \
		if [ -n "$first" ] && [[ "$first" != --* ]] && [[ "$first" != *::* ]]; then \
			TARGET_OPT=(--test "$first"); \
			TEST_ARGS=("$@"); \
		else \
			TEST_ARGS=("$first" "$@"); \
		fi; \
	fi; \
	echo "[$(ts)] 🟦 Phase 1/2 START — Live/Record (runs ignored, writes fixtures)"; \
	if YF_RECORD=1 cargo test --features {{FEATURES}} "${TARGET_OPT[@]}" -- --ignored --test-threads={{TEST_THREADS}} "${TEST_ARGS[@]}"; then \
		echo "[$(ts)] ✅ Phase 1/2 PASS — Live/Record passed"; \
		echo "[$(ts)] 🟩 Phase 2/2 START — Offline replay (cached fixtures)"; \
		if cargo test --features {{FEATURES}} "${TARGET_OPT[@]}" -- "${TEST_ARGS[@]}"; then \
			echo "[$(ts)] ✅ Phase 2/2 PASS — Offline replay passed"; \
			echo "[$(ts)] 🎉 Full test complete: BOTH phases passed"; \
		else \
			status=$?; \
			echo "[$(ts)] ❌ Phase 2/2 FAIL — Offline replay failed (exit $status)"; \
			echo "Tip: re-run only the offline pass with:"; \
			echo "  just offline {{args}}"; \
			exit $status; \
		fi; \
	else \
		status=$?; \
		echo "[$(ts)] ❌ Phase 1/2 FAIL — Live/Record failed (exit $status)"; \
		echo "Skipping offline. Tip: re-run only the live/record pass with:"; \
		echo "  just record {{args}}"; \
		exit $status; \
	fi
