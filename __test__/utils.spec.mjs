import test from "ava";
import fs from "node:fs";
import path from "path";
import { createHash } from "node:crypto";
import prettyBytes from "pretty-bytes";

import droplet, { DropletHandler, generateManifest } from "../index.js";

test("check alt thread util", async (t) => {
  let endtime1, endtime2;

  droplet.callAltThreadFunc(async () => {
    await new Promise((r) => setTimeout(r, 100));
    endtime1 = Date.now();
  });

  await new Promise((r) => setTimeout(r, 500));
  endtime2 = Date.now();

  const difference = endtime2 - endtime1;
  if (difference >= 600) {
    t.fail("likely isn't multithreaded, difference: " + difference);
  }

  t.pass();
});

test("list files", async (t) => {
  const dirName = "./.listfiles";
  if (fs.existsSync(dirName)) fs.rmSync(dirName, { recursive: true });
  fs.mkdirSync(dirName, { recursive: true });
  fs.mkdirSync(dirName + "/subdir", { recursive: true });
  fs.mkdirSync(dirName + "/subddir", { recursive: true });

  fs.writeFileSync(dirName + "/root.txt", "root");
  fs.writeFileSync(dirName + "/subdir/one.txt", "the first subdir");
  fs.writeFileSync(dirName + "/subddir/two.txt", "the second");

  const dropletHandler = new DropletHandler();
  const files = dropletHandler.listFiles(dirName);

  t.assert(
    files.sort().join("\n"),
    ["root.txt", "subddir/two.txt", "subdir/one.txt"].join("\n")
  );

  fs.rmSync(dirName, { recursive: true });
});

test("read file", async (t) => {
  const dirName = "./.test2";
  if (fs.existsSync(dirName)) fs.rmSync(dirName, { recursive: true });
  fs.mkdirSync(dirName, { recursive: true });

  const testString = "g'day what's up my koala bros\n".repeat(1000);

  fs.writeFileSync(dirName + "/TESTFILE", testString);

  const dropletHandler = new DropletHandler();

  const stream = dropletHandler.readFile(
    dirName,
    "TESTFILE",
    BigInt(0),
    BigInt(testString.length)
  );

  let finalString = "";

  for await (const chunk of stream.getStream()) {
    // Do something with each 'chunk'
    finalString += String.fromCharCode.apply(null, chunk);
  }

  t.assert(finalString == testString, "file strings don't match");
  fs.rmSync(dirName, { recursive: true });
});

test("read file offset", async (t) => {
  const dirName = "./.test3";
  if (fs.existsSync(dirName)) fs.rmSync(dirName, { recursive: true });
  fs.mkdirSync(dirName, { recursive: true });

  const testString = "0123456789";
  fs.writeFileSync(dirName + "/TESTFILE", testString);

  const dropletHandler = new DropletHandler();
  const stream = dropletHandler.readFile(
    dirName,
    "TESTFILE",
    BigInt(1),
    BigInt(4)
  );

  let finalString = "";

  for await (const chunk of stream.getStream()) {
    // Do something with each 'chunk'
    finalString += String.fromCharCode.apply(null, chunk);
  }

  const expectedString = testString.slice(1, 4);

  t.assert(
    finalString == expectedString,
    `file strings don't match: ${finalString} vs ${expectedString}`
  );
  fs.rmSync(dirName, { recursive: true });
});

test.skip("zip speed test", async (t) => {
  t.timeout(100_000_000);
  const dropletHandler = new DropletHandler();

  const stream = dropletHandler.readFile("./assets/TheGame.zip", "setup.exe");

  let totalRead = 0;
  let totalSeconds = 0;

  let lastTime = process.hrtime.bigint();
  const timeThreshold = BigInt(1_000_000_000);
  let runningTotal = 0;
  let runningTime = BigInt(0);
  for await (const chunk of stream.getStream()) {
    // Do something with each 'chunk'
    const currentTime = process.hrtime.bigint();
    const timeDiff = currentTime - lastTime;
    lastTime = currentTime;
    runningTime += timeDiff;

    runningTotal += chunk.length;

    if (runningTime >= timeThreshold) {
      console.log(`${prettyBytes(runningTotal)}/s`);
      totalRead += runningTotal;
      totalSeconds += 1;
      runningTime = BigInt(0);
      runningTotal = 0;
    }
  }

  const roughAverage = totalRead / totalSeconds;

  console.log(`total rough average: ${prettyBytes(roughAverage)}/s`);

  t.pass();
});

test("zip manifest test", async (t) => {
  const dropletHandler = new DropletHandler();
  const manifest = JSON.parse(
    await new Promise((r, e) =>
      generateManifest(
        dropletHandler,
        "./assets/TheGame.zip",
        (_, __) => {},
        (_, __) => {},
        (err, manifest) => (err ? e(err) : r(manifest))
      )
    )
  );

  for (const [filename, data] of Object.entries(manifest)) {
    console.log(filename);
    let start = 0;
    for (const [chunkIndex, length] of data.lengths.entries()) {
      const hash = createHash("md5");
      const stream = (
        await dropletHandler.readFile(
          "./assets/TheGame.zip",
          filename,
          BigInt(start),
          BigInt(start + length)
        )
      ).getStream();

      let streamLength = 0;
      await stream.pipeTo(
        new WritableStream({
          write(chunk) {
            streamLength += chunk.length;
            hash.update(chunk);
          },
        })
      );

      if (streamLength != length)
        return t.fail(
          `stream length for chunk index ${chunkIndex} was not expected: real: ${streamLength} vs expected: ${length}`
        );

      const digest = hash.digest("hex");
      if (data.checksums[chunkIndex] != digest)
        return t.fail(
          `checksums did not match for chunk index ${chunkIndex}: real: ${digest} vs expected: ${data.checksums[chunkIndex]}`
        );

      start += length;
    }
  }

  t.pass();
});

test.skip("partially compress zip test", async (t) => {
  const dropletHandler = new DropletHandler();

  const manifest = JSON.parse(
    await new Promise((r, e) =>
      generateManifest(
        dropletHandler,
        "./assets/my horror game.zip",
        (_, __) => {},
        (_, __) => {},
        (err, manifest) => (err ? e(err) : r(manifest))
      )
    )
  );

  return t.pass();
});
