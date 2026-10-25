use std::fs::File;
use droidbridge_core::{DexVm, Value};
use droidbridge_loader::{ApkPackage, ZipArchive};
use crate::view::{LayoutInflater, Rect, ViewNode, ViewType};

pub struct ActivityThread {
    pub vm: DexVm,
    pub package_name: String,
    pub apk_path: String,
    pub current_view_root: Option<ViewNode>,
    pub layout_invocations: Vec<u32>,
}

#[derive(Default, Debug, Clone)]
struct LayoutMetrics {
    total_nodes: usize,
    buttons: usize,
    text_views: usize,
    edit_texts: usize,
    image_views: usize,
}

fn analyze_layout(node: &droidbridge_loader::AxmlNode, m: &mut LayoutMetrics) {
    m.total_nodes += 1;
    let tag = &node.tag;
    if tag.ends_with("Button") || tag.ends_with(".WDSButton") || tag.contains("Button") {
        m.buttons += 1;
    } else if tag.ends_with("TextView") || tag.contains("TextView") {
        m.text_views += 1;
    } else if tag.ends_with("EditText") || tag.contains("EditText") {
        m.edit_texts += 1;
    } else if tag.ends_with("ImageView") || tag.contains("ImageView") {
        m.image_views += 1;
    }
    for child in &node.children {
        analyze_layout(child, m);
    }
}

fn compute_layout_score(m: &LayoutMetrics, xml_path: &str) -> usize {
    let mut score = m.total_nodes * 2
        + m.buttons * 20
        + m.text_views * 15
        + m.edit_texts * 25
        + m.image_views * 10;

    // Screens with both descriptive text and actionable buttons are full user interfaces
    if m.buttons > 0 && m.text_views > 0 {
        score += 80;
    }
    if m.buttons > 0 && m.edit_texts > 0 {
        score += 70;
    }
    if m.total_nodes >= 4 {
        score += 30;
    }
    if xml_path.contains("eula") || xml_path.contains("welcome") || xml_path.contains("register") || xml_path.contains("main") {
        score += 100;
    }
    score
}

impl ActivityThread {
    pub fn new(package_name: &str, apk_path: &str) -> Self {
        let mut thread = Self {
            vm: DexVm::new(),
            package_name: package_name.to_string(),
            apk_path: apk_path.to_string(),
            current_view_root: None,
            layout_invocations: Vec::new(),
        };

        thread.register_framework_shims();
        thread
    }

    fn register_framework_shims(&mut self) {
        // android.app.Activity.setContentView(int layoutResID)
        self.vm.register_native_method("android/app/Activity.setContentView", |args| {
            if let Some(layout_id) = args.first() {
                let id = layout_id.as_int() as u32;
                println!("[Android Framework Bridge] Activity.setContentView(layoutResID = 0x{:08x}) called!", id);
                Some(Value::Int(1))
            } else {
                Some(Value::Int(0))
            }
        });

        // android.view.View.findViewById(int id)
        self.vm.register_native_method("android/app/Activity.findViewById", |args| {
            if let Some(view_id) = args.first() {
                let id = view_id.as_int() as u32;
                println!("[Android Framework Bridge] Activity.findViewById(0x{:08x}) invoked", id);
                Some(Value::Ref(Some(id as usize)))
            } else {
                Some(Value::Null)
            }
        });

        // android.widget.TextView.setText(CharSequence text)
        self.vm.register_native_method("android/widget/TextView.setText", |args| {
            println!("[Android Framework Bridge] TextView.setText() invoked with {} args", args.len());
            Some(Value::Null)
        });

        // android.content.Context.getString(int resId)
        self.vm.register_native_method("android/content/Context.getString", |args| {
            if let Some(res_id) = args.first() {
                let id = res_id.as_int() as u32;
                println!("[Android Framework Bridge] Context.getString(0x{:08x}) invoked", id);
                Some(Value::Int(1))
            } else {
                Some(Value::Null)
            }
        });
    }

    /// Launch Activity by invoking its Dalvik bytecode onCreate() method and inflating its view tree
    pub fn launch_activity(
        &mut self,
        activity_class_name: &str,
        apk: &ApkPackage,
    ) -> Result<ViewNode, String> {
        println!("\n===============================================================");
        println!("  DROIDBRIDGE ACTIVITY THREAD: LAUNCHING ACTIVITY");
        println!("===============================================================");
        println!("  Package:          {}", apk.manifest.package_name);
        println!("  Target Activity:  {}", activity_class_name);

        // 1. Resolve Activity in Multi-DEX Class Index
        let multidex = apk.multidex.as_ref().ok_or("No Multi-DEX archive available")?;
        let class = multidex.find_class(activity_class_name);

        if let Some(cls) = class {
            println!("[+] Found class descriptor: {} (Methods: {})", cls.class_name, cls.methods.len());
            
            // Search for onCreate method
            if let Some(on_create) = cls.methods.iter().find(|m| m.method_name == "onCreate") {
                println!("[+] Found lifecycle callback: {} -> signature: {}", on_create.method_name, on_create.signature);

                if let Some(ref code) = on_create.code_item {
                    println!("[*] Executing real Dalvik bytecode instructions ({} code units, {} registers)...", code.insns.len(), code.registers_size);
                    let result = self.vm.execute_bytecode(
                        &code.insns,
                        code.registers_size as usize,
                        code.ins_size as usize,
                        &[],
                    );
                    println!("[+] Dalvik Execution Result: {:?}", result);
                } else {
                    println!("[*] Activity.onCreate() is abstract or native.");
                }
            } else {
                println!("[*] Activity defines no explicit onCreate(); executing standard framework lifecycle.");
            }
        } else {
            println!("[-] Class '{}' not found in DEX index; falling back to manifest entry.", activity_class_name);
        }

        // 2. Inflate layout: Select the richest candidate layout from resources.arsc
        let mut final_view_tree = None;
        if let Some(ref res) = apk.resources {
            let mut best_candidate: Option<(u32, String, droidbridge_loader::AxmlNode, usize)> = None;

            for pkg in &res.packages {
                let mut layout_candidates: Vec<(&u32, &String)> = pkg.entries.iter()
                    .filter(|(_, e)| e.type_name == "layout")
                    .filter_map(|(id, e)| {
                        if let droidbridge_loader::ResValue::String(ref path) = e.value {
                            Some((id, path))
                        } else {
                            None
                        }
                    })
                    .collect();

                layout_candidates.sort_by_key(|(id, _)| **id);

                if let Ok(mut f) = File::open(&self.apk_path) {
                    if let Ok(mut zip) = ZipArchive::new(&mut f) {
                        for (id, xml_path) in layout_candidates {
                            if let Ok(bytes) = zip.read_file(xml_path) {
                                if let Ok(node) = droidbridge_loader::AxmlDecoder::decode_tree(&bytes) {
                                    let mut metrics = LayoutMetrics::default();
                                    analyze_layout(&node, &mut metrics);
                                    let score = compute_layout_score(&metrics, xml_path);

                                    if score > best_candidate.as_ref().map(|b| b.3).unwrap_or(0) {
                                        best_candidate = Some((*id, xml_path.clone(), node, score));
                                    }
                                }
                            }
                        }
                    }
                }
            }

            if let Some((id, xml_path, node, score)) = best_candidate {
                println!("[*] Selected Android Layout: 0x{:08x} ({}) tag: <{}> (Richness Score: {}, Direct Children: {})", id, xml_path, node.tag, score, node.children.len());
                let mut view = LayoutInflater::inflate(&node, Some(res));
                LayoutInflater::measure_and_layout(
                    &mut view,
                    Rect { left: 0, top: 0, right: 420, bottom: 720 },
                );
                final_view_tree = Some(view);
            }
        }

        let view_root = final_view_tree.unwrap_or_else(|| {
            println!("[*] Synthesizing default Material Activity Root View...");
            let mut root = ViewNode {
                id: Some(0x01),
                id_name: Some("content".to_string()),
                view_type: ViewType::LinearLayout { orientation: crate::view::Orientation::Vertical },
                layout_params: crate::view::LayoutParams::default(),
                children: vec![
                    ViewNode {
                        id: Some(0x02),
                        id_name: Some("title".to_string()),
                        view_type: ViewType::TextView {
                            text: format!("{} Active", activity_class_name),
                            text_size: 20.0,
                            text_color: 0xffffffff,
                        },
                        layout_params: crate::view::LayoutParams::default(),
                        children: Vec::new(),
                        bounds: Rect::default(),
                    },
                ],
                bounds: Rect::default(),
            };
            LayoutInflater::measure_and_layout(&mut root, Rect { left: 0, top: 0, right: 420, bottom: 720 });
            root
        });

        self.current_view_root = Some(view_root.clone());
        Ok(view_root)
    }
}
