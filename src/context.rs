use std::sync::Mutex;

use lazy_static::lazy_static;
use log::Level;

lazy_static! {
    static ref GLOBAL_CONTEXT: Mutex<Option<GlobalContextValue>> = Mutex::new(None);
}

pub struct GlobalContext {
}

impl GlobalContext {
    pub fn new(name: &str) -> GlobalContext {
        GlobalContext::new_conditional(Level::iter().next().unwrap(), name)
    }

    pub fn new_conditional(min_level: Level, name: &str) -> GlobalContext {
        let message = format!("[{}] ", name);

        {
            let mut context = GLOBAL_CONTEXT.lock().unwrap();
            if context.is_some() {
                panic!("An attempt to set a nested global context");
            }
            context.replace(GlobalContextValue {
                min_level,
                message
            });
        }

        GlobalContext{}
    }

    pub(crate) fn get(level: Level) -> String {
        match GLOBAL_CONTEXT.lock().unwrap().as_ref() {
            Some(context) if level >= context.min_level => context.message.clone(),
            _ => String::new(),
        }
    }
}

impl Drop for GlobalContext {
    fn drop(&mut self) {
        *GLOBAL_CONTEXT.lock().unwrap() = None;
    }
}

struct GlobalContextValue {
    min_level: Level,
    message: String,
}