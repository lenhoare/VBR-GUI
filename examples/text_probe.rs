fn main() {
    let svg = r#"<?xml version='1.0' encoding='UTF-8'?>
<svg xmlns='http://www.w3.org/2000/svg' width='300' height='120' viewBox='0 0 300 120'>
  <rect x='0' y='0' width='300' height='120' fill='#111111'/>
  <text x='20' y='70' font-family='DejaVu Sans' font-size='36' fill='#ffffff'>Hello</text>
</svg>"#;

    let mut opt = usvg::Options {
        font_family: "DejaVu Sans".to_string(),
        ..usvg::Options::default()
    };
    opt.fontdb_mut().load_system_fonts();

    let tree = usvg::Tree::from_str(svg, &opt).expect("parse svg");
    let mut pixmap = tiny_skia::Pixmap::new(300, 120).unwrap();
    resvg::render(&tree, tiny_skia::Transform::default(), &mut pixmap.as_mut());
    pixmap.save_png("text_probe.png").unwrap();

    let mut bright = 0usize;
    for y in 0..120 {
        for x in 0..300 {
            let p = pixmap.pixel(x, y).unwrap();
            if p.red() > 180 || p.green() > 180 || p.blue() > 180 {
                bright += 1;
            }
        }
    }
    println!("bright_pixels={}", bright);
}
