fn main() {
  boxy_gui::build()
    .run(tauri::generate_context!())
    .expect("tauri runtime error");
}
