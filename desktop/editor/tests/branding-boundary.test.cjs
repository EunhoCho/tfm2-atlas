const test = require("node:test");
const assert = require("node:assert/strict");
const fs = require("node:fs");
const path = require("node:path");

const root = path.resolve(__dirname, "..");
const read = (file) => fs.readFileSync(path.join(root, file), "utf8");

test("Editor uses Atlas 1.0.33 branding and Editor-only credit", () => {
  const packageJson = JSON.parse(read("package.json"));
  const html = read("renderer/editor.html");
  const renderer = read("renderer/editor.js");
  const readme = read("README.md");
  const notices = read("THIRD_PARTY_NOTICES.md");
  const upstream = read("UPSTREAM_NOTICE.md");
  const combined = `${html}\n${renderer}\n${readme}\n${notices}\n${upstream}`;

  assert.equal(packageJson.name, "tfm2-atlas-editor");
  assert.equal(packageJson.version, "1.0.33");
  assert.equal(packageJson.build.appId, "io.ehcho.tfm2.atlas.editor");
  assert.match(combined, /TFM2 Atlas Editor/);
  assert.match(combined, /Inspired by TFM2 Editor by jal-io/);
  assert.match(upstream, /independent implementation/i);
  assert.doesNotMatch(combined, /creditsDashboard|TFM2 Atlas Dashboard/);
});
