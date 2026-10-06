fn main() {
    if let Err(error) = {{ crate_name }}::export_payloads() {
        eprintln!("{error}");
        std::process::exit(1);
    }
}
