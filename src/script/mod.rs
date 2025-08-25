use boa_engine::{Context, JsValue, Source};
// use mlua::{FromLuaMulti, Function, Lua};
use napi::Result;
use rhai::AST;

pub enum ScriptType {
  Rhai,
  Lua,
  Javascript,
}

#[napi]
pub struct Script(ScriptInner);

pub enum ScriptInner {
  Rhai { script: AST },
  // Lua { script: Function },
  Javascript { script: boa_engine::Script },
}

#[napi]
pub struct ScriptEngine {
  rhai_engine: rhai::Engine,
  // lua_engine: Lua,
  js_engine: Context,
}

#[napi]
impl ScriptEngine {
  #[napi(constructor)]
  pub fn new() -> Self {
    ScriptEngine {
      rhai_engine: rhai::Engine::new(),
      // lua_engine: Lua::new(),
      js_engine: Context::default(),
    }
  }

  #[napi]
  pub fn build_rhai_script(&self, content: String) -> Result<Script> {
    let script = self
      .rhai_engine
      .compile(content.clone())
      .map_err(|e| napi::Error::from_reason(e.to_string()))?;
    Ok(Script(ScriptInner::Rhai { script }))
  }

  /*
  #[napi]
  pub fn build_lua_script(&self, content: String) -> Result<Script> {
    let func = self
      .lua_engine
      .load(content.clone())
      .into_function()
      .map_err(|e| napi::Error::from_reason(e.to_string()))?;
    Ok(Script(ScriptInner::Lua { script: func }))
  }
  */

  #[napi]
  pub fn build_js_script(&mut self, content: String) -> Result<Script> {
    let source = Source::from_bytes(content.as_bytes());
    let script = boa_engine::Script::parse(source, None, &mut self.js_engine)
      .map_err(|e| napi::Error::from_reason(e.to_string()))?;

    Ok(Script(ScriptInner::Javascript { script }))
  }

  fn execute_rhai_script<T>(&self, ast: &AST) -> Result<T>
  where
    T: Clone + 'static,
  {
    let v = self
      .rhai_engine
      .eval_ast::<T>(ast)
      .map_err(|e| napi::Error::from_reason(e.to_string()))?;

    Ok(v)
  }

  /*
  fn execute_lua_script<T>(&self, function: &Function) -> Result<T>
  where
    T: FromLuaMulti,
  {
    let v = function
      .call::<T>(())
      .map_err(|e| napi::Error::from_reason(e.to_string()))?;
    Ok(v)
  }
   */

  fn execute_js_script(&mut self, func: &boa_engine::Script) -> Result<JsValue> {
    let v = func
      .evaluate(&mut self.js_engine)
      .map_err(|e| napi::Error::from_reason(e.to_string()))?;

    Ok(v)
  }

  #[napi]
  pub fn execute(&mut self, script: &mut Script) -> Result<()> {
    match &script.0 {
      ScriptInner::Rhai { script } => {
        self.execute_rhai_script::<()>(script)?;
      }
      /*ScriptInner::Lua { script } => {
        self.execute_lua_script::<()>(script)?;
      }*/
      ScriptInner::Javascript { script } => {
        self.execute_js_script(script)?;
      }
    };
    Ok(())
  }

  #[napi]
  pub fn fetch_strings(&mut self, script: &mut Script) -> Result<Vec<String>> {
    Ok(match &script.0 {
      ScriptInner::Rhai { script } => self.execute_rhai_script(script)?,
      //ScriptInner::Lua { script } => self.execute_lua_script(script)?,
      ScriptInner::Javascript { script } => {
        let v = self.execute_js_script(script)?;

        serde_json::from_value(
          v.to_json(&mut self.js_engine)
            .map_err(|e| napi::Error::from_reason(e.to_string()))?,
        )
        .map_err(|e| napi::Error::from_reason(e.to_string()))?
      }
    })
  }
}
