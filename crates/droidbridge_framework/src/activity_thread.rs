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

        // 2. Inflate layout: Try to find a default layout from resources.arsc or synthesize root
        let mut final_view_tree = None;
        if let Some(ref res) = apk.resources {
            for pkg in &res.packages {
                // Find first layout resource
                for (id, entry) in &pkg.entries {
                    if entry.type_name == "layout" {
                        if let droidbridge_loader::ResValue::String(ref xml_path) = entry.value {
                            println!("[*] Auto-inflating primary layout from resources: 0x{:08x} ({})", id, xml_path);
                            if let Ok(mut f) = File::open(&self.apk_path) {
                                if let Ok(mut zip) = ZipArchive::new(&mut f) {
                                    if let Ok(bytes) = zip.read_file(xml_path) {
                                        if let Ok(node) = droidbridge_loader::AxmlDecoder::decode_tree(&bytes) {
                                            let mut view = LayoutInflater::inflate(&node, Some(res));
                                            LayoutInflater::measure_and_layout(
                                                &mut view,
                                                Rect { left: 0, top: 0, right: 420, bottom: 720 },
                                            );
                                            final_view_tree = Some(view);
                                            break;
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
                if final_view_tree.is_some() {
                    break;
                }
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
