use std::borrow::Borrow;

fn main() {
    let svg_data = std::fs::read("vbr_ui.svg").expect("read svg");

    let opt = usvg::Options {
        font_family: "DejaVu Sans".to_string(),
        ..usvg::Options::default()
    };

    let tree = usvg::Tree::from_data(&svg_data, &opt).expect("parse svg");
    println!("SVG: {}x{}", tree.size().width(), tree.size().height());

    // Walk tree: usvg::Group has children() -> &[Node]
    // Node has borrow() -> Ref<'_, NodeKind>
    // NodeKind variants: Group, Path, Image, Text
    fn count_nodes(g: &usvg::Group) -> usize {
        let mut count = 1;
        for child in g.children() {
            let kind = child.borrow();
            if let usvg::NodeKind::Text(_) = &*kind {
                println!("Found text node");
            } else if let usvg::NodeKind::Group(sub) = &*kind {
                count += count_nodes(sub);
            } else {
                count += 1;
            }
        }
        count
    }
    let total = count_nodes(tree.root());
    println!("Total nodes: {}", total);

    // Render and check pixels
    let mut pixmap = tiny_skia::Pixmap::new(390, 844).unwrap();
    resvg::render(&tree, tiny_skia::Transform::default(), &mut pixmap.as_mut());
    pixmap.save_png("render_test_output.png").unwrap();

    // Check multiple regions
    for (label, x, y) in [
        ("title area", 70, 35),
        ("background", 0, 0),
        ("bottom toolbar", 200, 800),
        ("bottom sheet bg", 100, 500),
        ("outside scrim", 300, 200),
        ("bottom pane sheet", 100, 500),
    ] {
        if let Some(px) = pixmap.pixel(x, y) {
            println!("Pixel ({},{}): rgba({},{},{},{}) [{}]",
                x, y, px.red(), px.green(), px.blue(), px.alpha(), label);
        }
    }

    // Check if any pixel is NOT very dark (could be text)
    let mut light_pixels = 0;
    for y in 0..100 {
        for x in 0..390 {
            if let Some(px) = pixmap.pixel(x, y) {
                // Text would be ~204 (0xcc) - check for pixels brighter than 100
                if px.red() > 80 && px.alpha() > 0 {
                    light_pixels += 1;
                }
            }
        }
    }
    println!("Pixels brighter than 80 in top 100 rows: {}", light_pixels);

    // Specifically check the title area (x:66-250, y:30-40)
    let mut title_pixels = 0;
    for y in 30..=40 {
        for x in 66..=250 {
            if let Some(px) = pixmap.pixel(x, y) {
                if px.red() > 100 {
                    title_pixels += 1;
                }
            }
        }
    }
    println!("Bright pixels in title region (66-250, 30-40): {}", title_pixels);
}
