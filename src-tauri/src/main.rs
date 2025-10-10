// Prevents additional console window on Windows in release
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]
// Silence warnings from objc crate's old cfg attributes
#![allow(unexpected_cfgs)]

fn main() {
    tnnl_lib::run()
}
