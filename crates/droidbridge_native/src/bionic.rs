use std::collections::HashMap;
use std::ffi::{c_char, c_void, CStr};

pub type NativeFunction = unsafe extern "C" fn() -> isize;

// Bionic liblog shim:
// int __android_log_print(int prio, const char *tag, const char *fmt, ...)
pub unsafe extern "C" fn shim_android_log_print(
    prio: i32,
    tag: *const c_char,
    fmt: *const c_char,
) -> i32 {
    let tag_str = if !tag.is_null() {
        CStr::from_ptr(tag).to_string_lossy()
    } else {
        "AndroidNative".into()
    };
    let fmt_str = if !fmt.is_null() {
        CStr::from_ptr(fmt).to_string_lossy()
    } else {
        "".into()
    };
    println!("[Bionic liblog prio={}] {}: {}", prio, tag_str, fmt_str);
    0
}

pub unsafe extern "C" fn shim_android_log_write(
    prio: i32,
    tag: *const c_char,
    text: *const c_char,
) -> i32 {
    shim_android_log_print(prio, tag, text)
}

// Bionic memory shims:
pub unsafe extern "C" fn shim_malloc(size: usize) -> *mut c_void {
    let layout = std::alloc::Layout::from_size_align(size, 8).unwrap_or(std::alloc::Layout::new::<u8>());
    std::alloc::alloc(layout) as *mut c_void
}

pub unsafe extern "C" fn shim_free(ptr: *mut c_void) {
    if !ptr.is_null() {
        // Safe dealloc for standard dummy allocation
        let layout = std::alloc::Layout::new::<u8>();
        std::alloc::dealloc(ptr as *mut u8, layout);
    }
}

pub struct BionicSymbolTable {
    symbols: HashMap<String, usize>,
}

impl Default for BionicSymbolTable {
    fn default() -> Self {
        let mut table = Self {
            symbols: HashMap::new(),
        };
        table.register("__android_log_print", shim_android_log_print as *const () as usize);
        table.register("__android_log_write", shim_android_log_write as *const () as usize);
        table.register("malloc", shim_malloc as *const () as usize);
        table.register("free", shim_free as *const () as usize);
        table
    }
}

impl BionicSymbolTable {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn register(&mut self, symbol_name: &str, addr: usize) {
        self.symbols.insert(symbol_name.to_string(), addr);
    }

    pub fn resolve(&self, symbol_name: &str) -> Option<usize> {
        self.symbols.get(symbol_name).copied()
    }
}
