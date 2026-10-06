use gurthang::Hooks;

fn main() {
    if let Err(error) = {{ crate_name }}::App::export_payloads() {
        eprintln!("{error}");
        std::process::exit(1);
    }
}
