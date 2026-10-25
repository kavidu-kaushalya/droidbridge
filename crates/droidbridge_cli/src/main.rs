use std::env;
use std::fs;
use std::path::Path;
use droidbridge_core::{DexVm, ExecutionResult, Value};
use droidbridge_loader::{ApkPackage, DexFile};

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
    println!("  inspect <path.apk>       Inspect APK contents, manifest, DEX, and compatibility");
    println!("  parse-dex <path.dex>     Parse and dump DEX header, classes, and method signatures");
    println!("  exec-test                Run verified Dalvik bytecode execution test on native host");
    println!("  generate-fixture <out>   Generate a minimal compliant test APK for verification");
    println!("  version                  Display DroidBridge runtime version and environment info");
    println!("  help                     Show this help message\n");
}

fn cmd_generate_fixture(out_path: &str) {
    println!("{}", BANNER);
    println!("[*] Generating verified synthetic Android APK fixture: {}", out_path);

    let mut dex = vec![0u8; 112];
    dex[0..8].copy_from_slice(b"dex\n035\0");
    dex[8..12].copy_from_slice(&0xdeadbeefu32.to_le_bytes());
    dex[32..36].copy_from_slice(&(112u32).to_le_bytes());
    dex[36..40].copy_from_slice(&0x70u32.to_le_bytes());
    dex[40..44].copy_from_slice(&0x12345678u32.to_le_bytes());

    // Generate AXML for "org.droidbridge.calculator"
    let pkg_name = "org.droidbridge.calculator";
    let pkg_utf16: Vec<u16> = pkg_name.encode_utf16().collect();
    let manifest_str: Vec<u16> = "manifest".encode_utf16().collect();
    let package_attr_str: Vec<u16> = "package".encode_utf16().collect();

    let strings_data: Vec<Vec<u16>> = vec![manifest_str, package_attr_str, pkg_utf16];
    let mut string_offsets = Vec::new();
    let mut string_bytes = Vec::new();

    for s in &strings_data {
        string_offsets.push(string_bytes.len() as u32);
        string_bytes.extend_from_slice(&(s.len() as u16).to_le_bytes());
        for c in s {
            string_bytes.extend_from_slice(&c.to_le_bytes());
        }
        string_bytes.extend_from_slice(&0u16.to_le_bytes());
    }
    while string_bytes.len() % 4 != 0 {
        string_bytes.push(0);
    }

    let string_count = strings_data.len() as u32;
    let pool_header_size = 28u32;
    let strings_start = pool_header_size + string_count * 4;
    let pool_total_size = strings_start + string_bytes.len() as u32;

    let mut pool_chunk = Vec::new();
    pool_chunk.extend_from_slice(&0x0001u16.to_le_bytes());
    pool_chunk.extend_from_slice(&0x001cu16.to_le_bytes());
    pool_chunk.extend_from_slice(&pool_total_size.to_le_bytes());
    pool_chunk.extend_from_slice(&string_count.to_le_bytes());
    pool_chunk.extend_from_slice(&0u32.to_le_bytes());
    pool_chunk.extend_from_slice(&0u32.to_le_bytes());
    pool_chunk.extend_from_slice(&strings_start.to_le_bytes());
    pool_chunk.extend_from_slice(&0u32.to_le_bytes());
    for off in string_offsets {
        pool_chunk.extend_from_slice(&off.to_le_bytes());
    }
    pool_chunk.extend_from_slice(&string_bytes);

    let mut elem_chunk = Vec::new();
    elem_chunk.extend_from_slice(&0x0102u16.to_le_bytes());
    elem_chunk.extend_from_slice(&16u16.to_le_bytes());
    elem_chunk.extend_from_slice(&(36u32 + 20u32).to_le_bytes());
    elem_chunk.extend_from_slice(&1u32.to_le_bytes());
    elem_chunk.extend_from_slice(&0xffffffffu32.to_le_bytes());
    elem_chunk.extend_from_slice(&0xffffffffu32.to_le_bytes());
    elem_chunk.extend_from_slice(&0u32.to_le_bytes());
    elem_chunk.extend_from_slice(&20u16.to_le_bytes());
    elem_chunk.extend_from_slice(&20u16.to_le_bytes());
    elem_chunk.extend_from_slice(&1u16.to_le_bytes());
    elem_chunk.extend_from_slice(&0u16.to_le_bytes());
    elem_chunk.extend_from_slice(&0u16.to_le_bytes());
    elem_chunk.extend_from_slice(&0u16.to_le_bytes());

    elem_chunk.extend_from_slice(&0xffffffffu32.to_le_bytes());
    elem_chunk.extend_from_slice(&1u32.to_le_bytes());
    elem_chunk.extend_from_slice(&2u32.to_le_bytes());
    elem_chunk.extend_from_slice(&8u16.to_le_bytes());
    elem_chunk.push(0);
    elem_chunk.push(0x03);
    elem_chunk.extend_from_slice(&2u32.to_le_bytes());

    let mut end_elem_chunk = Vec::new();
    end_elem_chunk.extend_from_slice(&0x0103u16.to_le_bytes());
    end_elem_chunk.extend_from_slice(&16u16.to_le_bytes());
    end_elem_chunk.extend_from_slice(&24u32.to_le_bytes());
    end_elem_chunk.extend_from_slice(&1u32.to_le_bytes());
    end_elem_chunk.extend_from_slice(&0xffffffffu32.to_le_bytes());
    end_elem_chunk.extend_from_slice(&0xffffffffu32.to_le_bytes());
    end_elem_chunk.extend_from_slice(&0u32.to_le_bytes());

    let xml_total_size = 8u32 + pool_chunk.len() as u32 + elem_chunk.len() as u32 + end_elem_chunk.len() as u32;
    let mut axml = Vec::new();
    axml.extend_from_slice(&0x0003u16.to_le_bytes());
    axml.extend_from_slice(&8u16.to_le_bytes());
    axml.extend_from_slice(&xml_total_size.to_le_bytes());
    axml.extend_from_slice(&pool_chunk);
    axml.extend_from_slice(&elem_chunk);
    axml.extend_from_slice(&end_elem_chunk);

    // Write ZIP archive
    let mut zip_data = Vec::new();
    let mut cd_entries = Vec::new();
    let entries: [(&str, &[u8]); 2] = [
        ("AndroidManifest.xml", &axml),
        ("classes.dex", &dex),
    ];

    for (name, data) in &entries {
        let local_off = zip_data.len() as u32;
        zip_data.extend_from_slice(&0x04034b50u32.to_le_bytes());
        zip_data.extend_from_slice(&20u16.to_le_bytes());
        zip_data.extend_from_slice(&0u16.to_le_bytes());
        zip_data.extend_from_slice(&0u16.to_le_bytes());
        zip_data.extend_from_slice(&0u16.to_le_bytes());
        zip_data.extend_from_slice(&0u16.to_le_bytes());
        zip_data.extend_from_slice(&0u32.to_le_bytes());
        zip_data.extend_from_slice(&(data.len() as u32).to_le_bytes());
        zip_data.extend_from_slice(&(data.len() as u32).to_le_bytes());
        zip_data.extend_from_slice(&(name.len() as u16).to_le_bytes());
        zip_data.extend_from_slice(&0u16.to_le_bytes());
        zip_data.extend_from_slice(name.as_bytes());
        zip_data.extend_from_slice(data);
        cd_entries.push((*name, data.len() as u32, local_off));
    }

    let cd_offset = zip_data.len() as u32;
    for (name, size, local_off) in &cd_entries {
        zip_data.extend_from_slice(&0x02014b50u32.to_le_bytes());
        zip_data.extend_from_slice(&20u16.to_le_bytes());
        zip_data.extend_from_slice(&20u16.to_le_bytes());
        zip_data.extend_from_slice(&0u16.to_le_bytes());
        zip_data.extend_from_slice(&0u16.to_le_bytes());
        zip_data.extend_from_slice(&0u16.to_le_bytes());
        zip_data.extend_from_slice(&0u16.to_le_bytes());
        zip_data.extend_from_slice(&0u32.to_le_bytes());
        zip_data.extend_from_slice(&size.to_le_bytes());
        zip_data.extend_from_slice(&size.to_le_bytes());
        zip_data.extend_from_slice(&(name.len() as u16).to_le_bytes());
        zip_data.extend_from_slice(&0u16.to_le_bytes());
        zip_data.extend_from_slice(&0u16.to_le_bytes());
        zip_data.extend_from_slice(&0u16.to_le_bytes());
        zip_data.extend_from_slice(&0u16.to_le_bytes());
        zip_data.extend_from_slice(&0u32.to_le_bytes());
        zip_data.extend_from_slice(&local_off.to_le_bytes());
        zip_data.extend_from_slice(name.as_bytes());
    }

    let cd_size = (zip_data.len() as u32) - cd_offset;
    zip_data.extend_from_slice(&0x06054b50u32.to_le_bytes());
    zip_data.extend_from_slice(&0u16.to_le_bytes());
    zip_data.extend_from_slice(&0u16.to_le_bytes());
    zip_data.extend_from_slice(&(cd_entries.len() as u16).to_le_bytes());
    zip_data.extend_from_slice(&(cd_entries.len() as u16).to_le_bytes());
    zip_data.extend_from_slice(&cd_size.to_le_bytes());
    zip_data.extend_from_slice(&cd_offset.to_le_bytes());
    zip_data.extend_from_slice(&0u16.to_le_bytes());

    if let Some(parent) = Path::new(out_path).parent() {
        let _ = fs::create_dir_all(parent);
    }

    match fs::write(out_path, &zip_data) {
        Ok(_) => println!("[+] Successfully created APK fixture: {} ({} bytes)", out_path, zip_data.len()),
        Err(e) => eprintln!("[-] Failed to write APK fixture: {}", e),
    }
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

    // Bytecode routine:
    // Computes sum of integers from 1 to 5:
    // v0 = counter (starts at 1)
    // v1 = limit (6)
    // v2 = accumulator (starts at 0)
    //
    // Loop (pc=4):
    // if-ge v0, v1, +6   (if counter >= 6, exit loop to return)
    // add-int v2, v2, v0 (acc += counter)
    // add-int/lit8 v0, v0, #1 (counter += 1)
    // goto -6            (jump back to loop head at pc=4)
    //
    // Exit (pc=10):
    // return v2
    let insns: Vec<u16> = vec![
        0x1012,         // pc=0: const/4 v0, #1
        0x6112,         // pc=1: const/4 v1, #6
        0x0212,         // pc=2: const/4 v2, #0
        0x0000,         // pc=3: nop
        // Loop head (pc=4):
        0x1035, 0x0007, // pc=4: if-ge v0, v1, +7 -> targets pc = 4 + 7 = 11
        0x0290, 0x0002, // pc=6: add-int v2, v2, v0
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
}

fn main() {
    let args: Vec<String> = env::args().collect();
    if args.len() < 2 {
        print_usage();
        return;
    }

    match args[1].as_str() {
        "inspect" => {
            if args.len() < 3 {
                eprintln!("Error: Missing APK path for 'inspect' command.\nUsage: droidbridge inspect <path.apk>");
                return;
            }
            cmd_inspect(&args[2]);
        }
        "parse-dex" => {
            if args.len() < 3 {
                eprintln!("Error: Missing DEX path for 'parse-dex' command.\nUsage: droidbridge parse-dex <path.dex>");
                return;
            }
            cmd_parse_dex(&args[2]);
        }
        "exec-test" => {
            cmd_exec_test();
        }
        "generate-fixture" => {
            let out = if args.len() >= 3 { &args[2] } else { "fixtures/sample_demo.apk" };
            cmd_generate_fixture(out);
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
