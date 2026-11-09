// Prevents additional console window on Windows in release, DO NOT REMOVE!!
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

fn main() {
    let test : String = String::from("test");
    let test2 : String = String::from("test2");
    let test3 : String = String::from("test3");

    open_mosaik_lib::run()
}
