use std::collections::HashMap;
use std::path::Path;
use std::time::{Duration, Instant};

const PANE_ANIMATION_MS: u64 = 160;

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

#[derive(Debug, Clone)]
struct PaneTransition {
    section: String,
    show: bool,
    start: Instant,
    duration: Duration,
    pane_from: (f64, f64),
    pane_to: (f64, f64),
    main_from: (f64, f64),
    main_to: (f64, f64),
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
    nudge_left: f64,
    nudge_right: f64,
    nudge_top: f64,
    nudge_bottom: f64,
    pane_size: HashMap<String, f64>,
    pane_transform: HashMap<String, (f64, f64)>,
    main_transform: (f64, f64),
    transition: Option<PaneTransition>,
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

        let (nudge_left, nudge_right, nudge_top, nudge_bottom) = extract_main_nudges(&svg_current);
        let pane_size = extract_pane_sizes(&svg_current);

        let mut pane_transform = HashMap::new();
        pane_transform.insert("left".to_string(), (0.0, 0.0));
        pane_transform.insert("right".to_string(), (0.0, 0.0));
        pane_transform.insert("top".to_string(), (0.0, 0.0));
        pane_transform.insert("bottom".to_string(), (0.0, 0.0));

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
            nudge_left,
            nudge_right,
            nudge_top,
            nudge_bottom,
            pane_size,
            pane_transform,
            main_transform: (0.0, 0.0),
            transition: None,
        })
    }

    pub fn svg_data(&self) -> &[u8] {
        &self.svg_data
    }

    pub fn dimensions(&self) -> (u32, u32) {
        (self.width, self.height)
    }

    pub fn is_animating(&self) -> bool {
        self.transition.is_some()
    }

    pub fn tick_animation(&mut self) {
        let Some(tr) = self.transition.clone() else {
            return;
        };

        let elapsed = Instant::now().saturating_duration_since(tr.start);
        let t = (elapsed.as_secs_f64() / tr.duration.as_secs_f64()).clamp(0.0, 1.0);
        // Smooth ease-out for less mechanical pane motion.
        let te = ease_out_cubic(t);

        let px = lerp(tr.pane_from.0, tr.pane_to.0, te);
        let py = lerp(tr.pane_from.1, tr.pane_to.1, te);
        let mx = lerp(tr.main_from.0, tr.main_to.0, te);
        let my = lerp(tr.main_from.1, tr.main_to.1, te);

        self.pane_transform.insert(tr.section.clone(), (px, py));
        self.main_transform = (mx, my);

        self.set_section_transform(&tr.section, px, py);
        self.set_main_transform(mx, my);

        if (t - 1.0).abs() < f64::EPSILON {
            if tr.show {
                self.pane_visibility.insert(tr.section.clone(), true);
            } else {
                self.pane_visibility.insert(tr.section.clone(), false);
                self.set_section_display(&tr.section, false);
            }
            self.transition = None;
        }

        self.svg_data = self.svg_current.as_bytes().to_vec();
    }

    pub fn handle_click(&mut self, x: f64, y: f64) -> String {
        if self.transition.is_some() {
            return format!("CLICK: animating at ({:.0}, {:.0})", x, y);
        }

        if let Some(w) = self.top_hit_for_point(x, y).cloned() {
            if let (Some(fill_id), Some(down_fill)) = (&w.fill_id, &w.mousedown_fill) {
                self.set_fill_for_fill_id(fill_id, down_fill);
                self.svg_data = self.svg_current.as_bytes().to_vec();
            }
            self.pressed_widget_id = Some(w.id.clone());

            if let Some(action) = &w.action {
                self.apply_builtin_action(action);
            }

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

    fn apply_builtin_action(&mut self, action: &str) {
        match action {
            "show-left-overlay" => self.start_transition("left", true, false),
            "show-left-nudge" => self.start_transition("left", true, true),
            "show-right-overlay" => self.start_transition("right", true, false),
            "show-right-nudge" => self.start_transition("right", true, true),
            "show-top-overlay" => self.start_transition("top", true, false),
            "show-bottom-overlay" => self.start_transition("bottom", true, false),
            "hide-left" => self.start_transition("left", false, false),
            "hide-right" => self.start_transition("right", false, false),
            "hide-top" => self.start_transition("top", false, false),
            "hide-bottom" => self.start_transition("bottom", false, false),
            "hide-all" => {
                for s in ["left", "right", "top", "bottom"] {
                    self.pane_visibility.insert(s.to_string(), false);
                    self.set_section_display(s, false);
                    self.set_section_transform(s, 0.0, 0.0);
                }
                self.main_transform = (0.0, 0.0);
                self.set_main_transform(0.0, 0.0);
                self.transition = None;
                self.svg_data = self.svg_current.as_bytes().to_vec();
            }
            _ => {}
        }
    }

    fn start_transition(&mut self, section: &str, show: bool, nudge: bool) {
        // Hide other panes when showing one.
        if show {
            for s in ["left", "right", "top", "bottom"] {
                if s != section {
                    self.pane_visibility.insert(s.to_string(), false);
                    self.set_section_display(s, false);
                    self.set_section_transform(s, 0.0, 0.0);
                }
            }
            self.set_section_display(section, true);
            self.pane_visibility.insert(section.to_string(), true);
        }

        let size = self.pane_size.get(section).copied().unwrap_or(0.0);
        let off = offscreen_offset(section, size);

        let pane_from = if show {
            off
        } else {
            self.pane_transform.get(section).copied().unwrap_or((0.0, 0.0))
        };
        let pane_to = if show { (0.0, 0.0) } else { off };

        let main_from = self.main_transform;
        let main_to = if show && nudge { self.nudge_target(section) } else { (0.0, 0.0) };

        self.transition = Some(PaneTransition {
            section: section.to_string(),
            show,
            start: Instant::now(),
            duration: Duration::from_millis(PANE_ANIMATION_MS),
            pane_from,
            pane_to,
            main_from,
            main_to,
        });

        let sec = section.to_string();
        self.set_section_transform(&sec, pane_from.0, pane_from.1);
        self.svg_data = self.svg_current.as_bytes().to_vec();
    }

    fn nudge_target(&self, section: &str) -> (f64, f64) {
        match section {
            "left" => (self.nudge_left, 0.0),
            "right" => (-self.nudge_right, 0.0),
            "top" => (0.0, self.nudge_top),
            "bottom" => (0.0, -self.nudge_bottom),
            _ => (0.0, 0.0),
        }
    }

    fn set_section_display(&mut self, section: &str, visible: bool) {
        let sec_attr = format!("vbr:section=\"{}\"", section);
        let Some(sec_pos) = self.svg_current.find(&sec_attr) else {
            return;
        };

        let start = self.svg_current[..sec_pos].rfind('<').unwrap_or(sec_pos);
        let end = self.svg_current[sec_pos..]
            .find('>')
            .map(|i| sec_pos + i)
            .unwrap_or(self.svg_current.len());

        let mut tag = self.svg_current[start..=end].to_string();
        let target = if visible { "inline" } else { "none" };

        if let Some(pos) = tag.find("display=\"") {
            let val_start = pos + "display=\"".len();
            if let Some(rel_end) = tag[val_start..].find('"') {
                let val_end = val_start + rel_end;
                tag.replace_range(val_start..val_end, target);
            }
        } else {
            let insert_at = tag.rfind('>').unwrap_or(tag.len());
            let insert = format!(" display=\"{}\"", target);
            tag.insert_str(insert_at, &insert);
        }

        self.svg_current.replace_range(start..=end, &tag);
    }

    fn set_section_transform(&mut self, section: &str, dx: f64, dy: f64) {
        let sec_attr = format!("vbr:section=\"{}\"", section);
        let Some(sec_pos) = self.svg_current.find(&sec_attr) else {
            return;
        };

        let start = self.svg_current[..sec_pos].rfind('<').unwrap_or(sec_pos);
        let end = self.svg_current[sec_pos..]
            .find('>')
            .map(|i| sec_pos + i)
            .unwrap_or(self.svg_current.len());

        let mut tag = self.svg_current[start..=end].to_string();
        let transform_value = if dx == 0.0 && dy == 0.0 {
            None
        } else {
            Some(format!("translate({:.0} {:.0})", dx, dy))
        };

        if let Some(pos) = tag.find("transform=\"") {
            let val_start = pos + "transform=\"".len();
            if let Some(rel_end) = tag[val_start..].find('"') {
                let val_end = val_start + rel_end;
                if let Some(v) = transform_value {
                    tag.replace_range(val_start..val_end, &v);
                } else {
                    let rm_start = pos.saturating_sub(1);
                    let rm_end = val_end + 1;
                    if rm_end <= tag.len() {
                        tag.replace_range(rm_start..rm_end, "");
                    }
                }
            }
        } else if let Some(v) = transform_value {
            let insert_at = tag.rfind('>').unwrap_or(tag.len());
            let insert = format!(" transform=\"{}\"", v);
            tag.insert_str(insert_at, &insert);
        }

        self.svg_current.replace_range(start..=end, &tag);
        self.pane_transform.insert(section.to_string(), (dx, dy));
    }

    fn set_main_transform(&mut self, dx: f64, dy: f64) {
        let main_attr = "id=\"vbr-main\"";
        let Some(main_pos) = self.svg_current.find(main_attr) else {
            return;
        };

        let start = self.svg_current[..main_pos].rfind('<').unwrap_or(main_pos);
        let end = self.svg_current[main_pos..]
            .find('>')
            .map(|i| main_pos + i)
            .unwrap_or(self.svg_current.len());

        let mut tag = self.svg_current[start..=end].to_string();
        let transform_value = if dx == 0.0 && dy == 0.0 {
            None
        } else {
            Some(format!("translate({:.0} {:.0})", dx, dy))
        };

        if let Some(pos) = tag.find("transform=\"") {
            let val_start = pos + "transform=\"".len();
            if let Some(rel_end) = tag[val_start..].find('"') {
                let val_end = val_start + rel_end;
                if let Some(v) = transform_value {
                    tag.replace_range(val_start..val_end, &v);
                } else {
                    let rm_start = pos.saturating_sub(1);
                    let rm_end = val_end + 1;
                    if rm_end <= tag.len() {
                        tag.replace_range(rm_start..rm_end, "");
                    }
                }
            }
        } else if let Some(v) = transform_value {
            let insert_at = tag.rfind('>').unwrap_or(tag.len());
            let insert = format!(" transform=\"{}\"", v);
            tag.insert_str(insert_at, &insert);
        }

        self.svg_current.replace_range(start..=end, &tag);
        self.main_transform = (dx, dy);
    }
}

fn lerp(a: f64, b: f64, t: f64) -> f64 {
    a + (b - a) * t
}

fn ease_out_cubic(t: f64) -> f64 {
    1.0 - (1.0 - t).powi(3)
}

fn offscreen_offset(section: &str, size: f64) -> (f64, f64) {
    match section {
        "left" => (-size, 0.0),
        "right" => (size, 0.0),
        "top" => (0.0, -size),
        "bottom" => (0.0, size),
        _ => (0.0, 0.0),
    }
}

fn extract_main_nudges(svg: &str) -> (f64, f64, f64, f64) {
    let doc = match roxmltree::Document::parse(svg) {
        Ok(d) => d,
        Err(_) => return (0.0, 0.0, 0.0, 0.0),
    };
    let ns = doc
        .root_element()
        .lookup_namespace_uri(Some("vbr"))
        .unwrap_or("http://vbr.dev/ui");

    for n in doc.descendants() {
        if !n.is_element() {
            continue;
        }
        if n.tag_name().name() == "g" && n.attribute("id") == Some("vbr-main") {
            let left = n
                .attribute((ns, "nudge-left"))
                .and_then(|v| v.parse::<f64>().ok())
                .unwrap_or(0.0);
            let right = n
                .attribute((ns, "nudge-right"))
                .and_then(|v| v.parse::<f64>().ok())
                .unwrap_or(0.0);
            let top = n
                .attribute((ns, "nudge-top"))
                .and_then(|v| v.parse::<f64>().ok())
                .unwrap_or(0.0);
            let bottom = n
                .attribute((ns, "nudge-bottom"))
                .and_then(|v| v.parse::<f64>().ok())
                .unwrap_or(0.0);
            return (left, right, top, bottom);
        }
    }

    (0.0, 0.0, 0.0, 0.0)
}

fn extract_pane_sizes(svg: &str) -> HashMap<String, f64> {
    let mut out = HashMap::new();
    let doc = match roxmltree::Document::parse(svg) {
        Ok(d) => d,
        Err(_) => return out,
    };
    let ns = doc
        .root_element()
        .lookup_namespace_uri(Some("vbr"))
        .unwrap_or("http://vbr.dev/ui");

    for n in doc.descendants() {
        if !n.is_element() || n.tag_name().name() != "g" {
            continue;
        }
        let Some(section) = n.attribute((ns, "section")) else {
            continue;
        };
        match section {
            "left" | "right" => {
                let w = n
                    .attribute((ns, "width"))
                    .and_then(|v| v.parse::<f64>().ok())
                    .unwrap_or(0.0);
                out.insert(section.to_string(), w);
            }
            "top" | "bottom" => {
                let h = n
                    .attribute((ns, "height"))
                    .and_then(|v| v.parse::<f64>().ok())
                    .unwrap_or(0.0);
                out.insert(section.to_string(), h);
            }
            _ => {}
        }
    }

    out
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
                let mousedown_fill = node
                    .attribute((ns, "mousedown-fill"))
                    .map(|s| s.to_string());
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
