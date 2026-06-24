// Prevents additional console window on Windows in release, DO NOT REMOVE!!
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod web_api_proxy;

fn main() {
    web_api_proxy::start_web_api_proxy();
    llm_wiki_lib::run();
}
