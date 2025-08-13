import test from "ava";
import fs from "node:fs";
import path from "path";

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

  const stream = dropletHandler.readFile(dirName, "TESTFILE");

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
  const stream = dropletHandler.readFile(dirName, "TESTFILE", BigInt(1), BigInt(4));

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

test("zip file reader", async (t) => {
  t.timeout(10_000);
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

  const stream = dropletHandler.readFile(
    "./assets/TheGame.zip",
    "setup.exe",
    BigInt(10),
    BigInt(20)
  );


  let finalString = "";
  for await (const chunk of stream.getStream()) {
    // Do something with each 'chunk'
    finalString = String.fromCharCode.apply(null, chunk);
    if(finalString.length > 100) break;
  }

  t.pass();
});
