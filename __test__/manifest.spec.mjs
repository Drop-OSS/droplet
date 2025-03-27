import test from "ava";
import fs from "node:fs";
import path from "path";

import { generateManifest } from "../index.js";

test("numerous small file", async (t) => {
  // Setup test dir
  const dirName = "./.test/nsf";
  if (fs.existsSync(dirName)) fs.rmSync(dirName, { recursive: true });
  fs.mkdirSync(dirName, { recursive: true });

  // Config
  const testAmount = 100;

  for (let i = 0; i < testAmount; i++) {
    const fileName = path.join(dirName, i.toString());
    fs.writeFileSync(fileName, i.toString());
  }

  const manifest = JSON.parse(
    await new Promise((r, e) =>
      generateManifest(
        dirName,
        (_, __) => {},
        (_, __) => {},
        (err, manifest) => (err ? e(err) : r(manifest))
      )
    )
  );

  // Check the first few checksums
  const checksums = [
    "cfcd208495d565ef66e7dff9f98764da",
    "c4ca4238a0b923820dcc509a6f75849b",
    "c81e728d9d4c2f636f067f89cc14862c",
  ];
  for (let index in checksums) {
    const entry = manifest[index.toString()];
    if (!entry) return t.fail(`manifest missing file ${index}`);

    const checksum = entry.checksums[0];
    t.is(checksum, checksums[index], `checksums do not match for ${index}`);
  }

  // Check all entries are there, and the right length
  for (let i = 0; i < testAmount; i++) {
    const entry = manifest[i.toString()];
    if (!entry) return t.fail(`manifest missing file ${i}`);

    t.is(entry.lengths[0], i.toString().length);
  }

  fs.rmSync(dirName, { recursive: true });
});