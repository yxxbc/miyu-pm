#!/usr/bin/env bash
# miyu-pm TUI shared library.
# fzf-based interactive frontend around the miyu-pm Rust CLI.
#
# Inspired by SHORiN-KiWATA/shorin-pac:
#   - CLI stays the headless execution layer;
#   - TUI only handles fuzzy search / multi-select / preview / confirmation.

if [[ -n "${MIYU_PM_TUI_LOADED:-}" ]]; then
    return 0
fi
MIYU_PM_TUI_LOADED=1

PMT_LIB_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PMT_ROOT="$(cd "${PMT_LIB_DIR}/../.." && pwd)"

# --- locate miyu-pm binary ---
PMT_BIN="${MIYU_PM_BIN:-}"
if [[ -z "$PMT_BIN" ]]; then
    for candidate in \
        "$PMT_ROOT/target/debug/miyu-pm" \
        "$PMT_ROOT/target/release/miyu-pm"; do
        if [[ -x "$candidate" ]]; then
            PMT_BIN="$candidate"
            break
        fi
    done
fi
if [[ -z "$PMT_BIN" ]]; then
    PMT_BIN="$(command -v miyu-pm 2>/dev/null || true)"
fi
if [[ -z "$PMT_BIN" || ! -x "$PMT_BIN" ]]; then
    echo "miyu-pm TUI: miyu-pm binary not found." >&2
    echo "Build it first: cargo build" >&2
    echo "Or set MIYU_PM_BIN=/path/to/miyu-pm" >&2
    return 1
fi

PMT_REGISTRY="${MIYU_PM_REGISTRY:-$PMT_ROOT/registry/index.json}"
PMT_MIYU_HOME="${MIYU_PM_MIYU_HOME:-$HOME/.miyu}"
PMT_PM_HOME="${MIYU_PM_PM_HOME:-$HOME/.miyu-pm}"
PMT_STATE_FILE="${PMT_PM_HOME}/state/installed.json"
PMT_NO_SETUP="${MIYU_PM_TUI_NO_SETUP:-0}"
PMT_DRY_RUN="${MIYU_PM_TUI_DRY_RUN:-0}"

# --- small helpers -----------------------------------------------------------

pmt_err() {
    echo "miyu-pm TUI: $*" >&2
}

pmt_require() {
    for cmd in fzf jq; do
        if ! command -v "$cmd" >/dev/null 2>&1; then
            pmt_err "missing required command: $cmd"
            return 1
        fi
    done
}

pmt_require_registry() {
    if [[ ! -r "$PMT_REGISTRY" ]]; then
        pmt_err "registry not readable: $PMT_REGISTRY"
        pmt_err "set MIYU_PM_REGISTRY or run from the miyu-pm project root"
        return 1
    fi
}

pmt_miyu() {
    "$PMT_BIN" \
        --miyu-home "$PMT_MIYU_HOME" \
        --pm-home "$PMT_PM_HOME" \
        --registry "$PMT_REGISTRY" \
        "$@"
}

pmt_install_extra_args() {
    if [[ "$PMT_NO_SETUP" == "1" ]]; then
        printf '%s' "--no-setup"
    fi
    if [[ "$PMT_DRY_RUN" == "1" ]]; then
        printf '%s' "--dry-run"
    fi
}

pmt_fzf_base() {
    local header="$1"
    fzf \
        --multi \
        --height=80% \
        --layout=reverse \
        --border \
        --header="$header" \
        --bind 'ctrl-a:select-all,ctrl-d:deselect-all' \
        --preview-window='right:55%:wrap'
}

pmt_preview_cmd() {
    # Consumed by fzf's --preview. fzf exports {} as the current line.
    local name
    name="$(printf '%s' "$1" | cut -f1)"
    if [[ -z "$name" ]]; then
        return 0
    fi
    pmt_miyu info "$name" 2>/dev/null || true
    echo ""
    echo "--- audit preview ---"
    pmt_miyu audit "$name" 2>/dev/null || true
}

pmt_selected_names() {
    # stdin: fzf selected lines (tab separated); stdout: one package name per line
    cut -f1
}

pmt_available_rows() {
    jq -r '
        .packages[]
        | [.name, ("[" + .type + "]"), ("v" + .version), (.display_name // "")]
        | @tsv
    ' "$PMT_REGISTRY"
}

pmt_installed_rows() {
    if [[ ! -r "$PMT_STATE_FILE" ]]; then
        return 0
    fi
    jq -r '
        .installed[]
        | [.name, ("[" + .type + "]"), ("v" + .version), (.mcp_id // "")]
        | @tsv
    ' "$PMT_STATE_FILE"
}
