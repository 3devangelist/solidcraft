<!-- Rascunho da descrição do PR. NÃO fazer commit deste ficheiro (tal como PROGRESS.md e _to_delete/). -->
<!-- Título sugerido: Extrude and Hole: To/From Object can take a sketch or construction point by reference -->

## Why

This change supports **top-down design**: you fix levels and sizes in two to four layout (master) sketches, and features take their start and end from points of that layout, not from faces of other solids.

- A layout point exists before any solid, so you don't need helper bodies or typed distances that go stale.
- Editing the layout moves every feature that refers to it.
- Named points (`"to_ref": { "sketch": "Layout", "point": "p4" }`) are also something an AI assistant driving SolidCraft over MCP can read, write and check. It cannot click on a face.

A note on how this was made: I am an industrial designer and CAD teacher, not a programmer. I built this with an AI coding assistant as an **MVP**, a working starting point that shows the behaviour I need. Please feel free to change or reimplement it in whatever way fits the codebase best.

The full rationale is in **WHY_POINT_REFERENCES.md**, attached below. It covers bottom-up vs top-down, why a face is not enough, the AI angle, practice and teaching, other CAD tools and references.

## Change

Extrude **To Object** and **Start: From Object**, and Hole **To Object**, can now take a **point**: a point of any visible sketch (not only the profile's own sketch) or a construction point. The point is kept **by reference**, so the extrusion end, the extrusion start or the hole bottom follows the point when the model recomputes.

Before this change, these inputs only took faces or vertices of bodies. To bound an extrusion at a level marked in another sketch, you had to build a helper body or type a distance, and that distance did not follow later edits.

Behaviour:

- **Picking:** while the To Object or From Object input of an Extrude, or the To Object input of a Hole, is active, a click in the canvas on a point of another visible sketch or on a construction point selects it. The field shows `<Sketch> · <point>`, for example `Sketch2 · p9`.
- **Measuring:** a point is treated as a **level**. The extent is measured only along the normal of the profile's sketch plane, which is the signed height of the point above that plane. No target face is looked up. This means a point that lies on a face parallel to the extrusion, for example a point on the floor or on a wall, still gives a valid end or start. It does not fail with "nothing lies between the profile and the To object".
- **Stored data:**
  - `Extent` gains `from_ref: Option<GeoRef>`. It sits next to the existing `to_ref`.
  - The hole keeps its existing `to_ref`.
  - Both are optional, use `#[serde(default)]` and are skipped when `None`, so older designs load and save unchanged.
  - A pending format file `crates/doc/src/format/pending/to-object-point-reference.rs` records the change.
- **Recompute:** `eval.rs` resolves `to_ref` (Extrude and Hole) and `from_ref` (Extrude) through `construct::point`. If the referenced point no longer exists, the feature keeps the position where it was picked and shows the warning `To Object point: … ; using the position where it was picked` (or `From Object point: …`). It does not fail.
- **Hole thread depth** is not affected.
- **Commands and schema:**
  - `solid.extrude` accepts the optional parameters `to_ref` and `from_ref`.
  - The Hole accepts the existing `to_ref`.
  - Each parameter takes either `{ "sketch": <id or name>, "point": "<id>" }` or `{ "construct_point": "<name>" }` (`PointRefParam` in `schema/features.rs`).
  - The key list in `schema/features_tests.rs` is updated.
- **Editing:** reopening an existing Extrude or Hole restores the picked point in its field.

What does not change:

- Faces, vertices and distances in To Object and From Object work exactly as before.
- The behaviour of `extrude_target`, including the automatic To Object mode, is left unchanged.
- No kernel code is touched: there are no changes under `crates/kernel/` or `crates/geom/`. This is UI, document and engine work only, so it does not fall under the paused kernel work.

Changes a default: no

## Validation

All runs were on Windows 10, from base `aca80f2`.

Targeted runs:

| Command | Result |
|---|---|
| `cargo test -p solidcraft-doc --lib` | 447 passed |
| `cargo test -p solidcraft-engine --lib to_point` (plus the schema key test) | 15 passed |
| `cargo test -p solidcraft-ui-egui --lib -- extrude hole_to_object` | 75 passed |
| `cargo clippy -p solidcraft-doc -p solidcraft-engine -p solidcraft-ui-egui --tests -- -D warnings` | clean |
| `cargo fmt --check` | clean |
| `cargo test --profile ci -p solidcraft-engine --lib hostile_params_never_panic -- --include-ignored` | passed |
| `cargo build --release` | ok |

`cargo xtask ci` (full):

- The repository checks, workspace clippy and the wasm check passed.
- About 6,000 tests passed.
- 9 tests failed in the parallel run. Each of them passed when re-run on its own:
  - `agent_chat` ×2;
  - `home::a_sample_picture…`;
  - `preview::many_faces_are_classified_quickly`;
  - `render::embedded_hdr_capture…`;
  - `pattern_of_512_hole_cuts…` (157 s);
  - `s25_autosave_recovery`;
  - `control_server::subscribed_connections…`;
  - `solidcraft-gpu --test capture` (12 tests).

  The failures came from the local machine:
  - `TEMP` was an 8.3 short path (`RICARD~1`);
  - the slow, old CPU was under load.

  None of these tests touch the changed code.
- The `solidcraft-gpu` lib tests could not finish on this machine (RTX 2080, Windows 10). The path-tracer shader compile ran out of memory (`NVVM compilation failed`, `LLVM ERROR: out of memory`), which matches the open upstream issues #7292 and #6561. This PR does not touch GPU or render code.

New tests:

- Engine (`crates/engine/src/cmd/to_point_tests.rs`):
  - `the_end_follows_a_point_of_another_sketch`, `the_start_follows_a_point_of_another_sketch`, `the_end_follows_a_construction_point`.
  - `without_a_reference_the_end_stays_where_it_was_picked`, `a_reference_must_be_a_point`.
  - `a_reference_that_is_gone_keeps_the_picked_place_with_a_warning`.
  - `a_blind_tapped_hole_follows_a_point_of_another_sketch`, `a_hole_follows_a_construction_point`, `a_hole_without_a_reference_keeps_its_depth`. The thread depth is unchanged.
  - `a_point_on_a_face_along_the_extrusion_is_only_a_level`: the floor case.
  - `start_and_end_points_on_a_face_along_the_extrusion_are_levels`.
    - Expected: the solid spans 10 mm to 25 mm along the sketch normal, with volume 10 × 6 × 15 = 900 mm³.
    - Measured: the same.
- UI (`crates/ui-egui/src/extrude_auto_to_object_tests.rs`, `preselect_tests.rs`):
  - A real canvas click on a point of another sketch fills To Object, and the field label reads `Marks · <point>`.
  - The same click fills From Object, and the command carries `from_ref`.

Manual checks were done in the release build:

1. A profile on a wall, with **Start = point 1** and **To = point 2**, both taken from another sketch. The solid is bounded by the two levels, measured along the wall sketch's normal.
2. A solid from two rectangles of the profile sketch, up To the outermost point of another sketch (`Sketch2 · p4`).
3. A cut of the small front rectangle, bounded by points of the small top rectangle. Then the reverse, bottom to top (`Sketch1 · p6` → `p8`), picking points with the body visible and without hiding it.
4. Editing an existing feature restores the picked points in its fields.
5. A perpendicular face in To is still refused, the same as on `main`.

## Screenshots of the behavior (when observable)

<!-- Anexar no GitHub (arrastar para a caixa do PR). Não fazer commit das imagens. Anexar também o WHY_POINT_REFERENCES.md. -->

- [ ] Extrude dialog with `To Object: Sketch2 · p10` and `Start: From Object: Sketch2 · p9`, and the preview between the two levels.
- [ ] Result: a New Body between the two levels.
- [ ] A solid from both rectangles up to the outermost point (`Sketch2 · p4`).
- [ ] A cut bounded by the top rectangle's points, and the reverse (bottom → top).
- [ ] Before/after for the floor case. On `main`, a point on a face along the extrusion is not accepted, or fails with "nothing lies between…". With this PR it is used as a level.

## Integration status

- Base: `aca80f2` ("Regenerate generated files and fold pending files").
- Still to do before opening:
  - fetch `upstream/main`;
  - rebase;
  - re-run the targeted tests, clippy and fmt.
- No conflicts have been checked yet.

Possible follow-ups, not part of this PR:

- In the automatic To Object mode (while picking profiles), a click on a construction point switches the extrude to To Object. Whether that is wanted should be decided separately.
- From Object accepts a face perpendicular to the profile plane. This behaviour already exists on `main`, so it can go in a separate issue.

🤖 Generated with [Claude Code](https://claude.com/claude-code)
