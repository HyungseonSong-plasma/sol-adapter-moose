use sol_adaptor_moose::backend::{MooseProcessRunner, WorkspaceLayout};
use std::fs;
use std::path::PathBuf;

fn temp_path(name: &str) -> PathBuf {
    std::env::temp_dir().join(format!("sol-adaptor-moose-{name}-{}", std::process::id()))
}

#[test]
fn workspace_layout_is_deterministic_and_rejects_unsafe_keys() {
    let root = temp_path("workspace");
    let layout = WorkspaceLayout::new(&root, "phase1.probe-001").unwrap();
    assert_eq!(
        layout.input_path(),
        root.join("runs/phase1.probe-001/input.i")
    );
    assert_eq!(
        layout.stdout_path(),
        root.join("runs/phase1.probe-001/stdout.log")
    );
    assert_eq!(
        layout.stderr_path(),
        root.join("runs/phase1.probe-001/stderr.log")
    );
    layout.prepare().unwrap();
    assert!(layout.run_dir().is_dir());
    fs::remove_dir_all(root).unwrap();

    assert!(WorkspaceLayout::new("/tmp", "../escape").is_err());
    assert!(WorkspaceLayout::new("/tmp", "has space").is_err());
}

#[cfg(unix)]
mod unix {
    use super::temp_path;
    use sol_adaptor_moose::backend::MooseProcessRunner;
    use std::fs;
    use std::os::unix::fs::PermissionsExt;
    use std::time::Duration;

    fn script(name: &str, body: &str) -> std::path::PathBuf {
        let path = temp_path(name);
        fs::write(&path, format!("#!/bin/sh\n{body}\n")).unwrap();
        let mut permissions = fs::metadata(&path).unwrap().permissions();
        permissions.set_mode(0o755);
        fs::set_permissions(&path, permissions).unwrap();
        path
    }

    #[test]
    fn child_stdout_and_stderr_are_captured_separately() {
        let executable = script(
            "capture.sh",
            "echo backend-out; echo backend-err >&2; exit 7",
        );
        let runner = MooseProcessRunner::new(&executable, Duration::from_secs(2));
        let output = runner.run(&[]).unwrap();
        assert_eq!(output.status_code, Some(7));
        assert!(!output.timed_out);
        assert_eq!(output.stdout_text().trim(), "backend-out");
        assert_eq!(output.stderr_text().trim(), "backend-err");
        fs::remove_file(executable).unwrap();
    }

    #[test]
    fn timeout_terminates_child_and_preserves_captured_streams() {
        let executable = script("timeout.sh", "echo started; echo diagnostics >&2; sleep 5");
        let runner = MooseProcessRunner::new(&executable, Duration::from_millis(100));
        let output = runner.run(&[]).unwrap();
        assert!(output.timed_out);
        assert_eq!(output.stdout_text().trim(), "started");
        assert_eq!(output.stderr_text().trim(), "diagnostics");
        fs::remove_file(executable).unwrap();
    }
}
