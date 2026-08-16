const fs = require("node:fs");
const path = require("node:path");

const root = path.resolve(__dirname, "..");
const ignoredDirectories = new Set([".git", ".build", "target", "node_modules", "dist", "release"]);
const binaryExtensions = new Set([".dll", ".exe", ".ico", ".png", ".ttf", ".zip"]);
const formerBrandWord = ["me", "ta"].join("");
const oldNames = [
  new RegExp(`tfm2[-_.]${formerBrandWord}[-_.]dashboard`, "i"),
  new RegExp(`io\\.ehcho\\.tfm2\\.${formerBrandWord}`, "i"),
  new RegExp(`TFM2\\.${formerBrandWord}\\.Dashboard`, "i"),
  new RegExp(`\\bTFM2 ${formerBrandWord} Editor\\b`, "i"),
  new RegExp(`\\b${formerBrandWord} Dashboard\\b`, "i"),
  new RegExp(`${formerBrandWord}-dashboard`, "i"),
  /1\.0\.0-rc\d+/i,
  /-rc33\b/i,
  /\bRC\d+\b/,
];

function files(directory) {
  return fs.readdirSync(directory, { withFileTypes: true }).flatMap((entry) => {
    if (entry.isDirectory() && ignoredDirectories.has(entry.name)) return [];
    const absolute = path.join(directory, entry.name);
    return entry.isDirectory() ? files(absolute) : [absolute];
  });
}

const errors = [];
for (const absolute of files(root)) {
  const relative = path.relative(root, absolute).replaceAll("\\", "/");
  if (relative === "tools/audit_naming.cjs") continue;
  if (oldNames.some((pattern) => pattern.test(relative))) errors.push(`${relative}: old identifier in path`);
  if (binaryExtensions.has(path.extname(relative).toLowerCase())) continue;
  const source = fs.readFileSync(absolute, "utf8");
  source.split(/\r?\n/).forEach((line, index) => {
    const creditLine = /GM선승진|DCinside TFM Gallery/.test(line)
      && relative.startsWith("desktop/dashboard/");
    if (!creditLine && oldNames.some((pattern) => pattern.test(line))) {
      errors.push(`${relative}:${index + 1}: ${line.trim()}`);
    }
    if (/jal-io|Inspired by TFM2 Editor/.test(line)
      && !relative.startsWith("desktop/editor/")
      && !relative.startsWith("atlas-editor/")) {
      errors.push(`${relative}:${index + 1}: Editor credit escaped its product boundary`);
    }
    if (/GM선승진|DCinside TFM Gallery/.test(line) && !relative.startsWith("desktop/dashboard/")) {
      errors.push(`${relative}:${index + 1}: Dashboard credit escaped its product boundary`);
    }
  });
}

const versions = [
  ["engine/Cargo.toml", /version\s*=\s*"1\.0\.33"/],
  ["atlas-core/Cargo.toml", /version\s*=\s*"1\.0\.33"/],
  ["atlas-client-055/Cargo.toml", /version\s*=\s*"1\.0\.33"/],
  ["atlas-editor/Cargo.toml", /version\s*=\s*"1\.0\.33"/],
  ["desktop/dashboard/package.json", /"version"\s*:\s*"1\.0\.33"/],
  ["desktop/editor/package.json", /"version"\s*:\s*"1\.0\.33"/],
];
for (const [relative, expected] of versions) {
  if (!expected.test(fs.readFileSync(path.join(root, relative), "utf8"))) {
    errors.push(`${relative}: version is not 1.0.33`);
  }
}

if (errors.length) {
  process.stderr.write(`${errors.join("\n")}\n`);
  process.exitCode = 1;
} else {
  process.stdout.write("Atlas naming audit passed.\n");
}
