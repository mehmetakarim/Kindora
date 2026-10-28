// Release derlemesinde Windows'ta ek konsol penceresi açılmaz.
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

fn main() {
    kindora_desktop_lib::run();
}
