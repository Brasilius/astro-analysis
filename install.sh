#!/bin/sh
# Install the TUI from source without requiring a repository checkout.
set -eu

main() {
    for command in cargo curl tar mktemp; do
        if ! command -v "$command" >/dev/null 2>&1; then
            printf 'Missing required command: %s\n' "$command" >&2
            printf 'Install Rust (https://rustup.rs), curl, and tar, then retry.\n' >&2
            exit 1
        fi
    done

    install_dir=${ASTRO_ANALYSIS_INSTALL_DIR:-"$HOME/.local/bin"}
    case "$install_dir" in
        /*) ;;
        *) printf 'ASTRO_ANALYSIS_INSTALL_DIR must be an absolute path.\n' >&2; exit 1 ;;
    esac

    work_dir=$(mktemp -d)
    trap 'rm -rf "$work_dir"' EXIT
    trap 'exit 1' HUP INT TERM

    printf 'Downloading astro-analysis source...\n'
    curl --fail --silent --show-error --location \
        https://codeload.github.com/Brasilius/astro-analysis/tar.gz/refs/heads/main \
        --output "$work_dir/source.tar.gz"
    mkdir "$work_dir/source"
    tar -xzf "$work_dir/source.tar.gz" -C "$work_dir/source" --strip-components=1

    printf 'Building astro-analysis (this may take a few minutes)...\n'
    cargo install --locked --path "$work_dir/source" --root "$work_dir/install"
    mkdir -p "$install_dir"
    cp "$work_dir/install/bin/astro-analysis" "$install_dir/astro-analysis"
    chmod 755 "$install_dir/astro-analysis"

    printf '\nInstalled %s/astro-analysis\n' "$install_dir"
    case ":$PATH:" in
        *":$install_dir:"*) ;;
        *)
            printf 'Add this directory to PATH in your shell configuration: %s\n' "$install_dir"
            printf 'For the default directory, run: export PATH="$HOME/.local/bin:$PATH"\n'
            ;;
    esac
    printf 'Start the TUI with: astro-analysis\n'
}

main "$@"
