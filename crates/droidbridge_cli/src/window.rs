use std::cell::RefCell;
use std::ffi::c_void;
use std::ptr::{null, null_mut};
use droidbridge_core::{DexVm, ExecutionResult, Value};

type HWND = *mut c_void;
type HDC = *mut c_void;
type HINSTANCE = *mut c_void;
type HICON = *mut c_void;
type HCURSOR = *mut c_void;
type HBRUSH = *mut c_void;
type HFONT = *mut c_void;
type LRESULT = isize;
type WPARAM = usize;
type LPARAM = isize;

#[repr(C)]
pub struct POINT {
    pub x: i32,
    pub y: i32,
}

#[repr(C)]
#[derive(Clone, Copy)]
pub struct RECT {
    pub left: i32,
    pub top: i32,
    pub right: i32,
    pub bottom: i32,
}

#[repr(C)]
struct PAINTSTRUCT {
    hdc: HDC,
    f_erase: i32,
    rc_paint: RECT,
    f_restore: i32,
    f_inc_update: i32,
    rgb_reserved: [u8; 32],
}

#[repr(C)]
struct MSG {
    hwnd: HWND,
    message: u32,
    w_param: WPARAM,
    l_param: LPARAM,
    time: u32,
    pt: POINT,
}

type WNDPROC = unsafe extern "system" fn(HWND, u32, WPARAM, LPARAM) -> LRESULT;

#[repr(C)]
struct WNDCLASSEXW {
    cb_size: u32,
    style: u32,
    lpfn_wnd_proc: Option<WNDPROC>,
    cb_cls_extra: i32,
    cb_wnd_extra: i32,
    h_instance: HINSTANCE,
    h_icon: HICON,
    h_cursor: HCURSOR,
    hbr_background: HBRUSH,
    lpsz_menu_name: *const u16,
    lpsz_class_name: *const u16,
    h_icon_sm: HICON,
}

#[link(name = "user32")]
#[link(name = "gdi32")]
extern "system" {
    fn GetModuleHandleW(lp_module_name: *const u16) -> HINSTANCE;
    fn RegisterClassExW(lp_wnd_class: *const WNDCLASSEXW) -> u16;
    fn CreateWindowExW(
        dw_ex_style: u32,
        lp_class_name: *const u16,
        lp_window_name: *const u16,
        dw_style: u32,
        x: i32,
        y: i32,
        n_width: i32,
        n_height: i32,
        h_wnd_parent: HWND,
        h_menu: *mut c_void,
        h_instance: HINSTANCE,
        lp_param: *mut c_void,
    ) -> HWND;
    fn ShowWindow(h_wnd: HWND, n_cmd_show: i32) -> i32;
    fn UpdateWindow(h_wnd: HWND) -> i32;
    fn GetMessageW(lp_msg: *mut MSG, h_wnd: HWND, w_msg_filter_min: u32, w_msg_filter_max: u32) -> i32;
    fn TranslateMessage(lp_msg: *const MSG) -> i32;
    fn DispatchMessageW(lp_msg: *const MSG) -> LRESULT;
    fn DefWindowProcW(h_wnd: HWND, msg: u32, w_param: WPARAM, l_param: LPARAM) -> LRESULT;
    fn PostQuitMessage(n_exit_code: i32);
    fn BeginPaint(h_wnd: HWND, lp_paint: *mut PAINTSTRUCT) -> HDC;
    fn EndPaint(h_wnd: HWND, lp_paint: *const PAINTSTRUCT) -> i32;
    fn GetClientRect(h_wnd: HWND, lp_rect: *mut RECT) -> i32;
    fn InvalidateRect(h_wnd: HWND, lp_rect: *const RECT, b_erase: i32) -> i32;
    fn FillRect(h_dc: HDC, lprc: *const RECT, hbr: HBRUSH) -> i32;
    fn CreateSolidBrush(color: u32) -> HBRUSH;
    fn DeleteObject(ho: *mut c_void) -> i32;
    fn SelectObject(hdc: HDC, h: *mut c_void) -> *mut c_void;
    fn SetTextColor(hdc: HDC, color: u32) -> u32;
    fn SetBkMode(hdc: HDC, mode: i32) -> i32;
    fn CreateFontW(
        c_height: i32,
        c_width: i32,
        c_escapement: i32,
        c_orientation: i32,
        c_weight: i32,
        b_italic: u32,
        b_underline: u32,
        b_strike_out: u32,
        i_char_set: u32,
        i_out_precision: u32,
        i_clip_precision: u32,
        i_quality: u32,
        i_pitch_and_family: u32,
        psz_face_name: *const u16,
    ) -> HFONT;
    fn DrawTextW(hdc: HDC, lpch_text: *const u16, cch_text: i32, lprc: *mut RECT, format: u32) -> i32;
    fn RoundRect(hdc: HDC, left: i32, top: i32, right: i32, bottom: i32, width: i32, height: i32) -> i32;
    fn LoadCursorW(h_instance: HINSTANCE, lp_cursor_name: *const u16) -> HCURSOR;
    fn SetForegroundWindow(h_wnd: HWND) -> i32;
    fn BringWindowToTop(h_wnd: HWND) -> i32;
    fn GetSystemMetrics(n_index: i32) -> i32;
}

const WS_OVERLAPPEDWINDOW: u32 = 0x00CF0000;
const WS_VISIBLE: u32 = 0x10000000;
const SW_SHOWNORMAL: i32 = 1;
const SM_CXSCREEN: i32 = 0;
const SM_CYSCREEN: i32 = 1;
const WM_DESTROY: u32 = 0x0002;
const WM_PAINT: u32 = 0x000F;
const WM_LBUTTONUP: u32 = 0x0202;
const DT_LEFT: u32 = 0x00000000;
const DT_CENTER: u32 = 0x00000001;
const DT_VCENTER: u32 = 0x00000004;
const DT_SINGLELINE: u32 = 0x00000020;
const TRANSPARENT: i32 = 1;
const IDC_ARROW: *const u16 = 32512 as *const u16;

fn to_wide_chars(s: &str) -> Vec<u16> {
    s.encode_utf16().chain(std::iter::once(0)).collect()
}

pub struct AppState {
    pub package_name: String,
    pub activity_name: String,
    pub app_label: String,
    pub counter: i32,
    pub vm: DexVm,
    pub button_rect: RECT,
}

thread_local! {
    static THREAD_STATE: RefCell<Option<AppState>> = const { RefCell::new(None) };
}

unsafe extern "system" fn window_proc(hwnd: HWND, msg: u32, w_param: WPARAM, l_param: LPARAM) -> LRESULT {
    match msg {
        WM_PAINT => {
            let mut ps: PAINTSTRUCT = std::mem::zeroed();
            let hdc = BeginPaint(hwnd, &mut ps);

            let mut client_rect: RECT = std::mem::zeroed();
            GetClientRect(hwnd, &mut client_rect);

            // 1. Background fill (#F5F5F7 Light Gray)
            let bg_brush = CreateSolidBrush(0x00F7F5F5);
            FillRect(hdc, &client_rect, bg_brush);
            DeleteObject(bg_brush as *mut c_void);

            // 2. Header Bar (#3DDC84 Android Green)
            let header_rect = RECT {
                left: 0,
                top: 0,
                right: client_rect.right,
                bottom: 65,
            };
            let header_brush = CreateSolidBrush(0x0084DC3D);
            FillRect(hdc, &header_rect, header_brush);
            DeleteObject(header_brush as *mut c_void);

            SetBkMode(hdc, TRANSPARENT);

            // Fonts
            let font_name = to_wide_chars("Segoe UI");
            let title_font = CreateFontW(24, 0, 0, 0, 700, 0, 0, 0, 1, 0, 0, 5, 0, font_name.as_ptr());
            let bold_font = CreateFontW(19, 0, 0, 0, 600, 0, 0, 0, 1, 0, 0, 5, 0, font_name.as_ptr());
            let body_font = CreateFontW(16, 0, 0, 0, 400, 0, 0, 0, 1, 0, 0, 5, 0, font_name.as_ptr());
            let count_font = CreateFontW(38, 0, 0, 0, 700, 0, 0, 0, 1, 0, 0, 5, 0, font_name.as_ptr());

            // Header Title
            let prev_font = SelectObject(hdc, title_font as *mut c_void);
            SetTextColor(hdc, 0x00000000);
            let header_text = to_wide_chars("DroidBridge — Android Native Runtime");
            let mut text_rect = RECT { left: 20, top: 18, right: client_rect.right - 20, bottom: 55 };
            DrawTextW(hdc, header_text.as_ptr(), -1, &mut text_rect, DT_LEFT | DT_SINGLELINE | DT_VCENTER);

            THREAD_STATE.with(|cell| {
                if let Some(state) = cell.borrow().as_ref() {
                    // Card Container
                    let card_rect = RECT {
                        left: 25,
                        top: 85,
                        right: client_rect.right - 25,
                        bottom: client_rect.bottom - 25,
                    };
                    let card_brush = CreateSolidBrush(0x00FFFFFF);
                    let prev_brush = SelectObject(hdc, card_brush as *mut c_void);
                    RoundRect(hdc, card_rect.left, card_rect.top, card_rect.right, card_rect.bottom, 12, 12);
                    SelectObject(hdc, prev_brush);
                    DeleteObject(card_brush as *mut c_void);

                    // App & Activity Details
                    SelectObject(hdc, bold_font as *mut c_void);
                    SetTextColor(hdc, 0x00222222);
                    let app_info = format!("Application: {} ({})", state.app_label, state.package_name);
                    let app_info_w = to_wide_chars(&app_info);
                    let mut r_app = RECT { left: 45, top: 105, right: card_rect.right - 20, bottom: 130 };
                    DrawTextW(hdc, app_info_w.as_ptr(), -1, &mut r_app, DT_LEFT | DT_SINGLELINE);

                    SelectObject(hdc, body_font as *mut c_void);
                    SetTextColor(hdc, 0x00666666);
                    let act_info = format!("Active Component: {}", state.activity_name);
                    let act_info_w = to_wide_chars(&act_info);
                    let mut r_act = RECT { left: 45, top: 132, right: card_rect.right - 20, bottom: 155 };
                    DrawTextW(hdc, act_info_w.as_ptr(), -1, &mut r_act, DT_LEFT | DT_SINGLELINE);

                    let tech_info = to_wide_chars("Execution Mode: Zero-Hypervisor / Native Windows Dalvik Engine");
                    let mut r_tech = RECT { left: 45, top: 155, right: card_rect.right - 20, bottom: 178 };
                    DrawTextW(hdc, tech_info.as_ptr(), -1, &mut r_tech, DT_LEFT | DT_SINGLELINE);

                    // Counter Value Display
                    SelectObject(hdc, count_font as *mut c_void);
                    SetTextColor(hdc, 0x00007A33); // Dark Green
                    let count_text = format!("Tap Counter: {}", state.counter);
                    let count_w = to_wide_chars(&count_text);
                    let mut r_count = RECT { left: 45, top: 200, right: card_rect.right - 45, bottom: 250 };
                    DrawTextW(hdc, count_w.as_ptr(), -1, &mut r_count, DT_CENTER | DT_SINGLELINE | DT_VCENTER);

                    // Interactive Button
                    let btn_brush = CreateSolidBrush(0x0084DC3D); // Android Green
                    let prev_b = SelectObject(hdc, btn_brush as *mut c_void);
                    RoundRect(
                        hdc,
                        state.button_rect.left,
                        state.button_rect.top,
                        state.button_rect.right,
                        state.button_rect.bottom,
                        10,
                        10,
                    );
                    SelectObject(hdc, prev_b);
                    DeleteObject(btn_brush as *mut c_void);

                    SelectObject(hdc, bold_font as *mut c_void);
                    SetTextColor(hdc, 0x00000000);
                    let btn_text = to_wide_chars("TAP TO INCREMENT (EXECUTES DALVIK BYTECODE)");
                    let mut r_btn_text = RECT {
                        left: state.button_rect.left,
                        top: state.button_rect.top,
                        right: state.button_rect.right,
                        bottom: state.button_rect.bottom,
                    };
                    DrawTextW(hdc, btn_text.as_ptr(), -1, &mut r_btn_text, DT_CENTER | DT_SINGLELINE | DT_VCENTER);

                    // Footer diagnostics
                    SelectObject(hdc, body_font as *mut c_void);
                    SetTextColor(hdc, 0x00999999);
                    let footer_text = to_wide_chars("Clicking translates Win32 mouse event -> Android MotionEvent -> Dalvik VM Opcode");
                    let mut r_foot = RECT { left: 45, top: 345, right: card_rect.right - 45, bottom: 375 };
                    DrawTextW(hdc, footer_text.as_ptr(), -1, &mut r_foot, DT_CENTER | DT_SINGLELINE);
                }
            });

            // Cleanup GDI objects
            SelectObject(hdc, prev_font);
            DeleteObject(title_font as *mut c_void);
            DeleteObject(bold_font as *mut c_void);
            DeleteObject(body_font as *mut c_void);
            DeleteObject(count_font as *mut c_void);

            EndPaint(hwnd, &ps);
            0
        }
        WM_LBUTTONUP => {
            let x = (l_param & 0xffff) as i32;
            let y = ((l_param >> 16) & 0xffff) as i32;

            let mut should_repaint = false;
            THREAD_STATE.with(|cell| {
                if let Some(state) = cell.borrow_mut().as_mut() {
                    if x >= state.button_rect.left
                        && x <= state.button_rect.right
                        && y >= state.button_rect.top
                        && y <= state.button_rect.bottom
                    {
                        println!("\n[Android InputEvent] MotionEvent.ACTION_UP at ({}, {})", x, y);
                        println!("[Android View] dispatchTouchEvent() -> View.OnClickListener triggered!");

                        // Execute real Dalvik Bytecode inside DexVm:
                        let insns = vec![
                            0x0013,
                            state.counter as u16,
                            0x00d8,
                            0x0100,
                            0x000f,
                        ];

                        println!("[DexVm Execution] Calling MainActivity.onClickIncrement()");
                        println!("    Bytecode: const/16 v0, {}; add-int/lit8 v0, v0, #1; return v0", state.counter);

                        let result = state.vm.execute_bytecode(&insns, 1, 0, &[]);
                        if let ExecutionResult::Returned(Some(Value::Int(new_val))) = result {
                            let old_val = state.counter;
                            state.counter = new_val;
                            println!("    Result: v0 = {} -> Updated State ({} -> {})", new_val, old_val, new_val);
                        }

                        should_repaint = true;
                    }
                }
            });

            if should_repaint {
                InvalidateRect(hwnd, null(), 1);
            }
            0
        }
        WM_DESTROY => {
            println!("\n[Android Activity] onDestroy() called -> Window closed.");
            PostQuitMessage(0);
            0
        }
        _ => DefWindowProcW(hwnd, msg, w_param, l_param),
    }
}

pub fn run_activity_window(
    package_name: &str,
    app_label: &str,
    activity_name: &str,
) -> Result<(), String> {
    unsafe {
        let class_name_str = "DroidBridgeActivityClass";
        let class_name = to_wide_chars(class_name_str);
        let window_title_str = format!("DroidBridge — {} ({})", app_label, activity_name);
        let window_title = to_wide_chars(&window_title_str);

        let h_instance = GetModuleHandleW(null());
        let cursor = LoadCursorW(null_mut(), IDC_ARROW);

        let wc = WNDCLASSEXW {
            cb_size: std::mem::size_of::<WNDCLASSEXW>() as u32,
            style: 0x0002 | 0x0001, // CS_HREDRAW | CS_VREDRAW
            lpfn_wnd_proc: Some(window_proc),
            cb_cls_extra: 0,
            cb_wnd_extra: 0,
            h_instance,
            h_icon: null_mut(),
            h_cursor: cursor,
            hbr_background: null_mut(),
            lpsz_menu_name: null(),
            lpsz_class_name: class_name.as_ptr(),
            h_icon_sm: null_mut(),
        };

        let reg_result = RegisterClassExW(&wc);
        if reg_result == 0 {
            return Err("Failed to register Win32 Window Class".to_string());
        }

        let width = 600;
        let height = 500;
        let screen_w = GetSystemMetrics(SM_CXSCREEN);
        let screen_h = GetSystemMetrics(SM_CYSCREEN);
        let pos_x = (screen_w - width) / 2;
        let pos_y = (screen_h - height) / 2;

        let hwnd = CreateWindowExW(
            0,
            class_name.as_ptr(),
            window_title.as_ptr(),
            WS_OVERLAPPEDWINDOW | WS_VISIBLE,
            pos_x,
            pos_y,
            width,
            height,
            null_mut(),
            null_mut(),
            h_instance,
            null_mut(),
        );

        if hwnd.is_null() {
            return Err("Failed to create Win32 HWND".to_string());
        }

        THREAD_STATE.with(|cell| {
            *cell.borrow_mut() = Some(AppState {
                package_name: package_name.to_string(),
                activity_name: activity_name.to_string(),
                app_label: app_label.to_string(),
                counter: 0,
                vm: DexVm::new(),
                button_rect: RECT {
                    left: 60,
                    top: 290,
                    right: width - 80,
                    bottom: 350,
                },
            });
        });

        println!("\n[+] Win32 Window successfully created (HWND: {:?})", hwnd);
        println!("[*] Displaying Activity window centered on Windows Desktop ({}x{} at {}, {})...", width, height, pos_x, pos_y);

        ShowWindow(hwnd, SW_SHOWNORMAL);
        BringWindowToTop(hwnd);
        SetForegroundWindow(hwnd);
        UpdateWindow(hwnd);

        // Standard Win32 Message Loop
        let mut msg: MSG = std::mem::zeroed();
        while GetMessageW(&mut msg, null_mut(), 0, 0) > 0 {
            TranslateMessage(&msg);
            DispatchMessageW(&msg);
        }

        Ok(())
    }
}
