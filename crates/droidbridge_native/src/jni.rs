use std::ffi::c_void;

#[repr(C)]
pub struct JNINativeInterface {
    pub reserved0: *mut c_void,
    pub reserved1: *mut c_void,
    pub reserved2: *mut c_void,
    pub reserved3: *mut c_void,

    pub get_version: unsafe extern "C" fn(env: *mut JNIEnv) -> i32,
    pub find_class: unsafe extern "C" fn(env: *mut JNIEnv, name: *const i8) -> *mut c_void,
    pub get_method_id: unsafe extern "C" fn(env: *mut JNIEnv, clazz: *mut c_void, name: *const i8, sig: *const i8) -> *mut c_void,
    pub get_string_utf_chars: unsafe extern "C" fn(env: *mut JNIEnv, string: *mut c_void, is_copy: *mut u8) -> *const i8,
    pub release_string_utf_chars: unsafe extern "C" fn(env: *mut JNIEnv, string: *mut c_void, chars: *const i8),
}

#[repr(C)]
pub struct JNIEnv {
    pub functions: *const JNINativeInterface,
}

unsafe impl Sync for JNINativeInterface {}
unsafe impl Send for JNINativeInterface {}
unsafe impl Sync for JNIEnv {}
unsafe impl Send for JNIEnv {}

unsafe extern "C" fn jni_get_version(_env: *mut JNIEnv) -> i32 {
    // Return JNI_VERSION_1_6 = 0x00010006
    0x00010006
}

unsafe extern "C" fn jni_find_class(_env: *mut JNIEnv, _name: *const i8) -> *mut c_void {
    1 as *mut c_void // Mock jclass handle
}

unsafe extern "C" fn jni_get_method_id(_env: *mut JNIEnv, _clazz: *mut c_void, _name: *const i8, _sig: *const i8) -> *mut c_void {
    1 as *mut c_void // Mock jmethodID handle
}

unsafe extern "C" fn jni_get_string_utf_chars(_env: *mut JNIEnv, _string: *mut c_void, is_copy: *mut u8) -> *const i8 {
    if !is_copy.is_null() {
        *is_copy = 1;
    }
    b"HelloFromJNI\0".as_ptr() as *const i8
}

unsafe extern "C" fn jni_release_string_utf_chars(_env: *mut JNIEnv, _string: *mut c_void, _chars: *const i8) {
    // No-op for static string
}

pub static GLOBAL_JNI_INTERFACE: JNINativeInterface = JNINativeInterface {
    reserved0: std::ptr::null_mut(),
    reserved1: std::ptr::null_mut(),
    reserved2: std::ptr::null_mut(),
    reserved3: std::ptr::null_mut(),
    get_version: jni_get_version,
    find_class: jni_find_class,
    get_method_id: jni_get_method_id,
    get_string_utf_chars: jni_get_string_utf_chars,
    release_string_utf_chars: jni_release_string_utf_chars,
};

pub static mut GLOBAL_JNI_ENV: JNIEnv = JNIEnv {
    functions: &GLOBAL_JNI_INTERFACE,
};

pub struct JniBridge;

impl JniBridge {
    pub fn get_default_env() -> *mut JNIEnv {
        &raw mut GLOBAL_JNI_ENV
    }

    /// Mangle a Java/Kotlin class and method name to standard JNI export symbol name
    /// e.g. "com/example/MainActivity" + "nativeAdd" -> "Java_com_example_MainActivity_nativeAdd"
    pub fn mangle_symbol_name(class_name: &str, method_name: &str) -> String {
        let clean_class = class_name
            .trim_start_matches('L')
            .trim_end_matches(';')
            .replace('/', "_")
            .replace('.', "_");
        format!("Java_{}_{}", clean_class, method_name)
    }
}
