use std::path::Path;
use std::process::{Command, Output};

fn cli(args: &[&str]) -> Output {
    Command::new(env!("CARGO_BIN_EXE_copal-cli"))
        .args(args)
        .output()
        .unwrap()
}

fn ok(args: &[&str]) -> String {
    let out = cli(args);
    assert!(
        out.status.success(),
        "{args:?} failed: {}",
        String::from_utf8_lossy(&out.stderr)
    );
    String::from_utf8(out.stdout).unwrap()
}

#[test]
fn full_terminal_workflow() {
    let dir = tempfile::tempdir().unwrap();
    let file = dir.path().join("part.copal");
    let f = file.to_str().unwrap();

    ok(&["new", f, "--printer", "elegoo/saturn-4-ultra-16k"]);
    ok(&["run", f, "object.add_cube", "size=10", "name=Test cube"]);
    ok(&["run", f, "object.move", "id=1", "x=-5"]);

    let script = dir.path().join("steps.copalscript");
    std::fs::write(
        &script,
        "# rotate and drop\nobject.rotate id=1 x=45\nobject.drop id=1\n",
    )
    .unwrap();
    assert!(ok(&["script", f, script.to_str().unwrap()]).contains("Applied 2 commands"));

    let info: serde_json::Value = serde_json::from_str(&ok(&["--json", "info", f])).unwrap();
    assert_eq!(info["name"], "part");
    assert_eq!(info["printer"], "elegoo/saturn-4-ultra-16k");
    assert_eq!(info["objects"][0]["name"], "Test cube");
    assert_eq!(info["objects"][0]["position"][0], -5.0);
    assert_eq!(info["objects"][0]["rotation_deg"][0], 45.0);
}

#[test]
fn dry_run_does_not_save() {
    let dir = tempfile::tempdir().unwrap();
    let f = dir.path().join("p.copal");
    let f = f.to_str().unwrap();
    ok(&["new", f]);
    ok(&["run", "--dry-run", f, "object.add_cube", "size=1"]);
    assert!(ok(&["info", f]).contains("Objects  0"));
}

#[test]
fn errors_exit_with_failure_and_a_message() {
    let dir = tempfile::tempdir().unwrap();
    let f = dir.path().join("p.copal");
    let f = f.to_str().unwrap();
    ok(&["new", f]);

    let out = cli(&["new", f]);
    assert!(!out.status.success());
    assert!(String::from_utf8_lossy(&out.stderr).contains("already exists"));

    let out = cli(&["run", f, "object.move", "id=42", "x=1"]);
    assert!(!out.status.success());
    assert!(String::from_utf8_lossy(&out.stderr).contains("no object with id 42"));

    let out = cli(&["info", "/definitely/missing.copal"]);
    assert!(!out.status.success());
    assert!(!Path::new("/definitely/missing.copal").exists());
}

#[test]
fn lists_commands() {
    let text = ok(&["commands"]);
    assert!(text.contains("object.add_cube"));
    let list: serde_json::Value = serde_json::from_str(&ok(&["--json", "commands"])).unwrap();
    assert!(list.as_array().unwrap().len() >= 10);
}
