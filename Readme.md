# lsp-gpt

A language server implementation that uses the GPT-4 API to provide LSP-compliant suggestions, specifically for PlantUML files.

The main idea of the project is to not write the actual language server, but forward language server requests to an LLM and have it generate the appropriate LSP-protocol-compliant responses for the language client.

## Features

- Code completions for PlantUML syntax
- Hover information
- Document diagnostics
- Workspace folder support for few-shot learning
- Configurable prompts per LSP method

## Installation

```bash
# Install from source using cargo
cargo install --path .
```

## Usage

### Prerequisites

Set your OpenAI API key and organization ID as environment variables:

```bash
export OPENAI_API_KEY=your_api_key
export OPENAI_ORG_ID=your_org_id
```

The language server will read these from the environment or from a `secrets.txt` file in the project root (format: `KEY=value` per line).

### With Neovim

```vim
:lua vim.lsp.start({ name = 'lsp-gpt', cmd = { 'lsp-gpt' }, root_dir = vim.loop.cwd() })
```

### With Visual Studio Code

An extension is available in `clients/vscode/`. See the extension documentation for setup instructions.

## Configuration

The language server loads configuration from `assets/config.json`. You can specify:

- `languageId`: The language identifier (e.g., `plantuml`)
- `fileExtensions`: Array of file extensions to handle
- Prompt configurations in `assets/prompts/config.json`

Each prompt configuration can specify:
- `method`: The LSP method (e.g., `textDocument/completion`)
- `model`: The GPT model to use
- `modelTemperature`: Temperature setting
- `file`: Path to the prompt file

## Project Structure

- `src/` - Rust source code for the language server
- `assets/prompts/` - Prompt configuration files
- `clients/vscode/` - VS Code extension

