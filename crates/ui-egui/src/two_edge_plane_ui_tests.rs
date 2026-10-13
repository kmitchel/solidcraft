//! Plane Through Two Edges follows the ordinary construction dialog workflow.
use crate::{dialogs, scenario::Harness};
use serde_json::json;
use solidcraft_engine::{Sel, doc::FeatureKind, geom::Vec3};

fn block() -> Harness {
    let mut h = Harness::new();
    h.app.run("solid.box", json!({"length":40,"width":30,"height":20,"name":"Block"})).unwrap();
    h.frames(3);
    h
}

fn plane(h: &Harness, name: &str) -> solidcraft_engine::geom::Plane {
    solidcraft_engine::view::construction_planes(&h.app.session).into_iter().find(|(_, n, _)| n == name).unwrap().2
}

#[test]
fn two_edges_preselection_preview_apply_edit_and_source_change() {
    let mut h = block();
    h.call("ui.view", json!({"view":"fit"}));
    h.step(&json!({"shot":"two-edges-before"})).unwrap();
    h.app.session.selection =
        [Vec3::new(20., 0., 0.), Vec3::new(20., 30., 20.)].into_iter().map(|point| dialogs::edge_sel(&h.app.session, point).unwrap()).collect();
    h.app.start("construct.plane.two_edges");
    let dialog = h.app.dialog.as_ref().unwrap();
    assert!(dialog.previews());
    assert!(dialog.inputs.iter().all(|input| input.items.len() == 1));
    let commands = dialogs::apply_commands(&h.app, dialog).unwrap();
    assert!(commands.iter().all(|(_, params)| ["a", "b"].iter().all(|key| params[key]["name"].as_str().is_some())));
    let count = h.app.session.doc.features.len();
    let history = h.app.session.undo.len();
    let preview = crate::preview::compute(&h.app.session, &commands, crate::preview::Colors::from_tokens(&crate::theme::Tokens::dark())).unwrap();
    assert!(!preview.scene.xray.is_empty());
    assert!(!preview.scene.lines.is_empty());
    assert_eq!(h.app.session.doc.features.len(), count);
    assert_eq!(h.app.session.undo.len(), history);
    h.frames(4);
    h.step(&json!({"widget":"OK"})).unwrap();
    assert!(h.app.dialog.is_none());
    let feature = h.app.session.doc.features.last().unwrap().clone();
    assert!(plane(&h, &feature.name).height(Vec3::new(20., 30., 20.)).abs() < 1e-7);
    h.step(&json!({"shot":"two-edges-after"})).unwrap();
    h.app.run("timeline.edit", json!({"feature":"Block","set":{"height":"40 mm"}})).unwrap();
    assert!(plane(&h, &feature.name).height(Vec3::new(20., 30., 40.)).abs() < 1e-7);
    h.step(&json!({"call":"ui.editFeature","params":{"feature":feature.name}})).unwrap();
    h.frames(4);
    h.step(&json!({"widget":"OK"})).unwrap();
    assert_eq!(h.app.session.doc.feature(feature.id).unwrap().kind, feature.kind);
    h.step(&json!({"call":"ui.editFeature","params":{"feature":feature.name}})).unwrap();
    let replacement = dialogs::edge_sel(&h.app.session, Vec3::new(20., 0., 40.)).unwrap();
    let dialog = h.app.dialog.as_mut().unwrap();
    dialog.active = 1;
    dialog.pick(&h.app.session, replacement);
    h.frames(4);
    h.step(&json!({"widget":"OK"})).unwrap();
    assert!(plane(&h, &feature.name).normal().y.abs() > 0.999);
    h.app.run("edit.undo", json!({})).unwrap();
    assert_eq!(h.app.session.doc.feature(feature.id).unwrap().kind, feature.kind);
    h.app.run("edit.redo", json!({})).unwrap();
    assert!(plane(&h, &feature.name).normal().y.abs() > 0.999);
    let restored = solidcraft_engine::Session::new(solidcraft_engine::doc::Document::from_json(&h.app.session.doc.to_json()).unwrap());
    assert!(restored.model.results.iter().all(|result| result.error.is_none()));
}

#[test]
fn two_edges_invalid_pair_and_cancel_leave_document_unchanged() {
    let mut h = block();
    let original = h.app.session.doc.clone();
    let edge = dialogs::edge_sel(&h.app.session, Vec3::new(20., 0., 0.)).unwrap();
    h.app.session.selection = vec![edge.clone()];
    h.app.start("construct.plane.two_edges");
    assert!(dialogs::apply_commands(&h.app, h.app.dialog.as_ref().unwrap()).is_err());
    h.app.dialog.as_mut().unwrap().inputs.get_mut(1).unwrap().items = vec![edge];
    let commands = dialogs::apply_commands(&h.app, h.app.dialog.as_ref().unwrap()).unwrap();
    assert!(h.app.session.preview(&commands).is_err());
    h.frames(4);
    h.step(&json!({"widget":"Cancel"})).unwrap();
    assert_eq!(h.app.session.doc.features, original.features);
}

#[test]
fn two_edges_edit_replacement_uses_the_feature_component_frame() {
    for author in ["Part", "root"] {
        let mut h = Harness::new();
        h.app.run("component.create", json!({"name":"Part"})).unwrap();
        h.app.run("solid.box", json!({"length":40,"width":30,"height":20})).unwrap();
        h.app.run("occurrence.move", json!({"occurrence":"Part:1","translate":[100,0,80]})).unwrap();
        h.app.run("component.activate", json!({"component":author})).unwrap();
        h.app.session.selection = [Vec3::new(120., 0., 80.), Vec3::new(120., 30., 100.)]
            .into_iter()
            .map(|point| dialogs::edge_sel(&h.app.session, point).unwrap())
            .collect();
        h.app.start("construct.plane.two_edges");
        h.frames(4);
        h.step(&json!({"widget":"OK"})).unwrap();
        assert!(h.app.dialog.is_none(), "{author}");
        let feature = h.app.session.doc.features.last().unwrap().clone();
        assert!(plane(&h, &feature.name).height(Vec3::new(120., 30., 100.)).abs() < 1e-7);
        h.app.run("component.activate", json!({"component":"root"})).unwrap();
        h.step(&json!({"call":"ui.editFeature","params":{"feature":feature.name}})).unwrap();
        let replacement = dialogs::edge_sel(&h.app.session, Vec3::new(120., 0., 100.)).unwrap();
        let dialog = h.app.dialog.as_mut().unwrap();
        dialog.active = 1;
        dialog.pick(&h.app.session, replacement);
        h.frames(4);
        h.step(&json!({"widget":"OK"})).unwrap();
        assert!(h.app.dialog.is_none(), "{author}");
        assert!(plane(&h, &feature.name).normal().y.abs() > 0.999);
        assert!(plane(&h, &feature.name).height(Vec3::new(120., 0., 100.)).abs() < 1e-7);
    }
}

#[test]
fn two_edges_sketch_preselection_keeps_duplicate_curve_ids_in_their_sketches() {
    let mut h = Harness::new();
    let mut picks = Vec::new();
    for y in [0, 30] {
        let sketch = h.app.run("sketch.create", json!({"plane":"XY"})).unwrap()["sketch"].as_u64().unwrap();
        h.app.run("sketch.options", json!({"sketch_3d":true})).unwrap();
        let curve = h.app.run("sketch.line3d", json!({"points":[[0,y,10],[40,y,10]]})).unwrap()["curves"][0].as_str().unwrap().to_string();
        h.app.run("sketch.finish", json!({})).unwrap();
        picks.push(Sel::ExternalSketchCurve { sketch, id: curve });
    }
    h.app.session.selection = picks.clone();
    h.app.start("construct.plane.two_edges");
    h.frames(4);
    h.step(&json!({"widget":"OK"})).unwrap();
    assert!(h.app.dialog.is_none());
    let feature = h.app.session.doc.features.last().unwrap();
    assert!(matches!(feature.kind, FeatureKind::ConstructionPlane { .. }));
    assert!(plane(&h, &feature.name).height(Vec3::new(20., 15., 10.)).abs() < 1e-7);
    let definition = feature.kind.clone();
    h.app.run("edit.undo", json!({})).unwrap();
    h.app.session.selection.clear();
    h.app.start("construct.plane.two_edges");
    for pick in picks {
        let Sel::ExternalSketchCurve { sketch, id } = pick else { panic!("not a sketch curve") };
        let hit = crate::viewport::Hit::SketchCurve { sketch, id: id.clone(), straight: true };
        crate::viewport::dialog_pick(&mut h.app, &hit, Sel::SketchCurve { id });
    }
    h.frames(4);
    h.step(&json!({"widget":"OK"})).unwrap();
    assert!(h.app.dialog.is_none());
    assert_eq!(h.app.session.doc.features.last().unwrap().kind, definition);
}
