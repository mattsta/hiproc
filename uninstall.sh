#!/bin/bash
#
# hiproc Uninstall Script
# 
# This script removes the hiproc installation:
# - Removes config file from ~/.config/hiproc/
# - Removes hp binary from ~/bin/
# - Removes shell completions (optional)
#

set -euo pipefail  # Exit on error, undefined vars, pipe failures

# Parse command line arguments
DRY_RUN=true
INTERACTIVE=true

while [[ $# -gt 0 ]]; do
    case $1 in
        --uninstall|--run)
            DRY_RUN=false
            shift
            ;;
        --yes|-y)
            INTERACTIVE=false
            shift
            ;;
        --help|-h)
            echo "Usage: $0 [OPTIONS]"
            echo "Options:"
            echo "  --uninstall, --run  Actually perform the uninstallation (default: dry-run only)"
            echo "  --yes, -y          Skip interactive prompts (use defaults)"
            echo "  --help, -h         Show this help message"
            exit 0
            ;;
        *)
            echo "Unknown option: $1"
            echo "Use --help for usage information"
            exit 1
            ;;
    esac
done

# Colors for output
readonly RED='\033[0;31m'
readonly GREEN='\033[0;32m'
readonly BLUE='\033[0;34m'
readonly YELLOW='\033[1;33m'
readonly MAGENTA='\033[0;35m'
readonly NC='\033[0m' # No Color

# Configuration
readonly CONFIG_DIR="$HOME/.config/hiproc"
readonly CONFIG_FILE="$CONFIG_DIR/config.toml"
readonly BIN_DIR="$HOME/bin"
readonly BINARY_NAME="hp"
readonly BINARY_PATH="$BIN_DIR/$BINARY_NAME"
readonly BINARY_CONFIG="$BIN_DIR/hiproc.toml"

# Logging functions
log_info() {
    echo -e "${BLUE}[INFO]${NC} $1"
}

log_success() {
    echo -e "${GREEN}[SUCCESS]${NC} $1"
}

log_warning() {
    echo -e "${YELLOW}[WARNING]${NC} $1"
}

log_error() {
    echo -e "${RED}[ERROR]${NC} $1" >&2
}

log_dry_run() {
    echo -e "${MAGENTA}[DRY-RUN]${NC} $1"
}

log_would_do() {
    echo -e "${MAGENTA}[WOULD]${NC} $1"
}

# Display header
display_header() {
    echo
    if [[ "$DRY_RUN" == true ]]; then
        echo -e "${MAGENTA}╔══════════════════════════════════════════╗${NC}"
        echo -e "${MAGENTA}║${NC}      hiproc Uninstall Script (DRY RUN)   ${MAGENTA}║${NC}"
        echo -e "${MAGENTA}║${NC}     Remove Command Memory Tool           ${MAGENTA}║${NC}"
        echo -e "${MAGENTA}╚══════════════════════════════════════════╝${NC}"
        echo
        echo -e "${MAGENTA}This is a DRY RUN - no changes will be made to your system.${NC}"
        echo -e "${MAGENTA}Run with --uninstall to actually remove hiproc.${NC}"
        echo
    else
        echo -e "${RED}╔══════════════════════════════════════════╗${NC}"
        echo -e "${RED}║${NC}          hiproc Uninstall Script         ${RED}║${NC}"
        echo -e "${RED}║${NC}     Remove Command Memory Tool          ${RED}║${NC}"
        echo -e "${RED}╚══════════════════════════════════════════╝${NC}"
        echo
        echo -e "${YELLOW}REMOVAL MODE - Files will be deleted from your system.${NC}"
        echo
    fi
}

# Perform dry-run analysis
perform_dry_run_analysis() {
    if [[ "$DRY_RUN" != true ]]; then
        return 0
    fi
    
    echo
    log_dry_run "Analyzing what would be removed..."
    echo
    
    local found_items=false
    
    # Configuration analysis
    echo -e "${BLUE}Configuration Files:${NC}"
    if [[ -f "$CONFIG_FILE" ]]; then
        log_would_do "Remove global config: $CONFIG_FILE"
        found_items=true
    else
        echo -e "  ${MAGENTA}→${NC} Global config not found: $CONFIG_FILE"
    fi
    
    if [[ -f "$BINARY_CONFIG" ]]; then
        log_would_do "Remove binary config: $BINARY_CONFIG"
        found_items=true
    else
        echo -e "  ${MAGENTA}→${NC} Binary config not found: $BINARY_CONFIG"
    fi
    
    if [[ -d "$CONFIG_DIR" ]]; then
        if [[ $(find "$CONFIG_DIR" -type f | wc -l) -eq 0 ]] || [[ $(find "$CONFIG_DIR" -type f | wc -l) -eq 1 && -f "$CONFIG_FILE" ]]; then
            log_would_do "Remove empty config directory: $CONFIG_DIR"
        else
            echo -e "  ${MAGENTA}→${NC} Config directory not empty, would keep: $CONFIG_DIR"
        fi
    fi
    
    echo
    
    # Binary analysis
    echo -e "${BLUE}Binary:${NC}"
    if [[ -f "$BINARY_PATH" ]]; then
        log_would_do "Remove binary: $BINARY_PATH"
        found_items=true
    else
        echo -e "  ${MAGENTA}→${NC} Binary not found: $BINARY_PATH"
    fi
    
    echo
    
    # Shell completions analysis
    echo -e "${BLUE}Shell Completions:${NC}"
    if [[ "$INTERACTIVE" == true ]]; then
        log_would_do "Ask if you want to remove shell completions"
    else
        log_would_do "Skip completion removal (non-interactive mode)"
    fi
    
    local completions_found=false
    
    # Check for existing completions
    if [[ -f "$HOME/.local/share/bash-completion/completions/hp" ]]; then
        echo -e "  ${MAGENTA}→${NC} Found bash completion: $HOME/.local/share/bash-completion/completions/hp"
        completions_found=true
    fi
    
    if [[ -f "$HOME/.zsh/completions/hp" ]]; then
        echo -e "  ${MAGENTA}→${NC} Found zsh completion: $HOME/.zsh/completions/hp"
        completions_found=true
    fi
    
    if [[ -f "$HOME/.config/fish/completions/hp.fish" ]]; then
        echo -e "  ${MAGENTA}→${NC} Found fish completion: $HOME/.config/fish/completions/hp.fish"
        completions_found=true
    fi
    
    if [[ "$completions_found" == false ]]; then
        echo -e "  ${MAGENTA}→${NC} No shell completions found"
    fi
    
    echo
    
    # Summary
    if [[ "$found_items" == true ]]; then
        echo -e "${MAGENTA}╔══════════════════════════════════════════╗${NC}"
        echo -e "${MAGENTA}║${NC}              DRY RUN COMPLETE            ${MAGENTA}║${NC}"
        echo -e "${MAGENTA}╚══════════════════════════════════════════╝${NC}"
        echo
        echo -e "${YELLOW}hiproc installation found. To actually remove it, run:${NC}"
        echo -e "  ${RED}$0 --uninstall${NC}"
        echo
        echo -e "${YELLOW}To run non-interactively:${NC}"
        echo -e "  ${RED}$0 --uninstall --yes${NC}"
    else
        echo -e "${GREEN}╔══════════════════════════════════════════╗${NC}"
        echo -e "${GREEN}║${NC}         hiproc NOT FOUND                 ${GREEN}║${NC}"
        echo -e "${GREEN}╚══════════════════════════════════════════╝${NC}"
        echo
        echo -e "${GREEN}No hiproc installation found on this system.${NC}"
    fi
    
    echo
    exit 0
}

# Confirm uninstall
confirm_uninstall() {
    if [[ "$INTERACTIVE" == false ]]; then
        return 0
    fi
    
    echo -e "${YELLOW}This will remove:${NC}"
    [[ -f "$CONFIG_FILE" ]] && echo -e "  ${RED}•${NC} Global config: $CONFIG_FILE"
    [[ -d "$CONFIG_DIR" ]] && echo -e "  ${RED}•${NC} Config directory: $CONFIG_DIR"
    [[ -f "$BINARY_CONFIG" ]] && echo -e "  ${RED}•${NC} Binary config: $BINARY_CONFIG"
    [[ -f "$BINARY_PATH" ]] && echo -e "  ${RED}•${NC} Binary: $BINARY_PATH"
    echo -e "  ${RED}•${NC} Shell completions (if any)"
    echo
    
    read -p "Are you sure you want to uninstall hiproc? (y/N): " -n 1 -r
    echo
    if [[ ! $REPLY =~ ^[Yy]$ ]]; then
        log_info "Uninstall cancelled"
        exit 0
    fi
}

# Remove config files
remove_config() {
    log_info "Removing configuration..."
    
    # Remove global config file
    if [[ -f "$CONFIG_FILE" ]]; then
        log_info "Removing global config file: $CONFIG_FILE"
        rm -f "$CONFIG_FILE" || log_warning "Failed to remove global config file"
    else
        log_info "Global config file not found: $CONFIG_FILE"
    fi
    
    # Remove binary-directory config file
    if [[ -f "$BINARY_CONFIG" ]]; then
        log_info "Removing binary-directory config file: $BINARY_CONFIG"
        rm -f "$BINARY_CONFIG" || log_warning "Failed to remove binary-directory config file"
    else
        log_info "Binary-directory config file not found: $BINARY_CONFIG"
    fi
    
    # Remove config directory if empty
    if [[ -d "$CONFIG_DIR" ]]; then
        if rmdir "$CONFIG_DIR" 2>/dev/null; then
            log_success "Removed empty config directory: $CONFIG_DIR"
        else
            log_info "Config directory not empty, keeping: $CONFIG_DIR"
        fi
    fi
}

# Remove binary
remove_binary() {
    log_info "Removing binary..."
    
    if [[ -f "$BINARY_PATH" ]]; then
        log_info "Removing binary: $BINARY_PATH"
        rm -f "$BINARY_PATH" || log_warning "Failed to remove binary"
        log_success "Binary removed successfully"
    else
        log_info "Binary not found: $BINARY_PATH"
    fi
}

# Remove shell completions
remove_completions() {
    log_info "Removing shell completions..."
    
    read -p "Do you want to remove shell completions? (y/N): " -n 1 -r
    echo
    if [[ ! $REPLY =~ ^[Yy]$ ]]; then
        log_info "Skipping completion removal"
        return 0
    fi
    
    local removed_any=false
    
    # Bash completions
    local bash_completion="$HOME/.local/share/bash-completion/completions/hp"
    if [[ -f "$bash_completion" ]]; then
        log_info "Removing bash completion: $bash_completion"
        rm -f "$bash_completion" || log_warning "Failed to remove bash completion"
        removed_any=true
    fi
    
    # Zsh completions
    local zsh_completion="$HOME/.zsh/completions/hp"
    if [[ -f "$zsh_completion" ]]; then
        log_info "Removing zsh completion: $zsh_completion"
        rm -f "$zsh_completion" || log_warning "Failed to remove zsh completion"
        removed_any=true
    fi
    
    # Fish completions
    local fish_completion="$HOME/.config/fish/completions/hp.fish"
    if [[ -f "$fish_completion" ]]; then
        log_info "Removing fish completion: $fish_completion"
        rm -f "$fish_completion" || log_warning "Failed to remove fish completion"
        removed_any=true
    fi
    
    if [[ "$removed_any" == true ]]; then
        log_success "Shell completions removed"
    else
        log_info "No shell completions found"
    fi
}

# Display summary
display_summary() {
    echo
    echo -e "${GREEN}╔══════════════════════════════════════════╗${NC}"
    echo -e "${GREEN}║${NC}         Uninstall Complete!              ${GREEN}║${NC}"
    echo -e "${GREEN}╚══════════════════════════════════════════╝${NC}"
    echo
    echo "hiproc has been removed from your system."
    echo
    echo -e "${YELLOW}Note:${NC} If you added $BIN_DIR to your PATH in your shell"
    echo "profile, you may want to remove that line manually."
    echo
    echo "Thank you for using hiproc! 👋"
    echo
}

# Main function
main() {
    display_header
    perform_dry_run_analysis
    
    # If we get here, we're in uninstall mode
    confirm_uninstall
    remove_config
    remove_binary
    remove_completions
    display_summary
}

# Run main function
main "$@"
