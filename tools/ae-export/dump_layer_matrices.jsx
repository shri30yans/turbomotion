// Run in After Effects: File > Scripts > Run Script File...
// Builds a 3D layer, sets Orientation and X/Y/Z Rotation for a list of cases (including a nonzero Z in the
// orientation and a Y beyond 90 degrees), and writes where the layer's three unit axes land in composition space.
// Output: ae_rotation.json next to this script (or the path you pick). Copy it to
//   crates/comp/tests/golden/ae_rotation.json
// and `cargo test -p turbomotion-comp --test ae_rotation -- --ignored` asserts the engine's matrices equal AE's.
//
// Axes: x_axis, y_axis, z_axis are toWorld([1,0,0]) - toWorld([0,0,0]) and so on, with the anchor point and
// position at the origin and no scale, i.e. the columns of the layer's rotation matrix (comp space: x right,
// y down, z into the screen).

(function () {
  var cases = [
    { name: "orientation_30_40_0", orientation: [30, 40, 0], rotation: [0, 0, 0] },
    { name: "orientation_z_only", orientation: [0, 0, 50], rotation: [0, 0, 0] },
    { name: "orientation_x_then_z", orientation: [30, 0, 40], rotation: [0, 0, 0] },
    { name: "orientation_y_then_z", orientation: [0, 40, 50], rotation: [0, 0, 0] },
    { name: "orientation_all_three", orientation: [30, 40, 50], rotation: [0, 0, 0] },
    { name: "orientation_y_beyond_90", orientation: [20, 120, 30], rotation: [0, 0, 0] },
    { name: "orientation_negative", orientation: [-70, -100, 160], rotation: [0, 0, 0] },
    { name: "rotation_30_40_0", orientation: [0, 0, 0], rotation: [30, 40, 0] },
    { name: "rotation_all_three", orientation: [0, 0, 0], rotation: [30, 40, 50] },
    { name: "rotation_y_beyond_90", orientation: [0, 0, 0], rotation: [20, 120, 30] },
    { name: "rotation_winds_past_360", orientation: [0, 0, 0], rotation: [0, 0, 400] },
    { name: "both", orientation: [30, 40, 50], rotation: [10, -20, 70] },
    { name: "both_y_beyond_90", orientation: [10, 100, -20], rotation: [30, 130, 40] }
  ];

  var proj = app.project || app.newProject();
  var comp = proj.items.addComp("rotation-dump", 1920, 1080, 1, 1, 30);
  var layer = comp.layers.addSolid([1, 1, 1], "probe", 100, 100, 1);
  layer.threeDLayer = true;
  layer.property("Transform").property("Anchor Point").setValue([0, 0, 0]);
  layer.property("Transform").property("Position").setValue([0, 0, 0]);

  function sub(a, b) { return [a[0] - b[0], a[1] - b[1], a[2] - b[2]]; }
  function json(v) { return JSON.stringify ? JSON.stringify(v) : v.toSource(); }

  var out = [];
  for (var i = 0; i < cases.length; i++) {
    var c = cases[i];
    var t = layer.property("Transform");
    t.property("Orientation").setValue(c.orientation);
    t.property("X Rotation").setValue(c.rotation[0]);
    t.property("Y Rotation").setValue(c.rotation[1]);
    t.property("Z Rotation").setValue(c.rotation[2]);
    var o = layer.toWorld([0, 0, 0]);
    out.push({
      name: c.name,
      orientation: c.orientation,
      rotation: c.rotation,
      x_axis: sub(layer.toWorld([1, 0, 0]), o),
      y_axis: sub(layer.toWorld([0, 1, 0]), o),
      z_axis: sub(layer.toWorld([0, 0, 1]), o)
    });
  }

  // ExtendScript has no JSON in older versions: write it by hand.
  function ser(v) {
    if (v instanceof Array) { var p = []; for (var k = 0; k < v.length; k++) p.push(ser(v[k])); return "[" + p.join(", ") + "]"; }
    if (typeof v === "object") { var q = []; for (var key in v) q.push('"' + key + '": ' + ser(v[key])); return "{" + q.join(", ") + "}"; }
    if (typeof v === "string") return '"' + v + '"';
    return String(v);
  }
  var text = '{"ae_version": "' + app.version + '", "cases": [\n  ' + (function () { var l = []; for (var n = 0; n < out.length; n++) l.push(ser(out[n])); return l.join(",\n  "); })() + "\n]}\n";

  var file = new File(Folder.desktop.fsName + "/ae_rotation.json");
  file = file.saveDlg("Save ae_rotation.json", "JSON:*.json") || file;
  file.open("w");
  file.write(text);
  file.close();
  alert("Wrote " + file.fsName);
})();
