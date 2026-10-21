use std::io::Cursor;

use copal_core::{CommandError, Document, ObjectId, ProjectError, Session, project, registry};
use glam::Vec3;

fn session_with_cube() -> Session {
    let mut s = Session::new(Document::new("Test"));
    s.run_line("object.add_cube size=10").unwrap();
    s
}

#[test]
fn add_move_undo_redo() {
    let mut s = session_with_cube();
    let id = s.doc.objects()[0].id;
    s.run_line(&format!("object.move id={id} x=5 y=-2"))
        .unwrap();
    assert_eq!(
        s.doc.object(id).unwrap().transform.translation,
        Vec3::new(5.0, -2.0, 0.0)
    );

    assert_eq!(
        s.history.undo(&mut s.doc).unwrap().as_deref(),
        Some("object.move id=1 x=5 y=-2")
    );
    assert_eq!(s.doc.object(id).unwrap().transform.translation, Vec3::ZERO);

    s.history.undo(&mut s.doc).unwrap();
    assert!(s.doc.objects().is_empty());
    assert_eq!(s.history.undo(&mut s.doc).unwrap(), None);

    s.history.redo(&mut s.doc).unwrap();
    s.history.redo(&mut s.doc).unwrap();
    assert_eq!(
        s.doc.object(id).unwrap().transform.translation,
        Vec3::new(5.0, -2.0, 0.0)
    );
    assert!(!s.history.can_redo());
}

#[test]
fn new_command_clears_redo() {
    let mut s = session_with_cube();
    s.run_line("object.rotate id=1 z=90").unwrap();
    s.history.undo(&mut s.doc).unwrap();
    assert!(s.history.can_redo());
    s.run_line("object.scale id=1 factor=2").unwrap();
    assert!(!s.history.can_redo());
}

#[test]
fn remove_and_undo_restores_same_object_in_place() {
    let mut s = Session::new(Document::new("Test"));
    s.run_script("object.add_cube size=1 name=A\nobject.add_cube size=2 name=B\nobject.add_cube size=3 name=C")
        .unwrap();
    let before = s.doc.clone();
    s.run_line("object.remove id=2").unwrap();
    assert_eq!(s.doc.objects().len(), 2);
    s.history.undo(&mut s.doc).unwrap();
    assert_eq!(s.doc, before);
}

#[test]
fn ids_are_never_reused() {
    let mut s = session_with_cube();
    s.run_line("object.remove id=1").unwrap();
    s.run_line("object.add_cube size=5").unwrap();
    assert_eq!(s.doc.objects()[0].id, ObjectId(2));
}

#[test]
fn drop_to_plate_after_rotation() {
    let mut s = session_with_cube();
    s.run_script("object.move id=1 z=7\nobject.rotate id=1 x=45\nobject.drop id=1")
        .unwrap();
    let b = s.doc.objects()[0].world_bounds().unwrap();
    assert!(b.min.z.abs() < 1e-4, "min z = {}", b.min.z);
}

#[test]
fn errors_are_clear_and_change_nothing() {
    let mut s = session_with_cube();
    let rev = s.history.revision();
    assert_eq!(
        s.run_line("object.move id=9 x=1"),
        Err(CommandError::NoSuchObject(ObjectId(9)))
    );
    assert!(matches!(
        s.run_line("object.jump id=1"),
        Err(CommandError::UnknownCommand(_))
    ));
    assert!(matches!(
        s.run_line("object.move id=1 w=3"),
        Err(CommandError::UnknownArg { .. })
    ));
    assert!(matches!(
        s.run_line("object.scale id=1 factor=0"),
        Err(CommandError::Invalid(_))
    ));
    assert_eq!(s.history.revision(), rev);
}

#[test]
fn script_reports_failing_line() {
    let mut s = Session::new(Document::default());
    let err = s
        .run_script("# setup\nobject.add_cube size=1\n\nobject.move id=5 x=1")
        .unwrap_err();
    assert!(err.to_string().starts_with("line 4:"), "{err}");
    assert_eq!(s.doc.objects().len(), 1);
}

#[test]
fn every_recorded_command_replays_to_the_same_document() {
    let mut s = Session::new(Document::new("Replay"));
    s.run_script(
        "project.set_printer printer=elegoo/saturn-4-ultra-16k\n\
         project.set_resin resin=\"anycubic/abs-like-pro-2-grey\"\n\
         object.add_cube x=10 y=20 z=5 name=\"Base plate\"\n\
         object.move id=1 x=3.5\n\
         object.rotate id=1 z=30\n\
         object.scale id=1 x=1.5\n\
         object.rename id=1 name=\"Renamed base\"\n\
         object.drop id=1\n\
         project.rename name=\"My project\"",
    )
    .unwrap();
    let recorded: Vec<String> = s.history.undo_labels().map(str::to_string).collect();

    let mut replay = Session::new(Document::new("Replay"));
    replay.run_script(&recorded.join("\n")).unwrap();
    assert_eq!(replay.doc, s.doc);
}

#[test]
fn registry_is_sorted_and_documented() {
    let names: Vec<_> = registry().iter().map(|c| c.name).collect();
    let mut sorted = names.clone();
    sorted.sort();
    assert_eq!(names, sorted);
    for c in registry() {
        assert!(
            c.usage.starts_with(c.name),
            "usage of {} must start with its name",
            c.name
        );
        assert!(!c.summary.is_empty());
    }
}

#[test]
fn project_round_trip_in_memory() {
    let mut s = session_with_cube();
    s.run_script("project.set_printer printer=elegoo/saturn-4-ultra-16k\nobject.move id=1 x=12\nobject.add_cube size=3")
        .unwrap();
    let mut buf = Cursor::new(Vec::new());
    project::write_to(&s.doc, &mut buf).unwrap();
    buf.set_position(0);
    let back = project::read_from(buf).unwrap();
    assert_eq!(back, s.doc);
}

#[test]
fn project_save_load_and_dirty_flag() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("part.copal");
    let mut s = session_with_cube();
    assert!(s.is_dirty());
    s.save_as(&path).unwrap();
    assert!(!s.is_dirty());
    assert!(!dir.path().join("part.copal.tmp").exists());

    let opened = Session::open(&path).unwrap();
    assert_eq!(opened.doc, s.doc);
    assert!(!opened.is_dirty());

    s.run_line("object.move id=1 y=1").unwrap();
    assert!(s.is_dirty());
    assert!(s.save().unwrap());
    assert_eq!(Session::open(&path).unwrap().doc, s.doc);
}

#[test]
fn rejects_non_projects_and_newer_versions() {
    let err = project::read_from(Cursor::new(b"not a zip".to_vec())).unwrap_err();
    assert!(matches!(err, ProjectError::Zip(_)), "{err}");

    let mut buf = Cursor::new(Vec::new());
    project::write_to(&Document::default(), &mut buf).unwrap();
    let mut zip = zip::ZipArchive::new(Cursor::new(buf.into_inner())).unwrap();
    let mut json = String::new();
    std::io::Read::read_to_string(&mut zip.by_name("project.json").unwrap(), &mut json).unwrap();
    let newer = json.replace("\"version\": 1", "\"version\": 99");

    let mut out = zip::ZipWriter::new(Cursor::new(Vec::new()));
    out.start_file("project.json", zip::write::SimpleFileOptions::default())
        .unwrap();
    std::io::Write::write_all(&mut out, newer.as_bytes()).unwrap();
    let bytes = out.finish().unwrap().into_inner();
    let err = project::read_from(Cursor::new(bytes)).unwrap_err();
    assert!(matches!(err, ProjectError::TooNew { found: 99 }), "{err}");
}
