//! Midplane's interactive path uses the same associative command as scripts.
use crate::{dialogs, scenario::Harness};
use serde_json::json;
use solvecraft_engine::{Sel, doc::FeatureKind, geom::Vec3};

fn part() -> Harness {
    let mut h = Harness::new();
    h.app.run("solid.box", json!({"length":40,"width":30,"height":20,"name":"Block"})).unwrap();
    h.frames(3);
    h
}

fn midplane(h: &Harness, name: &str) -> solvecraft_engine::geom::Plane {
    solvecraft_engine::view::construction_planes(&h.app.session).into_iter().find(|(_, n, _)| n == name).unwrap().2
}

#[test]
fn midplane_preselection_preview_apply_and_undo() {
    let mut h = part();
    let top = dialogs::face_sel(&h.app.session, Vec3::new(20.0, 15.0, 20.0)).unwrap();
    h.app.session.selection = vec![Sel::Plane { name: "XY".into() }, top];
    h.app.start("construct.plane.midplane");
    let d = h.app.dialog.as_ref().unwrap();
    assert!(d.previews());
    assert_eq!(d.inputs.iter().map(|i| i.items.len()).collect::<Vec<_>>(), vec![1, 1]);
    let commands = dialogs::apply_commands(&h.app, d).unwrap();
    assert!(commands[0].1["b"].get("face").is_some(), "face must remain associative");
    let before = h.app.session.doc.features.len();
    let undo = h.app.session.undo.len();
    let p = h.app.session.preview(&commands).unwrap();
    let colors = crate::preview::Colors::from_tokens(&crate::theme::Tokens::get());
    let scene = crate::preview::build(&p.before, &p.after, colors).scene;
    assert!(!scene.xray.is_empty(), "preview must draw the construction plane");
    assert!(!scene.lines.is_empty(), "preview boundaries must use the rendered line buffer");
    assert_eq!(h.app.session.doc.features.len(), before);
    assert_eq!(h.app.session.undo.len(), undo);
    h.frames(4);
    h.step(&json!({"widget":"OK"})).unwrap();
    assert!(h.app.dialog.is_none());
    let name = h.app.session.doc.features.last().unwrap().name.clone();
    assert!((midplane(&h, &name).origin.z - 10.0).abs() < 1e-8);
    h.app.run("edit.undo", json!({})).unwrap();
    assert_eq!(h.app.session.doc.features.len(), before);
    h.app.run("edit.redo", json!({})).unwrap();
    assert!((midplane(&h, &name).origin.z - 10.0).abs() < 1e-8);
}

#[test]
fn midplane_edit_preserves_names_and_can_replace_a_reference() {
    let mut h = part();
    h.app.run("construct.plane.midplane", json!({"a":"XY","b":{"face":[20,15,20]},"name":"Mid"})).unwrap();
    let feature = h.app.session.doc.find_feature("Mid").unwrap().clone();
    h.step(&json!({"call":"ui.editFeature","params":{"feature":"Mid"}})).unwrap();
    h.frames(5);
    h.step(&json!({"widget":"OK"})).unwrap();
    let edited = h.app.session.doc.find_feature("Mid").unwrap();
    assert_eq!(edited.id, feature.id);
    assert_eq!(edited.kind, feature.kind);
    h.step(&json!({"call":"ui.editFeature","params":{"feature":"Mid"}})).unwrap();
    h.app.dialog.as_mut().unwrap().inputs.get_mut(1).unwrap().items = vec![Sel::Plane { name: "XZ".into() }];
    h.frames(4);
    h.step(&json!({"widget":"OK"})).unwrap();
    assert!(h.app.dialog.is_none());
    assert_ne!(h.app.session.doc.find_feature("Mid").unwrap().kind, feature.kind);
    assert!(midplane(&h, "Mid").normal().z.abs() > 0.6);
}

#[test]
fn midplane_requires_two_inputs_and_cancel_does_not_change_document() {
    let mut h = part();
    let original = h.app.session.doc.clone();
    let undo = h.app.session.undo.len();
    h.app.session.selection = vec![Sel::Plane { name: "XY".into() }];
    h.app.start("construct.plane.midplane");
    assert!(dialogs::apply_commands(&h.app, h.app.dialog.as_ref().unwrap()).is_err());
    h.frames(4);
    h.step(&json!({"widget":"Cancel"})).unwrap();
    assert_eq!(h.app.session.doc.features, original.features);
    assert_eq!(h.app.session.undo.len(), undo);
}

#[test]
fn midplane_from_offset_planes_follows_source_edits() {
    let mut h = Harness::new();
    h.app.run("parameters.add", json!({"name":"height","expression":"20 mm"})).unwrap();
    h.app.run("construct.plane.offset", json!({"base":"XY","offset":"height","name":"Upper"})).unwrap();
    h.app.session.selection = vec![Sel::Plane { name: "XY".into() }, Sel::Plane { name: "Upper".into() }];
    h.app.start("construct.plane.midplane");
    h.frames(4);
    h.step(&json!({"widget":"OK"})).unwrap();
    let feature = h.app.session.doc.features.last().unwrap().clone();
    assert!(matches!(feature.kind, FeatureKind::ConstructionPlane { .. }));
    assert!((midplane(&h, &feature.name).origin.z - 10.0).abs() < 1e-8);
    h.app.run("parameters.change", json!({"name":"height","expression":"40 mm"})).unwrap();
    assert!((midplane(&h, &feature.name).origin.z - 20.0).abs() < 1e-8);
}

#[test]
fn midplane_face_editor_refills_after_source_moves() {
    let mut h = Harness::new();
    h.app.run("parameters.add", json!({"name":"height","expression":"20 mm"})).unwrap();
    h.app.run("solid.box", json!({"length":40,"width":30,"height":"height"})).unwrap();
    h.app.run("construct.plane.midplane", json!({"a":"XY","b":{"face":[20,15,20]},"name":"Mid"})).unwrap();
    h.app.run("parameters.change", json!({"name":"height","expression":"50 mm"})).unwrap();
    let original = h.app.session.doc.find_feature("Mid").unwrap().kind.clone();
    h.step(&json!({"call":"ui.editFeature","params":{"feature":"Mid"}})).unwrap();
    let input = &h.app.dialog.as_ref().unwrap().inputs.get(1).unwrap().items;
    assert!(matches!(input.first(), Some(Sel::Face { point, .. }) if (point.z - 50.0).abs() < 1e-8));
    h.frames(4);
    h.step(&json!({"widget":"OK"})).unwrap();
    assert_eq!(h.app.session.doc.find_feature("Mid").unwrap().kind, original);
    assert!((midplane(&h, "Mid").origin.z - 25.0).abs() < 1e-8);
}
