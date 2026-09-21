use sol_adaptor_moose::backend::{execute_backend_input, MooseProcessRunner, WorkspaceLayout};
use std::fs;
use std::path::PathBuf;
use std::time::Duration;

fn temp_path(name: &str) -> PathBuf {
    std::env::temp_dir().join(format!("sol-adaptor-moose-{name}-{}", std::process::id()))
}

#[cfg(unix)]
mod unix {
    use super::*;

    fn fixture(name: &str) -> PathBuf {
        PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("tests")
            .join("fixtures")
            .join("process")
            .join(name)
    }

    #[test]
    fn backend_input_executes_in_deterministic_workspace_and_persists_logs() {
        let root = temp_path("backend-workspace");
        let executable = fixture("backend_workspace_success.sh");
        let runner = MooseProcessRunner::new(&executable, Duration::from_secs(2));
        let layout = WorkspaceLayout::new(&root, "backend-only-001").unwrap();
        let output = execute_backend_input(&runner, &layout, "[Mesh]\n[]\n").unwrap();

        assert!(output.success());
        assert_eq!(
            fs::read_to_string(layout.input_path()).unwrap(),
            "[Mesh]\n[]\n"
        );
        let stdout = fs::read_to_string(layout.stdout_path()).unwrap();
        let stderr = fs::read_to_string(layout.stderr_path()).unwrap();
        assert!(stdout.contains(&format!("cwd={}", layout.run_dir().display())));
        assert!(stdout.contains("arg1=-i arg2=input.i"));
        assert_eq!(stderr.trim(), "backend diagnostics");

        fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn failed_backend_execution_still_persists_captured_logs() {
        let root = temp_path("backend-workspace-failure");
        let executable = fixture("backend_workspace_failure.sh");
        let runner = MooseProcessRunner::new(&executable, Duration::from_secs(2));
        let layout = WorkspaceLayout::new(&root, "backend-only-failure").unwrap();
        let output = execute_backend_input(&runner, &layout, "[Mesh]\n[]\n").unwrap();

        assert_eq!(output.status_code, Some(9));
        assert!(!output.success());
        assert_eq!(
            fs::read_to_string(layout.stdout_path()).unwrap().trim(),
            "partial output"
        );
        assert_eq!(
            fs::read_to_string(layout.stderr_path()).unwrap().trim(),
            "failure diagnostics"
        );

        fs::remove_dir_all(root).unwrap();
    }
}
