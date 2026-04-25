#[path = "../src/runtime.rs"]
mod runtime;

fn main() {
    let mut rt = runtime::VbrRuntime::new(std::path::Path::new("vbr_ui.svg")).unwrap();
    println!("{}", rt.handle_click(30.0, 590.0));
    let changed = rt.handle_text_input("abc");
    println!("changed={}", changed);
    let s = String::from_utf8(rt.svg_data().to_vec()).unwrap();
    if let Some(pos) = s.find("id=\"input_notes_text\"") {
        let start = s[..pos].rfind('<').unwrap();
        let end = s[pos..].find("</text>").map(|i| pos + i + 7).unwrap();
        println!("FRAG:{}", &s[start..end]);
    }
}
