# Galatea

```
 ██████   █████  ██       █████  ████████ ███████  █████  
██       ██   ██ ██      ██   ██    ██    ██      ██   ██ 
██   ███ ███████ ██      ███████    ██    █████   ███████ 
██    ██ ██   ██ ██      ██   ██    ██    ██      ██   ██ 
 ██████  ██   ██ ███████ ██   ██    ██    ███████ ██   ██  
                                                  
"From Stone to Systems, Breathe Life into Your Infrastructure"
        
~ Server & Workstation Configuration Manager ~
```
[![License](https://img.shields.io/badge/license-MIT-blue.svg)](LICENSE)
[![Rust](https://img.shields.io/badge/rust-stable-orange.svg)](https://www.rust-lang.org/)

> **The Name "Galatea" Across Culture and Myth**
> 
> This project's name draws inspiration from multiple cultural references:
> 
> - **Greek Mythology**: In Greek myth, Galatea was a statue carved by Pygmalion who fell in love with his creation. Aphrodite, moved by his devotion, brought the statue to life. This transformation from inanimate to living represents our software's ability to bring systems to life through automation.
> 
> - **Claymore Manga/Anime**: Galatea, known as "God-Eye Galatea" in the Claymore series, possesses extraordinary sensory abilities and the power to manipulate others remotely. Similarly, our software offers the ability to monitor and control systems from afar with precision.
> 
> - **Bicentennial Man**: In this film, Galatea is a robot designed to be the companion to the protagonist Andrew. This parallel reflects our software's role as a faithful companion to system administrators, evolving and adapting to meet their needs.
> 
> Like these interpretations, Galatea (the software) transforms static configurations into living systems, senses the state of your infrastructure, and serves as a reliable companion in system management.

Galatea is an advanced server and workstation configuration and deployment tool built in Rust. It provides a modular, streamlined approach to system setup through an intuitive Text-based User Interface (TUI), allowing system administrators to efficiently manage, install, and configure software components.



## Table of Contents

- [Features](#features)
- [System Requirements](#system-requirements)
- [Installation](#installation)
- [Usage](#usage)
- [Architecture](#architecture)
- [Configuration](#configuration)
- [Development](#development)
- [Troubleshooting](#troubleshooting)
- [Contributing](#contributing)
- [License](#license)
- [Acknowledgments](#acknowledgments)

## Features

Galatea offers a comprehensive set of features designed to streamline server and workstation configuration:

### Core Features

- **Modular Configuration Management**: Organize configuration operations into atomic tasks and composable stacks
- **Interactive TUI**: Navigate easily through tasks and stacks with a responsive terminal interface
- **Multi-platform Support**: Works on Linux systems, with support for other platforms in development
- **Multiple Script Support**: Seamlessly handles Bash scripts and Ansible playbooks
- **Dependency Management**: Organize tasks in stacks with traceable dependencies
- **Comprehensive Logging**: Detailed logging system to track all operations
- **Flexible Configuration**: Support for both local and remote configurations via URLs
- **State Management**: Tracks installation state for proper uninstallation and remediation

### Advanced Features

- **Parallel Execution**: Execute multiple tasks simultaneously for efficient deployment
- **Task Templates**: Create reusable templates for common configurations
- **Conditional Execution**: Control task execution with conditional logic
- **Error Recovery**: Built-in mechanisms to recover from failed installations
- **Execution Simulation**: Test installations with simulation mode before actual execution
- **Multi-architecture Support**: Handle different CPU architectures within the same configuration
- **Customizable Themes**: Multiple UI themes including high contrast for accessibility

## System Requirements

### Minimum Requirements

- **Operating System**: Linux (Debian, Ubuntu, CentOS, Fedora, or other major distributions)
- **Privileges**: Root access for component installation (sudo)
- **Disk Space**: 50MB for the application, additional space for task downloads
- **Memory**: 64MB minimum, 128MB recommended
- **Rust**: If compiling from source, Rust 1.70.0 or higher

### Optional Requirements

- **Ansible**: Version 2.9+ for Ansible-type tasks
- **Network**: Internet connection for downloading remote tasks and configurations
- **Git**: For version control integration (optional)

## Installation

### Precompiled Binaries (Recommended)

```bash
# Download the latest release for your platform
curl -LO https://github.com/yourusername/galatea/releases/latest/download/galatea-linux-x86_64.tar.gz

# Extract the archive
tar -xzf galatea-linux-x86_64.tar.gz

# Move the binary to a location in your PATH
sudo mv galatea /usr/local/bin/

# Set executable permissions
sudo chmod +x /usr/local/bin/galatea

# Verify installation
galatea --version
```

### Building from Source

```bash
# Ensure you have Rust installed (https://rustup.rs/)
rustup update stable

# Clone the repository
git clone https://github.com/yourusername/galatea.git
cd galatea

# Build with Cargo in release mode
cargo build --release

# Install the binary (optional)
sudo cp target/release/galatea /usr/local/bin/

# Create example configuration (optional)
sudo galatea --create-example /etc/galatea/galatea.yaml
```

### Package Managers

#### Debian/Ubuntu

```bash
# Add the repository (coming soon)
sudo add-apt-repository ppa:galatea/stable
sudo apt update
sudo apt install galatea
```

#### Arch Linux (AUR)

```bash
# Using yay
yay -S galatea
```

## Usage

Galatea requires root privileges to install and configure system components:

```bash
sudo galatea
```

### Command-line Parameters

```
USAGE:
    galatea [OPTIONS]

OPTIONS:
    -c, --config <FILE>             Specify a custom configuration file
    --create-example <FILE>         Create an example configuration file
    --log-dir <DIR>                 Specify a directory for log files [default: /var/log/galatea]
    --no-root-check                 Disable root permission check (useful for testing)
    -h, --help                      Print help information
    -V, --version                   Print version information
```

### TUI Navigation

The TUI provides an intuitive interface for managing tasks and stacks:

- **General Navigation**:
  - `Tab`: Navigate between UI elements
  - `Arrow keys`: Move between items in lists
  - `Enter`: Select/deselect items or confirm actions
  - `Esc`: Cancel or go back

- **Function Keys**:
  - `F1`: View logs
  - `F10`: Show main menu

- **Keyboard Shortcuts**:
  - `Ctrl+Q`: Quit application
  - `Ctrl+R`: Refresh view
  - `Ctrl+S`: Save changes
  - `Ctrl+L`: Clear log view

### Basic Workflow

1. **Start Galatea**: Launch the application with `sudo galatea`
2. **Explore Tasks**: Navigate to the "Task Management" section to view available tasks
3. **Select Tasks**: Use `Enter` to select individual tasks or use task stacks
4. **Install**: Click the "Install Selected" button to execute the selected tasks
5. **Monitor**: View logs during installation to track progress
6. **Verify**: Confirm successful installation through the status indicators

## Architecture

Galatea is built on a modular architecture that separates concerns and promotes maintainability.

### Core Components

#### Tasks

Tasks are atomic units of configuration that can be of different types:

- **Bash**: Shell scripts that execute installation commands
- **Ansible**: Ansible playbooks for more complex configurations
- **Mixed**: Combination of both Bash and Ansible

Each task is defined by:
- Name and description
- Script type (Bash, Ansible, Mixed)
- URL to download the task from
- Dependencies and tags for categorization
- Cleanup commands for uninstallation

Example task definition:
```yaml
- name: example_bash_task
  type: bash
  description: "An example Bash task that installs a package"
  url: "https://example.com/tasks/bash_task.tgz"
  requires_reboot: false
  tags:
    - example
    - bash
```

#### Stacks

Stacks are groups of tasks that are executed together to configure a specific aspect of the system. Each stack includes:

- Name and description
- List of tasks to execute
- Flag indicating if a reboot is required
- Tags for categorization

Example stack definition:
```yaml
- name: web_server
  description: "Stack to configure a web server"
  tasks:
    - example_bash_task
    - example_ansible_task
  requires_reboot: true
  tags:
    - web
    - server
```

### Execution Flow

1. **Configuration Loading**: Galatea loads configuration from files
2. **Task Discovery**: Available tasks are discovered from configured sources
3. **Dependency Resolution**: Task dependencies are resolved
4. **User Selection**: User selects tasks or stacks to execute
5. **Download**: Selected tasks are downloaded from their URLs
6. **Validation**: Task integrity and prerequisites are validated
7. **Execution**: Tasks are executed in the appropriate order
8. **State Tracking**: Installation state is recorded
9. **Cleanup**: Temporary files are removed after successful installation

## Configuration

Galatea uses YAML configuration files that can be located in:

1. `/etc/galatea/galatea.yaml` (system-wide configuration)
2. `./galatea.yaml` (in the executable directory)
3. Custom path specified with `--config`

### Core Configuration Options

```yaml
# Directories for various components
tasks_dir: /var/lib/galatea/tasks
stacks_dir: /var/lib/galatea/stacks
state_dir: /var/lib/galatea/state

# Network settings
download_timeout: 60  # Timeout in seconds for downloads

# UI preferences
ui_theme: default  # Options: default, dark, high_contrast

# Remote sources for tasks and stacks
task_sources:
  - https://example.com/tasks/security.zip
  - https://example.com/tasks/monitoring.zip
  
stack_sources:
  - https://example.com/stacks/web_server.zip
  - https://example.com/stacks/database.zip
```

### Advanced Configuration (future release)

You can create more detailed configurations with additional options:

```yaml
# Advanced network settings
proxy_url: http://proxy.example.com:8080
ssl_verify: true
max_retries: 3
retry_delay: 5

# Execution settings
parallel_tasks: 2  # Number of tasks to execute in parallel
task_timeout: 300  # Maximum execution time per task in seconds
ansible_options: "--verbose"  # Additional options for Ansible

# Notification settings
notifications:
  email:
    enabled: true
    smtp_server: smtp.example.com
    from: galatea@example.com
    to: admin@example.com
  webhook:
    enabled: false
    url: https://hooks.example.com/galatea
```

## Development

### Project Structure

The project follows a modular structure:

```
galatea/
├── src/                 # Source code
│   ├── config.rs        # Configuration management
│   ├── downloader.rs    # File download and extraction
│   ├── executor.rs      # Script and command execution
│   ├── logger.rs        # Logging system
│   ├── main.rs          # Application entry point
│   ├── stack.rs         # Stack implementation
│   ├── task.rs          # Task implementation
│   ├── ui/              # User interface components
│   │   ├── app.rs       # Main application UI
│   │   ├── components/  # Reusable UI components
│   │   ├── log_view.rs  # Log viewing UI
│   │   ├── stack_view.rs # Stack management UI
│   │   ├── task_view.rs # Task management UI
│   │   └── theme.rs     # UI theming
│   └── utils.rs         # Utility functions
├── example/             # Example configurations and tasks
├── tests/               # Integration tests
├── Cargo.toml           # Project manifest
└── README.md            # Project documentation
```

### Build Profiles

The project includes several build profiles:

- **release**: Optimized for production performance
- **dev**: Optimized for development
- **test**: Configured for testing
- **bench**: Configured for benchmarking
- **release-fast**: Faster to compile version for pre-production testing

To build with a specific profile:

```bash
cargo build --profile=release-fast
```

### Creating Custom Tasks

You can create custom tasks for Galatea by following these steps:

1. **Create a Task Directory**: Organize your task files in a directory

2. **Bash Task Example**:
   ```bash
   #!/bin/bash
   # install.sh - Example installation script
   
   # Check if running as root
   if [ "$(id -u)" -ne 0 ]; then
     echo "This script must be run as root" >&2
     exit 1
   fi
   
   # Install package
   apt-get update
   apt-get install -y example-package
   
   # Create configuration
   mkdir -p /etc/example
   echo "configuration=value" > /etc/example/config
   
   # Enable service
   systemctl enable example
   systemctl start example
   
   exit 0
   ```

3. **Ansible Task Example**:
   ```yaml
   # playbook.yaml - Example Ansible playbook
   ---
   - name: Install Example
     hosts: all
     become: yes
     tasks:
       - name: Update package cache
         apt:
           update_cache: yes
           
       - name: Install example package
         apt:
           name: example-package
           state: present
           
       - name: Create configuration directory
         file:
           path: /etc/example
           state: directory
           mode: '0755'
           
       - name: Create configuration file
         copy:
           dest: /etc/example/config
           content: "configuration=value"
           
       - name: Enable and start service
         systemd:
           name: example
           enabled: yes
           state: started
   ```

4. **Package Your Task**: Compress your task files into a .zip, .tar.gz, or .tgz file

5. **Create a Task Configuration**:
   ```yaml
   # tasks.conf
   tasks:
     - name: custom_task
       type: bash  # or ansible
       description: "Custom task to install example package"
       url: "https://your-server.com/tasks/custom_task.zip"
       cleanup_command: "apt-get remove -y example-package"
       tags:
         - custom
         - example
   ```

6. **Test Your Task**: Install your task using Galatea and verify it works as expected

### Extending Galatea

Galatea can be extended in several ways:

1. **Custom Task Types**: Add support for new script types by extending the `ScriptType` enum in `task.rs`

2. **UI Themes**: Create custom themes by adding new functions in `ui/theme.rs`

3. **Plugins**: Implement a plugin system using dynamic loading (planned feature)

## Troubleshooting

### Common Issues

#### Installation Failures

- **Problem**: Task fails to download
  - **Solution**: Check your internet connection and the URL accessibility. Verify proxy settings if applicable.

- **Problem**: Permission denied errors during installation
  - **Solution**: Ensure Galatea is running with root privileges using `sudo`.

- **Problem**: Failed to execute Ansible playbooks
  - **Solution**: Verify Ansible is installed and available in your PATH.

#### UI Issues

- **Problem**: UI appears corrupted or unreadable
  - **Solution**: Try switching to a different theme using the settings menu. The high_contrast theme works well with most terminals.

- **Problem**: Function keys not working
  - **Solution**: Some terminal emulators capture function keys. Try using alternative key bindings or configure your terminal.

### Logs (to be improved)

Detailed logs are stored in `/var/log/galatea/` by default. Examine these logs to diagnose issues:

```bash
# View the most recent log
cat "$(ls -t /var/log/galatea/*.log | head -1)"

# Follow log in real-time
tail -f "$(ls -t /var/log/galatea/*.log | head -1)"
```

### Reporting Issues

When reporting issues, please include:

1. Galatea version (`galatea --version`)
2. Operating system and version
3. Logs from `/var/log/galatea/`
4. Steps to reproduce the issue
5. Expected vs. actual behavior

## Contributing

We welcome contributions to Galatea! Please follow these steps:

1. **Fork the Repository**: Create your own fork of the project

2. **Create a Branch**: Create a branch for your feature or bugfix
   ```bash
   git checkout -b feature/your-feature-name
   ```

3. **Make Changes**: Implement your changes, following the code style

4. **Add Tests**: Write tests for your changes

5. **Run Tests**: Ensure all tests pass
   ```bash
   cargo test
   ```

6. **Format Code**: Format your code using rustfmt
   ```bash
   cargo fmt
   ```

7. **Submit a Pull Request**: Create a PR with a clear description of your changes

### Code Style

- Follow the Rust style guide
- Use meaningful variable and function names
- Write clear comments and documentation
- Keep functions small and focused
- Use proper error handling

## License

This project is licensed under the MIT License - see the [LICENSE](LICENSE) file for details.

## Development Tools

This project has been developed using:

- **[Visual Studio Code](https://code.visualstudio.com/)** by Microsoft - A lightweight but powerful source code editor with excellent Rust support through extensions
  - Enhanced with **[GitHub Copilot](https://github.com/features/copilot)** for AI-assisted coding suggestions and autocompletion
  
- **[RustRover](https://www.jetbrains.com/rust/)** by JetBrains - A dedicated Rust IDE with advanced code assistance and analysis
  - Also powered by **GitHub Copilot** integration for intelligent code completion and generation
  
- **[Claude AI](https://www.anthropic.com/claude)** by Anthropic - Used for code review, documentation generation, and development assistance

These tools and AI assistants have been instrumental in creating a high-quality, maintainable codebase and comprehensive documentation, dramatically accelerating development while maintaining high standards of code quality.

---

**Project Status**: Active Development  
**Last Updated**: May 8, 2025  
**Contact**: mk@mitocode.eu