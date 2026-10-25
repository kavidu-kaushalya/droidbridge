use std::collections::HashMap;
use droidbridge_loader::{AxmlAttributeValue, AxmlNode, ResourceTable};

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Orientation {
    Vertical,
    Horizontal,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum SizeRule {
    MatchParent,
    WrapContent,
    Exact(i32),
}

#[derive(Debug, Clone, Copy, Default, PartialEq)]
pub struct Rect {
    pub left: i32,
    pub top: i32,
    pub right: i32,
    pub bottom: i32,
}

impl Rect {
    pub fn width(&self) -> i32 {
        (self.right - self.left).max(0)
    }

    pub fn height(&self) -> i32 {
        (self.bottom - self.top).max(0)
    }
}

#[derive(Debug, Clone, Copy, Default, PartialEq)]
pub struct Insets {
    pub left: i32,
    pub top: i32,
    pub right: i32,
    pub bottom: i32,
}

#[derive(Debug, Clone, PartialEq)]
pub struct LayoutParams {
    pub width: SizeRule,
    pub height: SizeRule,
    pub margin: Insets,
    pub padding: Insets,
    pub weight: f32,
}

impl Default for LayoutParams {
    fn default() -> Self {
        Self {
            width: SizeRule::WrapContent,
            height: SizeRule::WrapContent,
            margin: Insets::default(),
            padding: Insets::default(),
            weight: 0.0,
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub enum ViewType {
    LinearLayout { orientation: Orientation },
    TextView { text: String, text_size: f32, text_color: u32 },
    Button { text: String, bg_color: u32, text_color: u32 },
    ImageView { src: String, res_id: Option<u32> },
    EditText { hint: String, text: String },
    FrameLayout,
    RelativeLayout,
    Generic(String),
}

#[derive(Debug, Clone)]
pub struct ViewNode {
    pub id: Option<u32>,
    pub id_name: Option<String>,
    pub view_type: ViewType,
    pub layout_params: LayoutParams,
    pub children: Vec<ViewNode>,
    pub bounds: Rect,
}

pub struct LayoutInflater;

impl LayoutInflater {
    pub fn inflate(root: &AxmlNode, resources: Option<&ResourceTable>) -> ViewNode {
        let tag = root.tag.as_str();

        // Extract ID
        let mut id = None;
        let mut id_name = None;
        if let Some(val) = Self::get_attr(&root.attributes, "id") {
            match val {
                AxmlAttributeValue::Reference(r) => {
                    id = Some(*r);
                }
                AxmlAttributeValue::String(s) => {
                    id_name = Some(s.clone());
                    if let Some(res) = resources {
                        let name_part = s.trim_start_matches("@+id/").trim_start_matches("@id/");
                        if let Some(entry) = res.find_by_name("id", name_part) {
                            id = Some(entry.res_id);
                        }
                    }
                }
                _ => {}
            }
        }

        // Layout Parameters
        let mut params = LayoutParams::default();
        if let Some(val) = Self::get_attr(&root.attributes, "layout_width") {
            params.width = Self::parse_size_rule(val);
        }
        if let Some(val) = Self::get_attr(&root.attributes, "layout_height") {
            params.height = Self::parse_size_rule(val);
        }
        if let Some(AxmlAttributeValue::Dimension(d, _)) = Self::get_attr(&root.attributes, "layout_margin") {
            let m = *d as i32;
            params.margin = Insets { left: m, top: m, right: m, bottom: m };
        }
        if let Some(AxmlAttributeValue::Dimension(d, _)) = Self::get_attr(&root.attributes, "padding") {
            let p = *d as i32;
            params.padding = Insets { left: p, top: p, right: p, bottom: p };
        }

        // View Type Specialization
        let view_type = if tag == "LinearLayout" || tag.ends_with(".LinearLayout") {
            let mut orientation = Orientation::Vertical;
            if let Some(AxmlAttributeValue::String(s)) = Self::get_attr(&root.attributes, "orientation") {
                if s == "horizontal" || s == "0" {
                    orientation = Orientation::Horizontal;
                }
            }
            ViewType::LinearLayout { orientation }
        } else if tag == "Button" || tag.ends_with("Button") || tag.ends_with(".WDSButton") {
            let mut text = Self::resolve_text(&root.attributes, "text", resources);
            if text.is_empty() {
                text = Self::resolve_text(&root.attributes, "contentDescription", resources);
            }
            if text.is_empty() {
                text = "Continue".to_string();
            }
            let bg_color = Self::resolve_color(&root.attributes, "background", resources).unwrap_or(0xff25d366); // WhatsApp Green #25D366
            let text_color = Self::resolve_color(&root.attributes, "textColor", resources).unwrap_or(0xffffffff);
            ViewType::Button { text, bg_color, text_color }
        } else if tag == "TextView" || tag.ends_with("TextView") {
            let text = Self::resolve_text(&root.attributes, "text", resources);
            let text_size = Self::resolve_float(&root.attributes, "textSize").unwrap_or(15.0);
            let text_color = Self::resolve_color(&root.attributes, "textColor", resources).unwrap_or(0xffffffff);
            ViewType::TextView { text, text_size, text_color }
        } else if tag == "ImageView" || tag.ends_with("ImageView") {
            let mut src = String::new();
            let mut res_id = None;
            if let Some(val) = Self::get_attr(&root.attributes, "src") {
                match val {
                    AxmlAttributeValue::String(s) => src = s.clone(),
                    AxmlAttributeValue::Reference(r) => {
                        res_id = Some(*r);
                        if let Some(res) = resources {
                            if let Some(path) = res.resolve_resource_path(*r) {
                                src = path.to_string();
                            }
                        }
                    }
                    _ => {}
                }
            }
            ViewType::ImageView { src, res_id }
        } else if tag == "EditText" || tag.ends_with("EditText") {
            let hint = Self::resolve_text(&root.attributes, "hint", resources);
            let text = Self::resolve_text(&root.attributes, "text", resources);
            ViewType::EditText { hint, text }
        } else if tag == "FrameLayout" || tag.ends_with(".FrameLayout") {
            ViewType::FrameLayout
        } else if tag == "RelativeLayout" || tag.ends_with(".RelativeLayout") {
            ViewType::RelativeLayout
        } else {
            ViewType::Generic(tag.to_string())
        };

        let mut children = Vec::new();
        for child_node in &root.children {
            children.push(Self::inflate(child_node, resources));
        }

        ViewNode {
            id,
            id_name,
            view_type,
            layout_params: params,
            children,
            bounds: Rect::default(),
        }
    }

    fn get_attr<'a>(attrs: &'a HashMap<String, AxmlAttributeValue>, key: &str) -> Option<&'a AxmlAttributeValue> {
        if let Some(v) = attrs.get(key) {
            return Some(v);
        }
        for (k, v) in attrs {
            if k.ends_with(key) {
                return Some(v);
            }
        }
        None
    }

    fn parse_size_rule(val: &AxmlAttributeValue) -> SizeRule {
        match val {
            AxmlAttributeValue::String(s) => {
                if s == "match_parent" || s == "fill_parent" || s == "-1" {
                    SizeRule::MatchParent
                } else if s == "wrap_content" || s == "-2" {
                    SizeRule::WrapContent
                } else {
                    SizeRule::WrapContent
                }
            }
            AxmlAttributeValue::Integer(i) => {
                if *i == -1 {
                    SizeRule::MatchParent
                } else if *i == -2 {
                    SizeRule::WrapContent
                } else {
                    SizeRule::Exact(*i)
                }
            }
            AxmlAttributeValue::Dimension(d, _) => SizeRule::Exact(*d as i32),
            _ => SizeRule::WrapContent,
        }
    }

    fn resolve_text(attrs: &HashMap<String, AxmlAttributeValue>, key: &str, resources: Option<&ResourceTable>) -> String {
        if let Some(val) = Self::get_attr(attrs, key) {
            match val {
                AxmlAttributeValue::String(s) => {
                    if s.starts_with("@string/") {
                        let name = s.trim_start_matches("@string/");
                        if let Some(res) = resources {
                            if let Some(entry) = res.find_by_name("string", name) {
                                if let droidbridge_loader::ResValue::String(ref val_str) = entry.value {
                                    return val_str.clone();
                                }
                            }
                        }
                    }
                    return s.clone();
                }
                AxmlAttributeValue::Reference(r) => {
                    if let Some(res) = resources {
                        if let Some(s) = res.resolve_string(*r) {
                            return s.to_string();
                        }
                    }
                }
                _ => {}
            }
        }
        String::new()
    }

    fn resolve_float(attrs: &HashMap<String, AxmlAttributeValue>, key: &str) -> Option<f32> {
        if let Some(val) = Self::get_attr(attrs, key) {
            match val {
                AxmlAttributeValue::Dimension(d, _) => return Some(*d),
                AxmlAttributeValue::Integer(i) => return Some(*i as f32),
                _ => {}
            }
        }
        None
    }

    fn resolve_color(attrs: &HashMap<String, AxmlAttributeValue>, key: &str, resources: Option<&ResourceTable>) -> Option<u32> {
        if let Some(val) = Self::get_attr(attrs, key) {
            match val {
                AxmlAttributeValue::Color(c) => return Some(*c),
                AxmlAttributeValue::Reference(r) => {
                    if let Some(res) = resources {
                        if let Some(entry) = res.resolve_entry(*r) {
                            if let droidbridge_loader::ResValue::Color(c) = entry.value {
                                return Some(c);
                            }
                        }
                    }
                }
                _ => {}
            }
        }
        None
    }

    /// Layout measurement engine: calculates absolute pixel bounds for a view hierarchy
    pub fn measure_and_layout(root: &mut ViewNode, parent_rect: Rect) {
        root.bounds = parent_rect;

        let padding = root.layout_params.padding;
        let inner_left = parent_rect.left + padding.left;
        let inner_top = parent_rect.top + padding.top;
        let inner_right = parent_rect.right - padding.right;
        let inner_bottom = parent_rect.bottom - padding.bottom;

        match root.view_type {
            ViewType::LinearLayout { orientation } => {
                let count = root.children.len();
                if count == 0 {
                    return;
                }

                if orientation == Orientation::Vertical {
                    let total_available_h = (inner_bottom - inner_top).max(0);
                    let mut current_y = inner_top;

                    for child in &mut root.children {
                        let h = match child.layout_params.height {
                            SizeRule::Exact(px) => px,
                            SizeRule::MatchParent => total_available_h / (count as i32),
                            SizeRule::WrapContent => 48, // default Android item height
                        };

                        let w = match child.layout_params.width {
                            SizeRule::Exact(px) => px,
                            SizeRule::MatchParent => inner_right - inner_left,
                            SizeRule::WrapContent => (inner_right - inner_left).min(200),
                        };

                        let child_rect = Rect {
                            left: inner_left + child.layout_params.margin.left,
                            top: current_y + child.layout_params.margin.top,
                            right: inner_left + w - child.layout_params.margin.right,
                            bottom: current_y + h - child.layout_params.margin.bottom,
                        };

                        Self::measure_and_layout(child, child_rect);
                        current_y += h + child.layout_params.margin.top + child.layout_params.margin.bottom;
                    }
                } else {
                    let total_available_w = (inner_right - inner_left).max(0);
                    let mut current_x = inner_left;

                    for child in &mut root.children {
                        let w = match child.layout_params.width {
                            SizeRule::Exact(px) => px,
                            SizeRule::MatchParent => total_available_w / (count as i32),
                            SizeRule::WrapContent => 100,
                        };

                        let h = match child.layout_params.height {
                            SizeRule::Exact(px) => px,
                            SizeRule::MatchParent => inner_bottom - inner_top,
                            SizeRule::WrapContent => 48,
                        };

                        let child_rect = Rect {
                            left: current_x + child.layout_params.margin.left,
                            top: inner_top + child.layout_params.margin.top,
                            right: current_x + w - child.layout_params.margin.right,
                            bottom: inner_top + h - child.layout_params.margin.bottom,
                        };

                        Self::measure_and_layout(child, child_rect);
                        current_x += w + child.layout_params.margin.left + child.layout_params.margin.right;
                    }
                }
            }
            _ => {
                // Leaf view bounds already assigned
            }
        }
    }
}
