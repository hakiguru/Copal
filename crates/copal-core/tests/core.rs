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
         object.duplicate id=1 count=2\n\
         object.mirror id=2 axis=y\n\
         object.arrange spacing=4\n\
         print.set exposure_s=1.8 lift_mm=3,1 anti_aliasing=8\n\
         supports.set overhang_angle_deg=50 raft=false\n\
         hollow.set enabled=true wall_thickness_mm=2\n\
         layout.set keep_on_plate=false\n\
         object.remove id=3\n\
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
    let newer = json.replace(
        &format!("\"version\": {}", project::FORMAT_VERSION),
        "\"version\": 99",
    );

    let mut out = zip::ZipWriter::new(Cursor::new(Vec::new()));
    out.start_file("project.json", zip::write::SimpleFileOptions::default())
        .unwrap();
    std::io::Write::write_all(&mut out, newer.as_bytes()).unwrap();
    let bytes = out.finish().unwrap().into_inner();
    let err = project::read_from(Cursor::new(bytes)).unwrap_err();
    assert!(matches!(err, ProjectError::TooNew { found: 99 }), "{err}");
}

#[test]
fn resin_applies_its_profile_and_undo_restores() {
    let mut s = Session::new(Document::default());
    let before = s.doc.settings;
    s.run_line("project.set_printer printer=elegoo/saturn-4-ultra-16k")
        .unwrap();
    assert_eq!(s.doc.settings, before, "no resin yet, settings stay");
    s.run_line("project.set_resin resin=anycubic/abs-like-pro-2-grey")
        .unwrap();
    assert_eq!(s.doc.settings.print.exposure.layer_height_mm, 0.03);
    assert_eq!(s.doc.settings.print.motion.lift_speed, [45.0, 180.0]);
    s.history.undo(&mut s.doc).unwrap();
    assert_eq!(s.doc.settings, before);
    assert_eq!(s.doc.resin, None);
    assert!(
        s.run_line("project.set_printer printer=nobody/nothing")
            .is_err()
    );
}

#[test]
fn settings_commands_check_names_and_values() {
    let mut s = Session::new(Document::default());
    assert!(
        s.run_line("print.set exposure_s=2.2 bottom_layers=6")
            .is_ok()
    );
    assert_eq!(s.doc.settings.print.exposure.exposure_s, 2.2);
    assert_eq!(s.doc.settings.print.exposure.bottom_layers, 6);
    for bad in [
        "print.set exposur_s=2",
        "print.set exposure_s=fast",
        "print.set exposure_s=-1",
        "print.set anti_aliasing=3",
        "print.set lift_mm=1",
        "print.set layer_height_mm=0",
        "supports.set overhang_angle_deg=95",
        "print.set",
    ] {
        assert!(s.run_line(bad).is_err(), "{bad} should fail");
    }
    assert_eq!(s.history.undo_labels().count(), 1);
}

#[test]
fn a_full_settings_line_restores_a_preset() {
    let mut a = Document::default();
    a.settings.print.exposure.exposure_s = 1.35;
    a.settings.print.motion.lift_mm = [2.5, 1.0];
    a.settings.supports.raft = false;
    let mut s = Session::new(Document::default());
    for section in copal_core::Section::ALL {
        s.run_line(&copal_core::SetSettings::line_for(section, &a.settings))
            .unwrap();
    }
    assert_eq!(s.doc.settings, a.settings);
}

#[test]
fn import_places_models_side_by_side_and_undoes() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("block.stl");
    let mut mesh = copal_mesh::Mesh::cuboid(glam::Vec3::new(20.0, 10.0, 5.0));
    for p in &mut mesh.positions {
        *p += glam::Vec3::new(100.0, 50.0, 7.0);
    }
    std::fs::write(&path, copal_mesh::import::write_stl(&mesh)).unwrap();
    let mut s = Session::new(Document::default());
    let line = format!(
        "object.import path={}",
        copal_core::script::quote(&path.to_string_lossy())
    );
    s.run_line(&line).unwrap();
    s.run_line(&line).unwrap();
    let objects = s.doc.objects();
    assert_eq!(objects.len(), 2);
    assert_eq!(objects[0].name, "block");
    let (a, b) = (
        objects[0].world_bounds().unwrap(),
        objects[1].world_bounds().unwrap(),
    );
    assert!(
        a.min.z.abs() < 1e-4 && b.min.z.abs() < 1e-4,
        "both stand on the plate"
    );
    assert!(
        a.center().x.abs() < 1e-4 && a.center().y.abs() < 1e-4,
        "first is centered"
    );
    let apart =
        a.max.x <= b.min.x || b.max.x <= a.min.x || a.max.y <= b.min.y || b.max.y <= a.min.y;
    assert!(apart, "second one does not overlap the first");
    s.history.undo(&mut s.doc).unwrap();
    assert_eq!(s.doc.objects().len(), 1);
    s.history.redo(&mut s.doc).unwrap();
    assert_eq!(s.doc.objects().len(), 2);
}

#[test]
fn rotating_keeps_a_model_on_the_plate() {
    let mut s = Session::new(Document::default());
    s.run_script("object.add_cube size=10\nobject.rotate id=1 x=45")
        .unwrap();
    let b = s.doc.objects()[0].world_bounds().unwrap();
    assert!(b.min.z.abs() < 1e-4, "{b:?}");
    s.run_script("layout.set keep_on_plate=false\nobject.rotate id=1 x=45")
        .unwrap();
    let b = s.doc.objects()[0].world_bounds().unwrap();
    assert!(b.min.z.abs() > 1.0, "{b:?}");
}

#[test]
fn version_one_projects_still_open() {
    let mut buf = Cursor::new(Vec::new());
    project::write_to(&Document::new("old"), &mut buf).unwrap();
    let mut zip = zip::ZipArchive::new(Cursor::new(buf.into_inner())).unwrap();
    let mut json = String::new();
    std::io::Read::read_to_string(&mut zip.by_name("project.json").unwrap(), &mut json).unwrap();
    let mut v: serde_json::Value = serde_json::from_str(&json).unwrap();
    v["version"] = 1.into();
    v.as_object_mut().unwrap().remove("settings");
    let mut out = zip::ZipWriter::new(Cursor::new(Vec::new()));
    out.start_file("project.json", zip::write::SimpleFileOptions::default())
        .unwrap();
    std::io::Write::write_all(&mut out, v.to_string().as_bytes()).unwrap();
    let bytes = out.finish().unwrap().into_inner();
    let doc = project::read_from(Cursor::new(bytes)).unwrap();
    assert_eq!(doc.name, "old");
    assert_eq!(doc.settings, copal_core::ProjectSettings::default());
}

#[test]
fn supports_hollow_and_holes_follow_the_object_and_undo() {
    let mut s = Session::new(Document::new("prep"));
    s.run_script("object.add_cube size=20\nobject.rotate id=1 x=30 y=-15")
        .unwrap();
    let bottom = s.doc.objects()[0].world_bounds().unwrap().min.z;
    assert!(bottom.abs() < 1e-3);
    s.run_line("supports.auto id=1").unwrap();
    let o = &s.doc.objects()[0];
    let set = o.prep.supports.clone().unwrap();
    assert!(set.supports.len() > 10);
    let lifted = o.world_bounds().unwrap().min.z;
    assert!(
        (lifted - 6.0).abs() < 1e-3,
        "lift 5 mm + raft 1 mm, got {lifted}"
    );

    // Moving keeps the supports (they are relative to the object).
    s.run_line("object.move id=1 x=12").unwrap();
    assert_eq!(s.doc.objects()[0].prep.supports.as_deref(), Some(&*set));
    // Dropping is refused, rotating removes them and puts the model back down.
    assert!(s.run_line("object.drop id=1").is_err());
    s.run_line("object.rotate id=1 z=10").unwrap();
    assert!(s.doc.objects()[0].prep.supports.is_none());
    assert!(s.doc.objects()[0].world_bounds().unwrap().min.z.abs() < 1e-3);
    s.history.undo(&mut s.doc).unwrap();
    assert_eq!(s.doc.objects()[0].prep.supports.as_deref(), Some(&*set));

    // Remove two supports, resize one, add one by hand under the model.
    let ids: Vec<u32> = set.supports.iter().map(|x| x.id).collect();
    s.run_line(&format!("supports.remove id=1 sid={},{}", ids[0], ids[1]))
        .unwrap();
    s.run_line(&format!("supports.resize id=1 sid={} diameter=1.6", ids[2]))
        .unwrap();
    let now = s.doc.objects()[0].prep.supports.clone().unwrap();
    assert_eq!(now.supports.len(), set.supports.len() - 2);
    assert_eq!(now.get(ids[2]).unwrap().diameter, 1.6);
    let tip = now.get(ids[2]).unwrap().tip + s.doc.objects()[0].transform.translation;
    s.run_line(&format!(
        "supports.add id=1 x={} y={} z={}",
        tip.x + 1.5,
        tip.y,
        tip.z
    ))
    .unwrap();

    // Hollow and a drain hole.
    s.run_line("object.hollow id=1 thickness=2 voxel=0.6")
        .unwrap();
    assert!(s.doc.objects()[0].prep.cavity.is_some());
    let b = s.doc.objects()[0].world_bounds().unwrap();
    let c = b.center();
    let top = copal_core::prepare::object_bvh(&s.doc.objects()[0])
        .raycast(Vec3::new(c.x, c.y, b.max.z + 5.0), Vec3::NEG_Z, 0.0, 100.0)
        .unwrap()
        .point;
    s.run_line(&format!(
        "holes.add id=1 x={} y={} z={}",
        top.x, top.y, top.z
    ))
    .unwrap();
    assert_eq!(s.doc.objects()[0].prep.holes.len(), 1);

    // Everything survives saving.
    let mut buf = Cursor::new(Vec::new());
    project::write_to(&s.doc, &mut buf).unwrap();
    buf.set_position(0);
    let back = project::read_from(buf).unwrap();
    assert_eq!(back, s.doc);

    s.run_line("supports.clear id=1").unwrap();
    assert!(s.doc.objects()[0].prep.supports.is_none());
    s.run_line("object.unhollow id=1").unwrap();
    assert!(s.doc.objects()[0].prep.cavity.is_none());
}

#[test]
fn center_reset_overhangs_and_automatic_drain_holes() {
    let mut s = Session::new(Document::new("tools"));
    s.run_script(
        "object.add_cube size=20\nobject.move id=1 x=30 y=-12\nobject.scale id=1 factor=2",
    )
    .unwrap();
    // Center: the middle of the plate, height unchanged.
    s.run_line("object.center id=1").unwrap();
    let b = s.doc.objects()[0].world_bounds().unwrap();
    assert!(b.center().truncate().length() < 1e-3, "{:?}", b.center());
    assert!(b.min.z.abs() < 1e-3);
    s.history.undo(&mut s.doc).unwrap();
    assert_eq!(s.doc.objects()[0].transform.translation.x, 30.0);

    // Reset: rotation by default, scale or both on request, and the model stays on the plate.
    s.run_line("object.rotate id=1 x=40 y=10").unwrap();
    s.run_line("object.reset id=1").unwrap();
    let t = s.doc.objects()[0].transform;
    assert_eq!(t.rotation_deg, Vec3::ZERO);
    assert_eq!(t.scale, Vec3::splat(2.0));
    assert!(s.doc.objects()[0].world_bounds().unwrap().min.z.abs() < 1e-3);
    s.run_line("object.mirror id=1 axis=x").unwrap();
    s.run_line("object.reset id=1 what=all").unwrap();
    assert_eq!(
        s.doc.objects()[0].transform.scale,
        Vec3::new(-1.0, 1.0, 1.0),
        "a mirrored model stays mirrored"
    );
    assert!(s.run_line("object.reset id=1 what=everything").is_err());
    s.history.undo(&mut s.doc).unwrap();
    assert_eq!(s.doc.objects()[0].transform.scale.x, -2.0);
    s.run_line("object.reset id=1 what=all").unwrap();

    // A cube flat on the plate has nothing hanging over; tilted and lifted it has.
    let flat = copal_core::prepare::overhang_report(&s.doc, ObjectId(1)).unwrap();
    assert_eq!((flat.regions, flat.minima), (0, 0));
    assert!(flat.contacts.is_empty());
    s.run_script("object.rotate id=1 x=33 y=-16\nobject.move id=1 z=6")
        .unwrap();
    let tilted = copal_core::prepare::overhang_report(&s.doc, ObjectId(1)).unwrap();
    assert_eq!((tilted.regions, tilted.minima), (1, 1));
    assert!((tilted.area - 400.0).abs() < 1.0, "{}", tilted.area);

    // Drain holes need a hollow; then they go in at the lowest points, straight down.
    assert!(s.run_line("holes.auto id=1").is_err());
    s.run_line("object.hollow id=1 thickness=2 voxel=0.6")
        .unwrap();
    s.run_line("holes.auto id=1 count=2 diameter=2.5").unwrap();
    let o = &s.doc.objects()[0];
    assert_eq!(o.prep.holes.len(), 2);
    let m = o.transform.matrix();
    for h in &o.prep.holes {
        assert_eq!(h.diameter, 2.5);
        let dir = m.transform_vector3(h.dir).normalize();
        assert!(dir.z > 0.99, "holes go straight up into the model: {dir}");
        let p = m.transform_point3(h.point);
        assert!(p.z < 14.0, "near the bottom: {p}");
    }
    assert!(s.run_line("holes.auto id=1 count=40").is_err());
    s.history.undo(&mut s.doc).unwrap();
    assert!(s.doc.objects()[0].prep.holes.is_empty());
}

#[test]
fn auto_orientation_turns_a_flat_brick_and_undoes() {
    let mut s = Session::new(Document::new("orient"));
    s.run_script("object.add_cube x=60 y=40 z=10\nobject.move id=1 x=25 y=-10\nsupports.auto id=1")
        .unwrap();
    let before = s.doc.objects()[0].clone();
    let found =
        copal_core::prepare::orient_object(&s.doc, ObjectId(1), Default::default()).unwrap();
    // Lying flat, the whole 60 × 40 mm face is one layer and hangs over when lifted.
    assert!(found.before.peak_area > 2000.0, "{:?}", found.before);
    assert!(
        found.after.peak_area < found.before.peak_area * 0.6,
        "{:?}",
        found.after
    );
    assert!(
        found.after.growth < found.before.growth * 0.4,
        "{:?}",
        found.after
    );

    s.run_line("object.orient id=1").unwrap();
    let o = &s.doc.objects()[0];
    assert_eq!(o.transform, found.transform);
    assert_ne!(o.transform.rotation_deg, Vec3::ZERO);
    // On the plate, at the same place, and the old supports are gone.
    let (b, old) = (o.world_bounds().unwrap(), before.world_bounds().unwrap());
    assert!(b.min.z.abs() < 1e-3, "{}", b.min.z);
    assert!((b.center().truncate() - old.center().truncate()).length() < 1e-2);
    assert!(o.prep.supports.is_none());
    assert_eq!(
        s.history.undo(&mut s.doc).unwrap().as_deref(),
        Some("object.orient id=1")
    );
    assert_eq!(s.doc.objects()[0], before);

    // Other goals, and a clear error for a wrong one.
    s.run_line("object.orient id=1 goal=fast").unwrap();
    let low = s.doc.objects()[0].world_bounds().unwrap().size().z;
    s.run_line("object.orient id=1 goal=strong").unwrap();
    let tall = s.doc.objects()[0].world_bounds().unwrap().size().z;
    assert!(low < tall, "{low} {tall}");
    assert!(s.run_line("object.orient id=1 goal=pretty").is_err());
    // A model bigger than the printer cannot be turned to fit.
    s.run_line("object.scale id=1 factor=20").unwrap();
    let e = s.run_line("object.orient id=1").unwrap_err().to_string();
    assert!(e.contains("does not fit"), "{e}");
}

#[test]
fn prepare_makes_a_model_ready_in_one_undo_step() {
    let mut s = Session::new(Document::new("ready"));
    s.run_line("object.add_cube x=40 y=30 z=24").unwrap();
    let before = s.doc.clone();
    s.run_line("object.prepare id=1 hollow=true").unwrap();
    let o = &s.doc.objects()[0];
    assert_ne!(o.transform.rotation_deg, Vec3::ZERO, "it was turned");
    assert!(o.prep.cavity.is_some(), "it is hollow");
    assert_eq!(o.prep.holes.len(), 2);
    let set = o.prep.supports.clone().unwrap();
    assert!(
        set.supports.len() > 10 && !set.braces.is_empty(),
        "{} supports, {} braces, turned {}",
        set.supports.len(),
        set.braces.len(),
        o.transform.rotation_deg
    );
    assert!((o.world_bounds().unwrap().min.z - 6.0).abs() < 1e-3);
    // The holes open the hollow and nothing is left in mid-air.
    let check = copal_core::prepare::hole_check(&s.doc, ObjectId(1)).unwrap();
    assert!(
        check.ok() && check.cavities == 1 && check.single == 0,
        "{check:?}"
    );
    assert!(
        copal_core::prepare::unsupported_islands(&s.doc, ObjectId(1))
            .unwrap()
            .is_empty()
    );
    // No support stands in the mouth of a hole.
    let m = o.transform.matrix();
    for h in &o.prep.holes {
        let mouth = m.transform_point3(h.point);
        for x in &set.supports {
            let tip = x.tip + o.transform.translation;
            // The rim of a hole may be held, the hole itself stays free.
            assert!(tip.distance(mouth) >= h.diameter / 2.0 * 0.9, "{x:?}");
        }
    }
    // The recorded line does the same again on a fresh project.
    let line = s.history.undo_labels().last().unwrap().to_string();
    assert_eq!(
        line,
        "object.prepare id=1 orient=balanced hollow=true holes=true supports=auto"
    );
    let mut replay = Session::new(Document::new("ready"));
    replay
        .run_script(&format!("object.add_cube x=40 y=30 z=24\n{line}"))
        .unwrap();
    assert_eq!(replay.doc, s.doc);
    // One step back and everything is as it was.
    s.history.undo(&mut s.doc).unwrap();
    assert_eq!(s.doc, before);

    // Steps can be left out: keep the orientation and the model solid, islands only.
    s.run_line("object.prepare id=1 orient=off supports=islands")
        .unwrap();
    let o = &s.doc.objects()[0];
    assert_eq!(o.transform.rotation_deg, Vec3::ZERO);
    assert!(o.prep.cavity.is_none() && o.prep.holes.is_empty());
    assert!(o.prep.has_supports());
}

#[test]
fn drain_holes_are_checked_and_reach_the_hollow_by_themselves() {
    let mut s = Session::new(Document::new("holes"));
    s.run_script("object.add_cube size=30\nobject.move id=1 z=6\nobject.hollow id=1 thickness=3")
        .unwrap();
    let id = ObjectId(1);
    let check = |s: &Session| copal_core::prepare::hole_check(&s.doc, id).unwrap();
    // No hole yet: the hollow is closed.
    let c = check(&s);
    assert_eq!((c.cavities, c.closed), (1, 1));
    assert!(!c.ok());
    assert!(c.summary(0).contains("no drain hole"), "{}", c.summary(0));

    // A hole that is too short ends in the wall.
    s.run_line("holes.add id=1 x=15 y=0 z=20 depth=1").unwrap();
    let c = check(&s);
    assert_eq!(c.blind, vec![0]);
    assert_eq!(c.closed, 1);
    // Without a depth the hole goes through the 3 mm wall.
    s.run_line("holes.clear id=1").unwrap();
    s.run_line("holes.add id=1 x=15 y=0 z=20").unwrap();
    let h = s.doc.objects()[0].prep.holes[0];
    assert!(h.depth > 3.0 && h.depth < 6.0, "{h:?}");
    let c = check(&s);
    assert!(c.ok(), "{c:?}");
    assert_eq!(c.single, 1, "one hole drains slowly");
    assert!(c.summary(1).contains("second hole"), "{}", c.summary(1));
    s.run_line("holes.add id=1 x=-15 y=0 z=20").unwrap();
    let c = check(&s);
    assert!(c.ok() && c.single == 0);
    assert_eq!(c.summary(2), "2 holes open the hollow");
    // A solid model has nothing to check.
    s.run_line("object.unhollow id=1").unwrap();
    assert_eq!(check(&s), Default::default());
}
