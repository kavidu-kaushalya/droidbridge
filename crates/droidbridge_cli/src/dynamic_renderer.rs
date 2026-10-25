use std::cell::RefCell;
use std::ffi::c_void;
use std::ptr::{null, null_mut};
use droidbridge_core::DexVm;
use droidbridge_framework::{LayoutInflater, Rect, ViewNode, ViewType};

type HWND = *mut c_void;
type HDC = *mut c_void;
type HINSTANCE = *mut c_void;
type HCURSOR = *mut c_void;
type HBRUSH = *mut c_void;
type HFONT = *mut c_void;
type LRESULT = isize;
type WPARAM = usize;
type LPARAM = isize;

#[repr(C)]
#[derive(Clone, Copy)]
pub struct WinRect {
    pub left: i32,
    pub top: i32,
    pub right: i32,
    pub bottom: i32,
}

#[repr(C)]
struct PAINTSTRUCT {
    hdc: HDC,
    f_erase: i32,
    rc_paint: WinRect,
    f_restore: i32,
    f_inc_update: i32,
    rgb_reserved: [u8; 32],
}

#[repr(C)]
struct POINT {
    x: i32,
    y: i32,
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
    h_icon: *mut c_void,
    h_cursor: HCURSOR,
    hbr_background: HBRUSH,
    lpsz_menu_name: *const u16,
    lpsz_class_name: *const u16,
    h_icon_sm: *mut c_void,
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
    fn GetClientRect(h_wnd: HWND, lp_rect: *mut WinRect) -> i32;
    fn InvalidateRect(h_wnd: HWND, lp_rect: *const WinRect, b_erase: i32) -> i32;
    fn FillRect(h_dc: HDC, lprc: *const WinRect, hbr: HBRUSH) -> i32;
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
    fn DrawTextW(hdc: HDC, lpch_text: *const u16, cch_text: i32, lprc: *mut WinRect, format: u32) -> i32;
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
const WM_SIZE: u32 = 0x0005;
const WM_LBUTTONUP: u32 = 0x0202;
const DT_LEFT: u32 = 0x00000000;
const DT_CENTER: u32 = 0x00000001;
const DT_VCENTER: u32 = 0x00000004;
const DT_SINGLELINE: u32 = 0x00000020;
const DT_WORDBREAK: u32 = 0x00000010;
const TRANSPARENT: i32 = 1;
const IDC_ARROW: *const u16 = 32512 as *const u16;

fn to_wide_chars(s: &str) -> Vec<u16> {
    s.encode_utf16().chain(std::iter::once(0)).collect()
}

fn rgb_to_bgr(rgb: u32) -> u32 {
    let r = (rgb >> 16) & 0xff;
    let g = (rgb >> 8) & 0xff;
    let b = rgb & 0xff;
    (b << 16) | (g << 8) | r
}

struct DynamicRenderState {
    app_title: String,
    root_view: ViewNode,
    last_event: String,
    vm: DexVm,
}

thread_local! {
    static RENDERER_STATE: RefCell<Option<DynamicRenderState>> = RefCell::new(None);
}

pub fn run_dynamic_layout_window(app_title: &str, mut root_view: ViewNode) -> Result<(), String> {
    unsafe {
        let class_name = to_wide_chars("DroidBridgeDynamicViewWindow");
        let window_title = to_wide_chars(&format!("{} — DroidBridge Dynamic Layout Renderer", app_title));
        let h_instance = GetModuleHandleW(null());

        let wnd_class = WNDCLASSEXW {
            cb_size: std::mem::size_of::<WNDCLASSEXW>() as u32,
            style: 0,
            lpfn_wnd_proc: Some(window_proc),
            cb_cls_extra: 0,
            cb_wnd_extra: 0,
            h_instance,
            h_icon: null_mut(),
            h_cursor: LoadCursorW(null_mut(), IDC_ARROW),
            hbr_background: CreateSolidBrush(0x00221B12), // Android dark #121B22
            lpsz_menu_name: null(),
            lpsz_class_name: class_name.as_ptr(),
            h_icon_sm: null_mut(),
        };

        RegisterClassExW(&wnd_class);

        let screen_w = GetSystemMetrics(SM_CXSCREEN);
        let screen_h = GetSystemMetrics(SM_CYSCREEN);
        let width = 450;
        let height = 750;
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
            return Err("Failed to create Win32 HWND for Dynamic Layout".to_string());
        }

        // Initial layout pass
        LayoutInflater::measure_and_layout(
            &mut root_view,
            Rect { left: 16, top: 65, right: width - 36, bottom: height - 120 },
        );

        RENDERER_STATE.with(|cell| {
            *cell.borrow_mut() = Some(DynamicRenderState {
                app_title: app_title.to_string(),
                root_view,
                last_event: "Initialized Android View Hierarchy".to_string(),
                vm: DexVm::new(),
            });
        });

        println!("\n[+] Dynamic Android Layout Window created (HWND: {:?})", hwnd);
        println!("[*] Window Centered: {}x{} at ({}, {})", width, height, pos_x, pos_y);

        ShowWindow(hwnd, SW_SHOWNORMAL);
        BringWindowToTop(hwnd);
        SetForegroundWindow(hwnd);
        UpdateWindow(hwnd);

        let mut msg: MSG = std::mem::zeroed();
        while GetMessageW(&mut msg, null_mut(), 0, 0) > 0 {
            TranslateMessage(&msg);
            DispatchMessageW(&msg);
        }

        Ok(())
    }
}

unsafe extern "system" fn window_proc(hwnd: HWND, msg: u32, wparam: WPARAM, lparam: LPARAM) -> LRESULT {
    match msg {
        WM_SIZE => {
            let width = (lparam & 0xffff) as i32;
            let height = ((lparam >> 16) & 0xffff) as i32;

            RENDERER_STATE.with(|cell| {
                if let Some(state) = cell.borrow_mut().as_mut() {
                    LayoutInflater::measure_and_layout(
                        &mut state.root_view,
                        Rect { left: 16, top: 65, right: width - 16, bottom: height - 60 },
                    );
                }
            });

            let client_rect = WinRect { left: 0, top: 0, right: width, bottom: height };
            InvalidateRect(hwnd, &client_rect, 1);
            0
        }
        WM_PAINT => {
            let mut ps: PAINTSTRUCT = std::mem::zeroed();
            let hdc = BeginPaint(hwnd, &mut ps);

            let mut client_rect: WinRect = std::mem::zeroed();
            GetClientRect(hwnd, &mut client_rect);

            // 1. Paint background (#121b22)
            let bg_brush = CreateSolidBrush(0x00221B12);
            FillRect(hdc, &client_rect, bg_brush);
            DeleteObject(bg_brush as *mut c_void);

            RENDERER_STATE.with(|cell| {
                if let Some(state) = cell.borrow().as_ref() {
                    // 2. Paint Header App Bar
                    let header_rect = WinRect {
                        left: 0,
                        top: 0,
                        right: client_rect.right,
                        bottom: 50,
                    };
                    let header_brush = CreateSolidBrush(0x00545E07); // WhatsApp Header Green #075E54
                    FillRect(hdc, &header_rect, header_brush);
                    DeleteObject(header_brush as *mut c_void);

                    SetBkMode(hdc, TRANSPARENT);
                    SetTextColor(hdc, 0x00ffffff);

                    let header_font = CreateFontW(
                        22, 0, 0, 0, 700, 0, 0, 0, 1, 0, 0, 2, 0,
                        to_wide_chars("Segoe UI").as_ptr(),
                    );
                    SelectObject(hdc, header_font as *mut c_void);

                    let mut title_r = WinRect {
                        left: 20,
                        top: 0,
                        right: client_rect.right - 20,
                        bottom: 50,
                    };
                    let title_wide = to_wide_chars(&state.app_title);
                    DrawTextW(hdc, title_wide.as_ptr(), -1, &mut title_r, DT_LEFT | DT_VCENTER | DT_SINGLELINE);
                    DeleteObject(header_font as *mut c_void);

                    // 3. Render Inflated Android View Tree
                    render_view_node(hdc, &state.root_view);

                    // 4. Render Bottom Status Bar
                    let status_rect = WinRect {
                        left: 0,
                        top: client_rect.bottom - 40,
                        right: client_rect.right,
                        bottom: client_rect.bottom,
                    };
                    let status_brush = CreateSolidBrush(0x0015110B); // #0B1115
                    FillRect(hdc, &status_rect, status_brush);
                    DeleteObject(status_brush as *mut c_void);

                    let status_font = CreateFontW(
                        14, 0, 0, 0, 400, 0, 0, 0, 1, 0, 0, 2, 0,
                        to_wide_chars("Consolas").as_ptr(),
                    );
                    SelectObject(hdc, status_font as *mut c_void);
                    SetTextColor(hdc, 0x0078DE25); // Android green #25DE78

                    let mut stat_r = WinRect {
                        left: 15,
                        top: client_rect.bottom - 40,
                        right: client_rect.right - 15,
                        bottom: client_rect.bottom,
                    };
                    let status_wide = to_wide_chars(&format!("• [Native Engine] {}", state.last_event));
                    DrawTextW(hdc, status_wide.as_ptr(), -1, &mut stat_r, DT_LEFT | DT_VCENTER | DT_SINGLELINE);
                    DeleteObject(status_font as *mut c_void);
                }
            });

            EndPaint(hwnd, &ps);
            0
        }
        WM_LBUTTONUP => {
            let x = (lparam & 0xffff) as i32;
            let y = ((lparam >> 16) & 0xffff) as i32;

            RENDERER_STATE.with(|cell| {
                if let Some(state) = cell.borrow_mut().as_mut() {
                    if let Some((tag, text)) = hit_test_node(&state.root_view, x, y) {
                        println!("[+] Android View Clicked: <{}> \"{}\"", tag, text);
                        state.last_event = format!("Clicked <{}> \"{}\"", tag, text);

                        // Execute Dalvik VM bytecode instructions upon interaction
                        let bytecode = [
                            0x1012, // const/4 v0, #1
                            0x0090, // add-int/2addr v0, v0
                            0x000f, // return v0
                        ];
                        let res = state.vm.execute_bytecode(&bytecode, 2, 0, &[]);
                        println!("[DexVm] Executed Dalvik UI event handler bytecode: {:?}", res);
                    }
                }
            });

            let mut client_rect: WinRect = std::mem::zeroed();
            GetClientRect(hwnd, &mut client_rect);
            InvalidateRect(hwnd, &client_rect, 1);
            0
        }
        WM_DESTROY => {
            PostQuitMessage(0);
            0
        }
        _ => DefWindowProcW(hwnd, msg, wparam, lparam),
    }
}

unsafe fn render_view_node(hdc: HDC, node: &ViewNode) {
    let r = WinRect {
        left: node.bounds.left,
        top: node.bounds.top,
        right: node.bounds.right,
        bottom: node.bounds.bottom,
    };

    match &node.view_type {
        ViewType::LinearLayout { .. } | ViewType::FrameLayout | ViewType::RelativeLayout => {
            // Container - render children
            for child in &node.children {
                render_view_node(hdc, child);
            }
        }
        ViewType::Button { text, bg_color, text_color } => {
            let btn_bgr = rgb_to_bgr(*bg_color);
            let brush = CreateSolidBrush(btn_bgr);
            SelectObject(hdc, brush as *mut c_void);
            RoundRect(hdc, r.left, r.top, r.right, r.bottom, 12, 12);
            DeleteObject(brush as *mut c_void);

            let font = CreateFontW(
                18, 0, 0, 0, 700, 0, 0, 0, 1, 0, 0, 2, 0,
                to_wide_chars("Segoe UI").as_ptr(),
            );
            SelectObject(hdc, font as *mut c_void);
            SetTextColor(hdc, rgb_to_bgr(*text_color));

            let mut tr = r;
            let text_wide = to_wide_chars(if text.is_empty() { "Button" } else { text });
            DrawTextW(hdc, text_wide.as_ptr(), -1, &mut tr, DT_CENTER | DT_VCENTER | DT_SINGLELINE);
            DeleteObject(font as *mut c_void);
        }
        ViewType::TextView { text, text_size, text_color } => {
            let font = CreateFontW(
                (*text_size as i32).max(12), 0, 0, 0, 500, 0, 0, 0, 1, 0, 0, 2, 0,
                to_wide_chars("Segoe UI").as_ptr(),
            );
            SelectObject(hdc, font as *mut c_void);
            SetTextColor(hdc, rgb_to_bgr(*text_color));

            let mut tr = r;
            let display_text = if text.is_empty() { "Android TextView" } else { text };
            let text_wide = to_wide_chars(display_text);
            DrawTextW(hdc, text_wide.as_ptr(), -1, &mut tr, DT_LEFT | DT_VCENTER | DT_WORDBREAK);
            DeleteObject(font as *mut c_void);
        }
        ViewType::ImageView { src, .. } => {
            let brush = CreateSolidBrush(0x003A2E20); // Placeholder icon color
            SelectObject(hdc, brush as *mut c_void);
            RoundRect(hdc, r.left, r.top, r.right, r.bottom, 16, 16);
            DeleteObject(brush as *mut c_void);

            let font = CreateFontW(
                16, 0, 0, 0, 600, 0, 0, 0, 1, 0, 0, 2, 0,
                to_wide_chars("Segoe UI").as_ptr(),
            );
            SelectObject(hdc, font as *mut c_void);
            SetTextColor(hdc, 0x00FFFFFF);

            let mut tr = r;
            let display_text = if src.is_empty() { "[Image]" } else { src };
            let text_wide = to_wide_chars(display_text);
            DrawTextW(hdc, text_wide.as_ptr(), -1, &mut tr, DT_CENTER | DT_VCENTER | DT_SINGLELINE);
            DeleteObject(font as *mut c_void);
        }
        ViewType::EditText { hint, text } => {
            let brush = CreateSolidBrush(0x002B241D); // Box fill
            SelectObject(hdc, brush as *mut c_void);
            RoundRect(hdc, r.left, r.top, r.right, r.bottom, 8, 8);
            DeleteObject(brush as *mut c_void);

            let font = CreateFontW(
                16, 0, 0, 0, 400, 0, 0, 0, 1, 0, 0, 2, 0,
                to_wide_chars("Segoe UI").as_ptr(),
            );
            SelectObject(hdc, font as *mut c_void);

            let mut tr = WinRect {
                left: r.left + 12,
                top: r.top,
                right: r.right - 12,
                bottom: r.bottom,
            };

            if text.is_empty() {
                SetTextColor(hdc, 0x00888888); // Hint color
                let hint_wide = to_wide_chars(if hint.is_empty() { "Enter text..." } else { hint });
                DrawTextW(hdc, hint_wide.as_ptr(), -1, &mut tr, DT_LEFT | DT_VCENTER | DT_SINGLELINE);
            } else {
                SetTextColor(hdc, 0x00FFFFFF);
                let text_wide = to_wide_chars(text);
                DrawTextW(hdc, text_wide.as_ptr(), -1, &mut tr, DT_LEFT | DT_VCENTER | DT_SINGLELINE);
            }
            DeleteObject(font as *mut c_void);
        }
        ViewType::Generic(tag) => {
            for child in &node.children {
                render_view_node(hdc, child);
            }
            if node.children.is_empty() {
                if tag.contains("Lottie") || tag.contains("Animation") {
                    let brush = CreateSolidBrush(0x0033281E);
                    SelectObject(hdc, brush as *mut c_void);
                    RoundRect(hdc, r.left, r.top, r.right, r.bottom, 12, 12);
                    DeleteObject(brush as *mut c_void);

                    let font = CreateFontW(
                        15, 0, 0, 0, 600, 0, 0, 0, 1, 0, 0, 2, 0,
                        to_wide_chars("Segoe UI").as_ptr(),
                    );
                    SelectObject(hdc, font as *mut c_void);
                    SetTextColor(hdc, 0x0078DE25);
                    let mut tr = r;
                    let text_wide = to_wide_chars("[Animation Preview: Lottie View]");
                    DrawTextW(hdc, text_wide.as_ptr(), -1, &mut tr, DT_CENTER | DT_VCENTER | DT_SINGLELINE);
                    DeleteObject(font as *mut c_void);
                } else if !tag.ends_with("Space") && tag != "View" {
                    let font = CreateFontW(
                        14, 0, 0, 0, 400, 0, 0, 0, 1, 0, 0, 2, 0,
                        to_wide_chars("Segoe UI").as_ptr(),
                    );
                    SelectObject(hdc, font as *mut c_void);
                    SetTextColor(hdc, 0x00999999);
                    let mut tr = r;
                    let text_wide = to_wide_chars(&format!("<{}>", tag));
                    DrawTextW(hdc, text_wide.as_ptr(), -1, &mut tr, DT_CENTER | DT_VCENTER | DT_SINGLELINE);
                    DeleteObject(font as *mut c_void);
                }
            }
        }
    }
}

fn hit_test_node(node: &ViewNode, x: i32, y: i32) -> Option<(String, String)> {
    if x >= node.bounds.left && x <= node.bounds.right && y >= node.bounds.top && y <= node.bounds.bottom {
        // Test children first for topmost hit
        for child in &node.children {
            if let Some(hit) = hit_test_node(child, x, y) {
                return Some(hit);
            }
        }

        match &node.view_type {
            ViewType::Button { text, .. } => Some(("Button".to_string(), text.clone())),
            ViewType::EditText { hint, text } => Some(("EditText".to_string(), if text.is_empty() { hint.clone() } else { text.clone() })),
            ViewType::TextView { text, .. } => Some(("TextView".to_string(), text.clone())),
            ViewType::ImageView { src, .. } => Some(("ImageView".to_string(), src.clone())),
            _ => None,
        }
    } else {
        None
    }
}
