use std::collections::HashMap;

pub struct VectorStore {
    pub data: HashMap<String, Vec<f32>>,
}

impl VectorStore {
    pub fn new() -> Self {
        Self { data: HashMap::new() }
    }

    pub fn insert(&mut self, id: String, vec: Vec<f32>) {
        self.data.insert(id, vec);
    }

    pub fn search(&self, _query: Vec<f32>, top_k: usize) -> Vec<String> {
        self.data.keys().take(top_k).cloned().collect()
    }
}