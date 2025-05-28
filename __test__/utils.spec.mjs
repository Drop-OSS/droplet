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
  if (difference > 500 || difference < 300) {
    t.fail("timing is not close enough: " + difference);
  }

  t.pass();
});

test("read file", async (t) => {
  const dirName = "./.test2";
  if (fs.existsSync(dirName)) fs.rmSync(dirName, { recursive: true });
  fs.mkdirSync(dirName, { recursive: true });

  const testString = "g'day what's up my koala bros\n".repeat(10000);

  fs.writeFileSync("./.test2/TESTFILE", testString);

  const stream = droplet.readFile("./.test2", "TESTFILE");

  let finalString = "";

  for await (const chunk of stream) {
    // Do something with each 'chunk'
    finalString += String.fromCharCode.apply(null, chunk);
  }

  t.assert(finalString == testString, "file strings don't match");
  fs.rmSync(dirName, { recursive: true });
});
