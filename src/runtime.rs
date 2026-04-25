use std::collections::HashMap;
use std::path::Path;

#[derive(Debug, Clone)]
pub struct WidgetInfo {
    pub id: String,
    pub widget_type: String,
    pub action: Option<String>,
    pub section: String,
    pub bounds: (f64, f64, f64, f64),
    pub z_index: usize,
    pub fill_id: Option<String>,
    pub normal_fill: Option<String>,
    pub mousedown_fill: Option<String>,
}

pub struct VbrRuntime {
    svg_data: Vec<u8>,
    svg_current: String,
    tree: Option<usvg::Tree>,
    pane_visibility: HashMap<String, bool>,
    width: u32,
    height: u32,
    hit_table: Vec<WidgetInfo>,
    widgets_by_id: HashMap<String, WidgetInfo>,
    pressed_widget_id: Option<String>,
    section_order: Vec<String>,
}

impl VbrRuntime {
    pub fn new(svg_path: &Path) -> Result<Self, Box<dyn std::error::Error>> {
        let svg_data = std::fs::read(svg_path)?;
        let svg_current = std::str::from_utf8(&svg_data)?.to_string();

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

        let hit_table = build_hit_table(&svg_current);
        let mut widgets_by_id = HashMap::new();
        for w in &hit_table {
            widgets_by_id.insert(w.id.clone(), w.clone());
        }

        let section_order = vec![
            "bottom".to_string(),
            "top".to_string(),
            "left".to_string(),
            "right".to_string(),
            "main".to_string(),
        ];

        Ok(Self {
            svg_data,
            svg_current,
            tree: Some(tree),
            pane_visibility,
            width,
            height,
            hit_table,
            widgets_by_id,
            pressed_widget_id: None,
            section_order,
        })
    }

    pub fn svg_data(&self) -> &[u8] {
        &self.svg_data
    }

    pub fn dimensions(&self) -> (u32, u32) {
        (self.width, self.height)
    }

    pub fn handle_click(&mut self, x: f64, y: f64) -> String {
        if let Some(w) = self.top_hit_for_point(x, y).cloned() {
            if let (Some(fill_id), Some(down_fill)) = (&w.fill_id, &w.mousedown_fill) {
                self.set_fill_for_fill_id(fill_id, down_fill);
                self.svg_data = self.svg_current.as_bytes().to_vec();
            }
            self.pressed_widget_id = Some(w.id.clone());

            let action_str = match &w.action {
                Some(a) => format!(" action=\"{}\"", a),
                None => String::new(),
            };
            return format!(
                "CLICK: widget=\"{}\" type=\"{}\" section=\"{}\"{} at ({:.0}, {:.0})",
                w.id, w.widget_type, w.section, action_str, x, y
            );
        }

        format!("CLICK: no widget at ({:.0}, {:.0})", x, y)
    }

    pub fn handle_mouse_up(&mut self) {
        let Some(widget_id) = self.pressed_widget_id.take() else {
            return;
        };
        let (fill_id, normal_fill) = {
            let Some(w) = self.widgets_by_id.get(&widget_id) else {
                return;
            };
            (w.fill_id.clone(), w.normal_fill.clone())
        };

        if let (Some(fill_id), Some(normal_fill)) = (fill_id, normal_fill) {
            self.set_fill_for_fill_id(&fill_id, &normal_fill);
            self.svg_data = self.svg_current.as_bytes().to_vec();
        }
    }

    fn top_hit_for_point(&self, x: f64, y: f64) -> Option<&WidgetInfo> {
        for section in &self.section_order {
            let visible = self.pane_visibility.get(section).copied().unwrap_or(false);
            if !visible {
                continue;
            }

            let mut winner: Option<&WidgetInfo> = None;
            for w in &self.hit_table {
                if &w.section != section {
                    continue;
                }
                let (wx, wy, ww, wh) = w.bounds;
                if x >= wx && x <= wx + ww && y >= wy && y <= wy + wh {
                    match winner {
                        Some(curr) if curr.z_index > w.z_index => {}
                        _ => winner = Some(w),
                    }
                }
            }

            if winner.is_some() {
                return winner;
            }
        }
        None
    }

    fn set_fill_for_fill_id(&mut self, fill_id: &str, fill_color: &str) {
        let needle = format!("vbr:fill-id=\"{}\"", fill_id);
        let Some(attr_pos) = self.svg_current.find(&needle) else {
            return;
        };

        let start = self.svg_current[..attr_pos].rfind('<').unwrap_or(attr_pos);
        let end = self.svg_current[attr_pos..]
            .find('>')
            .map(|i| attr_pos + i)
            .unwrap_or(self.svg_current.len());

        let mut tag = self.svg_current[start..=end].to_string();

        if let Some(fill_pos) = tag.find("fill=\"") {
            let val_start = fill_pos + "fill=\"".len();
            if let Some(rel_end_quote) = tag[val_start..].find('"') {
                let val_end = val_start + rel_end_quote;
                tag.replace_range(val_start..val_end, fill_color);
            }
        } else {
            let insert_at = tag.rfind('/').unwrap_or(tag.len() - 1);
            let insert = format!(" fill=\"{}\"", fill_color);
            tag.insert_str(insert_at, &insert);
        }

        self.svg_current.replace_range(start..=end, &tag);
    }
}

fn build_hit_table(svg: &str) -> Vec<WidgetInfo> {
    let doc = match roxmltree::Document::parse(svg) {
        Ok(d) => d,
        Err(_) => return Vec::new(),
    };

    let ns = doc
        .root_element()
        .lookup_namespace_uri(Some("vbr"))
        .unwrap_or("http://vbr.dev/ui");

    let mut widgets = Vec::new();
    let mut order_counter: usize = 0;
    walk_for_widgets(
        &doc.root_element(),
        ns,
        "main",
        &mut widgets,
        &mut order_counter,
    );
    widgets
}

fn walk_for_widgets(
    node: &roxmltree::Node,
    ns: &str,
    current_section: &str,
    widgets: &mut Vec<WidgetInfo>,
    order_counter: &mut usize,
) {
    if !node.is_element() {
        return;
    }

    let mut section_here = current_section;
    if node.tag_name().name() == "g" {
        if let Some(s) = node.attribute((ns, "section")) {
            section_here = s;
        }

        if let Some(wtype) = node.attribute((ns, "type")) {
            if let Some(bounds) = compute_bounds(node) {
                let id = node.attribute("id").unwrap_or("unnamed").to_string();
                let action = node.attribute((ns, "action")).map(|s| s.to_string());
                let mousedown_fill = node.attribute((ns, "mousedown-fill")).map(|s| s.to_string());
                let (fill_id, normal_fill) = find_fill_target(node, ns);

                widgets.push(WidgetInfo {
                    id,
                    widget_type: wtype.to_string(),
                    action,
                    section: section_here.to_string(),
                    bounds,
                    z_index: *order_counter,
                    fill_id,
                    normal_fill,
                    mousedown_fill,
                });
                *order_counter += 1;
            }
        }
    }

    for child in node.children() {
        walk_for_widgets(&child, ns, section_here, widgets, order_counter);
    }
}

fn find_fill_target(node: &roxmltree::Node, ns: &str) -> (Option<String>, Option<String>) {
    for child in node.children() {
        if !child.is_element() {
            continue;
        }
        if let Some(fill_id) = child.attribute((ns, "fill-id")) {
            let normal_fill = child.attribute("fill").map(|s| s.to_string());
            return (Some(fill_id.to_string()), normal_fill);
        }
    }
    (None, None)
}

fn compute_bounds(node: &roxmltree::Node) -> Option<(f64, f64, f64, f64)> {
    let mut min_x = f64::MAX;
    let mut min_y = f64::MAX;
    let mut max_x = f64::MIN;
    let mut max_y = f64::MIN;
    let mut found = false;

    for child in node.children() {
        if !child.is_element() {
            continue;
        }

        match child.tag_name().name() {
            "rect" => {
                let x = parse_attr(&child, "x").unwrap_or(0.0);
                let y = parse_attr(&child, "y").unwrap_or(0.0);
                let w = parse_attr(&child, "width")?;
                let h = parse_attr(&child, "height")?;
                update_extents(
                    x,
                    y,
                    x + w,
                    y + h,
                    &mut min_x,
                    &mut min_y,
                    &mut max_x,
                    &mut max_y,
                );
                found = true;
            }
            "circle" => {
                let cx = parse_attr(&child, "cx").unwrap_or(0.0);
                let cy = parse_attr(&child, "cy").unwrap_or(0.0);
                let r = parse_attr(&child, "r")?;
                update_extents(
                    cx - r,
                    cy - r,
                    cx + r,
                    cy + r,
                    &mut min_x,
                    &mut min_y,
                    &mut max_x,
                    &mut max_y,
                );
                found = true;
            }
            "line" => {
                let x1 = parse_attr(&child, "x1")?;
                let y1 = parse_attr(&child, "y1")?;
                let x2 = parse_attr(&child, "x2")?;
                let y2 = parse_attr(&child, "y2")?;
                update_extents(
                    x1,
                    y1,
                    x2,
                    y2,
                    &mut min_x,
                    &mut min_y,
                    &mut max_x,
                    &mut max_y,
                );
                found = true;
            }
            "text" => {
                let x = parse_attr(&child, "x").unwrap_or(0.0);
                let y = parse_attr(&child, "y").unwrap_or(0.0);
                let text = child.text().unwrap_or("");
                let est_w = text.len() as f64 * 7.0;
                let est_h = 20.0;
                update_extents(
                    x,
                    y - est_h,
                    x + est_w,
                    y,
                    &mut min_x,
                    &mut min_y,
                    &mut max_x,
                    &mut max_y,
                );
                found = true;
            }
            _ => {}
        }
    }

    if found {
        Some((min_x, min_y, max_x - min_x, max_y - min_y))
    } else {
        None
    }
}

#[inline]
fn update_extents(
    x1: f64,
    y1: f64,
    x2: f64,
    y2: f64,
    min_x: &mut f64,
    min_y: &mut f64,
    max_x: &mut f64,
    max_y: &mut f64,
) {
    *min_x = min_x.min(x1);
    *min_y = min_y.min(y1);
    *max_x = max_x.max(x2);
    *max_y = max_y.max(y2);
}

fn parse_attr(node: &roxmltree::Node, name: &str) -> Option<f64> {
    node.attribute(name)?.parse::<f64>().ok()
}
