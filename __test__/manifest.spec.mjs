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
    await generateManifest(
      dirName,
      (_, __) => {},
      (_, __) => {}
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

test.skip("performance test", async (t) => {
  t.timeout(5 * 60 * 1000);
  const dirName = "./.test/pt";
  if (fs.existsSync(dirName)) fs.rmSync(dirName, { recursive: true });
  fs.mkdirSync(dirName, { recursive: true });

  const fileSize = 1 * 1000 * 1000 * 1000; // 1GB

  const randomStream = fs.createReadStream("/dev/random", {
    start: 0,
    end: fileSize,
  });
  const outputStream = fs.createWriteStream(path.join(dirName, "file.bin"));
  await new Promise((r) => {
    randomStream.pipe(outputStream);
    randomStream.on("end", r);
  });

  const start = Date.now();
  await generateManifest(
    dirName,
    (_, __) => {},
    (_, __) => {}
  );
  const end = Date.now();

  t.pass(`Took ${end - start}ms to process ${fileSize / (1000 * 1000)}MB`);

  fs.rmSync(dirName, { recursive: true });
});

test("special characters", async (t) => {
  // Setup test dir
  const dirName = "./.test/sc";
  if (fs.existsSync(dirName)) fs.rmSync(dirName, { recursive: true });
  fs.mkdirSync(dirName, { recursive: true });

  // Config
  const fileNames = ["Technická podpora.rtf", "Servicio técnico.rtf"];

  for (let i = 0; i < fileNames.length; i++) {
    const fileName = path.join(dirName, fileNames[i]);
    fs.writeFileSync(fileName, i.toString());
  }

  const manifest = JSON.parse(
    await generateManifest(
      dirName,
      (_, __) => {},
      (_, __) => {}
    )
  );

  // Check the first few checksums
  const checksums = [
    "cfcd208495d565ef66e7dff9f98764da",
    "c4ca4238a0b923820dcc509a6f75849b",
  ];
  for (let index in checksums) {
    const entry = manifest[fileNames[index]];
    if (!entry) return t.fail(`manifest missing file ${index}`);

    const checksum = entry.checksums[0];
    t.is(checksum, checksums[index], `checksums do not match for ${index}`);
  }

  fs.rmSync(dirName, { recursive: true });
});
