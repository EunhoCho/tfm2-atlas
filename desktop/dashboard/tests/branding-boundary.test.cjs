const test = require("node:test");
const assert = require("node:assert/strict");
const fs = require("node:fs");
const path = require("node:path");

const root = path.resolve(__dirname, "..");
const read = (file) => fs.readFileSync(path.join(root, file), "utf8");

test("Dashboard uses Atlas 1.0.33 branding and Dashboard-only credit", () => {
  const packageJson = JSON.parse(read("package.json"));
  const html = read("renderer/index.html");
  const renderer = read("renderer/app.js");
  const messages = read("src/i18n.cjs");
  const readme = read("README.md");
  const notice = read("UPSTREAM_NOTICE.md");
  const combined = `${html}\n${renderer}\n${messages}\n${readme}\n${notice}`;

  assert.equal(packageJson.name, "tfm2-atlas-dashboard");
  assert.equal(packageJson.version, "1.0.33");
  assert.equal(packageJson.build.appId, "io.ehcho.tfm2.atlas.dashboard");
  assert.match(combined, /TFM2 Atlas Dashboard/);
  assert.match(combined, /Inspired by TFM2 Meta Dashboard by DNA and GM선승진 from DCinside TFM Gallery/);
  assert.match(notice, /DashboardApp\/LICENSE.*Electron/s);
  assert.match(notice, /does not grant rights/);
  assert.doesNotMatch(combined, /creditsEditor|TFM2 Atlas Editor|Open Editor|Editor 열기/);
});
