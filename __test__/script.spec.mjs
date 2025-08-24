import test from "ava";
import { ScriptEngine } from "../index.js";

test("lua syntax fail", (t) => {
  const scriptEngine = new ScriptEngine();

  const luaIshCode = `
    print("hello world);
    `;

  try {
    const script = scriptEngine.buildLuaScript(luaIshCode);
  } catch {
    return t.pass();
  }
  t.fail();
});

test("js syntax fail", (t) => {
  const scriptEngine = new ScriptEngine();

  const jsIshCode = `
    const v = "hello world;
    `;

  try {
    const script = scriptEngine.buildJsScript(jsIshCode);
  } catch {
    return t.pass();
  }
  t.fail();
});

test("js", (t) => {
  const scriptEngine = new ScriptEngine();

  const jsModule = `
    const v = "1" + "2";
    ["1", "2", "3", v]
       `;

  const script = scriptEngine.buildJsScript(jsModule);

  scriptEngine.fetchStrings(script);

  t.pass();
});

test("lua", (t) => {
  const scriptEngine = new ScriptEngine();

  const luaModule = `
    local arr = {"1", "2"};
    return arr;
    `;

  const script = scriptEngine.buildLuaScript(luaModule);

  scriptEngine.fetchStrings(script);

  t.pass();
});
