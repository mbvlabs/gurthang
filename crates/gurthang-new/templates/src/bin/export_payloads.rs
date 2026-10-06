fn main() {
    if let Err(error) = {{ crate_name }}::controllers::export_payloads() {
        eprintln!("{error}");
        std::process::exit(1);
    }
}
