extern {
  #[link(name="env")]
  fn log_event(id: *const u8);
}

fn main() {
    unsafe { log_event(::std::ptr::null()); }
}