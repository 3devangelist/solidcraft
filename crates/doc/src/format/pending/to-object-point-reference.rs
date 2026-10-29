//! An extrude's or a hole's To Object can keep the sketch point or construction point it was
//! picked on (`to_ref`, beside the picked place `to`), and an extrude's From Object likewise
//! (`from_ref`, beside `from`), so the end, the start or the hole's bottom follows that point
//! when the design changes. Older designs have no reference: their To and From Object stay at
//! the stored place, as before. The last release refuses the newer design format instead of
//! silently dropping the reference.
