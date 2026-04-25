use std::collections::HashMap;
use std::path::Path;

/// Core runtime state for the VBR UI.
/// Stage 1: minimal — loads SVG, manages which SVG is active.
/// Stage 2+: pane visibility, widget state, hit testing, actions.
pub struct VbrRuntime {
    /// The raw SVG data currently loaded.
    svg_data: Vec<u8>,
    /// Parsed usvg tree (for attribute lookups in later stages).
    tree: Option<usvg::Tree>,
    /// Pane visibility: "main" | "left" | "right" | "top" | "bottom" -> visible/hidden
    pane_visibility: HashMap<String, bool>,
    /// Canvas dimensions
    width: u32,
    height: u32,
}

impl VbrRuntime {
    pub fn new(svg_path: &Path) -> Result<Self, Box<dyn std::error::Error>> {
        let svg_data = std::fs::read(svg_path)?;

        let opt = usvg::Options::default();
        let tree = usvg::Tree::from_data(&svg_data, &opt)?;

        let width = tree.size().width() as u32;
        let height = tree.size().height() as u32;

        let mut pane_visibility = HashMap::new();
        pane_visibility.insert("main".to_string(), true);
        pane_visibility.insert("left".to_string(), false);
        pane_visibility.insert("right".to_string(), false);
        pane_visibility.insert("top".to_string(), false);
        pane_visibility.insert("bottom".to_string(), false);

        Ok(Self {
            svg_data,
            tree: Some(tree),
            pane_visibility,
            width,
            height,
        })
    }

    /// The raw SVG data for the renderer.
    pub fn svg_data(&self) -> &[u8] {
        &self.svg_data
    }

    pub fn dimensions(&self) -> (u32, u32) {
        (self.width, self.height)
    }

    // Later: toggle pane visibility, query widget state, dispatch actions.
}
