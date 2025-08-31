#!/usr/bin/env bash
#
# hiproc Setup Script
# 
# This script automates the complete installation of hiproc:
# - Installs config file to ~/.config/hiproc/config.toml
# - Builds the hp binary using Cargo
# - Copies the hp binary to ~/bin/
# - Sets up shell completions (optional)
#

set -euo pipefail  # Exit on error, undefined vars, pipe failures

# Parse command line arguments
DRY_RUN=true
INTERACTIVE=true

while [[ $# -gt 0 ]]; do
    case $1 in
        --install|--run)
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
            echo "  --install, --run    Actually perform the installation (default: dry-run only)"
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
readonly SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
readonly CONFIG_DIR="$HOME/.config/hiproc"
readonly CONFIG_FILE="$CONFIG_DIR/config.toml"
readonly BIN_DIR="$HOME/bin"
readonly SOURCE_CONFIG="$SCRIPT_DIR/hiproc.toml"
readonly RUST_PROJECT_DIR="$SCRIPT_DIR/rust/hp"
readonly BINARY_NAME="hp"

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

# Error handling
error_exit() {
    log_error "$1"
    exit 1
}

# Check if command exists
command_exists() {
    command -v "$1" >/dev/null 2>&1
}

# Display header
display_header() {
    echo
    if [[ "$DRY_RUN" == true ]]; then
        echo -e "${MAGENTA}╔══════════════════════════════════════════╗${NC}"
        echo -e "${MAGENTA}║${NC}        hiproc Setup Script (DRY RUN)     ${MAGENTA}║${NC}"
        echo -e "${MAGENTA}║${NC}     Command Memory & Execution Tool      ${MAGENTA}║${NC}"
        echo -e "${MAGENTA}╚══════════════════════════════════════════╝${NC}"
        echo
        echo -e "${MAGENTA}This is a DRY RUN - no changes will be made to your system.${NC}"
        echo -e "${MAGENTA}Run with --install to actually perform the installation.${NC}"
        echo
    else
        echo -e "${BLUE}╔══════════════════════════════════════════╗${NC}"
        echo -e "${BLUE}║${NC}           hiproc Setup Script            ${BLUE}║${NC}"
        echo -e "${BLUE}║${NC}     Command Memory & Execution Tool     ${BLUE}║${NC}"
        echo -e "${BLUE}╚══════════════════════════════════════════╝${NC}"
        echo
        echo -e "${GREEN}INSTALLATION MODE - Changes will be made to your system.${NC}"
        echo
    fi
}

# Check prerequisites
check_prerequisites() {
    log_info "Checking prerequisites..."
    
    # Check if we're in the right directory
    if [[ ! -f "$SOURCE_CONFIG" ]]; then
        error_exit "hiproc.toml not found. Please run this script from the hiproc project root directory."
    fi
    
    if [[ ! -d "$RUST_PROJECT_DIR" ]]; then
        error_exit "Rust project directory not found at: $RUST_PROJECT_DIR"
    fi
    
    # Check for required tools
    if ! command_exists cargo; then
        error_exit "Cargo is not installed. Please install Rust and Cargo first: https://rustup.rs/"
    fi
    
    if ! command_exists rustc; then
        error_exit "Rust compiler is not installed. Please install Rust first: https://rustup.rs/"
    fi
    
    if ! command_exists uv; then
        log_info "uv not found. Installing uv using pip..."
        if ! command_exists pip && ! command_exists pip3; then
            error_exit "Neither uv nor pip is available. Please install Python and pip first."
        fi
        
        # Try pip3 first, then pip
        local pip_cmd="pip3"
        if ! command_exists pip3; then
            pip_cmd="pip"
        fi
        
        log_info "Running: $pip_cmd install uv -U"
        if ! $pip_cmd install uv -U; then
            error_exit "Failed to install uv. Please install it manually: https://docs.astral.sh/uv/getting-started/installation/"
        fi
        
        # Try to find uv after installation
        find_and_use_uv() {
            # First, try rehashing the shell's command cache
            log_info "Refreshing shell command cache..."
            hash -r 2>/dev/null || true
            
            # Check if uv is now available
            if command_exists uv; then
                log_success "uv found in PATH after cache refresh"
                return 0
            fi
            
            # Try to find uv in common locations relative to pip
            log_info "Searching for uv binary in common locations..."
            local pip_path=""
            
            # Find pip location
            if command_exists which; then
                pip_path=$(which "$pip_cmd" 2>/dev/null)
            elif command_exists where; then
                pip_path=$(where "$pip_cmd" 2>/dev/null | head -n1)
            elif command_exists type; then
                pip_path=$(type -p "$pip_cmd" 2>/dev/null)
            fi
            
            if [[ -n "$pip_path" && -f "$pip_path" ]]; then
                local pip_dir=$(dirname "$pip_path")
                log_info "Found pip at: $pip_path"
                log_info "Checking directory: $pip_dir"
                
                # Look for uv in the same directory as pip
                if [[ -x "$pip_dir/uv" ]]; then
                    log_info "Found uv at: $pip_dir/uv"
                    # Create a function to use the full path
                    uv() { "$pip_dir/uv" "$@"; }
                    export -f uv 2>/dev/null || true
                    log_success "uv is now available (using full path)"
                    return 0
                fi
            fi
            
            # Try common Python user install locations
            local common_paths=(
                "$HOME/.local/bin/uv"
                "$HOME/Library/Python/*/bin/uv" 
                "/usr/local/bin/uv"
                "$HOME/.cargo/bin/uv"
                # Also try python -m site --user-base locations
                "$(python3 -m site --user-base 2>/dev/null)/bin/uv"
                "$(python -m site --user-base 2>/dev/null)/bin/uv"
            )
            
            for path_pattern in "${common_paths[@]}"; do
                # Handle glob patterns
                for uv_path in $path_pattern; do
                    if [[ -x "$uv_path" ]]; then
                        log_info "Found uv at: $uv_path"
                        # Create a function to use the full path
                        uv() { "$uv_path" "$@"; }
                        export -f uv 2>/dev/null || true
                        log_success "uv is now available (using full path)"
                        return 0
                    fi
                done
            done
            
            return 1
        }
        
        # Try to find and configure uv
        if ! find_and_use_uv; then
            error_exit "uv was installed but could not be found. Please restart your shell and try again, or install uv manually: https://docs.astral.sh/uv/getting-started/installation/"
        fi
        
        # Verify uv is working
        log_info "Verifying uv installation..."
        if ! uv --version >/dev/null 2>&1; then
            error_exit "uv was found but is not working correctly. Please install uv manually: https://docs.astral.sh/uv/getting-started/installation/"
        fi
        
        log_success "uv installation verified and working"
    fi
    
    log_success "Prerequisites check passed"
}

# Perform dry-run analysis
perform_dry_run_analysis() {
    if [[ "$DRY_RUN" != true ]]; then
        return 0
    fi
    
    echo
    log_dry_run "Analyzing what would be installed..."
    echo
    
    # Configuration analysis
    echo -e "${BLUE}Configuration:${NC}"
    log_would_do "Check/create config directory: $CONFIG_DIR"
    
    if [[ "$INTERACTIVE" == true ]]; then
        echo -e "  ${MAGENTA}→${NC} You will be asked where to install config file:"
        echo -e "     1. Global: $CONFIG_DIR/config.toml"
        echo -e "     2. Binary: $BIN_DIR/hiproc.toml (portable)"
        echo -e "     3. Both locations"
    else
        echo -e "  ${MAGENTA}→${NC} Default: Install global config at $CONFIG_DIR/config.toml"
    fi
    
    if [[ -f "$CONFIG_DIR/config.toml" ]]; then
        log_would_do "Existing global config found - would ask to overwrite"
    fi
    if [[ -f "$BIN_DIR/hiproc.toml" ]]; then
        log_would_do "Existing binary config found - would ask to overwrite"
    fi
    
    echo
    
    # Prerequisites analysis
    echo -e "${BLUE}Prerequisites:${NC}"
    if ! command_exists uv; then
        log_would_do "Install uv using pip (uv not found)"
        log_would_do "Refresh shell command cache (hash -r)"
        log_would_do "Search for uv in common Python install locations"
        
        # Show what paths would be searched
        echo -e "  ${MAGENTA}→${NC} Would search for uv in:"
        echo -e "     • Same directory as pip"
        echo -e "     • $HOME/.local/bin/"
        echo -e "     • $HOME/Library/Python/*/bin/"
        echo -e "     • /usr/local/bin/"
        echo -e "     • $HOME/.cargo/bin/"
    else
        echo -e "  ${MAGENTA}→${NC} uv is already installed"
    fi
    
    echo
    
    # Build analysis
    echo -e "${BLUE}Build Process:${NC}"
    log_would_do "Build Rust binary: cargo build --release in $RUST_PROJECT_DIR"
    log_would_do "Binary will be created at: $RUST_PROJECT_DIR/target/release/$BINARY_NAME"
    
    if [[ -f "$SCRIPT_DIR/pyproject.toml" ]]; then
        log_would_do "Setup Python environment: uv sync in $SCRIPT_DIR"
        log_would_do "Python server will be available via: uv run hiproc"
    else
        echo -e "  ${MAGENTA}→${NC} No pyproject.toml found - would skip Python environment setup"
    fi
    
    echo
    
    # Installation analysis
    echo -e "${BLUE}Binary Installation:${NC}"
    log_would_do "Check/create bin directory: $BIN_DIR"
    log_would_do "Copy binary: $RUST_PROJECT_DIR/target/release/$BINARY_NAME → $BIN_DIR/$BINARY_NAME"
    log_would_do "Make binary executable: chmod +x $BIN_DIR/$BINARY_NAME"
    
    if [[ -f "$BIN_DIR/$BINARY_NAME" ]]; then
        log_would_do "Existing binary found - would ask to overwrite"
    fi
    
    echo
    
    # Shell completions analysis
    echo -e "${BLUE}Shell Completions:${NC}"
    if [[ "$INTERACTIVE" == true ]]; then
        log_would_do "Ask if you want to set up shell completions"
        local shell_name=$(basename "$SHELL")
        echo -e "  ${MAGENTA}→${NC} Detected shell: $shell_name"
        case "$shell_name" in
            bash)
                echo -e "  ${MAGENTA}→${NC} Would install to: $HOME/.local/share/bash-completion/completions/hp"
                ;;
            zsh)
                echo -e "  ${MAGENTA}→${NC} Would install to: $HOME/.zsh/completions/hp"
                ;;
            fish)
                echo -e "  ${MAGENTA}→${NC} Would install to: $HOME/.config/fish/completions/hp.fish"
                ;;
            *)
                echo -e "  ${MAGENTA}→${NC} Unsupported shell - would skip completions"
                ;;
        esac
    else
        log_would_do "Skip shell completions (non-interactive mode)"
    fi
    
    echo
    
    # PATH analysis
    echo -e "${BLUE}PATH Configuration:${NC}"
    if [[ ":$PATH:" == *":$BIN_DIR:"* ]]; then
        log_would_do "$BIN_DIR is already in your PATH - binary will be accessible"
    else
        log_would_do "$BIN_DIR is NOT in your PATH - would show instructions to add it"
        echo -e "  ${MAGENTA}→${NC} Would suggest adding: export PATH=\"\$HOME/bin:\$PATH\""
    fi
    
    echo
    
    # Summary
    echo -e "${MAGENTA}╔══════════════════════════════════════════╗${NC}"
    echo -e "${MAGENTA}║${NC}              DRY RUN COMPLETE            ${MAGENTA}║${NC}"
    echo -e "${MAGENTA}╚══════════════════════════════════════════╝${NC}"
    echo
    echo -e "${YELLOW}To actually perform the installation, run:${NC}"
    echo -e "  ${GREEN}$0 --install${NC}"
    echo
    echo -e "${YELLOW}To run non-interactively with defaults:${NC}"
    echo -e "  ${GREEN}$0 --install --yes${NC}"
    echo
    
    exit 0
}

# Setup configuration
setup_config() {
    log_info "Setting up configuration..."
    
    echo "hiproc supports three config file locations (in order of precedence):"
    echo "  1. Global: ~/.config/hiproc/config.toml"
    echo "  2. Binary directory: ~/bin/hiproc.toml (portable)"
    echo "  3. Local: ./hiproc.toml (project-specific, highest precedence)"
    echo
    
    # Ask user preference
    echo "Where would you like to install the config file?"
    echo "  1) Global (~/.config/hiproc/config.toml) - Recommended"
    echo "  2) Binary directory (~/bin/hiproc.toml) - Portable"
    echo "  3) Both locations"
    read -p "Enter your choice (1/2/3): " -n 1 -r
    echo
    
    case $REPLY in
        1)
            install_global_config
            ;;
        2)
            install_binary_config
            ;;
        3)
            install_global_config
            install_binary_config
            ;;
        *)
            log_info "Invalid choice. Installing global config by default."
            install_global_config
            ;;
    esac
}

# Install global config
install_global_config() {
    log_info "Installing global configuration..."
    
    # Create config directory if it doesn't exist
    if [[ ! -d "$CONFIG_DIR" ]]; then
        log_info "Creating config directory: $CONFIG_DIR"
        mkdir -p "$CONFIG_DIR" || error_exit "Failed to create config directory"
    fi
    
    # Install config file
    if [[ -f "$CONFIG_FILE" ]]; then
        log_warning "Global config file already exists at: $CONFIG_FILE"
        read -p "Do you want to overwrite it? (y/N): " -n 1 -r
        echo
        if [[ ! $REPLY =~ ^[Yy]$ ]]; then
            log_info "Skipping global config file installation"
            return 0
        fi
    fi
    
    log_info "Installing global config file: $SOURCE_CONFIG -> $CONFIG_FILE"
    cp "$SOURCE_CONFIG" "$CONFIG_FILE" || error_exit "Failed to copy global config file"
    
    log_success "Global configuration installed successfully"
}

# Install binary-directory config
install_binary_config() {
    log_info "Installing binary-directory configuration..."
    
    local binary_config="$BIN_DIR/hiproc.toml"
    
    # Make sure bin directory exists
    if [[ ! -d "$BIN_DIR" ]]; then
        log_info "Creating bin directory: $BIN_DIR"
        mkdir -p "$BIN_DIR" || error_exit "Failed to create bin directory"
    fi
    
    # Install config file
    if [[ -f "$binary_config" ]]; then
        log_warning "Binary-directory config file already exists at: $binary_config"
        read -p "Do you want to overwrite it? (y/N): " -n 1 -r
        echo
        if [[ ! $REPLY =~ ^[Yy]$ ]]; then
            log_info "Skipping binary-directory config file installation"
            return 0
        fi
    fi
    
    log_info "Installing binary-directory config file: $SOURCE_CONFIG -> $binary_config"
    cp "$SOURCE_CONFIG" "$binary_config" || error_exit "Failed to copy binary-directory config file"
    
    log_success "Binary-directory configuration installed successfully"
}

# Build the binary
build_binary() {
    log_info "Building hiproc binary..."
    
    cd "$RUST_PROJECT_DIR" || error_exit "Failed to enter Rust project directory"
    
    log_info "Running: cargo build --release"
    if ! cargo build --release; then
        error_exit "Failed to build hiproc binary"
    fi
    
    # Verify binary was created
    local binary_path="$RUST_PROJECT_DIR/target/release/$BINARY_NAME"
    if [[ ! -f "$binary_path" ]]; then
        error_exit "Binary not found after build: $binary_path"
    fi
    
    log_success "Binary built successfully: $binary_path"
    cd - > /dev/null
}

# Setup Python environment
setup_python_environment() {
    log_info "Setting up Python environment..."
    
    # Check if we have Python dependencies
    if [[ ! -f "$SCRIPT_DIR/pyproject.toml" ]]; then
        log_warning "No pyproject.toml found, skipping Python environment setup"
        return 0
    fi
    
    cd "$SCRIPT_DIR" || error_exit "Failed to enter project directory"
    
    log_info "Running: uv sync"
    if ! uv sync; then
        error_exit "Failed to setup Python environment with uv sync"
    fi
    
    log_success "Python environment setup completed"
    log_info "You can now run the server with: uv run hiproc"
    cd - > /dev/null
}

# Install the binary
install_binary() {
    log_info "Installing hiproc binary..."
    
    # Create bin directory if it doesn't exist
    if [[ ! -d "$BIN_DIR" ]]; then
        log_info "Creating bin directory: $BIN_DIR"
        mkdir -p "$BIN_DIR" || error_exit "Failed to create bin directory"
    fi
    
    local source_binary="$RUST_PROJECT_DIR/target/release/$BINARY_NAME"
    local target_binary="$BIN_DIR/$BINARY_NAME"
    
    # Check if binary already exists
    if [[ -f "$target_binary" ]]; then
        log_warning "Binary already exists at: $target_binary"
        read -p "Do you want to overwrite it? (y/N): " -n 1 -r
        echo
        if [[ ! $REPLY =~ ^[Yy]$ ]]; then
            log_info "Skipping binary installation"
            return 0
        fi
    fi
    
    log_info "Copying binary: $source_binary -> $target_binary"
    cp "$source_binary" "$target_binary" || error_exit "Failed to copy binary"
    
    # Make sure it's executable
    chmod +x "$target_binary" || error_exit "Failed to make binary executable"
    
    log_success "Binary installed successfully: $target_binary"
}

# Setup shell completions (optional)
setup_completions() {
    log_info "Setting up shell completions (optional)..."
    
    read -p "Do you want to set up shell completions? (y/N): " -n 1 -r
    echo
    if [[ ! $REPLY =~ ^[Yy]$ ]]; then
        log_info "Skipping shell completions"
        return 0
    fi
    
    # Detect shell
    local shell_name=$(basename "$SHELL")
    local completion_dir=""
    
    case "$shell_name" in
        bash)
            completion_dir="$HOME/.local/share/bash-completion/completions"
            ;;
        zsh)
            completion_dir="$HOME/.zsh/completions"
            ;;
        fish)
            completion_dir="$HOME/.config/fish/completions"
            ;;
        *)
            log_warning "Unsupported shell: $shell_name. Skipping completions."
            return 0
            ;;
    esac
    
    # Create completion directory
    mkdir -p "$completion_dir" || error_exit "Failed to create completion directory"
    
    # Generate completions
    local completion_file="$completion_dir/hp"
    if [[ "$shell_name" == "fish" ]]; then
        completion_file="$completion_dir/hp.fish"
    fi
    
    log_info "Generating $shell_name completions: $completion_file"
    "$BIN_DIR/$BINARY_NAME" generate-completions "$shell_name" > "$completion_file" || {
        log_warning "Failed to generate completions, but installation continues"
        return 0
    }
    
    log_success "Shell completions installed for $shell_name"
    log_info "You may need to restart your shell or source your profile"
}

# Check PATH
check_path() {
    log_info "Checking PATH configuration..."
    
    if [[ ":$PATH:" == *":$BIN_DIR:"* ]]; then
        log_success "$BIN_DIR is already in your PATH"
    else
        log_warning "$BIN_DIR is not in your PATH"
        echo
        echo "To use 'hp' from anywhere, add this line to your shell profile:"
        echo -e "${YELLOW}export PATH=\"\$HOME/bin:\$PATH\"${NC}"
        echo
        echo "Common shell profiles:"
        echo "  - Bash: ~/.bashrc or ~/.bash_profile"
        echo "  - Zsh: ~/.zshrc"
        echo "  - Fish: ~/.config/fish/config.fish"
    fi
}

# Test installation
test_installation() {
    log_info "Testing installation..."
    
    # Test if binary is accessible and working
    if command_exists "$BIN_DIR/$BINARY_NAME"; then
        log_success "hp binary is accessible"
        
        # Try to run help command
        if "$BIN_DIR/$BINARY_NAME" --help >/dev/null 2>&1; then
            log_success "hp binary is working correctly"
        else
            log_warning "hp binary exists but may have issues"
        fi
    else
        log_warning "hp binary is not in PATH. You'll need to use the full path: $BIN_DIR/$BINARY_NAME"
    fi
    
    # Test config file
    if [[ -f "$CONFIG_FILE" ]]; then
        log_success "Configuration file is installed"
    else
        log_error "Configuration file is missing"
    fi
}

# Display summary
display_summary() {
    echo
    echo -e "${GREEN}╔══════════════════════════════════════════╗${NC}"
    echo -e "${GREEN}║${NC}          Installation Complete!          ${GREEN}║${NC}"
    echo -e "${GREEN}╚══════════════════════════════════════════╝${NC}"
    echo
    echo "Installation Summary:"
    echo -e "  ${BLUE}•${NC} Config:  $CONFIG_FILE"
    echo -e "  ${BLUE}•${NC} Binary:  $BIN_DIR/$BINARY_NAME"
    echo
    echo "Next steps:"
    echo -e "  ${BLUE}1.${NC} Make sure $BIN_DIR is in your PATH"
    echo -e "  ${BLUE}2.${NC} Start the hiproc server: ${GREEN}uv run hiproc${NC}"
    echo -e "  ${BLUE}3.${NC} In another terminal, try: ${GREEN}hp --help${NC}"
    echo
    echo "Happy command management! 🚀"
    echo
}

# Main function
main() {
    display_header
    check_prerequisites
    perform_dry_run_analysis
    
    # If we get here, we're in installation mode
    setup_config
    build_binary
    setup_python_environment
    install_binary
    setup_completions
    check_path
    test_installation
    display_summary
}

# Run main function
main "$@"
