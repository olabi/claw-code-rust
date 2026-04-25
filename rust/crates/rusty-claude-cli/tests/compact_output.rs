use std::fs;
use std::io::Write;
use std::path::PathBuf;
use std::process::{Command, Output, Stdio};
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::{SystemTime, UNIX_EPOCH};

use mock_anthropic_service::{MockAnthropicService, SCENARIO_PREFIX};
use serde_json::Value;

static TEMP_COUNTER: AtomicU64 = AtomicU64::new(0);

#[test]
fn compact_flag_prints_only_final_assistant_text_without_tool_call_details() {
    // given a workspace pointed at the mock Anthropic service and a fixture file
    // that the read_file_roundtrip scenario will fetch through a tool call
    let runtime = tokio::runtime::Runtime::new().expect("tokio runtime should build");
    let server = runtime
        .block_on(MockAnthropicService::spawn())
        .expect("mock service should start");
    let base_url = server.base_url();

    let workspace = unique_temp_dir("compact-read-file");
    let config_home = workspace.join("config-home");
    let home = workspace.join("home");
    fs::create_dir_all(&workspace).expect("workspace should exist");
    fs::create_dir_all(&config_home).expect("config home should exist");
    fs::create_dir_all(&home).expect("home should exist");
    fs::write(workspace.join("fixture.txt"), "alpha parity line\n").expect("fixture should write");

    // when we run claw in compact text mode against a tool-using scenario
    let prompt = format!("{SCENARIO_PREFIX}read_file_roundtrip");
    let output = run_claw(
        &workspace,
        &config_home,
        &home,
        &base_url,
        &[
            "--model",
            "sonnet",
            "--permission-mode",
            "read-only",
            "--allowedTools",
            "read_file",
            "--compact",
            &prompt,
        ],
    );

    // then the command exits successfully and stdout contains exactly the final
    // assistant text with no tool call IDs, JSON envelopes, or spinner output
    assert!(
        output.status.success(),
        "compact run should succeed\nstdout:\n{}\n\nstderr:\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr),
    );
    let stdout = String::from_utf8(output.stdout).expect("stdout should be utf8");
    let trimmed = stdout.trim_end_matches('\n');
    assert_eq!(
        trimmed, "read_file roundtrip complete: alpha parity line",
        "compact stdout should contain only the final assistant text"
    );
    assert!(
        !stdout.contains("toolu_"),
        "compact stdout must not leak tool_use_id ({stdout:?})"
    );
    assert!(
        !stdout.contains("\"tool_uses\""),
        "compact stdout must not leak json envelopes ({stdout:?})"
    );
    assert!(
        !stdout.contains("Thinking"),
        "compact stdout must not include the spinner banner ({stdout:?})"
    );

    fs::remove_dir_all(&workspace).expect("workspace cleanup should succeed");
}

#[test]
fn compact_flag_streaming_text_only_emits_final_message_text() {
    // given a workspace pointed at the mock Anthropic service running the
    // streaming_text scenario which only emits a single assistant text block
    let runtime = tokio::runtime::Runtime::new().expect("tokio runtime should build");
    let server = runtime
        .block_on(MockAnthropicService::spawn())
        .expect("mock service should start");
    let base_url = server.base_url();

    let workspace = unique_temp_dir("compact-streaming-text");
    let config_home = workspace.join("config-home");
    let home = workspace.join("home");
    fs::create_dir_all(&workspace).expect("workspace should exist");
    fs::create_dir_all(&config_home).expect("config home should exist");
    fs::create_dir_all(&home).expect("home should exist");

    // when we invoke claw with --compact for the streaming text scenario
    let prompt = format!("{SCENARIO_PREFIX}streaming_text");
    let output = run_claw(
        &workspace,
        &config_home,
        &home,
        &base_url,
        &[
            "--model",
            "sonnet",
            "--permission-mode",
            "read-only",
            "--compact",
            &prompt,
        ],
    );

    // then stdout should be exactly the assistant text followed by a newline
    assert!(
        output.status.success(),
        "compact streaming run should succeed\nstdout:\n{}\n\nstderr:\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr),
    );
    let stdout = String::from_utf8(output.stdout).expect("stdout should be utf8");
    assert_eq!(
        stdout, "Mock streaming says hello from the parity harness.\n",
        "compact streaming stdout should contain only the final assistant text"
    );

    fs::remove_dir_all(&workspace).expect("workspace cleanup should succeed");
}

#[test]
fn compact_flag_with_json_output_emits_structured_json() {
    let runtime = tokio::runtime::Runtime::new().expect("tokio runtime should build");
    let server = runtime
        .block_on(MockAnthropicService::spawn())
        .expect("mock service should start");
    let base_url = server.base_url();

    let workspace = unique_temp_dir("compact-json");
    let config_home = workspace.join("config-home");
    let home = workspace.join("home");
    fs::create_dir_all(&workspace).expect("workspace should exist");
    fs::create_dir_all(&config_home).expect("config home should exist");
    fs::create_dir_all(&home).expect("home should exist");

    let prompt = format!("{SCENARIO_PREFIX}streaming_text");
    let output = run_claw(
        &workspace,
        &config_home,
        &home,
        &base_url,
        &[
            "--model",
            "sonnet",
            "--permission-mode",
            "read-only",
            "--output-format",
            "json",
            "--compact",
            &prompt,
        ],
    );

    assert!(
        output.status.success(),
        "compact json run should succeed
stdout:
{}

stderr:
{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr),
    );
    let stdout = String::from_utf8(output.stdout).expect("stdout should be utf8");
    let parsed: Value = serde_json::from_str(&stdout).expect("compact json stdout should parse");
    assert_eq!(
        parsed["message"],
        "Mock streaming says hello from the parity harness."
    );
    assert_eq!(parsed["compact"], true);
    assert_eq!(parsed["model"], "claude-sonnet-4-6");
    assert!(parsed["usage"].is_object());

    fs::remove_dir_all(&workspace).expect("workspace cleanup should succeed");
}

/// Regression: when the prompt is piped via stdin (no positional prompt arg),
/// `--compact` was silently dropped — the parser hardcoded `compact: false`
/// in the empty-rest branch, dispatching to the interactive `run_turn` path
/// and leaking the spinner ANSI escape sequences plus the streaming reasoning
/// trace into stdout. This breaks shell pipelines like
/// `echo "summarize Cargo.toml" | claw --compact | wc -l` (the documented
/// usage in --help). Verify that compact mode is honored on the stdin path.
#[test]
fn compact_flag_is_honored_when_prompt_arrives_via_stdin_pipe() {
    let runtime = tokio::runtime::Runtime::new().expect("tokio runtime should build");
    let server = runtime
        .block_on(MockAnthropicService::spawn())
        .expect("mock service should start");
    let base_url = server.base_url();

    let workspace = unique_temp_dir("compact-stdin-pipe");
    let config_home = workspace.join("config-home");
    let home = workspace.join("home");
    fs::create_dir_all(&workspace).expect("workspace should exist");
    fs::create_dir_all(&config_home).expect("config home should exist");
    fs::create_dir_all(&home).expect("home should exist");

    let prompt = format!("{SCENARIO_PREFIX}streaming_text");
    let output = run_claw_with_stdin(
        &workspace,
        &config_home,
        &home,
        &base_url,
        &[
            "--model",
            "sonnet",
            "--permission-mode",
            "read-only",
            "--compact",
        ],
        &prompt,
    );

    assert!(
        output.status.success(),
        "compact stdin run should succeed\nstdout:\n{}\n\nstderr:\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr),
    );
    let stdout = String::from_utf8(output.stdout).expect("stdout should be utf8");
    assert_eq!(
        stdout, "Mock streaming says hello from the parity harness.\n",
        "compact stdin stdout should contain only the final assistant text"
    );
    assert!(
        !stdout.contains("Thinking"),
        "compact stdin stdout must not include the spinner banner ({stdout:?})"
    );
    assert!(
        !stdout.contains("\x1b["),
        "compact stdin stdout must not leak ANSI escape sequences ({stdout:?})"
    );

    fs::remove_dir_all(&workspace).expect("workspace cleanup should succeed");
}

fn run_claw(
    cwd: &std::path::Path,
    config_home: &std::path::Path,
    home: &std::path::Path,
    base_url: &str,
    args: &[&str],
) -> Output {
    let mut command = Command::new(env!("CARGO_BIN_EXE_claw"));
    command
        .current_dir(cwd)
        .env_clear()
        .env("ANTHROPIC_API_KEY", "test-compact-key")
        .env("ANTHROPIC_BASE_URL", base_url)
        .env("CLAW_CONFIG_HOME", config_home)
        .env("HOME", home)
        .env("NO_COLOR", "1")
        .env("PATH", "/usr/bin:/bin")
        .args(args);
    command.output().expect("claw should launch")
}

/// Run claw with `stdin_input` piped to its stdin — simulates the
/// `echo "..." | claw --compact` shell idiom used by pipelines.
fn run_claw_with_stdin(
    cwd: &std::path::Path,
    config_home: &std::path::Path,
    home: &std::path::Path,
    base_url: &str,
    args: &[&str],
    stdin_input: &str,
) -> Output {
    let mut command = Command::new(env!("CARGO_BIN_EXE_claw"));
    command
        .current_dir(cwd)
        .env_clear()
        .env("ANTHROPIC_API_KEY", "test-compact-key")
        .env("ANTHROPIC_BASE_URL", base_url)
        .env("CLAW_CONFIG_HOME", config_home)
        .env("HOME", home)
        .env("NO_COLOR", "1")
        .env("PATH", "/usr/bin:/bin")
        .args(args)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());
    let mut child = command.spawn().expect("claw should launch");
    {
        let stdin = child.stdin.as_mut().expect("stdin pipe should attach");
        stdin
            .write_all(stdin_input.as_bytes())
            .expect("stdin write should succeed");
    }
    child
        .wait_with_output()
        .expect("claw should run to completion")
}

fn unique_temp_dir(label: &str) -> PathBuf {
    let millis = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("clock should be after epoch")
        .as_millis();
    let counter = TEMP_COUNTER.fetch_add(1, Ordering::Relaxed);
    std::env::temp_dir().join(format!(
        "claw-compact-{label}-{}-{millis}-{counter}",
        std::process::id()
    ))
}
