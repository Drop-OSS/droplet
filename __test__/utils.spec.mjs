import test from "ava";
import fs from "node:fs";
import path from "path";

import droplet from "../index.js";

test("check alt thread util", async (t) => {
  let endtime1, endtime2;

  droplet.callAltThreadFunc(async () => {
    await new Promise((r) => setTimeout(r, 1000));
    endtime1 = Date.now();
  });

  await new Promise((r) => setTimeout(r, 5000));
  endtime2 = Date.now();

  const difference = endtime2 - endtime1;
  if (difference > 4100 || difference < 3900) {
    t.fail("timing is not close enough");
  }

  t.pass();
});

test("read file", async (t) => {
  const dirName = "./.test2";
  if (fs.existsSync(dirName)) fs.rmSync(dirName, { recursive: true });
  fs.mkdirSync(dirName, { recursive: true });

  fs.writeFileSync("./.test2/TESTFILE", "g'day what's up my koala bros");

  const stream = droplet.readFile("./.test2", "TESTFILE");
  console.log(stream);

  for await (const chunk of stream) {
    // Do something with each 'chunk'
    console.log(chunk);
  }

  t.pass();
  fs.rmSync(dirName, { recursive: true });
});
