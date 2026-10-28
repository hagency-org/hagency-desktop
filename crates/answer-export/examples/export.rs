fn main() {
    let args: Vec<_> = std::env::args_os().collect();
    let text = std::fs::read_to_string(&args[1]).unwrap();
    let bytes = rinx_answer_export::pdf(&text).unwrap();
    std::fs::write(&args[2], bytes).unwrap();
}
