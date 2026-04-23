# 🦞 Claw Code — Rust Implementation

A high-performance Rust rewrite of the Claw Code CLI agent harness. Built for speed, safety, and native tool execution.

This fork is wired to talk to a **local Ollama server** via Ollama's OpenAI‑compatible endpoint, so you can run the `claw` CLI entirely offline against any model you have pulled locally (e.g. `qwen3.5:9b`, `llama3.1`, `gpt-oss` 등).

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

claw 는 Anthropic 공식 클라이언트 형태의 환경 변수를 그대로 사용합니다. Ollama 에게 보낼 때는 **API 키가 필요 없지만** CLI 의 auth 검사 코드 때문에 더미 값으로 세팅해 줍니다.

```bash
export ANTHROPIC_BASE_URL="http://127.0.0.1:11434/v1"
export ANTHROPIC_API_KEY="ollama"
export ANTHROPIC_AUTH_TOKEN="ollama"
```

영속화하려면 `~/.bashrc` (또는 `~/.zshrc`) 끝에 같은 줄을 추가하세요:

```bash
cat >> ~/.bashrc <<'EOF'

# --- claw-code / Ollama ---
export ANTHROPIC_BASE_URL="http://127.0.0.1:11434/v1"
export ANTHROPIC_API_KEY="ollama"
export ANTHROPIC_AUTH_TOKEN="ollama"
EOF

source ~/.bashrc
```

> 🔁 **원격 Anthropic API 로 되돌리고 싶다면** `ANTHROPIC_BASE_URL` 을 `unset` 하고 `ANTHROPIC_API_KEY` 에 실제 `sk-ant-...` 키를 넣으면 됩니다.

---

## 5. 리포지토리 클론 & 빌드

```bash
# (리포지토리를 아직 받지 않았다면)
git clone https://github.com/instructkr/claw-code.git
cd claw-code/rust

# 릴리스 빌드
cargo build --release
```

처음 빌드는 의존성 컴파일로 몇 분이 걸립니다. 결과물은 `rust/target/release/claw` 에 생성됩니다.

> 🛠️ **Windows 네이티브에서 빌드하다가 `STATUS_ACCESS_VIOLATION (0xc0000005)` 에러가 나면** 대부분 rustc 의 일시적 크래시입니다. `cargo build --release` 를 한 번 더 실행하면 통과되는 경우가 많고, 근본 해결을 위해 WSL 에서 빌드하는 쪽을 권장합니다. 저장소 루트 `rust/Cargo.toml` 에는 이미 크래시를 완화하는 release 프로파일(`opt-level = 0`, `codegen-units = 1`) 이 설정되어 있습니다.

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

---

## 7. 동작 원리 (짧게)

- `crates/api/src/lib.rs` 의 `AnthropicClient::stream_message` 가 Ollama `/chat/completions` 에 `stream: false` 로 요청을 보냅니다.
- 응답의 `choices[0].message.content` (thinking 모델에서 비어있으면 `reasoning` / `reasoning_content`) 를 꺼내서, Anthropic 스트림 포맷(`MessageStart → ContentBlockStart → TextDelta → ContentBlockStop → MessageDelta → MessageStop`) 으로 감싸 CLI 에 전달합니다.
- 즉, CLI 코드는 Anthropic 스트리밍을 그대로 소비하고, 백엔드만 Ollama 로 바뀐 구조입니다.

---

## Features

| Feature | Status |
|---------|--------|
| Anthropic API + streaming | ✅ |
| Local Ollama backend (OpenAI-compatible) | ✅ |
| OAuth login/logout | ✅ (원격 Anthropic 전용) |
| Interactive REPL (rustyline) | ✅ |
| Tool system (bash, read, write, edit, grep, glob) | ✅ |
| Web tools (search, fetch) | ✅ |
| Sub-agent orchestration | ✅ |
| Todo tracking | ✅ |
| Notebook editing | ✅ |
| CLAUDE.md / project memory | ✅ |
| Config file hierarchy (.claude.json) | ✅ |
| Permission system | ✅ |
| MCP server lifecycle | ✅ |
| Session persistence + resume | ✅ |
| Extended thinking (thinking blocks) | ✅ |
| Cost tracking + usage display | ✅ |
| Git integration | ✅ |
| Markdown terminal rendering (ANSI) | ✅ |
| Model aliases (opus/sonnet/haiku) | ✅ |
| Slash commands (/status, /compact, /clear, etc.) | ✅ |
| Hooks (PreToolUse/PostToolUse) | 🔧 Config only |
| Plugin system | 📋 Planned |
| Skills registry | 📋 Planned |

## Model Aliases

Short names resolve to the latest model versions (원격 Anthropic 사용 시):

| Alias | Resolves To |
|-------|------------|
| `opus` | `claude-opus-4-6` |
| `sonnet` | `claude-sonnet-4-6` |
| `haiku` | `claude-haiku-4-5-20251213` |

Ollama 를 쓸 때는 `--model` 에 `ollama pull` 로 받은 정확한 태그(예: `qwen3.5:9b`)를 넘기면 됩니다.

## CLI Flags

```
claw [OPTIONS] [COMMAND]

Options:
  --model MODEL                    Set the model (alias or full name)
  --dangerously-skip-permissions   Skip all permission checks
  --permission-mode MODE           Set read-only, workspace-write, or danger-full-access
  --allowedTools TOOLS             Restrict enabled tools
  --output-format FORMAT           Output format (text or json)
  --version, -V                    Print version info

Commands:
  prompt <text>      One-shot prompt (non-interactive)
  login              Authenticate via OAuth (원격 Anthropic 전용)
  logout             Clear stored credentials
  init               Initialize project config
  doctor             Check environment health
  self-update        Update to latest version
```

## Slash Commands (REPL)

| Command | Description |
|---------|-------------|
| `/help` | Show help |
| `/status` | Show session status (model, tokens, cost) |
| `/cost` | Show cost breakdown |
| `/compact` | Compact conversation history |
| `/clear` | Clear conversation |
| `/model [name]` | Show or switch model |
| `/permissions` | Show or switch permission mode |
| `/config [section]` | Show config (env, hooks, model) |
| `/memory` | Show CLAUDE.md contents |
| `/diff` | Show git diff |
| `/export [path]` | Export conversation |
| `/session [id]` | Resume a previous session |
| `/version` | Show version |

## Workspace Layout

```
rust/
├── Cargo.toml              # Workspace root
├── Cargo.lock
└── crates/
    ├── api/                # Anthropic API client + Ollama backend
    ├── commands/           # Shared slash-command registry
    ├── compat-harness/     # TS manifest extraction harness
    ├── runtime/            # Session, config, permissions, MCP, prompts
    ├── rusty-claude-cli/   # Main CLI binary (`claw`)
    └── tools/              # Built-in tool implementations
```

### Crate Responsibilities

- **api** — HTTP client, Anthropic ↔ Ollama 변환, SSE stream parser, request/response types, auth (API key + OAuth bearer)
- **commands** — Slash command definitions and help text generation
- **compat-harness** — Extracts tool/prompt manifests from upstream TS source
- **runtime** — `ConversationRuntime` agentic loop, `ConfigLoader` hierarchy, `Session` persistence, permission policy, MCP client, system prompt assembly, usage tracking
- **rusty-claude-cli** — REPL, one-shot prompt, streaming display, tool call rendering, CLI argument parsing
- **tools** — Tool specs + execution: Bash, ReadFile, WriteFile, EditFile, GlobSearch, GrepSearch, WebSearch, WebFetch, Agent, TodoWrite, NotebookEdit, Skill, ToolSearch, REPL runtimes

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

- **~20K lines** of Rust
- **6 crates** in workspace
- **Binary name:** `claw`
- **Default model:** `claude-opus-4-6` (원격) / `qwen3.5:9b` 등 (Ollama)
- **Default permissions:** `danger-full-access`

## License

See repository root.
