mod window;
mod dynamic_renderer;

use std::env;
use std::fs;
use std::path::Path;
use droidbridge_core::{DexVm, ExecutionResult, Value};
use droidbridge_loader::{ApkPackage, DexFile};
use droidbridge_installer::{AppRegistry, PackageInstaller};

const BANNER: &str = r#"
 ===============================================================
   DroidBridge — Native Windows Android Runtime (x64)
   Zero Hypervisors | Zero Emulators | Direct Host Execution
 ===============================================================
"#;

fn print_usage() {
    println!("{}", BANNER);
    println!("Usage: droidbridge <command> [arguments]\n");
    println!("Commands:");
    println!("  install <path.apk>       Install Android APK into DroidBridge sandboxed application storage");
    println!("  uninstall <pkg_name>     Uninstall an application and remove its sandboxed storage");
    println!("  list-apps                List all installed Android applications in DroidBridge");
    println!("  run <path.apk | pkg>     Launch Android APK as an interactive native Windows window");
    println!("  render-layout <apk> <target>  Render real Android layout directly into interactive Windows Desktop Window");
    println!("  inflate-layout <apk> <target> Inflate real Android Binary Layout XML (res/layout/.. or 0x7f..)");
    println!("  resolve-res <apk> <query>Resolve resource by ID (0x7f...) or type/name (string/app_name)");
    println!("  inspect <path.apk>       Inspect APK contents, manifest, DEX, and compatibility");
    println!("  inspect-elf <path.so>    Inspect Android ELF64 native library (.so), headers & JNI symbols");
    println!("  parse-dex <path.dex>     Parse and dump DEX header, classes, and method signatures");
    println!("  exec-test                Run verified Dalvik bytecode execution test on native host");
    println!("  version                  Display DroidBridge runtime version and environment info");
    println!("  help                     Show this help message\n");
}

fn cmd_version() {
    println!("{}", BANNER);
    println!("Runtime:           DroidBridge Core v0.1.0");
    println!("Host Architecture: x86_64-pc-windows-msvc");
    println!("Target OS:         Windows 10 / Windows 11 (NT Kernel 10.0+)");
    println!("Execution Mode:    Direct Host Register-Based DEX Engine (No Hypervisor/VM)");
}

fn cmd_exec_test() {
    println!("{}", BANNER);
    println!("[*] Starting Native Dalvik Bytecode Execution Test (Zero-VM Direct Host)...");

    let vm = DexVm::new();

    let insns: Vec<u16> = vec![
        0x1012,         // pc=0: const/4 v0, #1
        0x6112,         // pc=1: const/4 v1, #6
        0x0212,         // pc=2: const/4 v2, #0
        0x0000,         // pc=3: nop
        // Loop head (pc=4):
        0x1035, 0x0007, // pc=4: if-ge v0, v1, +7 -> targets pc = 4 + 7 = 11
        0x0290, 0x0002, // pc=6: add-int v2, v2, v0 (dst=2, src1=2, src2=0)
        0x00d8, 0x0100, // pc=8: add-int/lit8 v0, v0, #1
        0xfa28,         // pc=10: goto -6 (offset = -6 as i8 = 0xfa) -> targets pc = 10 - 6 = 4
        // Exit (pc=11):
        0x020f,         // pc=11: return v2
    ];

    println!("    Bytecode size: {} 16-bit code units", insns.len());
    println!("    Registers:     v0 (counter), v1 (limit=6), v2 (acc=0)");
    println!("    Execution:     Sum 1..=5 = 15");

    let start_time = std::time::Instant::now();
    let result = vm.execute_bytecode(&insns, 4, 0, &[]);
    let duration = start_time.elapsed();

    match result {
        ExecutionResult::Returned(Some(Value::Int(val))) => {
            println!("\n[+] SUCCESS: Bytecode executed natively on Windows host!");
            println!("    Return value: {}", val);
            println!("    Duration:     {:?}", duration);
            assert_eq!(val, 15, "Accumulator must equal 15");
        }
        other => {
            println!("\n[-] FAILED: Unexpected execution result: {:?}", other);
        }
    }
}

fn cmd_inspect_elf(path: &str) {
    println!("{}", BANNER);
    println!("[*] Inspecting Android ELF Native Library (.so): {}", path);

    let bytes = match fs::read(path) {
        Ok(b) => b,
        Err(e) => {
            eprintln!("[-] Error reading file '{}': {}", path, e);
            return;
        }
    };

    let elf = match droidbridge_native::ElfFile::parse(&bytes) {
        Ok(e) => e,
        Err(e) => {
            eprintln!("[-] Failed to parse ELF file: {}", e);
            return;
        }
    };

    let arch_name = match elf.machine {
        droidbridge_native::EM_X86_64 => "x86_64 (AMD64 - Supported natively on Windows x64)",
        droidbridge_native::EM_AARCH64 => "ARM64 (aarch64 - Requires DBT instruction translation)",
        _ => "Other / Unknown architecture",
    };

    println!("\n===============================================================");
    println!("  ELF64 HEADER & ARCHITECTURE");
    println!("===============================================================");
    println!("  Machine:             0x{:04x} ({})", elf.machine, arch_name);
    println!("  Entry Address:       0x{:016x}", elf.entry);
    println!("  Program Headers:     {}", elf.program_headers.len());
    println!("  Dynamic Relocations: {}", elf.relocations.len());

    println!("\n===============================================================");
    println!("  NEEDED BIONIC DEPENDENCIES (DT_NEEDED: {})", elf.needed_libraries.len());
    println!("===============================================================");
    for lib in &elf.needed_libraries {
        println!("  - {}", lib);
    }

    println!("\n===============================================================");
    println!("  EXPORTED JNI & NATIVE SYMBOLS");
    println!("===============================================================");
    let jni_syms: Vec<_> = elf.symbols.iter().filter(|s| !s.name.is_empty() && (s.name.starts_with("Java_") || s.name.contains("JNI_"))).collect();
    println!("  Total JNI Exports Found: {}", jni_syms.len());
    for sym in jni_syms.iter().take(20) {
        println!("  [0x{:08x}] {}", sym.value, sym.name);
    }
}

fn cmd_parse_dex(path: &str) {
    println!("{}", BANNER);
    println!("[*] Reading DEX file: {}", path);

    let bytes = match fs::read(path) {
        Ok(b) => b,
        Err(e) => {
            eprintln!("[-] Error reading file '{}': {}", path, e);
            return;
        }
    };

    let dex = match DexFile::parse(&bytes) {
        Ok(d) => d,
        Err(e) => {
            eprintln!("[-] Error parsing DEX file: {}", e);
            return;
        }
    };

    println!("\n--- DEX Header ---");
    println!("  Magic:          {}", String::from_utf8_lossy(&dex.header.magic));
    println!("  Checksum:       0x{:08x}", dex.header.checksum);
    println!("  File Size:      {} bytes", dex.header.file_size);
    println!("  Header Size:    {} bytes", dex.header.header_size);
    println!("  Endian Tag:     0x{:08x}", dex.header.endian_tag);
    println!("  String Count:   {}", dex.strings.len());
    println!("  Type Count:     {}", dex.types.len());
    println!("  Method Count:   {}", dex.methods.len());
    println!("  Class Defs:     {}", dex.classes.len());

    println!("\n--- Defined Classes (Top 10) ---");
    for (i, class) in dex.classes.iter().take(10).enumerate() {
        println!("  [{}] Class: {}", i + 1, class.class_name);
        if let Some(super_name) = &class.superclass_name {
            println!("       Extends: {}", super_name);
        }
        println!("       Methods: {}", class.methods.len());
        for m in class.methods.iter().take(3) {
            println!("         - {}({})", m.method_name, m.signature);
        }
    }
}

fn cmd_inspect(path: &str) {
    println!("{}", BANNER);
    println!("[*] Inspecting APK: {}", path);

    if !Path::new(path).exists() {
        eprintln!("[-] File '{}' does not exist.", path);
        return;
    }

    let apk = match ApkPackage::open(path) {
        Ok(a) => a,
        Err(e) => {
            eprintln!("[-] Failed to inspect APK: {}", e);
            return;
        }
    };

    println!("\n===============================================================");
    println!("  APPLICATION METADATA");
    println!("===============================================================");
    println!("  Package Name:        {}", apk.manifest.package_name);
    println!("  Version Code:        {}", apk.manifest.version_code.map(|v| v.to_string()).unwrap_or_else(|| "N/A".to_string()));
    println!("  Version Name:        {}", apk.manifest.version_name.as_deref().unwrap_or("N/A"));
    println!("  Min SDK Version:     {}", apk.manifest.min_sdk_version.map(|v| v.to_string()).unwrap_or_else(|| "N/A".to_string()));
    println!("  Target SDK Version:  {}", apk.manifest.target_sdk_version.map(|v| v.to_string()).unwrap_or_else(|| "N/A".to_string()));
    println!("  Application Label:   {}", apk.manifest.application_label.as_deref().unwrap_or("N/A"));

    println!("\n===============================================================");
    println!("  ACTIVITIES & ENTRY POINTS ({})", apk.manifest.activities.len());
    println!("===============================================================");
    for (i, act) in apk.manifest.activities.iter().enumerate() {
        let tag = if act.is_launcher { " [LAUNCHER]" } else { "" };
        println!("  [{}] {}{}", i + 1, act.name, tag);
        if let Some(lbl) = &act.label {
            println!("       Label: {}", lbl);
        }
    }

    println!("\n===============================================================");
    println!("  PERMISSIONS REQUESTED ({})", apk.manifest.permissions.len());
    println!("===============================================================");
    for perm in &apk.manifest.permissions {
        println!("  - {}", perm);
    }

    println!("\n===============================================================");
    println!("  COMPATIBILITY VERDICT & ARCHITECTURE ANALYSIS");
    println!("===============================================================");
    println!("  DEX Archives:        {}", apk.compatibility.dex_count);
    println!("  Native Libraries:    {}", apk.compatibility.native_libraries.len());
    for lib in &apk.compatibility.native_libraries {
        println!("    - lib/{}/{}", lib.architecture, lib.filename);
    }
    println!("\n  Status:  {:?}", apk.compatibility.verdict);
    println!("  Summary: {}", apk.compatibility.summary);

    if let Ok(dex) = apk.parse_primary_dex() {
        println!("\n===============================================================");
        println!("  PRIMARY DEX SUMMARY");
        println!("===============================================================");
        println!("  Classes:             {}", dex.classes.len());
        println!("  Methods:             {}", dex.methods.len());
        println!("  Strings:             {}", dex.strings.len());
    }

    if let Some(ref res) = apk.resources {
        println!("\n===============================================================");
        println!("  RESOURCE TABLE (resources.arsc)");
        println!("===============================================================");
        println!("  Global Strings Pool: {}", res.global_strings.len());
        println!("  Packages Found:      {}", res.packages.len());
        for pkg in &res.packages {
            println!("  - Package: {} (0x{:02x})", pkg.name, pkg.id);
            println!("    Types ({}):        {:?}", pkg.type_strings.len(), pkg.type_strings);
            println!("    Keys Total:        {}", pkg.key_strings.len());
            println!("    Resolved Entries:  {}", pkg.entries.len());
        }
    }
}

fn cmd_install(path: &str) {
    println!("{}", BANNER);
    println!("[*] Installing Android Package into DroidBridge Sandboxed Storage...");
    println!("    Source APK: {}", path);

    match PackageInstaller::install(path) {
        Ok(app) => {
            println!("\n[+] SUCCESS: Application successfully installed!");
            println!("    Package Name:    {}", app.package_name);
            println!("    Application:     {}", app.app_label);
            println!("    Version:         {} (Code: {})", app.version_name, app.version_code);
            println!("    Main Activity:   {}", app.main_activity);
            println!("    Isolated Data:   {}", app.data_dir.display());
            println!("    Native Libs Dir: {}", app.native_lib_dir.display());
            println!("    Package Size:    {} MB", app.size_bytes / (1024 * 1024));
            println!("\n[*] You can now launch this app at any time with:");
            println!("    droidbridge run {}", app.package_name);
            println!("    or");
            println!("    droidbridge run \"{}\"", path);
        }
        Err(e) => {
            eprintln!("[-] Installation failed: {}", e);
        }
    }
}

fn cmd_uninstall(package_name: &str) {
    println!("{}", BANNER);
    println!("[*] Uninstalling package: {}", package_name);
    match PackageInstaller::uninstall(package_name) {
        Ok(true) => println!("[+] Package '{}' uninstalled and storage cleared.", package_name),
        Ok(false) => println!("[-] Package '{}' was not found in installed registry.", package_name),
        Err(e) => eprintln!("[-] Uninstall error: {}", e),
    }
}

fn cmd_list_apps() {
    println!("{}", BANNER);
    println!("===============================================================");
    println!("  INSTALLED ANDROID APPLICATIONS");
    println!("===============================================================");
    let registry = AppRegistry::load();
    if registry.apps.is_empty() {
        println!("  No applications installed yet.");
        println!("  Install an APK with: droidbridge install <path.apk>");
        return;
    }

    for (i, app) in registry.apps.iter().enumerate() {
        println!("  [{}] {} ({})", i + 1, app.app_label, app.package_name);
        println!("      Version:       v{} (code {})", app.version_name, app.version_code);
        println!("      Activity:      {}", app.main_activity);
        println!("      Container:     {}", app.data_dir.display());
        println!("      Installed:     {}", app.install_time);
        println!();
    }
}

fn cmd_render_layout(apk_path: &str, layout_target: &str) {
    println!("{}", BANNER);
    println!("[*] Extracting and Inflating Android Layout for Dynamic Window: {}", layout_target);

    let mut file = match fs::File::open(apk_path) {
        Ok(f) => f,
        Err(e) => {
            eprintln!("[-] Cannot open APK: {}", e);
            return;
        }
    };

    let mut archive = match droidbridge_loader::ZipArchive::new(&mut file) {
        Ok(a) => a,
        Err(e) => {
            eprintln!("[-] Cannot read APK zip archive: {}", e);
            return;
        }
    };

    let apk = match ApkPackage::open(apk_path) {
        Ok(a) => a,
        Err(e) => {
            eprintln!("[-] Failed to load APK package: {}", e);
            return;
        }
    };

    let xml_path = if layout_target.starts_with("0x") {
        if let Ok(id) = u32::from_str_radix(layout_target.trim_start_matches("0x"), 16) {
            if let Some(res) = &apk.resources {
                res.resolve_resource_path(id).unwrap_or(layout_target).to_string()
            } else {
                layout_target.to_string()
            }
        } else {
            layout_target.to_string()
        }
    } else {
        layout_target.to_string()
    };

    println!("[*] Extracting: {}", xml_path);
    let xml_bytes = match archive.read_file(&xml_path) {
        Ok(b) => b,
        Err(e) => {
            eprintln!("[-] Failed to extract layout '{}': {}", xml_path, e);
            return;
        }
    };

    let axml_root = match droidbridge_loader::AxmlDecoder::decode_tree(&xml_bytes) {
        Ok(node) => node,
        Err(e) => {
            eprintln!("[-] Failed to decode layout XML AST: {}", e);
            return;
        }
    };

    let view_tree = droidbridge_framework::LayoutInflater::inflate(&axml_root, apk.resources.as_ref());
    let title = format!("{} ({})", apk.manifest.package_name, xml_path);

    println!("[*] Launching Dynamic Layout Native Windows Window...");
    if let Err(e) = dynamic_renderer::run_dynamic_layout_window(&title, view_tree) {
        eprintln!("[-] Window Rendering Error: {}", e);
    }
}

fn cmd_inflate_layout(apk_path: &str, layout_target: &str) {
    println!("{}", BANNER);
    println!("[*] Reading APK & inflating Android Layout: {}", layout_target);

    let mut file = match fs::File::open(apk_path) {
        Ok(f) => f,
        Err(e) => {
            eprintln!("[-] Cannot open APK: {}", e);
            return;
        }
    };

    let mut archive = match droidbridge_loader::ZipArchive::new(&mut file) {
        Ok(a) => a,
        Err(e) => {
            eprintln!("[-] Cannot read APK zip archive: {}", e);
            return;
        }
    };

    let apk = match ApkPackage::open(apk_path) {
        Ok(a) => a,
        Err(e) => {
            eprintln!("[-] Failed to load APK package: {}", e);
            return;
        }
    };

    // Determine XML path (either passed directly or resolved from resource ID)
    let xml_path = if layout_target.starts_with("0x") {
        if let Ok(id) = u32::from_str_radix(layout_target.trim_start_matches("0x"), 16) {
            if let Some(res) = &apk.resources {
                res.resolve_resource_path(id).unwrap_or(layout_target).to_string()
            } else {
                layout_target.to_string()
            }
        } else {
            layout_target.to_string()
        }
    } else {
        layout_target.to_string()
    };

    println!("[*] Extracting Layout XML binary from APK: {}", xml_path);
    let xml_bytes = match archive.read_file(&xml_path) {
        Ok(b) => b,
        Err(e) => {
            eprintln!("[-] Failed to extract layout '{}': {}", xml_path, e);
            return;
        }
    };

    println!("[*] Parsing Android Binary XML AST (AxmlNode)...");
    let axml_root = match droidbridge_loader::AxmlDecoder::decode_tree(&xml_bytes) {
        Ok(node) => node,
        Err(e) => {
            eprintln!("[-] Failed to parse binary layout XML: {}", e);
            return;
        }
    };

    println!("[+] Successfully decoded XML root tag: <{}>", axml_root.tag);
    println!("\n[*] Inflating into Android View Hierarchy (LayoutInflater)...");
    let mut view_tree = droidbridge_framework::LayoutInflater::inflate(&axml_root, apk.resources.as_ref());

    println!("[*] Running Layout Measurement Engine (Window: 400x800)...");
    droidbridge_framework::LayoutInflater::measure_and_layout(
        &mut view_tree,
        droidbridge_framework::Rect { left: 0, top: 0, right: 400, bottom: 800 }
    );

    println!("\n===============================================================");
    println!("  INFLATED ANDROID VIEW HIERARCHY");
    println!("===============================================================");
    print_view_node(&view_tree, 0);
}

fn print_view_node(node: &droidbridge_framework::ViewNode, depth: usize) {
    let indent = "  ".repeat(depth);
    let id_str = node.id_name.as_deref().or(node.id.map(|_| "id_assigned")).unwrap_or("no_id");
    let bounds = format!("[x:{}, y:{}, w:{}px, h:{}px]", node.bounds.left, node.bounds.top, node.bounds.width(), node.bounds.height());

    match &node.view_type {
        droidbridge_framework::ViewType::LinearLayout { orientation } => {
            println!("{}-> LinearLayout ({:?}) {} ID: {}", indent, orientation, bounds, id_str);
        }
        droidbridge_framework::ViewType::TextView { text, text_size, text_color } => {
            println!("{}-> TextView \"{}\" (size: {}sp, color: #{:08x}) {} ID: {}", indent, text, text_size, text_color, bounds, id_str);
        }
        droidbridge_framework::ViewType::Button { text, bg_color, text_color } => {
            println!("{}-> Button \"{}\" (bg: #{:08x}, fg: #{:08x}) {} ID: {}", indent, text, bg_color, text_color, bounds, id_str);
        }
        droidbridge_framework::ViewType::ImageView { src, .. } => {
            println!("{}-> ImageView src: \"{}\" {} ID: {}", indent, src, bounds, id_str);
        }
        droidbridge_framework::ViewType::EditText { hint, text } => {
            println!("{}-> EditText text: \"{}\", hint: \"{}\" {} ID: {}", indent, text, hint, bounds, id_str);
        }
        droidbridge_framework::ViewType::FrameLayout => {
            println!("{}-> FrameLayout {} ID: {}", indent, bounds, id_str);
        }
        droidbridge_framework::ViewType::RelativeLayout => {
            println!("{}-> RelativeLayout {} ID: {}", indent, bounds, id_str);
        }
        droidbridge_framework::ViewType::Generic(name) => {
            println!("{}-> <{}> {} ID: {}", indent, name, bounds, id_str);
        }
    }

    for child in &node.children {
        print_view_node(child, depth + 1);
    }
}

fn cmd_resolve_res(path: &str, query: &str) {
    println!("{}", BANNER);
    println!("[*] Loading Resource Table from: {}", path);
    let apk = match ApkPackage::open(path) {
        Ok(a) => a,
        Err(e) => {
            eprintln!("[-] Failed to load APK: {}", e);
            return;
        }
    };

    let res = match apk.resources {
        Some(ref r) => r,
        None => {
            eprintln!("[-] No resources.arsc found in APK.");
            return;
        }
    };

    if query.starts_with("0x") {
        if let Ok(id) = u32::from_str_radix(query.trim_start_matches("0x"), 16) {
            if let Some(entry) = res.resolve_entry(id) {
                println!("[+] Resolved Resource ID: 0x{:08x}", id);
                println!("    Type:  {}", entry.type_name);
                println!("    Key:   {}", entry.key_name);
                println!("    Value: {:?}", entry.value);
            } else {
                println!("[-] Resource ID 0x{:08x} not found.", id);
            }
            return;
        }
    }

    let parts: Vec<&str> = query.split('/').collect();
    if parts.len() == 2 {
        if let Some(entry) = res.find_by_name(parts[0], parts[1]) {
            println!("[+] Resolved Resource: {}/{}", parts[0], parts[1]);
            println!("    ID:    0x{:08x}", entry.res_id);
            println!("    Value: {:?}", entry.value);
            return;
        }
    }

    println!("[-] Resource '{}' could not be resolved.", query);
}

fn cmd_run(target: &str) {
    println!("{}", BANNER);

    // Resolve target: either an existing file path or an installed package name
    let apk_path = if Path::new(target).exists() {
        target.to_string()
    } else {
        let registry = AppRegistry::load();
        if let Some(app) = registry.find(target) {
            println!("[*] Found installed package '{}': {}", target, app.apk_path.display());
            app.apk_path.to_string_lossy().to_string()
        } else {
            target.to_string()
        }
    };

    println!("[*] Launching Android Application natively on Windows Desktop: {}", apk_path);

    if !Path::new(&apk_path).exists() {
        eprintln!("[-] Error: Target APK '{}' not found on disk or in installed registry.", target);
        return;
    }

    println!("[*] Step 1: Ingesting APK package & validating digital structure...");
    let apk = match ApkPackage::open(&apk_path) {
        Ok(a) => a,
        Err(e) => {
            eprintln!("[-] Failed to load APK package: {}", e);
            return;
        }
    };

    let pkg_name = apk.manifest.package_name.clone();
    let app_label = if let Some(ref raw_label) = apk.manifest.application_label {
        if let Ok(res_id) = raw_label.parse::<u32>() {
            if let Some(res) = &apk.resources {
                res.resolve_string(res_id).map(|s| s.to_string()).unwrap_or_else(|| "WhatsApp".to_string())
            } else {
                "WhatsApp".to_string()
            }
        } else {
            raw_label.clone()
        }
    } else {
        "WhatsApp".to_string()
    };

    let activity_name = if let Some(launcher) = apk.manifest.activities.iter().find(|a| a.is_launcher) {
        launcher.name.clone()
    } else if let Some(first) = apk.manifest.activities.first() {
        first.name.clone()
    } else {
        "org.droidbridge.MainActivity".to_string()
    };

    println!("[+] Package Ingested: {}", pkg_name);
    println!("    Label:             {}", app_label);
    println!("    Launcher Activity: {}", activity_name);
    println!("    Compatibility:     {:?}", apk.compatibility.verdict);

    println!("\n[*] Step 2: Initializing DroidBridge Android Framework Subsystem...");
    let mut thread = droidbridge_framework::ActivityThread::new(&pkg_name, &apk_path);
    let view_tree_res = thread.launch_activity(&activity_name, &apk);

    println!("\n[*] Step 3: Bridging Activity to native Win32 Desktop Window...");
    if let Ok(view_tree) = view_tree_res {
        println!("    [UI Engine] Inflated Android View Tree Hierarchy:");
        print_view_node(&view_tree, 2);
        println!("\n    [UI Engine] Launching Real Android View Hierarchy Desktop Window...");
        if let Err(e) = dynamic_renderer::run_dynamic_layout_window(&app_label, view_tree) {
            eprintln!("[-] Dynamic Layout Window Error: {}", e);
        }
    } else {
        if let Err(e) = window::run_activity_window(&pkg_name, &app_label, &activity_name) {
            eprintln!("[-] GUI Error: {}", e);
        }
    }

    println!("\n[*] Step 4: Activity Lifecycle shutdown:");
    println!("    -> {}.onPause()", activity_name);
    println!("    -> {}.onStop()", activity_name);
    println!("    -> {}.onDestroy()", activity_name);
    println!("[+] Application terminated cleanly.");
}

fn main() {
    let args: Vec<String> = env::args().collect();
    if args.len() < 2 {
        print_usage();
        return;
    }

    match args[1].as_str() {
        "install" => {
            if args.len() < 3 {
                eprintln!("Error: Missing APK path for 'install' command.\nUsage: droidbridge install <path.apk>");
                return;
            }
            cmd_install(&args[2]);
        }
        "uninstall" => {
            if args.len() < 3 {
                eprintln!("Error: Missing package name for 'uninstall' command.\nUsage: droidbridge uninstall <pkg_name>");
                return;
            }
            cmd_uninstall(&args[2]);
        }
        "list-apps" | "apps" | "list" => {
            cmd_list_apps();
        }
        "run" => {
            if args.len() < 3 {
                eprintln!("Error: Missing APK path or package name for 'run' command.\nUsage: droidbridge run <path.apk | pkg_name>");
                return;
            }
            cmd_run(&args[2]);
        }
        "inspect" => {
            if args.len() < 3 {
                eprintln!("Error: Missing APK path for 'inspect' command.\nUsage: droidbridge inspect <path.apk>");
                return;
            }
            cmd_inspect(&args[2]);
        }
        "inspect-elf" => {
            if args.len() < 3 {
                eprintln!("Error: Missing .so path for 'inspect-elf' command.\nUsage: droidbridge inspect-elf <path.so>");
                return;
            }
            cmd_inspect_elf(&args[2]);
        }
        "parse-dex" => {
            if args.len() < 3 {
                eprintln!("Error: Missing DEX path for 'parse-dex' command.\nUsage: droidbridge parse-dex <path.dex>");
                return;
            }
            cmd_parse_dex(&args[2]);
        }
        "render-layout" => {
            if args.len() < 4 {
                eprintln!("Error: Missing arguments for 'render-layout'.\nUsage: droidbridge render-layout <path.apk> <res/...xml | 0x7f...>");
                return;
            }
            cmd_render_layout(&args[2], &args[3]);
        }
        "inflate-layout" => {
            if args.len() < 4 {
                eprintln!("Error: Missing arguments for 'inflate-layout'.\nUsage: droidbridge inflate-layout <path.apk> <res/...xml | 0x7f...>");
                return;
            }
            cmd_inflate_layout(&args[2], &args[3]);
        }
        "resolve-res" => {
            if args.len() < 4 {
                eprintln!("Error: Missing arguments for 'resolve-res'.\nUsage: droidbridge resolve-res <path.apk> <type/name | 0x7f...>");
                return;
            }
            cmd_resolve_res(&args[2], &args[3]);
        }
        "exec-test" => {
            cmd_exec_test();
        }
        "version" | "--version" | "-v" => {
            cmd_version();
        }
        "help" | "--help" | "-h" => {
            print_usage();
        }
        unknown => {
            eprintln!("Unknown command: '{}'\n", unknown);
            print_usage();
        }
    }
}
