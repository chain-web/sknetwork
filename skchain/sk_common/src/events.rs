use std::collections::HashMap;
use std::fmt::{Display, Formatter, Result};

type Callback = Box<(dyn FnMut(LifeCycleEvents, String) + 'static)>;

pub struct SkEvents {
    handlers: HashMap<LifeCycleEvents, Vec<Callback>>,
    allHandlers: Vec<Callback>,
}

#[derive(Eq, Hash, PartialEq, Clone, Debug)]
pub enum LifeCycleEvents {
    BeforeInit,
    InitGenesis,
}

impl Display for LifeCycleEvents {
    fn fmt(&self, f: &mut Formatter) -> Result {
        write!(f, "{:?}", self)
    }
}

impl SkEvents {
    pub fn new() -> Self {
        SkEvents {
            handlers: HashMap::new(),
            allHandlers: Vec::new(),
        }
    }

    pub fn create_callback<F>(f: F) -> Callback
    where
        F: Fn(LifeCycleEvents, String) + 'static,
    {
        Box::new(f) as Callback
    }

    pub fn register<F>(&mut self, key: LifeCycleEvents, handler: F)
    where
        F: Fn(LifeCycleEvents, String) + 'static,
    {
        if !self.handlers.contains_key(&key) {
            self.handlers.insert(key.clone(), Vec::new());
        }
        let list = &mut self.handlers.get_mut(&key).unwrap();
        let cb = SkEvents::create_callback(handler);
        list.push(cb);
    }

    // emit event
    pub fn emit(&mut self, key: LifeCycleEvents, arg: String) {
        for cb in &mut self.allHandlers {
            cb(key.clone(), arg.clone())
        }
        if !self.handlers.contains_key(&key) {
            return;
        }
        let list = &mut self.handlers.get_mut(&key).unwrap();
        for cb in list.iter_mut() {
            cb(key.clone(), arg.clone())
        }
    }

    // run when emit all keys
    pub fn registerAll<F>(&mut self, handler: F)
    where
        F: Fn(LifeCycleEvents, String) + 'static,
    {
        let cb = SkEvents::create_callback(handler);
        self.allHandlers.push(cb);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_events() {
        let mut evt = SkEvents::new();
        fn test_reg(key: LifeCycleEvents, msgs: String) {
            assert_eq!(key.to_string(), LifeCycleEvents::BeforeInit.to_string());
            assert_eq!(msgs, "test11".to_string());
        }
        evt.register(LifeCycleEvents::BeforeInit, test_reg);
        evt.emit(LifeCycleEvents::BeforeInit, "test11".to_string());
    }

    #[test]
    fn test_all_events() {
        let mut evt = SkEvents::new();

        fn test_all(key: LifeCycleEvents, msgs: String) {
            assert_eq!(key, LifeCycleEvents::BeforeInit);
            assert_eq!(msgs, "test11".to_string());
        }
        evt.registerAll(test_all);
        evt.emit(LifeCycleEvents::BeforeInit, "test11".to_string());
    }
}
