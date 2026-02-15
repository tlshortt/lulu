fn main() {
    let args: Vec<String> = std::env::args().collect();

    if args.iter().any(|arg| arg == "--version") {
        println!("claude 1.2.3");
        return;
    }

    let should_fail = args.iter().any(|arg| arg.contains("fail"));
    let delay_ms = args
        .iter()
        .find_map(|arg| extract_delay_ms(arg))
        .unwrap_or(0);

    println!("{}", r#"{"type":"system","subtype":"init"}"#);
    println!(
        "{}",
        r#"{"type":"assistant","message":{"content":[{"type":"thinking","thinking":"Planning the response"}]}}"#
    );
    println!(
        "{}",
        r#"{"type":"assistant","message":{"content":[{"type":"text","text":"hello from test cli"}]}}"#
    );
    println!(
        "{}",
        r#"{"type":"assistant","message":{"content":[{"type":"tool_use","id":"tool-1","name":"read_file","input":{"path":"README.md"}}]}}"#
    );
    println!(
        "{}",
        r#"{"type":"user","message":{"content":[{"type":"tool_result","tool_use_id":"tool-1","content":{"ok":true}}]}}"#
    );

    if delay_ms > 0 {
        std::thread::sleep(std::time::Duration::from_millis(delay_ms));
    }

    if should_fail {
        println!("{}", r#"{"type":"result","subtype":"error","is_error":true}"#);
        std::process::exit(1);
    }

    println!("{}", r#"{"type":"result","subtype":"success","is_error":false}"#);
}

fn extract_delay_ms(arg: &str) -> Option<u64> {
    for token in arg.split_whitespace() {
        if let Some(raw) = token
            .strip_prefix("delay-ms=")
            .or_else(|| token.strip_prefix("delay=") )
            .or_else(|| token.strip_prefix("sleep-ms="))
            .or_else(|| token.strip_prefix("sleep="))
        {
            if let Ok(value) = raw.parse::<u64>() {
                return Some(value);
            }
        }
    }

    None
}
