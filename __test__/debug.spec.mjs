import test from "ava";
import { DropletHandler, generateManifest } from "../index.js";

test.skip("debug", async (t) => {
  const handler = new DropletHandler();

  console.log("created handler");

  const manifest = JSON.parse(
    await new Promise((r, e) =>
      generateManifest(
        handler,
        "./assets/TheGame.zip",
        (_, __) => {},
        (_, __) => {},
        (err, manifest) => (err ? e(err) : r(manifest))
      )
    )
  );

  return t.pass();
});
