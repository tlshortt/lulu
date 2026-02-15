fn main() {
    println!("{}", r#"{"type":"message","data":{"content":"hello from test cli"}}"#);
    println!("plain text marker");
    println!("{}", r#"{"type":"tool_call","data":{"tool_name":"read_file","args":{"path":"README.md"}}}"#);
    println!("{}", r#"{"type":"tool_result","data":{"tool_name":"read_file","result":{"ok":true}}}"#);
}
