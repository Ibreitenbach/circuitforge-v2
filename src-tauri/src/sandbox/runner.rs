use std::{time::{Duration, Instant}, process::Stdio};
use anyhow::Context;
use tokio::{process::Command, io::{AsyncReadExt}, time::timeout};

#[derive(Debug, Clone)]
pub struct ToolRun {
  pub cmd: Vec<String>,
  pub cwd: String,
  pub env: Vec<(String, String)>,
  pub timeout_ms: u64,
  pub max_bytes_stdout: usize,
  pub max_bytes_stderr: usize,
}

#[derive(Debug)]
pub struct ToolResult {
  pub exit_code: i32,
  pub stdout: Vec<u8>,
  pub stderr: Vec<u8>,
  pub duration_ms: u64,
}

pub struct SandboxRunner;

impl SandboxRunner {
  pub fn new() -> Self { Self }

  pub async fn run(&self, tool: ToolRun) -> anyhow::Result<ToolResult> {
    anyhow::ensure!(!tool.cmd.is_empty(), "empty cmd");
    let start = Instant::now();

    let mut c = Command::new(&tool.cmd[0]);
    if tool.cmd.len() > 1 {
      c.args(&tool.cmd[1..]);
    }
    c.current_dir(&tool.cwd)
      .stdin(Stdio::null())
      .stdout(Stdio::piped())
      .stderr(Stdio::piped());

    for (k, v) in tool.env.iter() {
      c.env(k, v);
    }

    let mut child = c.spawn().context("spawn tool")?;
    let mut stdout = child.stdout.take().context("no stdout")?;
    let mut stderr = child.stderr.take().context("no stderr")?;

    let read_limited = |mut r: tokio::process::ChildStdout, limit: usize| async move {
      let mut buf = Vec::with_capacity(limit.min(16 * 1024));
      let mut tmp = vec![0u8; 8192];
      while buf.len() < limit {
        let n = r.read(&mut tmp).await?;
        if n == 0 { break; }
        let remain = limit - buf.len();
        buf.extend_from_slice(&tmp[..n.min(remain)]);
        if n > remain { break; } // truncated
      }
      Ok::<Vec<u8>, std::io::Error>(buf)
    };

    let read_limited_err = |mut r: tokio::process::ChildStderr, limit: usize| async move {
      let mut buf = Vec::with_capacity(limit.min(16 * 1024));
      let mut tmp = vec![0u8; 8192];
      while buf.len() < limit {
        let n = r.read(&mut tmp).await?;
        if n == 0 { break; }
        let remain = limit - buf.len();
        buf.extend_from_slice(&tmp[..n.min(remain)]);
        if n > remain { break; }
      }
      Ok::<Vec<u8>, std::io::Error>(buf)
    };

    let tout = Duration::from_millis(tool.timeout_ms.max(1));
    let (stdout_buf, stderr_buf, status) = timeout(tout, async {
      let (o, e, s) = tokio::join!(
        read_limited(stdout, tool.max_bytes_stdout),
        read_limited_err(stderr, tool.max_bytes_stderr),
        child.wait()
      );
      let o = o?;
      let e = e?;
      let s = s?;
      Ok::<_, anyhow::Error>((o, e, s))
    }).await
      .map_err(|_| anyhow::anyhow!("tool timeout after {}ms", tool.timeout_ms))??;

    let exit_code = status.code().unwrap_or(-1);
    Ok(ToolResult {
      exit_code,
      stdout: stdout_buf,
      stderr: stderr_buf,
      duration_ms: start.elapsed().as_millis() as u64,
    })
  }
}
