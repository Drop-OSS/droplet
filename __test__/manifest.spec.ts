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

  const manifest: {
    [key: string]: { checksums: string[]; lengths: number[] };
  } = JSON.parse(
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
    "4b82be835c1f2bc3a447e7c6965c3979",
    "763807ecb543f8417dc1388aa9c669e9",
    "21981611048001c07cdbd95200a15a31",
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

test("single large file", async (t) => {
  // Setup test dir
  const dirName = "./.test/slf";
  if (fs.existsSync(dirName)) fs.rmSync(dirName, { recursive: true });
  fs.mkdirSync(dirName, { recursive: true });

  // Config
  const chunkSize = 1024 * 1024 * 64;
  const fileSize = chunkSize * 2 - 1; // Should be 4 chunks

  const testFile = path.join(dirName, "test.bin");
  const randomReadStream = fs.createReadStream("/dev/random", {
    end: fileSize,
    start: 0,
  });

  const writeStream = fs.createWriteStream(testFile);
  randomReadStream.pipe(writeStream);

  await new Promise((r) => randomReadStream.on("end", r));

  const manifest: {
    [key: string]: { lengths: number[] };
  } = JSON.parse(
    await new Promise((r, e) =>
      generateManifest(
        dirName,
        (_, __) => {},
        (_, __) => {},
        (err, manifest) => (err ? e(err) : r(manifest))
      )
    )
  );

  for (const [key, value] of Object.entries(manifest)) {
    for (const length of value.lengths) {
      t.is(length, chunkSize, "chunk size is not as expected");
    }
  }

  fs.rmSync(dirName, { recursive: true });
});
