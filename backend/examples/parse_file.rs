//! Debug helper: run the offline parser on a saved page.
//! `cargo run --example parse_file -- <url> <file.html>`

fn main() {
    let args: Vec<String> = std::env::args().collect();
    let [_, url, path] = args.as_slice() else {
        eprintln!("usage: parse_file <url> <file.html>");
        std::process::exit(2);
    };
    let html = std::fs::read_to_string(path).expect("read file");
    let url = url::Url::parse(url).expect("valid url");
    let (draft, sources) = rollcall::scraper::extract_offline(&url, &html);
    println!(
        "{}",
        serde_json::to_string_pretty(&serde_json::json!({ "draft": draft, "sources": sources }))
            .unwrap()
    );
}
