#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
//! Extrude and Hole To Object on a point of another sketch or a construction point (`to_ref`):
//! the end (the hole's bottom) follows the point as the parameters change, and stays where it
//! was when the point is gone.

use crate::Session;
use serde_json::{Value, json};

fn run(s: &mut Session, cmd: &str, params: Value) -> Value {
    s.execute(cmd, &params).unwrap_or_else(|e| panic!("{cmd}: {e}"))
}

/// A 10 × 10 square on XY ("Square") and, on XZ, a point held at ("5", "h") ("Marks").
/// Returns the point's id.
fn fixture(s: &mut Session) -> String {
    run(s, "parameters.add", json!({"name": "h", "expression": "20 mm"}));
    run(s, "sketch.create", json!({"plane": "XY", "name": "Square"}));
    run(s, "sketch.rectangle.two_point", json!({"p0": [0, 0], "p1": [10, 10]}));
    run(s, "sketch.finish", json!({}));
    run(s, "sketch.create", json!({"plane": "XZ", "name": "Marks"}));
    let r = run(s, "sketch.point", json!({"point": ["5", "h"]}));
    run(s, "sketch.finish", json!({}));
    r["created"]["points"][0].as_str().unwrap().to_string()
}

fn sketch_id(s: &Session, name: &str) -> u64 {
    s.doc.find_feature(name).unwrap().id
}

/// Where the point is now, in the world.
fn mark(s: &Session, point: &str) -> solidcraft_geom::Vec3 {
    let st = s.model.state();
    let ss = st.sketch(sketch_id(s, "Marks")).unwrap();
    ss.plane.to_world(ss.sketch.point(ss.sketch.point_index(point).unwrap()).unwrap())
}

fn top(s: &Session) -> f64 {
    assert!(s.model.results.iter().all(|r| r.error.is_none()), "{:?}", s.model.results.iter().filter_map(|r| r.error.clone()).collect::<Vec<_>>());
    let b = s.model.state().body("Block").unwrap().mesh().bounds();
    b.max.z.abs().max(b.min.z.abs())
}

fn extrude(s: &mut Session, to: solidcraft_geom::Vec3, to_ref: Option<Value>) {
    let mut p = json!({"sketch": "Square", "distance": 10, "to": [to.x, to.y, to.z], "operation": "new", "body_name": "Block"});
    if let Some(r) = to_ref {
        p["to_ref"] = r;
    }
    run(s, "solid.extrude", p);
}

#[test]
fn the_end_follows_a_point_of_another_sketch() {
    let mut s = Session::default();
    let point = fixture(&mut s);
    let at = mark(&s, &point);
    assert!((at.z.abs() - 20.0).abs() < 1e-9, "{at:?}");
    extrude(&mut s, at, Some(json!({"sketch": "Marks", "point": point})));
    assert!((top(&s) - 20.0).abs() < 1e-6);
    run(&mut s, "parameters.change", json!({"name": "h", "expression": "35 mm"}));
    assert!((mark(&s, &point).z.abs() - 35.0).abs() < 1e-9);
    assert!((top(&s) - 35.0).abs() < 1e-6, "the end follows the point");
    // Saved and opened again, the reference is still there.
    let json = s.doc.to_json();
    assert!(json.contains("\"to_ref\""), "{json}");
}

#[test]
fn without_a_reference_the_end_stays_where_it_was_picked() {
    let mut s = Session::default();
    let point = fixture(&mut s);
    let at = mark(&s, &point);
    extrude(&mut s, at, None);
    run(&mut s, "parameters.change", json!({"name": "h", "expression": "35 mm"}));
    assert!((top(&s) - 20.0).abs() < 1e-6);
}

#[test]
fn the_end_follows_a_construction_point() {
    let mut s = Session::default();
    let point = fixture(&mut s);
    let marks = sketch_id(&s, "Marks");
    run(&mut s, "construct.point.vertex", json!({"point": {"sketch": marks, "point": point}, "name": "Tip"}));
    let at = mark(&s, &point);
    extrude(&mut s, at, Some(json!({"construct_point": "Tip"})));
    run(&mut s, "parameters.change", json!({"name": "h", "expression": "42 mm"}));
    assert!((top(&s) - 42.0).abs() < 1e-6);
}

#[test]
fn a_reference_that_is_gone_keeps_the_picked_place_with_a_warning() {
    let mut s = Session::default();
    let point = fixture(&mut s);
    let at = mark(&s, &point);
    extrude(&mut s, at, Some(json!({"sketch": "Marks", "point": "p999"})));
    assert!((top(&s) - 20.0).abs() < 1e-6);
    let warned = s.model.results.iter().filter_map(|r| r.warning.as_deref()).any(|w| w.contains("To Object point"));
    assert!(warned);
}

/// A point lying on a body's face parallel to the extrusion (on a floor, the profile on a wall) is
/// still a point: the end stops level with it along the sketch plane's normal, and the face under
/// it neither trims the extrusion nor refuses it ("nothing lies between the profile…").
#[test]
fn a_point_on_a_face_along_the_extrusion_is_only_a_level() {
    let mut s = Session::default();
    run(&mut s, "solid.box", json!({"length": 40, "width": 60, "height": 10, "corner": [0, -30, -5], "body_name": "Floor"}));
    run(&mut s, "construct.plane.offset", json!({"base": "XY", "offset": 5, "name": "FloorTop"}));
    run(&mut s, "sketch.create", json!({"plane": "FloorTop", "name": "Marks"}));
    let point = run(&mut s, "sketch.point", json!({"point": [20, 25]}))["created"]["points"][0].as_str().unwrap().to_string();
    run(&mut s, "sketch.finish", json!({}));
    let at = mark(&s, &point);
    assert!((at.z - 5.0).abs() < 1e-9 && (at.y.abs() - 25.0).abs() < 1e-9, "on the floor's top: {at:?}");
    // On the wall (XZ), half in the floor: x 5..15, 2..8 up or down the wall.
    run(&mut s, "sketch.create", json!({"plane": "XZ", "name": "Wall"}));
    run(&mut s, "sketch.rectangle.two_point", json!({"p0": [5, 2], "p1": [15, 8]}));
    run(&mut s, "sketch.finish", json!({}));
    run(
        &mut s,
        "solid.extrude",
        json!({"sketch": "Wall", "distance": 10, "to": [at.x, at.y, at.z], "to_ref": {"sketch": "Marks", "point": point}, "operation": "new", "body_name": "Block"}),
    );
    assert!(s.model.results.iter().all(|r| r.error.is_none()), "{:?}", s.model.results.iter().filter_map(|r| r.error.clone()).collect::<Vec<_>>());
    let st = s.model.state();
    let block = st.body("Block").unwrap();
    let b = block.mesh().bounds();
    assert!(((b.max.y - b.min.y) - 25.0).abs() < 1e-6, "25 mm along the normal: {b:?}");
    let volume = solidcraft_kernel::measure(&block.body).unwrap().volume;
    assert!((volume - 10.0 * 6.0 * 25.0).abs() < 1e-6, "the whole prism, not trimmed by the floor: {volume}");
}

#[test]
fn a_reference_must_be_a_point() {
    let mut s = Session::default();
    fixture(&mut s);
    let e = s
        .execute("solid.extrude", &json!({"sketch": "Square", "distance": 10, "to": [0, 0, 20], "to_ref": {"face": [0, 0, 0]}}))
        .unwrap_err()
        .to_string();
    assert!(e.contains("to_ref"), "{e}");
    let e = s.execute("solid.extrude", &json!({"sketch": "Square", "distance": 10, "to_ref": {"construct_point": "Nope"}})).unwrap_err().to_string();
    assert!(e.contains("to_ref"), "{e}");
}

/// A 40 × 30 × 20 block and, on YZ (its side at x = 0), a point at (y 15, z 20 − d) ("Bottoms").
/// Returns the point's id.
fn block_with_depth_mark(s: &mut Session) -> String {
    run(s, "parameters.add", json!({"name": "d", "expression": "8 mm"}));
    run(s, "solid.box", json!({"length": 40, "width": 30, "height": 20, "body_name": "Block"}));
    run(s, "sketch.create", json!({"plane": "YZ", "name": "Bottoms"}));
    let r = run(s, "sketch.point", json!({"point": ["15", "20 - d"]}));
    run(s, "sketch.finish", json!({}));
    r["created"]["points"][0].as_str().unwrap().to_string()
}

fn block_volume(s: &Session) -> f64 {
    assert!(s.model.results.iter().all(|r| r.error.is_none()), "{:?}", s.model.results.iter().filter_map(|r| r.error.clone()).collect::<Vec<_>>());
    solidcraft_kernel::measure(&s.model.state().body("Block").unwrap().body).unwrap().volume
}

/// The volume a flat-bottomed M6 tapped hole (5 mm tap drill) `depth` deep takes away.
fn m6_volume(depth: f64) -> f64 {
    std::f64::consts::PI * 2.5 * 2.5 * depth
}

/// A blind tapped hole drilled To Object down to a sketch point: its depth follows the point, and
/// its thread stays the same.
#[test]
fn a_blind_tapped_hole_follows_a_point_of_another_sketch() {
    let mut s = Session::default();
    let point = block_with_depth_mark(&mut s);
    let full = 40.0 * 30.0 * 20.0;
    run(
        &mut s,
        "solid.hole",
        json!({"position": [10, 15, 20], "thread": "M6", "to": [0, 15, 12], "to_ref": {"sketch": "Bottoms", "point": point}, "name": "Tap"}),
    );
    assert!(((full - block_volume(&s)) - m6_volume(8.0)).abs() < 1e-3 * m6_volume(8.0), "{}", full - block_volume(&s));
    run(&mut s, "parameters.change", json!({"name": "d", "expression": "12 mm"}));
    assert!(((full - block_volume(&s)) - m6_volume(12.0)).abs() < 1e-3 * m6_volume(12.0), "{}", full - block_volume(&s));
    let solidcraft_doc::FeatureKind::Hole { thread, thread_depth, to_ref, .. } = &s.doc.find_feature("Tap").unwrap().kind else { panic!("hole") };
    assert_eq!(thread.as_deref(), Some("M6"));
    assert!(thread_depth.is_none());
    assert!(to_ref.is_some());
}

#[test]
fn a_hole_without_a_reference_keeps_its_depth() {
    let mut s = Session::default();
    block_with_depth_mark(&mut s);
    let full = 40.0 * 30.0 * 20.0;
    run(&mut s, "solid.hole", json!({"position": [10, 15, 20], "thread": "M6", "to": [0, 15, 12]}));
    run(&mut s, "parameters.change", json!({"name": "d", "expression": "12 mm"}));
    assert!(((full - block_volume(&s)) - m6_volume(8.0)).abs() < 1e-3 * m6_volume(8.0));
}

#[test]
fn a_hole_follows_a_construction_point() {
    let mut s = Session::default();
    let point = block_with_depth_mark(&mut s);
    let bottoms = sketch_id_of(&s, "Bottoms");
    run(&mut s, "construct.point.vertex", json!({"point": {"sketch": bottoms, "point": point}, "name": "Floor"}));
    let full = 40.0 * 30.0 * 20.0;
    run(&mut s, "solid.hole", json!({"position": [10, 15, 20], "diameter": 4, "to": [0, 15, 12], "to_ref": {"construct_point": "Floor"}}));
    run(&mut s, "parameters.change", json!({"name": "d", "expression": "5 mm"}));
    let want = std::f64::consts::PI * 4.0 * 5.0;
    assert!(((full - block_volume(&s)) - want).abs() < 1e-3 * want, "{}", full - block_volume(&s));
}

fn sketch_id_of(s: &Session, name: &str) -> u64 {
    s.doc.find_feature(name).unwrap().id
}

/// Where `point` of the sketch `sketch` is now, in the world.
fn point_in(s: &Session, sketch: &str, point: &str) -> solidcraft_geom::Vec3 {
    let st = s.model.state();
    let ss = st.sketch(sketch_id(s, sketch)).unwrap();
    ss.plane.to_world(ss.sketch.point(ss.sketch.point_index(point).unwrap()).unwrap())
}

/// The block's extent along z, nearer end first (as distances from XY, whichever side it is on).
fn z_span(s: &Session) -> (f64, f64) {
    assert!(s.model.results.iter().all(|r| r.error.is_none()), "{:?}", s.model.results.iter().filter_map(|r| r.error.clone()).collect::<Vec<_>>());
    let b = s.model.state().body("Block").unwrap().mesh().bounds();
    let (a, c) = (b.min.z.abs(), b.max.z.abs());
    (a.min(c), a.max(c))
}

/// From Object on a point of another sketch (`from_ref`): the start follows the point as the
/// parameters change, while the end stays on its own point.
#[test]
fn the_start_follows_a_point_of_another_sketch() {
    let mut s = Session::default();
    let end = fixture(&mut s);
    run(&mut s, "parameters.add", json!({"name": "b", "expression": "5 mm"}));
    run(&mut s, "sketch.create", json!({"plane": "XZ", "name": "Starts"}));
    let start = run(&mut s, "sketch.point", json!({"point": ["5", "b"]}))["created"]["points"][0].as_str().unwrap().to_string();
    run(&mut s, "sketch.finish", json!({}));
    let (to, from) = (mark(&s, &end), point_in(&s, "Starts", &start));
    run(
        &mut s,
        "solid.extrude",
        json!({"sketch": "Square", "distance": 10, "to": [to.x, to.y, to.z], "to_ref": {"sketch": "Marks", "point": end},
               "from": [from.x, from.y, from.z], "from_ref": {"sketch": "Starts", "point": start}, "operation": "new", "body_name": "Block"}),
    );
    let (lo, hi) = z_span(&s);
    assert!((lo - 5.0).abs() < 1e-6 && (hi - 20.0).abs() < 1e-6, "from the start point to the end point: {lo}..{hi}");
    run(&mut s, "parameters.change", json!({"name": "b", "expression": "8 mm"}));
    let (lo, hi) = z_span(&s);
    assert!((lo - 8.0).abs() < 1e-6 && (hi - 20.0).abs() < 1e-6, "the start follows its point: {lo}..{hi}");
    assert!(s.doc.to_json().contains("\"from_ref\""));
}

/// Start and end on points lying on a floor's top face, the profile on a wall: both are levels
/// along the sketch plane's normal (10 and 25 mm from the wall), the face under them plays no part.
#[test]
fn start_and_end_points_on_a_face_along_the_extrusion_are_levels() {
    let mut s = Session::default();
    run(&mut s, "solid.box", json!({"length": 40, "width": 60, "height": 10, "corner": [0, -30, -5], "body_name": "Floor"}));
    run(&mut s, "construct.plane.offset", json!({"base": "XY", "offset": 5, "name": "FloorTop"}));
    run(&mut s, "sketch.create", json!({"plane": "FloorTop", "name": "Marks"}));
    let p1 = run(&mut s, "sketch.point", json!({"point": [20, 10]}))["created"]["points"][0].as_str().unwrap().to_string();
    let p2 = run(&mut s, "sketch.point", json!({"point": [20, 25]}))["created"]["points"][0].as_str().unwrap().to_string();
    run(&mut s, "sketch.finish", json!({}));
    let (from, to) = (mark(&s, &p1), mark(&s, &p2));
    run(&mut s, "sketch.create", json!({"plane": "XZ", "name": "Wall"}));
    run(&mut s, "sketch.rectangle.two_point", json!({"p0": [5, 2], "p1": [15, 8]}));
    run(&mut s, "sketch.finish", json!({}));
    run(
        &mut s,
        "solid.extrude",
        json!({"sketch": "Wall", "distance": 10, "to": [to.x, to.y, to.z], "to_ref": {"sketch": "Marks", "point": p2},
               "from": [from.x, from.y, from.z], "from_ref": {"sketch": "Marks", "point": p1}, "operation": "new", "body_name": "Block"}),
    );
    assert!(s.model.results.iter().all(|r| r.error.is_none()), "{:?}", s.model.results.iter().filter_map(|r| r.error.clone()).collect::<Vec<_>>());
    let st = s.model.state();
    let block = st.body("Block").unwrap();
    let b = block.mesh().bounds();
    let (near, far) = (b.min.y.abs().min(b.max.y.abs()), b.min.y.abs().max(b.max.y.abs()));
    assert!((near - 10.0).abs() < 1e-6 && (far - 25.0).abs() < 1e-6, "10 to 25 mm along the normal: {b:?}");
    let volume = solidcraft_kernel::measure(&block.body).unwrap().volume;
    assert!((volume - 10.0 * 6.0 * 15.0).abs() < 1e-6, "the whole prism between the levels: {volume}");
}
