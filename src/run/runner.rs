use std::{path::PathBuf, process::Stdio};

use log::{error, info, warn};
use tokio::io::AsyncWriteExt;
use tokio::{io::AsyncReadExt, select};

use crate::problems::TestCase;

use super::{job::CaseStatus, manager::ShutdownReceiver};

#[derive(Debug, Clone)]
pub enum CaseError {
    Logic,
    //TimeLimitExceeded,
    Runtime(String),
    Compilation(String),
    Judge(String),
    Cancelled,
}

impl From<CaseError> for CaseStatus {
    fn from(val: CaseError) -> Self {
        let status = match val {
            CaseError::Logic => "Logic error".to_string(),
            //CaseError::TimeLimitExceeded => "Time limit exceeded".to_string(),
            CaseError::Runtime(_) => "Runtime error".to_string(),
            CaseError::Compilation(_) => "Compile error".to_string(),
            CaseError::Judge(_) => "Judge error".to_string(),
            CaseError::Cancelled => "Run Cancelled".to_string(),
        };
        let penalty = matches!(val, CaseError::Logic | CaseError::Runtime(_));
        CaseStatus::Failed(penalty, status)
    }
}

pub type CaseResult<T = ()> = Result<T, CaseError>;

pub struct Runner {
    run_cmd: String,
    compile_cmd: String,
    file_name: String,
    temp_path: PathBuf,
    shutdown_rx: ShutdownReceiver,
    cleaned: bool,
    #[allow(dead_code)]
    max_cpu_time: i64,
}

impl Runner {
    pub async fn new(
        id: u64,
        compile_cmd: &str,
        run_cmd: &str,
        file_name: &str,
        program: &str,
        max_cpu_time: i64,
        shutdown_rx: ShutdownReceiver,
    ) -> CaseResult<Self> {
        let now_nanos = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map_err(|e| CaseError::Judge(format!("Couldn't get time: {e:?}")))?
            .as_nanos();

        let dir_name = format!("run_job_wcpc_{id}_{}", now_nanos);

        let temp_path = std::env::temp_dir().join(dir_name);

        tokio::fs::create_dir(&temp_path)
            .await
            .map_err(|e| CaseError::Judge(format!("Couldn't create temp dir: {e:?}")))?;

        tokio::fs::write(temp_path.join(file_name), program.as_bytes())
            .await
            .map_err(|e| CaseError::Judge(format!("Couldn't write to program file: {e:?}")))?;

        Ok(Self {
            run_cmd: run_cmd.to_string(),
            compile_cmd: compile_cmd.to_string(),
            file_name: file_name.to_string(),
            temp_path,
            max_cpu_time,
            shutdown_rx,
            cleaned: false,
        })
    }

    pub async fn compile(&mut self) -> Result<(), CaseError> {
        if self.compile_cmd.is_empty() {
            Ok(())
        } else {
            let mut cmd = tokio::process::Command::new("bash");
            cmd.arg("-c")
                .arg(&self.compile_cmd)
                .current_dir(&self.temp_path)
                .stdin(Stdio::null())
                .stdout(Stdio::piped())
                .stderr(Stdio::piped())
                .kill_on_drop(true);
            let output = cmd
                .output()
                .await
                .map_err(|e| CaseError::Judge(format!("Couldn't run compile command: {e:?}")))?;
            if !output.status.success() {
                let std_err = String::from_utf8_lossy(&output.stderr).to_string();
                Err(CaseError::Compilation(std_err))
            } else {
                Ok(())
            }
        }
    }

    pub async fn run_cmd(&self, input: &str) -> CaseResult<String> {
        let mut cmd = tokio::process::Command::new("bash");

        cmd.arg("-c")
            .arg(&self.run_cmd)
            .current_dir(&self.temp_path)
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .kill_on_drop(true);

        let mut child = cmd
            .spawn()
            .map_err(|e| CaseError::Judge(format!("Couldn't spawn process: {e:?}")))?;

        let stdin = child
            .stdin
            .as_mut()
            .ok_or(CaseError::Judge("Couldn't open stdin".to_string()))?;

        stdin
            .write_all(input.as_bytes())
            .await
            .map_err(|e| CaseError::Judge(format!("Couldn't write to stdin: {e:?}")))?;

        let mut shutdown_rx = self.shutdown_rx.clone();

        let res = select! {
            res = child.wait() => {
                res.map_err(|e| CaseError::Judge(format!("Couldn't wait for process: {e:?}")))?
            }
            _ = shutdown_rx.changed() => {
                child.kill().await.map_err(|e| CaseError::Judge(format!("Couldn't kill process: {e:?}")))?;
                info!("Process killed forcefully");
                Err(CaseError::Cancelled)?
            }
        };

        // Sleep for a bit for pizzaz
        tokio::time::sleep(std::time::Duration::from_millis(500)).await;

        if res.success() {
            let mut stdout = child
                .stdout
                .ok_or(CaseError::Judge("Couldn't open stdout".to_string()))?;
            let mut output = String::new();
            stdout
                .read_to_string(&mut output)
                .await
                .map_err(|e| CaseError::Judge(format!("Couldn't read stdout: {e:?}")))?;
            Ok(output)
        } else {
            let path_str = self
                .temp_path
                .join(&self.file_name)
                .to_string_lossy()
                .to_string();
            let mut stderr = child
                .stderr
                .ok_or(CaseError::Judge("Couldn't open stderr".to_string()))?;
            let mut std_err = String::new();
            stderr
                .read_to_string(&mut std_err)
                .await
                .map_err(|e| CaseError::Judge(format!("Couldn't read stderr: {e:?}")))?;
            let code = res.code().unwrap_or(-1);
            let std_err = std_err.replace(&path_str, "<your program>");
            error!("Process exited with error {code}:\n\n {std_err}");
            Err(CaseError::Runtime(format!(
                "Process exited with error {code}:\n\n {std_err}"
            )))
        }
    }

    pub async fn run_case(&self, case: &TestCase) -> CaseResult<String> {
        let output = self.run_cmd(&case.stdin).await?;

        let res = case.check_output(&output, &case.expected_pattern);
        res.map_err(CaseError::Judge).and_then(
            |b| {
                if b {
                    Ok(output)
                } else {
                    Err(CaseError::Logic)
                }
            },
        )
    }

    pub async fn cleanup(&mut self) {
        self.cleaned = true;
        if let Err(why) = tokio::fs::remove_dir_all(&self.temp_path).await {
            error!("Couldn't remove temp dir: {why:?}");
        }
    }
}

impl Drop for Runner {
    fn drop(&mut self) {
        if !self.cleaned {
            warn!("Dropping Runner, prefer not to do this as it'll be sync (call Runner::cleanup)");
            std::fs::remove_dir_all(&self.temp_path).unwrap_or_else(|e| {
                error!("Couldn't remove temp dir: {e:?}");
            });
        }
    }
}
