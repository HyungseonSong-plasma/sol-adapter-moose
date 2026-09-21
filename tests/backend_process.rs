use sol_adaptor_moose::backend::WorkspaceLayout;
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
    use sol_adaptor_moose::backend::MooseProcessRunner;
    use std::time::Duration;

    #[test]
    fn child_stdout_and_stderr_are_captured_separately() {
        let runner = MooseProcessRunner::new("/bin/sh", Duration::from_secs(2));
        let output = runner
            .run(&["-c", "echo backend-out; echo backend-err >&2; exit 7"])
            .unwrap();
        assert_eq!(output.status_code, Some(7));
        assert!(!output.timed_out);
        assert_eq!(output.stdout_text().trim(), "backend-out");
        assert_eq!(output.stderr_text().trim(), "backend-err");
    }

    #[test]
    fn timeout_terminates_child_and_preserves_captured_streams() {
        let runner = MooseProcessRunner::new("/bin/sh", Duration::from_millis(100));
        let output = runner
            .run(&["-c", "echo started; echo diagnostics >&2; sleep 5"])
            .unwrap();
        assert!(output.timed_out);
        assert_eq!(output.stdout_text().trim(), "started");
        assert_eq!(output.stderr_text().trim(), "diagnostics");
    }
}
