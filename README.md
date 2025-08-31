# hiproc

## What / Why?

Had an idea for this system after my desktop kernel panicked and I lost all recent terminal history from 50 open terminal tabs.

Often I have specific commands for specific directories, but I don't always create individual management/run/build/server scripts in every single project for saving/remembering how the commands are built up and run with various arguments and parameters over time. Losing terminal history of carefully built up commands across a dozen active projects is always annoying and painful (especially if the saved terminal scrollback buffer also doesn't have the original commands either).

So, here's a weird server-client system where the server acts as a database and the client can save and restore specific commands.

The `hiproc` system logs commands you run with multiple scopes of:

- system name
- username
- current directory
- namespace
- custom command name
- command id (auto increment)

So if you want to "recall" a command in the future, you can run `hp suggest` to see previous commands you ran (with `hp save`) then you can have `hp` directly execute the saved command for you based on command id or command name scoped to current usage context (system, user, directory matches).

You can think of this system kinda like a shell history but for "more important commands" and it saves immediately instead of having problems with deciding between terminals sharing live history (disturbing to use) or terminals having unique history (then it gets corrupted/merged/forgotten on exit/crash).

The usage is fairly simple:

- Setup: `./setup.sh` to preview install operations then `./setup.sh --install` to actually install
- Run the server: `uv run hiproc`
- Use the client: `hp`

### Special Features

- The `hp` client binary is implemented in rust for best startup performance (avoids the 1-3 second "startup delay" if the command client were in python) and access to reusable libraries.
- The database runs as a standard fastapi web server, with command saved in sqlite, so all data should be quick to save/retrieve with minimal latency.
- You can use `hiproc` entirely over localhost, or you can run the server with a routeable IP address and have your `hp` client connect to a shared multi-system / multi-user history management service. You can even have `hiproc.toml` config files per directory if, for some reason, you want to have different directories talk to different backend `hiproc` servers when running `hp`.

### Provenance

Written with a combination of gemini 2.5 pro cli and claude sonnet 4 cli.

## Getting Started

**Your personal command-line memory, shared across systems.**

`hiproc` is a tool for saving, recalling, and organizing your command-line commands. It uses a central server to store your commands, making them accessible from any of your machines. A fast, native client (`hp`) provides a seamless way to interact with your command history.

The key feature of `hiproc` is **contextual recall**. When you save a command, `hiproc` remembers _who_ you were, _what machine_ you were on, and _what directory_ you were in. When you recall a command, it uses this context to find the most specific match first, preventing conflicts and ensuring you always run the right command for the job.

### Automated Setup (Recommended)

The fastest way to get started is with our automated setup script. It handles everything: building the binary, setting up Python environment, installing configuration, and setting up shell completions.

**Prerequisites:**

- Rust toolchain: Install via [rustup.rs](https://rustup.rs/)
- Python 3.10+ with pip
- uv (will be installed automatically if not present)

**One-Command Setup:**

1. **See what will be installed (dry-run):**

   ```bash
   ./setup.sh
   ```

   This shows you exactly what the script will do without making any changes.

2. **Install hiproc:**

   ```bash
   ./setup.sh --install
   ```

   This will:
   - Build the Rust `hp` binary
   - Set up Python environment with `uv sync`
   - Install config files
   - Copy binary to `~/bin/`
   - Set up shell completions (optional)

3. **Start the server:**

   ```bash
   uv run hiproc
   ```

   The server runs on `http://127.0.0.1:8128` by default.

4. **Test the client:**
   ```bash
   hp --help
   ```

**Non-Interactive Setup:**

```bash
./setup.sh --install --yes    # Use defaults, skip prompts
```

### Configuration Locations

The `hp` client loads configuration from three locations (in order of precedence):

1. **Global Config:** `~/.config/hiproc/config.toml` (recommended)
2. **Binary-Adjacent Config:** `~/bin/hiproc.toml` (portable)
3. **Local Config:** `./hiproc.toml` (project-specific, highest precedence)

**Example config file:**

```toml
server_url = "http://127.0.0.1:8128"
```

### Manual Setup (Alternative)

If you prefer manual setup or need custom configuration:

<details>
<summary>Click to expand manual setup instructions</summary>

#### 1. Python Server Setup

```bash
# Install dependencies
uv sync

# Run the server
uv run hiproc

# Custom host/port
uv run hiproc -- --host 0.0.0.0 --port 9999
```

#### 2. Rust Client Setup

```bash
# Build the binary
cd rust/hp
cargo build --release

# Install manually
cp target/release/hp ~/bin/hp  # or /usr/local/bin/hp
```

#### 3. Configure Client

Create `~/.config/hiproc/config.toml`:

```toml
server_url = "http://127.0.0.1:8128"
```

</details>

### Uninstalling

To remove hiproc from your system:

```bash
# See what will be removed (dry-run)
./uninstall.sh

# Actually remove hiproc
./uninstall.sh --uninstall

# Non-interactive removal
./uninstall.sh --uninstall --yes
```

This removes:

- Configuration files (`~/.config/hiproc/` and `~/bin/hiproc.toml`)
- Binary (`~/bin/hp`)
- Shell completions (optional)

## Web Interface

The `hiproc` server includes a simple, read-only web interface for browsing and searching commands. Once the server is running, you can access it by navigating to the server's address in your web browser (e.g., `http://127.0.0.1:8128`).

You will need to enter your username to fetch your commands.

## Full Command-Line Usage

**Important**: hiproc now features **intelligent command execution** with multiple ways to run commands:

- **Direct ID execution**: `hp exec 123` or `hp 123` for fastest access
- **Smart contextual matching**: `hp run deploy` finds the best "deploy" command for your context
- **Traditional namespace/name**: `hp webapp deploy` for explicit targeting
- **Command IDs**: Unique numbers shown in search results for precise identification

### Quick Start Examples

```bash
# Save commands with smart auto-detection (NEW!)
$ hp save "cargo build --release"        # Auto-detects as 'cargo' in current project namespace
$ hp save "ls -latrh" list               # Saves as 'list' with auto-detected namespace

# Execute and save in one step (NEW!)
$ hp do git status                        # Executes 'git status' and saves as 'git/status'
$ hp x npm test                           # Short alias for 'do' command

# Quick-save from shell history
$ npm run build
$ hp quick-save "build"                   # Saves "npm run build" with smart namespace detection

# Execute stored commands
$ hp 123                                  # Execute command ID 123 (fastest)
$ hp run deploy                           # Smart contextual execution of "deploy"
$ hp webapp deploy                        # Traditional explicit execution

# Browse and search
$ hp list                                 # Show all your commands in a table
$ hp search "git"                         # Search for commands containing "git"
```

### Core Execution Commands

#### `hp exec <id>` / `hp <id>` - Direct Execution by ID

The fastest way to execute a command when you know its ID. Supports argument passing and templating.

```bash
# Execute command directly by ID
$ hp exec 123
$ hp 123                                  # Short form

# Execute with arguments
$ hp exec 123 --verbose --dry-run
$ hp 123 --verbose --dry-run              # Arguments are appended

# Execute with template variables
$ hp exec 123 HOST:prod-db-1 PORT:5432
```

#### `hp run <name>` - Smart Contextual Execution

Intelligently finds the best command matching the name based on your current context (directory, user, hostname).

```bash
# Execute by name with smart matching
$ hp run deploy                           # Finds best "deploy" for current context
$ hp run test                             # Finds best "test" for current directory

# With namespace/scope hints for better matching
$ hp run deploy --namespace webapp        # Prefer webapp namespace
$ hp run deploy --scope team              # Prefer team scope

# With arguments
$ hp run deploy --prod -- --verbose       # Everything after -- goes to the command
```

**Smart Matching Priority:**

1. Exact context match (same user + hostname + directory + namespace)
2. User + hostname + namespace match
3. User + hostname + directory match
4. Directory pattern matching (similar project structure)
5. Namespace preference (most used in namespace)
6. Frequency-based (most recently/frequently used)
7. Global fallback (any matching command)

#### `hp find` - Interactive Fuzzy Search

Interactive search and execution with command ID display. Great for exploration and discovery.

```bash
# Find and execute your personal commands
$ hp find

# Find commands in a team's scope
$ hp find --scope=webapp-team

# Find commands belonging to another user
$ hp find --user=alice
```

#### `hp <namespace> <name>` / `hp <id>` - Traditional Recall

Traditional namespace/name execution and direct ID execution through the external subcommand interface.

```bash
# Traditional namespace/name execution
$ hp webapp start                         # Execute "start" from "webapp" namespace
$ hp webapp start -- --verbose           # With arguments

# Direct ID execution (alternative to hp exec)
$ hp 123                                  # Execute command with ID 123
$ hp 123 -- --verbose                     # With arguments
```

### Command Management

#### `hp save` - Save a Command

Manually save commands with full control over metadata.

```bash
$ hp save "npm run dev" --name "start" --namespace "webapp"
$ hp save "docker build ." --name "build" --namespace "myapp" --scope "team"
```

#### `hp quick-save <name>` - Quick-Save Last Command

**NEW**: Automatically save the last command from your shell history with intelligent namespace detection.

```bash
# After running a command:
$ npm run build
$ hp quick-save "build"                  # Saves "npm run build" as "build"

# With explicit namespace:
$ cargo test
$ hp quick-save "test" --namespace "rust-project"
```

**Smart Namespace Detection:**

- Detects project name from `package.json`, `Cargo.toml`, `pyproject.toml`, `pom.xml`
- Falls back to git repository name or directory name
- Saves you from typing repetitive namespace names

**Shell Integration:**

- Supports Bash, Zsh, and Fish shells
- Automatically detects your shell type
- Reads from standard history files

#### `hp search <query>` - Search Commands

Search for commands with rich metadata display including command IDs.

```bash
$ hp search "deploy"                      # Search all commands
$ hp search "deploy" --scope=webapp-team  # Filter by scope
$ hp search "test" --namespace=frontend   # Filter by namespace
$ hp search "docker" --user=alice         # Filter by user
```

#### `hp list` - List Commands with IDs

List your commands with their IDs for easy reference in edit/delete operations.

```bash
$ hp list                                 # List all your commands
$ hp list --namespace=webapp              # Filter by namespace
$ hp list --scope=team                    # Filter by scope
```

#### `hp info <id>` - Command Details

Show comprehensive information about a specific command.

```bash
$ hp info 123                             # Show full command details
```

Output includes: ID, name, namespace, scope, user, hostname, directory, creation time, usage stats, and the full command string.

#### `hp edit <id>` - Edit Command

Edit a command's content using your default terminal editor (`$EDITOR`).

```bash
$ hp edit 123                             # Opens command 123 in editor
```

#### `hp delete <id>` - Delete Command

Delete a command by its ID with confirmation details.

```bash
$ hp delete 123
> Deleted command with ID 123 ('start' from namespace 'webapp')
```

#### `hp shell <command...>` - Execute and Save

Execute a shell command and automatically save it to the `ad-hoc` namespace.

```bash
$ hp shell grep -r 'FIXME' ./src         # Execute and save to ad-hoc namespace
```

### Command Discovery Features

hiproc includes powerful discovery features to help you find commands you might have forgotten about, discover related commands, and understand your usage patterns.

#### `hp here` - Context-Aware Command Discovery

Show commands that are relevant to your current directory and project context. Perfect for discovering what commands you've used in similar projects or directories.

```bash
# Show commands relevant to current directory
$ hp here                                 # Shows contextual commands for current location

# Show more suggestions with similar commands
$ hp here --similar                       # Extended suggestions (up to 10 commands)

# Analyze project context and get namespace suggestions
$ hp here --project                       # Detect project type and suggest namespace
```

**Example Output:**

```
Commands relevant to current context (/home/user/projects/webapp):
  1. [123] build: npm run build (used 15 times, last: 03/15 14:30)
  2. [124] dev: npm run dev (used 8 times, last: 03/15 09:15)
  3. [125] test: npm test (used 5 times, last: 03/14 16:45)
```

#### `hp suggest` - Intelligent Command Suggestions

Get smart command suggestions based on your current context, usage patterns, and project type. This helps you discover commands that might be useful in your current situation.

```bash
# Get intelligent suggestions for current context
$ hp suggest                              # Default 5 suggestions

# Get more suggestions
$ hp suggest --limit 10                   # Up to 10 suggestions

# Get suggestions for specific project type
$ hp suggest --project-type rust          # Suggestions tailored for Rust projects
```

**Example Output:**

```
Intelligent command suggestions based on your context:
  1. [126] webapp/deploy: kubectl apply -f k8s/
     (used 3 times, last: 03/10 11:20)
  2. [127] webapp/logs: kubectl logs -f deployment/webapp
     (used 7 times, last: 03/14 16:30)
  3. [128] webapp/restart: kubectl rollout restart deployment/webapp
     (used 2 times, last: 03/12 09:45)
```

#### `hp similar <id>` - Find Similar Commands

Discover commands that are similar to a specific command ID. Great for finding variations or related commands you might have saved.

```bash
# Find commands similar to command 123
$ hp similar 123                          # Default 5 similar commands

# Find more similar commands
$ hp similar 123 --limit 8                # Up to 8 similar commands
```

**Example Output:**

```
Commands similar to ID 123:
  1. [124] webapp/build-prod: npm run build:production
     (used 4 times, last: 03/13 15:20)
  2. [125] webapp/build-dev: npm run build:development
     (used 2 times, last: 03/11 10:15)
  3. [126] api/build: cargo build --release
     (used 6 times, last: 03/14 14:45)
```

#### `hp analytics` - Usage Analytics and Insights

Get detailed analytics about your command usage patterns. Understand which commands you use most, execution methods, and usage trends.

```bash
# Get analytics for last 30 days (default)
$ hp analytics

# Get analytics for specific time period
$ hp analytics --days 7                   # Last 7 days
$ hp analytics --days 90                  # Last 90 days
```

**Example Output:**

```
Execution Analytics for user: alice
Period: Last 30 days

Total Executions: 156
Unique Commands: 23
Average per Day: 5.2

Most Used Commands:
  1. webapp/build: 42 executions
  2. webapp/dev: 28 executions
  3. api/test: 19 executions
  4. webapp/deploy: 15 executions

Execution Methods:
  id: 89        # Direct ID execution (hp 123)
  name: 45      # Smart name matching (hp run build)
  namespace_name: 22  # Traditional recall (hp webapp build)
```

### Utility Commands

#### `hp namespaces` - List Namespaces

Show all available namespaces in the system.

```bash
$ hp namespaces                           # List all namespaces
```

#### `hp rename <id> <new-namespace> <new-name>` - Rename Command

Rename a command's namespace and name.

```bash
$ hp rename 123 "new-namespace" "new-name"
```

#### `hp generate-completions <shell>` - Shell Completions

Generate shell completion scripts.

```bash
$ hp generate-completions bash > /usr/local/etc/bash_completion.d/hp
$ hp generate-completions zsh > /usr/local/share/zsh/site-functions/_hp
$ hp generate-completions fish > ~/.config/fish/completions/hp.fish
```

## Enhanced Features

### Intelligent Command Discovery

hiproc features comprehensive **smart discovery and analytics** that help you find, understand, and optimize your command usage:

- **Context-Aware Discovery** (`hp here`): Commands relevant to your current directory and project context
- **Intelligent Suggestions** (`hp suggest`): AI-driven recommendations based on usage patterns and project type
- **Similarity Matching** (`hp similar`): Find related commands and variations you might have forgotten
- **Usage Analytics** (`hp analytics`): Deep insights into your command execution patterns and trends
- **Pattern Recognition**: Identifies similar project structures and suggests relevant commands
- **Scope Intelligence**: Balances personal commands with team/shared commands appropriately

### Shell History Integration

Seamless integration with your shell history for effortless command saving:

- **Multi-Shell Support**: Works with Bash, Zsh, and Fish automatically
- **Smart Detection**: Automatically detects your shell type and history format
- **Project Context**: Intelligently detects project names from `package.json`, `Cargo.toml`, etc.
- **Zero Configuration**: Works out of the box with standard shell configurations

### Advanced Execution Modes

Multiple ways to execute commands for different workflows:

1. **Direct ID Execution** (`hp 123`): Fastest for frequent commands
2. **Contextual Name Matching** (`hp run deploy`): Smart discovery
3. **Traditional Explicit** (`hp webapp deploy`): Precise control
4. **Interactive Search** (`hp find`): Exploration and discovery

### Command Access Control

Secure, multi-user command sharing:

- **Personal Commands**: Private to individual users
- **Shared Scopes**: Team or project-wide command sharing
- **Granular Permissions**: Users can only modify their own commands
- **Access Inheritance**: Shared commands accessible but protected from modification

## Shell Tab Completion

To make recalling commands even faster, you can enable tab completion for your shell. This will allow you to auto-complete namespaces and command names.

**Installation:**

1.  Generate the completion script for your shell:

    ```bash
    # For Bash
    hp generate-completions bash > /usr/local/etc/bash_completion.d/hp

    # For Zsh
    hp generate-completions zsh > /usr/local/share/zsh/site-functions/_hp

    # For Fish
    hp generate-completions fish > ~/.config/fish/completions/hp.fish
    ```

2.  Restart your shell for the changes to take effect.

Now you can type `hp <namespace> <TAB>` to see a list of available commands.

## Comprehensive Examples

### Typical Development Workflow

```bash
# 1. Start a new project
$ cd ~/projects/my-webapp
$ npm init -y

# 2. Run some commands and quick-save them
$ npm install express
$ hp quick-save "install"                 # Saves as "install" in "my-webapp" namespace

$ npm run dev
$ hp quick-save "dev"                     # Saves as "dev" in "my-webapp" namespace

$ npm run build
$ hp quick-save "build"                   # Saves as "build" in "my-webapp" namespace

# 3. Later, execute commands by name (smart contextual matching)
$ hp run dev                              # Finds and runs "npm run dev" for this project
$ hp run build                            # Finds and runs "npm run build" for this project

# 4. Or execute by ID for maximum speed
$ hp list                                 # Shows: ID:123 "dev", ID:124 "build", etc.
$ hp 123                                  # Instantly runs the dev command

# 5. Move to different project directory
$ cd ~/projects/my-api
$ hp run dev                              # Finds best "dev" command for this context
```

### Team Collaboration

```bash
# 1. Save team-wide commands
$ hp save "docker-compose up -d" --name "start" --namespace "api" --scope "team"
$ hp save "kubectl apply -f k8s/" --name "deploy" --namespace "api" --scope "team"

# 2. Team members can discover and use shared commands
$ hp find --scope=team                   # Shows all team commands
$ hp run deploy --scope=team             # Executes team's deploy command

# 3. Override with personal versions when needed
$ hp save "kubectl apply -f k8s/ --dry-run" --name "deploy" --namespace "api"
$ hp run deploy                           # Personal version takes precedence in your context
```

### Discovery-Driven Workflow

```bash
# 1. Starting work in a new project directory
$ cd ~/projects/new-microservice

# 2. Discover what commands might be relevant here
$ hp here                                 # Shows commands from similar projects
$ hp suggest --project-type rust          # Get Rust-specific command suggestions

# 3. Found a useful build command, see what similar commands exist
$ hp similar 234                          # Find variations of the build command

# 4. Use analytics to understand your most productive patterns
$ hp analytics --days 7                   # See what you've been running lately

# 5. Quick-save frequently used commands as you work
$ cargo build --release
$ hp quick-save "build-release"           # Auto-detects namespace from Cargo.toml

# 6. Later, discover and execute contextually
$ hp here                                 # Now shows your new commands for this project
$ hp run build-release                    # Smart execution based on current context
```

### Advanced Templating

```bash
# 1. Save parameterized commands
$ hp save "ssh -i {{KEY_FILE}} {{USER}}@{{HOST}}" --name "connect" --namespace "servers"

# 2. Execute with parameters
$ hp run connect KEY_FILE:~/.ssh/prod.pem USER:ubuntu HOST:prod-db-1
$ hp 456 KEY_FILE:~/.ssh/staging.pem USER:admin HOST:staging-web-1

# 3. Mix parameters with regular arguments
$ hp save "docker run -e ENV={{ENVIRONMENT}} myapp:latest" --name "run" --namespace "docker"
$ hp run docker-run ENVIRONMENT:production -- --verbose --debug
```

### Command Discovery and Management

```bash
# Discovery and exploration
$ hp here                                 # Context-aware command discovery
$ hp suggest --project-type python       # Intelligent suggestions for Python projects
$ hp similar 123                          # Find commands similar to ID 123
$ hp analytics --days 14                  # Usage analytics for last 2 weeks

# Search and explore
$ hp search "docker"                      # Find all docker-related commands
$ hp search "test" --namespace=frontend  # Find test commands in frontend namespace
$ hp find --user=alice                   # Explore Alice's shared commands

# Command management
$ hp info 123                             # Show detailed command information
$ hp edit 123                             # Edit command in your editor
$ hp rename 123 "new-namespace" "new-name" # Rename command
$ hp delete 123                           # Delete command

# Organization
$ hp list --namespace=webapp              # Show all webapp commands
$ hp list --scope=team                    # Show all team commands
$ hp namespaces                           # Show all available namespaces
```

## Advanced Usage: Parameterized Commands

You can save command templates with placeholders, which you can fill in at runtime. This is useful for commands that you run often but with different arguments.

### Named Placeholders

Use `{{PLACEHOLDER}}` syntax in your command string, and provide the values at runtime with `KEY:VALUE` pairs.

```bash
# Save a templated ssh command
$ hp save "ssh -i mykey.pem ubuntu@{{HOST}}" --name "connect" --namespace "server"

# Execute it later with a specific host
$ hp server connect HOST:prod-db-1.example.com
# ...executes "ssh -i mykey.pem ubuntu@prod-db-1.example.com"
```

### Passthrough Arguments

Any arguments you provide that are _not_ in the `KEY:VALUE` format will be appended to the end of the command. This is great for commands that take optional flags.

```bash
# Save a generic git log command
$ hp save "git log --pretty=format:'%h %s'" --name "log" --namespace "git"

# Run it with extra flags
$ hp git log -- --author="Matt" -p
# ...executes "git log --pretty=format:'%h %s' --author="Matt" -p"
```

### Secrets Management: `{{SECRET_NAME}}`

Any placeholder that you _don't_ provide a value for will be treated as a secret. `hiproc` will first look for an environment variable with the same name (e.g., `API_KEY` for `{{API_KEY}}`). If it's not found, it will securely prompt you to enter the value in the terminal.

```bash
# Save a command with a secret API key
$ hp save "curl -H 'Authorization: Bearer {{PROD_API_KEY}}' https://api.myservice.com/v1/users" --name "get-users" --namespace "api"

# Run the command
$ hp api get-users
Enter value for secret 'PROD_API_KEY': ****
# ...executes the curl command with the key you provided.
```
