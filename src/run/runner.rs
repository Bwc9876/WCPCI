use std::process::Stdio;

use tokio::io::AsyncWriteExt;

use crate::problems::TestCase;

use super::job::CaseStatus;

#[derive(Debug, Clone)]
pub enum CaseError {
    Logic,
    //TimeLimitExceeded,
    Runtime(String),
    //Compilation(String),
    Judge(String),
}

impl From<CaseError> for CaseStatus {
    fn from(val: CaseError) -> Self {
        CaseStatus::Failed(match val {
            CaseError::Logic => "Logic error".to_string(),
            //CaseError::TimeLimitExceeded => "Time limit exceeded".to_string(),
            CaseError::Runtime(_) => "Runtime error".to_string(),
            //CaseError::Compilation(_) => "Compile error".to_string(),
            CaseError::Judge(_) => "Judge error".to_string(),
        })
    }
}

pub type CaseResult<T = ()> = Result<T, CaseError>;

pub struct Runner {
    temp_file: async_tempfile::TempFile,
    #[allow(dead_code)]
    max_cpu_time: i64,
}

impl Runner {
    pub async fn new(program: &str, max_cpu_time: i64) -> CaseResult<Self> {
        let temp_file = async_tempfile::TempFile::new()
            .await
            .map_err(|e| CaseError::Judge(format!("Couldn't create temp file: {e:?}")))?;
        let path = temp_file.file_path();

        tokio::fs::write(path, program.as_bytes())
            .await
            .map_err(|e| CaseError::Judge(format!("Couldn't write to temp file: {e:?}")))?;

        Ok(Self {
            temp_file,
            max_cpu_time,
        })
    }

    pub async fn run_cmd(&self, input: &str) -> CaseResult<String> {
        let mut cmd = tokio::process::Command::new("python");

        cmd.arg(self.temp_file.file_path())
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
        tokio::time::sleep(std::time::Duration::from_secs(1)).await;

        if output.status.success() {
            Ok(String::from_utf8_lossy(&output.stdout).to_string())
        } else {
            let path_str = self.temp_file.file_path().to_string_lossy().to_string();
            let std_err = String::from_utf8_lossy(&output.stderr)
                .to_string()
                .replace(&path_str, "<your program>");
            let code = output.status.code().unwrap_or(-1);
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
