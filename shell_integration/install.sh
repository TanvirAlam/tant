#!/usr/bin/env bash
# Tant Terminal Shell Integration Installer
# This script helps users set up shell integration for their preferred shell

set -e

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
INSTALL_DIR="${TANT_INSTALL_DIR:-$HOME/.tant}"

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m' # No Color

print_header() {
    echo -e "${BLUE}========================================${NC}"
    echo -e "${BLUE}  Tant Shell Integration Installer${NC}"
    echo -e "${BLUE}========================================${NC}"
    echo
}

print_success() {
    echo -e "${GREEN}✓${NC} $1"
}

print_error() {
    echo -e "${RED}✗${NC} $1"
}

print_warning() {
    echo -e "${YELLOW}⚠${NC} $1"
}

print_info() {
    echo -e "${BLUE}ℹ${NC} $1"
}

detect_shell() {
    if [ -n "$BASH_VERSION" ]; then
        echo "bash"
    elif [ -n "$ZSH_VERSION" ]; then
        echo "zsh"
    elif [ -n "$FISH_VERSION" ]; then
        echo "fish"
    else
        # Fallback to $SHELL
        basename "$SHELL"
    fi
}

install_for_bash() {
    local rc_file="${HOME}/.bashrc"
    local source_line="source \"${INSTALL_DIR}/tant.bash\""
    
    print_info "Installing for Bash..."
    
    # Create install directory
    mkdir -p "$INSTALL_DIR"
    
    # Copy integration script
    cp "${SCRIPT_DIR}/tant.bash" "${INSTALL_DIR}/tant.bash"
    print_success "Copied tant.bash to ${INSTALL_DIR}"
    
    # Check if already sourced
    if grep -q "tant.bash" "$rc_file" 2>/dev/null; then
        print_warning "Shell integration already present in ${rc_file}"
        return
    fi
    
    # Add source line to bashrc
    echo "" >> "$rc_file"
    echo "# Tant Terminal Shell Integration" >> "$rc_file"
    echo "$source_line" >> "$rc_file"
    
    print_success "Added integration to ${rc_file}"
    print_info "Restart your shell or run: source ${rc_file}"
}

install_for_zsh() {
    local rc_file="${HOME}/.zshrc"
    local source_line="source \"${INSTALL_DIR}/tant.zsh\""
    
    print_info "Installing for Zsh..."
    
    # Create install directory
    mkdir -p "$INSTALL_DIR"
    
    # Copy integration script
    cp "${SCRIPT_DIR}/tant.zsh" "${INSTALL_DIR}/tant.zsh"
    print_success "Copied tant.zsh to ${INSTALL_DIR}"
    
    # Check if already sourced
    if grep -q "tant.zsh" "$rc_file" 2>/dev/null; then
        print_warning "Shell integration already present in ${rc_file}"
        return
    fi
    
    # Add source line to zshrc
    echo "" >> "$rc_file"
    echo "# Tant Terminal Shell Integration" >> "$rc_file"
    echo "$source_line" >> "$rc_file"
    
    print_success "Added integration to ${rc_file}"
    print_info "Restart your shell or run: source ${rc_file}"
}

install_for_fish() {
    local config_dir="${HOME}/.config/fish/conf.d"
    
    print_info "Installing for Fish..."
    
    # Create config directory
    mkdir -p "$config_dir"
    
    # Copy integration script directly to Fish config
    cp "${SCRIPT_DIR}/tant.fish" "${config_dir}/tant.fish"
    print_success "Copied tant.fish to ${config_dir}"
    
    print_info "Restart your shell or run: source ${config_dir}/tant.fish"
}

uninstall() {
    print_info "Uninstalling Tant shell integration..."
    
    # Remove from bashrc
    if [ -f "${HOME}/.bashrc" ]; then
        sed -i.bak '/tant.bash/d' "${HOME}/.bashrc"
        sed -i.bak '/Tant Terminal Shell Integration/d' "${HOME}/.bashrc"
        print_success "Removed from .bashrc"
    fi
    
    # Remove from zshrc
    if [ -f "${HOME}/.zshrc" ]; then
        sed -i.bak '/tant.zsh/d' "${HOME}/.zshrc"
        sed -i.bak '/Tant Terminal Shell Integration/d' "${HOME}/.zshrc"
        print_success "Removed from .zshrc"
    fi
    
    # Remove from fish
    if [ -f "${HOME}/.config/fish/conf.d/tant.fish" ]; then
        rm "${HOME}/.config/fish/conf.d/tant.fish"
        print_success "Removed from fish config"
    fi
    
    # Remove install directory
    if [ -d "$INSTALL_DIR" ]; then
        rm -rf "$INSTALL_DIR"
        print_success "Removed ${INSTALL_DIR}"
    fi
    
    print_success "Uninstallation complete!"
}

show_help() {
    echo "Usage: $0 [OPTIONS]"
    echo
    echo "Options:"
    echo "  --shell SHELL    Install for specific shell (bash, zsh, fish)"
    echo "  --uninstall      Remove shell integration"
    echo "  --help           Show this help message"
    echo
    echo "If no shell is specified, the script will auto-detect your current shell."
}

main() {
    print_header
    
    local target_shell=""
    local do_uninstall=false
    
    # Parse arguments
    while [[ $# -gt 0 ]]; do
        case $1 in
            --shell)
                target_shell="$2"
                shift 2
                ;;
            --uninstall)
                do_uninstall=true
                shift
                ;;
            --help)
                show_help
                exit 0
                ;;
            *)
                print_error "Unknown option: $1"
                show_help
                exit 1
                ;;
        esac
    done
    
    # Handle uninstall
    if [ "$do_uninstall" = true ]; then
        uninstall
        exit 0
    fi
    
    # Detect shell if not specified
    if [ -z "$target_shell" ]; then
        target_shell=$(detect_shell)
        print_info "Detected shell: $target_shell"
    fi
    
    # Install for target shell
    case "$target_shell" in
        bash)
            install_for_bash
            ;;
        zsh)
            install_for_zsh
            ;;
        fish)
            install_for_fish
            ;;
        *)
            print_error "Unsupported shell: $target_shell"
            print_info "Supported shells: bash, zsh, fish"
            exit 1
            ;;
    esac
    
    echo
    print_success "Installation complete!"
    echo
    print_info "Shell integration provides:"
    echo "  • Accurate command block detection"
    echo "  • Exit code tracking per command"
    echo "  • Command duration tracking"
    echo "  • Reliable prompt/command separation"
    echo
}

main "$@"
