use std::collections::HashMap;

// simple session store
pub struct Mem {
    pub s: HashMap<String, Vec<String>>,
}

impl Mem {
    pub fn new() -> Self {
        Self { s: HashMap::new() }
    }

    pub fn add(&mut self, k: &str, v: String) {
        self.s.entry(k.to_string()).or_default().push(v);
    }
}

// simple graph
pub struct G {
    pub e: HashMap<String, Vec<String>>,
}

impl G {
    pub fn new() -> Self {
        Self { e: HashMap::new() }
    }

    pub fn link(&mut self, a: String, b: String) {
        self.e.entry(a).or_default().push(b);
    }
}

// simple vector store
pub struct V {
    pub d: HashMap<String, Vec<f32>>,
}

impl V {
    pub fn new() -> Self {
        Self { d: HashMap::new() }
    }

    pub fn put(&mut self, k: String, v: Vec<f32>) {
        self.d.insert(k, v);
    }
}