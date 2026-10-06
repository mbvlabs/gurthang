fn main() {
    if let Err(error) = __GURTHANG_CRATE_NAME__::views::inertia::export_payloads() {
        eprintln!("{error}");
        std::process::exit(1);
    }
}
