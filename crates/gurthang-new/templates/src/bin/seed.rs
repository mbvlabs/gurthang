fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();
    if args.iter().any(|arg| arg == "--list") {
        println!("development");
        return;
    }
    let name = args.first().map(String::as_str).unwrap_or("development");
    match name {
        "development" => println!("development seed: no records"),
        other => {
            eprintln!("unknown seed {other}");
            std::process::exit(1);
        }
    }
}
