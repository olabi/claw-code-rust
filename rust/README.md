# 🦞 Claw Code — Rust Implementation

A high-performance Rust rewrite of the Claw Code CLI agent harness. Built for speed, safety, and native tool execution.

For a task-oriented guide with copy/paste examples, see [`../USAGE.md`](../USAGE.md).

This fork is also wired to talk to a **local Ollama server** via Ollama's OpenAI‑compatible endpoint, so you can run the `claw` CLI entirely offline against any model you have pulled locally (e.g. `qwen3.5:9b`, `llama3.1`, `gpt-oss` 등).

---

## 0. 전체 흐름 한눈에 보기

```
Windows
 └─ WSL2 (Ubuntu)
     ├─ rustup + cargo  ─┐
     ├─ git, build-essential
     └─ claw-code 빌드  ─┴─►  ./target/release/claw
                                 │
                                 ▼  HTTP (OpenAI 호환)
                            Ollama 서버 (Windows or WSL)
                            http://127.0.0.1:11434/v1
```

- **WSL 안에서 빌드**하는 것을 권장합니다 (Windows 네이티브 rustc 가 간헐적으로 `STATUS_ACCESS_VIOLATION` 크래시를 냅니다).
- Ollama 서버는 Windows 쪽 Ollama Desktop / `ollama serve` 가 이미 떠 있으면 그대로 재사용합니다. WSL 에서 `127.0.0.1:11434` 로 접근 가능합니다.

---

## 1. WSL2 설치 (Windows 사용자만)

관리자 권한 **PowerShell** 에서:

```powershell
# 기본 배포판(Ubuntu) 포함해 WSL2 일괄 설치
wsl --install

# 이미 WSL 을 써본 적이 있다면 최신화만
wsl --update
wsl --set-default-version 2
```

설치 후 Windows 를 재부팅하고, 시작 메뉴에서 **Ubuntu** 를 실행해 최초 사용자 계정을 만듭니다.

이후 모든 명령은 Ubuntu(WSL) 쉘에서 실행한다고 가정합니다. 리포지토리를 Windows 쪽 `C:\kaggle\claw-code` 에 두었다면 WSL 에서는 `/mnt/c/kaggle/claw-code` 로 접근합니다:

```bash
cd /mnt/c/kaggle/claw-code/rust
```

> 💡 성능을 최대한 뽑고 싶다면 `~/` (WSL 네이티브 ext4) 로 체크아웃하세요. `/mnt/c` 는 9P 파일시스템이라 빌드가 훨씬 느립니다.

---

## 2. 빌드 도구 & 최신 Rust 설치

WSL Ubuntu 쉘에서:

```bash
# 빌드 툴체인과 자주 필요한 네이티브 라이브러리
sudo apt update
sudo apt install -y build-essential pkg-config libssl-dev curl git

# rustup 으로 최신 안정판 Rust 설치
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- -y
source "$HOME/.cargo/env"

# 최신 stable 로 업데이트
rustup update stable
rustup default stable

# 확인
rustc --version
cargo --version
```

> ℹ️ `cargo` 가 `command not found` 로 나오면 `source "$HOME/.cargo/env"` 를 다시 실행하거나 새 쉘을 열어 주세요. `~/.bashrc` 에 rustup 이 경로를 자동 추가합니다.

---

## 3. Ollama 설치 & 모델 pull

### 3-1. Ollama 설치

Windows 사용자는 [ollama.com/download](https://ollama.com/download) 에서 **Windows 설치 파일** 을 받는 것을 권장합니다. 설치 후 백그라운드에 자동으로 `ollama` 서비스가 뜨고 `127.0.0.1:11434` 에서 listen 합니다. WSL 에서도 그대로 접근됩니다.

WSL 안에서 직접 돌리고 싶다면:

```bash
curl -fsSL https://ollama.com/install.sh | sh
ollama serve &     # 백그라운드로 서버 기동
```

### 3-2. 모델 내려받기

```bash
# 예시: qwen3.5 9B (thinking 모델)
ollama pull qwen3.5:9b

# 다른 예시들
ollama pull llama3.1:8b
ollama pull gpt-oss:20b
```

정상 동작 확인:

```bash
curl http://127.0.0.1:11434/v1/chat/completions \
  -H "Content-Type: application/json" \
  -d '{"model":"qwen3.5:9b","messages":[{"role":"user","content":"hi"}],"stream":false}'
```

`choices[0].message.content` 에 응답 문자열이 담겨 있으면 준비 완료입니다.

---

## 4. 환경 변수 설정

claw 의 provider 라우터는 모델 이름이 Anthropic/xAI/DashScope prefix 에 매칭되지 않으면 **`OPENAI_BASE_URL` 이 설정돼 있는지 먼저 확인**하고, 있으면 OpenAI 호환 경로 (`/chat/completions`) 로 보냅니다. Ollama 는 이 경로를 그대로 구현하므로, 다음 두 줄이면 충분합니다.

```bash
export OPENAI_BASE_URL="http://127.0.0.1:11434/v1"
export OPENAI_API_KEY="ollama"   # Ollama 는 키 검증을 안 하지만 빈 값이면 CLI 가 막음
```

영속화하려면 `~/.bashrc` (또는 `~/.zshrc`) 끝에 같은 줄을 추가하세요:

```bash
cat >> ~/.bashrc <<'EOF'

# --- claw-code / Ollama ---
export OPENAI_BASE_URL="http://127.0.0.1:11434/v1"
export OPENAI_API_KEY="ollama"
EOF

source ~/.bashrc
```

원격 Anthropic API 를 쓸 때는:

```bash
export ANTHROPIC_API_KEY="sk-ant-..."
# 또는 프록시/OAuth bearer token
export ANTHROPIC_AUTH_TOKEN="anthropic-oauth-or-proxy-bearer-token"
# 필요 시 base URL 오버라이드
export ANTHROPIC_BASE_URL="https://your-proxy.com"
```

> 🔁 **원격 ↔ Ollama 전환**: Ollama 를 쓰고 싶으면 `OPENAI_BASE_URL` 만 Ollama 주소로 두면 됩니다. Anthropic 공식 엔드포인트로 돌아가려면 `unset OPENAI_BASE_URL` 후 `ANTHROPIC_API_KEY` 를 사용하세요. 모델 이름이 `opus/sonnet/haiku/claude-*` 로 시작하지 않으면 OpenAI 경로로 라우팅됩니다 (예: `qwen3.5:9b`, `llama3.1:8b`, `gpt-oss:20b`). 자세한 매트릭스는 [`docs/MODEL_COMPATIBILITY.md`](../docs/MODEL_COMPATIBILITY.md) 참조.

---

## 5. 리포지토리 클론 & 빌드

```bash
# (리포지토리를 아직 받지 않았다면)
git clone https://github.com/instructkr/claw-code.git
cd claw-code/rust

# 워크스페이스 빌드 (디버그)
cargo build --workspace

# 릴리스 빌드
cargo build --release
```

처음 빌드는 의존성 컴파일로 몇 분이 걸립니다. 결과물은 `rust/target/release/claw` 에 생성됩니다.

> 🛠️ **Windows 네이티브에서 빌드하다가 `STATUS_ACCESS_VIOLATION (0xc0000005)` 에러가 나면** 대부분 rustc 의 일시적 크래시입니다. `cargo build --release` 를 한 번 더 실행하면 통과되는 경우가 많고, 근본 해결을 위해 WSL 에서 빌드하는 쪽을 권장합니다.

---

## 6. 실행

### 6-1. 원샷 프롬프트

```bash
./target/release/claw --model qwen3.5:9b prompt "hi, say hello in one word"
```

또는 파이프로:

```bash
echo 'hi, say hello in one word' | ./target/release/claw --model qwen3.5:9b
```

JSON 출력 (자동화용):

```bash
./target/release/claw --output-format json prompt "summarize src/main.rs"
```

### 6-2. 대화형 REPL

```bash
./target/release/claw --model qwen3.5:9b
```

프롬프트가 뜨면 자유롭게 대화하면 되고, `/help`, `/status`, `/model`, `/clear` 같은 슬래시 커맨드를 사용할 수 있습니다.

### 6-3. 권한 모드 / 도구 제한

```bash
# 모든 권한 체크 스킵 (로컬 샌드박스 권장)
./target/release/claw --model qwen3.5:9b --dangerously-skip-permissions

# 읽기 전용
./target/release/claw --model qwen3.5:9b --permission-mode read-only
```

### 6-4. 개발 중 빠른 실행 (cargo run)

```bash
cargo run -p rusty-claude-cli -- --help
cargo run -p rusty-claude-cli -- --model claude-opus-4-6
cargo run -p rusty-claude-cli -- prompt "explain this codebase"
```

---

## 7. Ollama 백엔드 동작 원리 (짧게)

- `crates/api/src/providers/openai_compat.rs` 의 `OpenAiCompatClient` 가 `$OPENAI_BASE_URL/chat/completions` (Ollama 의 경우 `http://127.0.0.1:11434/v1/chat/completions`) 로 스트리밍 요청을 보냅니다.
- `crates/api/src/providers/mod.rs` 의 `detect_provider_kind` 가 모델 이름과 환경변수를 보고 `ProviderKind::Anthropic` / `Xai` / `OpenAi` 중 하나를 고릅니다. 모델 이름이 어떤 prefix 에도 맞지 않으면 `OPENAI_BASE_URL` 존재 여부를 먼저 확인합니다 — 이게 Ollama/LM Studio/vLLM 이 타는 경로입니다.
- OpenAI 호환 응답은 내부적으로 Anthropic `StreamEvent` 시퀀스(`MessageStart → ContentBlockStart → TextDelta → ContentBlockStop → MessageDelta → MessageStop`) 로 번역돼서 CLI/runtime 은 프로바이더와 무관하게 동일한 이벤트를 소비합니다.
- Qwen/DeepSeek 계열 "thinking" 모델처럼 `reasoning` / `reasoning_content` 필드를 쓰는 케이스도 `openai_compat` 이 처리합니다.

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

Ollama 를 쓸 때는 `--model` 에 `ollama pull` 로 받은 정확한 태그(예: `qwen3.5:9b`)를 넘기면 됩니다.

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

## 문제 해결 (Troubleshooting)

| 증상 | 원인 / 해결 |
|------|-------------|
| `Streaming not supported for local server yet` | 구버전 바이너리. `cargo build --release` 다시 하고 새 `./target/release/claw` 를 쓰세요. |
| `assistant stream produced no content` | thinking 모델이 `reasoning` 필드에만 답을 담은 경우. 이미 fallback 처리되어 있으니 최신 빌드인지 확인하세요. |
| `unexpected EOF during chunk size line` | Ollama 스트리밍 연결이 끊긴 경우. 현재 구현은 `stream: false` 로 요청하므로 발생하면 네트워크/Ollama 재시작을 확인하세요. |
| `STATUS_ACCESS_VIOLATION (0xc0000005)` (Windows) | rustc 일시 크래시. 같은 명령을 한 번 더 실행하거나 WSL 에서 빌드하세요. |
| `error: connection refused (127.0.0.1:11434)` | Ollama 서버가 내려가 있음. Windows 트레이에서 Ollama 가 떠 있는지 또는 `ollama serve` 를 다시 실행하세요. |
| `model "xxx" not found` | `ollama pull xxx` 로 먼저 모델을 받아야 합니다. `ollama list` 로 설치된 태그 확인. |

## Verification

리포지토리 루트(`rust/`)에서:

```bash
cargo fmt
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace
```

## Stats

- **9 crates** in workspace
- **Binary name:** `claw`
- **Default model:** `claude-opus-4-6` (원격) / `qwen3.5:9b` 등 (Ollama)
- **Default permissions:** `danger-full-access`

## License

See repository root.
