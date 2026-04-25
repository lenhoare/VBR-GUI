#[path = "../src/runtime.rs"]
mod runtime;
#[path = "../src/renderer.rs"]
mod renderer;

fn main() {
    let mut rt = runtime::VbrRuntime::new(std::path::Path::new("vbr_ui.svg")).unwrap();
    let _ = rt.handle_click(30.0, 590.0);
    let _ = rt.handle_text_input("abc");

    let r = renderer::Renderer::new(390, 844);
    let pix = r.render_svg(rt.svg_data()).unwrap();
    pix.save_png("/tmp/input_render_probe.png").unwrap();

    let mut bright = 0usize;
    for y in 578..620 {
        for x in 20..220 {
            let p = pix.pixel(x, y).unwrap();
            if p.red() > 150 || p.green() > 150 || p.blue() > 150 {
                bright += 1;
            }
        }
    }
    println!("bright_pixels_in_input_region={}", bright);

    let s = String::from_utf8(rt.svg_data().to_vec()).unwrap();
    if let Some(i) = s.find("input_notes_text") {
        println!("svg snippet:\n{}", &s[i.saturating_sub(80)..(i+180).min(s.len())]);
    }
}
