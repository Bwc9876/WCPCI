use std::{path::PathBuf, process::Stdio};

use log::error;
use tokio::io::AsyncWriteExt;

use crate::problems::TestCase;

use super::job::CaseStatus;

#[derive(Debug, Clone)]
pub enum CaseError {
    Logic,
    //TimeLimitExceeded,
    Runtime(String),
    Compilation(String),
    Judge(String),
}

impl From<CaseError> for CaseStatus {
    fn from(val: CaseError) -> Self {
        CaseStatus::Failed(match val {
            CaseError::Logic => "Logic error".to_string(),
            //CaseError::TimeLimitExceeded => "Time limit exceeded".to_string(),
            CaseError::Runtime(_) => "Runtime error".to_string(),
            CaseError::Compilation(_) => "Compile error".to_string(),
            CaseError::Judge(_) => "Judge error".to_string(),
        })
    }
}

pub type CaseResult<T = ()> = Result<T, CaseError>;

pub struct Runner {
    run_cmd: String,
    compile_cmd: String,
    file_name: String,
    temp_path: PathBuf,
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
    ) -> CaseResult<Self> {
        let now_nanos = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map_err(|e| CaseError::Judge(format!("Couldn't get time: {e:?}")))?
            .as_nanos();

        let dir_name = format!("run_jon_wcpc_{id}_{}", now_nanos);

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
                //.stdin(Stdio::null())
                .stdout(Stdio::piped())
                .stderr(Stdio::piped());
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
            .stderr(Stdio::piped());

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

        let output = child
            .wait_with_output()
            .await
            .map_err(|e| CaseError::Judge(format!("Couldn't get output: {e:?}")))?;

        // Sleep for a bit for pizzaz
        tokio::time::sleep(std::time::Duration::from_millis(500)).await;

        if output.status.success() {
            Ok(String::from_utf8_lossy(&output.stdout).to_string())
        } else {
            let path_str = self
                .temp_path
                .join(&self.file_name)
                .to_string_lossy()
                .to_string();
            let std_err = String::from_utf8_lossy(&output.stderr)
                .to_string()
                .replace(&path_str, "<your program>");
            let code = output.status.code().unwrap_or(-1);
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
}
