/// VBR UI Renderer — loads SVG and renders to a tiny_skia Pixmap via resvg.
pub struct Renderer {
    width: u32,
    height: u32,
}

impl Renderer {
    pub fn new(width: u32, height: u32) -> Self {
        Self { width, height }
    }

    /// Load SVG data and render it to a full Pixmap.
    pub fn render_svg(&self, svg_data: &[u8]) -> Result<tiny_skia::Pixmap, Box<dyn std::error::Error>> {
        let opt = usvg::Options {
            font_family: "DejaVu Sans".to_string(),
            ..usvg::Options::default()
        };
        let tree = usvg::Tree::from_data(svg_data, &opt)?;

        let mut pixmap = tiny_skia::Pixmap::new(self.width, self.height)
            .ok_or("failed to create pixmap")?;

        resvg::render(&tree, tiny_skia::Transform::default(), &mut pixmap.as_mut());

        Ok(pixmap)
    }

    pub fn dimensions(&self) -> (u32, u32) {
        (self.width, self.height)
    }
}
