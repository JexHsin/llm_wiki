use std::collections::HashMap;

pub struct ChatRequest {
    pub session_id: String,
    pub message: String,
}

pub struct ChatResponse {
    pub answer: String,
}

pub fn handle_chat(_req: ChatRequest) -> ChatResponse {
    ChatResponse {
        answer: "ok".to_string(),
    }
}
