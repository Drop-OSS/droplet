import test from "ava";
import fs from "node:fs";
import path from "path";

import droplet from "../index.js";

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

test("read file", async (t) => {
  const dirName = "./.test2";
  if (fs.existsSync(dirName)) fs.rmSync(dirName, { recursive: true });
  fs.mkdirSync(dirName, { recursive: true });

  const testString = "g'day what's up my koala bros\n".repeat(1000);

  fs.writeFileSync(dirName + "/TESTFILE", testString);

  const stream = droplet.readFile(dirName, "TESTFILE");

  let finalString = "";

  for await (const chunk of stream) {
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

  const stream = droplet.readFile(dirName, "TESTFILE", 1, 4);

  let finalString = "";

  for await (const chunk of stream) {
    // Do something with each 'chunk'
    finalString += String.fromCharCode.apply(null, chunk);
  }

  const expectedString = testString.slice(1, 5);

  t.assert(finalString == expectedString, "file strings don't match");
  fs.rmSync(dirName, { recursive: true });
});
