pub mod embedding;
pub mod store;

pub struct VectorService;

impl VectorService {
    pub fn new() -> Self { Self }

    pub fn embed(&self, text: &str) -> Vec<f32> {
        vec![0.0; 768]
    }

    pub fn search(&self, _query: &str, _top_k: usize) -> Vec<String> {
        vec![]
    }
}