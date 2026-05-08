# 🦞 Claw Code — Rust Implementation

A high-performance Rust rewrite of the Claw Code CLI agent harness. Built for speed, safety, and native tool execution.

For a task-oriented guide with copy/paste examples, see [`../USAGE.md`](../USAGE.md).

This fork is also wired to talk to a **local Ollama server** via Ollama's OpenAI‑compatible endpoint, so you can run the `claw` CLI entirely offline against any model you have pulled locally (e.g. `qwen3.5:9b`, `llama3.1`, `gpt-oss`, etc.).

---

## 0. Architecture at a Glance

```
Windows
 └─ WSL2 (Ubuntu)
     ├─ rustup + cargo  ─┐
     ├─ git, build-essential
     └─ claw-code build  ─┴─►  ./target/release/claw
                                 │
                                 ▼  HTTP (OpenAI-compatible)
                            Ollama server (Windows or WSL)
                            http://127.0.0.1:11434/v1
```

- **Building inside WSL is recommended** (the Windows-native rustc occasionally crashes with `STATUS_ACCESS_VIOLATION`).
- If the Ollama Desktop app or `ollama serve` is already running on the Windows side, it is reachable from WSL at `127.0.0.1:11434`.

---

## 1. Install WSL2 (Windows users only)

In an **Administrator PowerShell**:

```powershell
# Install WSL2 with the default distro (Ubuntu)
wsl --install

# If you already have WSL, just update it
wsl --update
wsl --set-default-version 2
```

After installation, reboot Windows and launch **Ubuntu** from the Start menu to create your initial user account.

All commands below assume you are running inside the Ubuntu (WSL) shell. If you cloned the repository on the Windows side at `C:\claw-code`, it is accessible from WSL at `/mnt/c/claw-code`:

```bash
cd /mnt/c/claw-code/rust
```

> 💡 For best build performance, consider checking out into `~/` (WSL-native ext4). The `/mnt/c` mount uses the 9P filesystem and builds are significantly slower.

---

## 2. Install Build Tools & Latest Rust

In the WSL Ubuntu shell:

```bash
# Build toolchain and commonly needed native libraries
sudo apt update
sudo apt install -y build-essential pkg-config libssl-dev curl git

# Install the latest stable Rust via rustup
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- -y
source "$HOME/.cargo/env"

# Update to the latest stable release
rustup update stable
rustup default stable

# Verify
rustc --version
cargo --version
```

> ℹ️ If `cargo` shows `command not found`, run `source "$HOME/.cargo/env"` again or open a new shell. rustup automatically adds itself to `~/.bashrc`.

---

## 3. Install Ollama & Pull a Model

### 3-1. Install Ollama

Windows users are recommended to download the **Windows installer** from [ollama.com/download](https://ollama.com/download). After installation, the `ollama` service runs in the background and listens on `127.0.0.1:11434`, which is accessible from WSL as well.

To run Ollama directly inside WSL:

```bash
curl -fsSL https://ollama.com/install.sh | sh
ollama serve &     # Start the server in the background
```

### 3-2. Pull a Model

```bash
# Example: qwen3.5 9B (thinking model)
ollama pull qwen3.5:9b

# Other examples
ollama pull llama3.1:8b
ollama pull gpt-oss:20b
```

Verify it works:

```bash
curl http://127.0.0.1:11434/v1/chat/completions \
  -H "Content-Type: application/json" \
  -d '{"model":"qwen3.5:9b","messages":[{"role":"user","content":"hi"}],"stream":false}'
```

If `choices[0].message.content` contains a response string, you are ready to go.

---

## 4. Environment Variables

The claw provider router checks whether a model name matches any Anthropic/xAI/DashScope prefix. If none match, it looks for **`OPENAI_BASE_URL`** first — if set, the request is routed to the OpenAI-compatible endpoint (`/chat/completions`). Since Ollama implements this endpoint natively, the following two lines are all you need:

```bash
export OPENAI_BASE_URL="http://127.0.0.1:11434/v1"
export OPENAI_API_KEY="ollama"   # Ollama doesn't validate keys, but the CLI requires a non-empty value
```

To persist these across sessions, append the same lines to `~/.bashrc` (or `~/.zshrc`):

```bash
cat >> ~/.bashrc <<'EOF'

# --- claw-code / Ollama ---
export OPENAI_BASE_URL="http://127.0.0.1:11434/v1"
export OPENAI_API_KEY="ollama"
EOF

source ~/.bashrc
```

For the remote Anthropic API:

```bash
export ANTHROPIC_API_KEY="sk-ant-..."
# Or a proxy/OAuth bearer token
export ANTHROPIC_AUTH_TOKEN="anthropic-oauth-or-proxy-bearer-token"
# Override the base URL if needed
export ANTHROPIC_BASE_URL="https://your-proxy.com"
```

> 🔁 **Switching between remote and Ollama**: To use Ollama, just set `OPENAI_BASE_URL` to the Ollama address. To switch back to the official Anthropic endpoint, run `unset OPENAI_BASE_URL` and use `ANTHROPIC_API_KEY`. Any model name that doesn't start with `opus/sonnet/haiku/claude-*` is routed via the OpenAI-compatible path (e.g. `qwen3.5:9b`, `llama3.1:8b`, `gpt-oss:20b`). See [`docs/MODEL_COMPATIBILITY.md`](../docs/MODEL_COMPATIBILITY.md) for the full routing matrix.

---

## 5. Clone & Build

```bash
# (If you haven't cloned the repository yet)
git clone https://github.com/instructkr/claw-code.git
cd claw-code/rust

# Debug build (workspace)
cargo build --workspace

# Release build
cargo build --release
```

The first build takes several minutes to compile dependencies. The resulting binary is at `rust/target/release/claw`.

> 🛠️ **If you see `STATUS_ACCESS_VIOLATION (0xc0000005)` while building on Windows natively**, this is usually a transient rustc crash. Running `cargo build --release` again often succeeds. For a permanent fix, build inside WSL instead.

---

## 6. Running

### 6-1. One-shot Prompt

```bash
./target/release/claw --model qwen3.5:9b prompt "hi, say hello in one word"
```

Or via pipe:

```bash
echo 'hi, say hello in one word' | ./target/release/claw --model qwen3.5:9b
```

JSON output (for automation):

```bash
./target/release/claw --output-format json prompt "summarize src/main.rs"
```

### 6-2. Interactive REPL

```bash
./target/release/claw --model qwen3.5:9b
```

Once the prompt appears, you can chat freely and use slash commands like `/help`, `/status`, `/model`, `/clear`.

### 6-3. Permission Modes / Tool Restrictions

```bash
# Skip all permission checks (use in a local sandbox)
./target/release/claw --model qwen3.5:9b --dangerously-skip-permissions

# Read-only mode
./target/release/claw --model qwen3.5:9b --permission-mode read-only
```

### 6-4. Quick Dev Runs (cargo run)

```bash
cargo run -p rusty-claude-cli -- --help
cargo run -p rusty-claude-cli -- --model claude-opus-4-6
cargo run -p rusty-claude-cli -- prompt "explain this codebase"
```

---

## 7. How the Ollama Backend Works (in brief)

- `OpenAiCompatClient` in `crates/api/src/providers/openai_compat.rs` sends streaming requests to `$OPENAI_BASE_URL/chat/completions` (for Ollama: `http://127.0.0.1:11434/v1/chat/completions`).
- `detect_provider_kind` in `crates/api/src/providers/mod.rs` inspects the model name and environment variables to select `ProviderKind::Anthropic` / `Xai` / `OpenAi`. If the model name doesn't match any known prefix, it checks for `OPENAI_BASE_URL` — this is the path that Ollama/LM Studio/vLLM takes.
- OpenAI-compatible responses are translated internally into Anthropic `StreamEvent` sequences (`MessageStart → ContentBlockStart → TextDelta → ContentBlockStop → MessageDelta → MessageStop`), so the CLI/runtime consumes the same events regardless of the provider.
- Models with "thinking" output (e.g. Qwen/DeepSeek) that use `reasoning` / `reasoning_content` fields are also handled by `openai_compat`.

---

## Mock parity harness

The workspace includes a deterministic Anthropic-compatible mock service and a clean-environment CLI harness for end-to-end parity checks.

```bash
cd rust/

# Run the scripted clean-environment harness
./scripts/run_mock_parity_harness.sh

# Or start the mock service manually for ad hoc CLI runs
cargo run -p mock-anthropic-service -- --bind 127.0.0.1:0
```

Harness coverage: `streaming_text`, `read_file_roundtrip`, `grep_chunk_assembly`, `write_file_allowed`, `write_file_denied`, `multi_tool_turn_roundtrip`, `bash_stdout_roundtrip`, `bash_permission_prompt_approved`, `bash_permission_prompt_denied`, `plugin_tool_roundtrip`.

Primary artifacts:

- `crates/mock-anthropic-service/` — reusable mock Anthropic-compatible service
- `crates/rusty-claude-cli/tests/mock_parity_harness.rs` — clean-env CLI harness
- `scripts/run_mock_parity_harness.sh` — reproducible wrapper
- `scripts/run_mock_parity_diff.py` — scenario checklist + PARITY mapping runner
- `mock_parity_scenarios.json` — scenario-to-PARITY manifest

## Features

| Feature | Status |
|---------|--------|
| Anthropic / OpenAI-compatible provider flows + streaming | ✅ |
| Local Ollama backend (OpenAI-compatible) | ✅ |
| Direct bearer-token auth via `ANTHROPIC_AUTH_TOKEN` | ✅ |
| Interactive REPL (rustyline) | ✅ |
| Tool system (bash, read, write, edit, grep, glob) | ✅ |
| Web tools (search, fetch) | ✅ |
| Sub-agent / agent surfaces | ✅ |
| Todo tracking | ✅ |
| Notebook editing | ✅ |
| CLAUDE.md / project memory | ✅ |
| Config file hierarchy (`.claw.json` + merged config sections) | ✅ |
| Permission system | ✅ |
| MCP server lifecycle + inspection | ✅ |
| Session persistence + resume | ✅ |
| Cost / usage / stats surfaces | ✅ |
| Git integration | ✅ |
| Markdown terminal rendering (ANSI) | ✅ |
| Model aliases (opus/sonnet/haiku) | ✅ |
| Direct CLI subcommands (`status`, `sandbox`, `agents`, `mcp`, `skills`, `doctor`) | ✅ |
| Slash commands (including `/skills`, `/agents`, `/mcp`, `/doctor`, `/plugin`, `/subagent`) | ✅ |
| Hooks (`/hooks`, config-backed lifecycle hooks) | ✅ |
| Plugin management surfaces | ✅ |
| Skills inventory / install surfaces | ✅ |
| Machine-readable JSON output across core CLI surfaces | ✅ |

## Model Aliases

Short names resolve to the latest Anthropic model versions:

| Alias | Resolves To |
|-------|------------|
| `opus` | `claude-opus-4-6` |
| `sonnet` | `claude-sonnet-4-6` |
| `haiku` | `claude-haiku-4-5-20251213` |

When using Ollama, pass the exact tag from `ollama pull` to `--model` (e.g. `qwen3.5:9b`).

## CLI Flags and Commands

Representative current surface:

```text
claw [OPTIONS] [COMMAND]

Flags:
  --model MODEL
  --output-format text|json
  --permission-mode MODE
  --dangerously-skip-permissions
  --allowedTools TOOLS
  --resume [SESSION.jsonl|session-id|latest]
  --version, -V

Top-level commands:
  prompt <text>
  help
  version
  status
  sandbox
  acp [serve]
  dump-manifests
  bootstrap-plan
  agents
  mcp
  skills
  system-prompt
  init
```

`claw acp` is a local discoverability surface for editor-first users: it reports the current ACP/Zed status without starting the runtime. As of April 16, 2026, claw-code does **not** ship an ACP/Zed daemon entrypoint yet, and `claw acp serve` is only a status alias until the real protocol surface lands.

The command surface is moving quickly. For the canonical live help text, run:

```bash
cargo run -p rusty-claude-cli -- --help
```

## Slash Commands (REPL)

Tab completion expands slash commands, model aliases, permission modes, and recent session IDs.

- session / visibility: `/help`, `/status`, `/sandbox`, `/cost`, `/resume`, `/session`, `/version`, `/usage`, `/stats`
- workspace / git: `/compact`, `/clear`, `/config`, `/memory`, `/init`, `/diff`, `/commit`, `/pr`, `/issue`, `/export`, `/hooks`, `/files`, `/release-notes`
- discovery / debugging: `/mcp`, `/agents`, `/skills`, `/doctor`, `/tasks`, `/context`, `/desktop`
- automation / analysis: `/review`, `/advisor`, `/insights`, `/security-review`, `/subagent`, `/team`, `/telemetry`, `/providers`, `/cron`, and more
- plugin management: `/plugin` (with aliases `/plugins`, `/marketplace`)

Notable claw-first surfaces:

- `/skills [list|install <path>|help]`
- `/agents [list|help]`
- `/mcp [list|show <server>|help]`
- `/doctor`
- `/plugin [list|install <path>|enable <name>|disable <name>|uninstall <id>|update <id>]`
- `/subagent [list|steer <target> <msg>|kill <id>]`

See [`../USAGE.md`](../USAGE.md) for examples and run `cargo run -p rusty-claude-cli -- --help` for the canonical command list.

## Workspace Layout

```text
rust/
├── Cargo.toml              # Workspace root
├── Cargo.lock
└── crates/
    ├── api/                # Provider clients (Anthropic / Ollama) + streaming + preflight
    ├── commands/           # Shared slash-command registry + help rendering
    ├── compat-harness/     # TS manifest extraction harness
    ├── mock-anthropic-service/ # Deterministic local Anthropic-compatible mock
    ├── plugins/            # Plugin metadata, manager, install/enable/disable surfaces
    ├── runtime/            # Session, config, permissions, MCP, prompts, auth/runtime loop
    ├── rusty-claude-cli/   # Main CLI binary (`claw`)
    ├── telemetry/          # Session tracing and usage telemetry types
    └── tools/              # Built-in tools, skill resolution, tool search, agent surfaces
```

### Crate Responsibilities

- **api** — provider clients (`providers/anthropic.rs`, `providers/openai_compat.rs` covering xAI/OpenAI/DashScope/Ollama/LM Studio/vLLM), SSE streaming, request/response types, auth (`ANTHROPIC_API_KEY` / `ANTHROPIC_AUTH_TOKEN` bearer-token / `OPENAI_API_KEY` / `XAI_API_KEY` / `DASHSCOPE_API_KEY`), request-size/context-window preflight, prompt cache
- **commands** — slash command definitions, parsing, help text generation, JSON/text command rendering
- **compat-harness** — extracts tool/prompt manifests from upstream TS source
- **mock-anthropic-service** — deterministic `/v1/messages` mock for CLI parity tests
- **plugins** — plugin metadata, install/enable/disable/update flows, plugin tool definitions, hook integration surfaces
- **runtime** — `ConversationRuntime`, config loading, session persistence, permission policy, MCP client lifecycle, system prompt assembly, usage tracking
- **rusty-claude-cli** — REPL, one-shot prompt, direct CLI subcommands, streaming display, tool call rendering, CLI argument parsing
- **telemetry** — session trace events and supporting telemetry payloads
- **tools** — tool specs + execution: Bash, ReadFile, WriteFile, EditFile, GlobSearch, GrepSearch, WebSearch, WebFetch, Agent, TodoWrite, NotebookEdit, Skill, ToolSearch, and runtime-facing tool discovery

## Troubleshooting

| Symptom | Cause / Fix |
|---------|-------------|
| `Streaming not supported for local server yet` | Stale binary. Run `cargo build --release` again and use the new `./target/release/claw`. |
| `assistant stream produced no content` | A thinking model placed its answer only in the `reasoning` field. This is handled via fallback — make sure you are on the latest build. |
| `unexpected EOF during chunk size line` | The Ollama streaming connection was dropped. Check your network and restart Ollama if needed. |
| `STATUS_ACCESS_VIOLATION (0xc0000005)` (Windows) | Transient rustc crash. Run the same command again, or build inside WSL instead. |
| `error: connection refused (127.0.0.1:11434)` | Ollama server is not running. Check the Windows system tray for Ollama, or run `ollama serve` again. |
| `model "xxx" not found` | You need to pull the model first with `ollama pull xxx`. Check installed tags with `ollama list`. |

## Verification

From the repository root (`rust/`):

```bash
cargo fmt
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace
```

## Stats

- **9 crates** in workspace
- **Binary name:** `claw`
- **Default model:** `claude-opus-4-6` (remote) / `qwen3.5:9b` etc. (Ollama)
- **Default permissions:** `danger-full-access`

## License

See repository root.
