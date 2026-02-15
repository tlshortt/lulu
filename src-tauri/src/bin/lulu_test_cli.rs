fn main() {
    if std::env::args().any(|arg| arg == "--version") {
        println!("claude 1.2.3");
        return;
    }

    let should_fail = std::env::args().any(|arg| arg.contains("fail"));

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

    if should_fail {
        println!("{}", r#"{"type":"result","subtype":"error","is_error":true}"#);
        std::process::exit(1);
    }

    println!("{}", r#"{"type":"result","subtype":"success","is_error":false}"#);
}
