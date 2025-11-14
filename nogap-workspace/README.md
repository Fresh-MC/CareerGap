# nogap-workspace/README.md

# NoGap Workspace

This repository contains the NoGap workspace, which includes a Rust library (`nogap_core`), a Rust CLI application (`nogap_cli`), and a GUI application (`nogap_dashboard`) built with Tauri.

## Projects

### nogap_core
- A Rust library that provides core functionality for the NoGap project, including the snapshot engine, YAML parser, and policy manager.

### nogap_cli
- A command-line interface for NoGap that wraps the functionality of `nogap_core`. It provides commands such as:
  - `--audit`: Audit the current state.
  - `--harden`: Harden the configuration.
  - `--rollback`: Rollback to a previous state.

### nogap_dashboard
- A GUI application built with Tauri that provides a user-friendly interface for interacting with NoGap functionalities. It includes Playwright tests for UI automation.

## Getting Started

To get started with the NoGap workspace, clone the repository and run the following commands:

```bash
# Navigate to the workspace directory
cd nogap-workspace

# Build the projects
cargo build

# Run the CLI application
cargo run --bin nogap_cli
```

## Testing

To run the Playwright tests for the GUI application, navigate to the `nogap_dashboard` directory and run:

```bash
# Navigate to the dashboard directory
cd nogap_dashboard

# Install dependencies
npm install

# Run Playwright tests
npx playwright test
```

## Contributing

Contributions are welcome! Please open an issue or submit a pull request for any improvements or features you'd like to add.