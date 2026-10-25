use std::cell::RefCell;
use std::ffi::c_void;
use std::ptr::{null, null_mut};
use droidbridge_core::{DexVm, ExecutionResult, Value};

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
struct RECT {
    left: i32,
    top: i32,
    right: i32,
    bottom: i32,
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
const DT_RIGHT: u32 = 0x00000002;
const DT_CENTER: u32 = 0x00000001;
const DT_VCENTER: u32 = 0x00000004;
const DT_SINGLELINE: u32 = 0x00000020;
const TRANSPARENT: i32 = 1;
const IDC_ARROW: *const u16 = 32512 as *const u16;

fn to_wide_chars(s: &str) -> Vec<u16> {
    s.encode_utf16().chain(std::iter::once(0)).collect()
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum CalcOp {
    None,
    Add,
    Sub,
    Mul,
    Div,
}

#[derive(Clone)]
struct CalcButton {
    label: &'static str,
    action: &'static str,
    color_bg: u32,
    color_text: u32,
    rect: RECT,
}

struct CalculatorState {
    expression: String,
    current_input: String,
    accumulator: i64,
    pending_op: CalcOp,
    has_result: bool,
    vm: DexVm,
    buttons: Vec<CalcButton>,
}

thread_local! {
    static CALC_STATE: RefCell<Option<CalculatorState>> = const { RefCell::new(None) };
}

unsafe extern "system" fn calc_window_proc(hwnd: HWND, msg: u32, w_param: WPARAM, l_param: LPARAM) -> LRESULT {
    match msg {
        WM_PAINT => {
            let mut ps: PAINTSTRUCT = std::mem::zeroed();
            let hdc = BeginPaint(hwnd, &mut ps);

            let mut client_rect: RECT = std::mem::zeroed();
            GetClientRect(hwnd, &mut client_rect);

            // 1. Fill Dark Background (#1E1E24)
            let bg_brush = CreateSolidBrush(0x00241E1E); // BGR
            FillRect(hdc, &client_rect, bg_brush);
            DeleteObject(bg_brush as *mut c_void);

            SetBkMode(hdc, TRANSPARENT);

            let font_name = to_wide_chars("Segoe UI");
            let title_font = CreateFontW(16, 0, 0, 0, 600, 0, 0, 0, 1, 0, 0, 5, 0, font_name.as_ptr());
            let expr_font = CreateFontW(20, 0, 0, 0, 400, 0, 0, 0, 1, 0, 0, 5, 0, font_name.as_ptr());
            let display_font = CreateFontW(44, 0, 0, 0, 700, 0, 0, 0, 1, 0, 0, 5, 0, font_name.as_ptr());
            let btn_font = CreateFontW(24, 0, 0, 0, 600, 0, 0, 0, 1, 0, 0, 5, 0, font_name.as_ptr());

            // Header Banner: "Android Calculator (DroidBridge ART)"
            SelectObject(hdc, title_font as *mut c_void);
            SetTextColor(hdc, 0x0084DC3D); // Android Green
            let header_str = to_wide_chars("● DroidBridge Native ART  |  com.android.calculator2");
            let mut r_header = RECT { left: 24, top: 16, right: client_rect.right - 24, bottom: 38 };
            DrawTextW(hdc, header_str.as_ptr(), -1, &mut r_header, DT_LEFT | DT_SINGLELINE);

            CALC_STATE.with(|cell| {
                if let Some(state) = cell.borrow().as_ref() {
                    // Display Container
                    let disp_rect = RECT {
                        left: 18,
                        top: 45,
                        right: client_rect.right - 18,
                        bottom: 165,
                    };
                    let disp_brush = CreateSolidBrush(0x00302828); // Dark card
                    let prev_b = SelectObject(hdc, disp_brush as *mut c_void);
                    RoundRect(hdc, disp_rect.left, disp_rect.top, disp_rect.right, disp_rect.bottom, 16, 16);
                    SelectObject(hdc, prev_b);
                    DeleteObject(disp_brush as *mut c_void);

                    // Expression text (e.g. "12 + 8")
                    SelectObject(hdc, expr_font as *mut c_void);
                    SetTextColor(hdc, 0x00A0A0A0); // Soft gray
                    let expr_w = to_wide_chars(&state.expression);
                    let mut r_expr = RECT { left: 32, top: 58, right: disp_rect.right - 20, bottom: 85 };
                    DrawTextW(hdc, expr_w.as_ptr(), -1, &mut r_expr, DT_RIGHT | DT_SINGLELINE);

                    // Current Value / Result text (e.g. "20")
                    SelectObject(hdc, display_font as *mut c_void);
                    SetTextColor(hdc, 0x00FFFFFF); // Pure white
                    let val_w = to_wide_chars(&state.current_input);
                    let mut r_val = RECT { left: 32, top: 90, right: disp_rect.right - 20, bottom: 155 };
                    DrawTextW(hdc, val_w.as_ptr(), -1, &mut r_val, DT_RIGHT | DT_SINGLELINE | DT_VCENTER);

                    // Draw all Keypad Buttons
                    SelectObject(hdc, btn_font as *mut c_void);
                    for btn in &state.buttons {
                        let brush = CreateSolidBrush(btn.color_bg);
                        let prev = SelectObject(hdc, brush as *mut c_void);
                        RoundRect(hdc, btn.rect.left, btn.rect.top, btn.rect.right, btn.rect.bottom, 14, 14);
                        SelectObject(hdc, prev);
                        DeleteObject(brush as *mut c_void);

                        SetTextColor(hdc, btn.color_text);
                        let lbl_w = to_wide_chars(btn.label);
                        let mut r_btn = RECT {
                            left: btn.rect.left,
                            top: btn.rect.top,
                            right: btn.rect.right,
                            bottom: btn.rect.bottom,
                        };
                        DrawTextW(hdc, lbl_w.as_ptr(), -1, &mut r_btn, DT_CENTER | DT_SINGLELINE | DT_VCENTER);
                    }
                }
            });

            // Cleanup
            DeleteObject(title_font as *mut c_void);
            DeleteObject(expr_font as *mut c_void);
            DeleteObject(display_font as *mut c_void);
            DeleteObject(btn_font as *mut c_void);

            EndPaint(hwnd, &ps);
            0
        }
        WM_LBUTTONUP => {
            let x = (l_param & 0xffff) as i32;
            let y = ((l_param >> 16) & 0xffff) as i32;

            let mut should_repaint = false;

            CALC_STATE.with(|cell| {
                if let Some(state) = cell.borrow_mut().as_mut() {
                    let mut clicked_action: Option<&'static str> = None;
                    for btn in &state.buttons {
                        if x >= btn.rect.left && x <= btn.rect.right && y >= btn.rect.top && y <= btn.rect.bottom {
                            clicked_action = Some(btn.action);
                            break;
                        }
                    }

                    if let Some(action) = clicked_action {
                        println!("\n[Android InputEvent] MotionEvent.ACTION_UP at ({}, {})", x, y);
                        println!("[Android View] OnClickListener triggered for Button '{}'", action);

                        match action {
                            "0" | "1" | "2" | "3" | "4" | "5" | "6" | "7" | "8" | "9" => {
                                if state.has_result || state.current_input == "0" {
                                    state.current_input = action.to_string();
                                    state.has_result = false;
                                } else {
                                    state.current_input.push_str(action);
                                }
                                println!("    [Display] current_input = {}", state.current_input);
                            }
                            "C" => {
                                state.expression.clear();
                                state.current_input = "0".to_string();
                                state.accumulator = 0;
                                state.pending_op = CalcOp::None;
                                state.has_result = false;
                                println!("    [Calculator Logic] Cleared all state.");
                            }
                            "DEL" => {
                                if !state.has_result && state.current_input.len() > 1 {
                                    state.current_input.pop();
                                } else {
                                    state.current_input = "0".to_string();
                                }
                            }
                            "+" | "-" | "*" | "/" => {
                                let val: i64 = state.current_input.parse().unwrap_or(0);
                                state.accumulator = val;
                                state.pending_op = match action {
                                    "+" => CalcOp::Add,
                                    "-" => CalcOp::Sub,
                                    "*" => CalcOp::Mul,
                                    "/" => CalcOp::Div,
                                    _ => CalcOp::None,
                                };
                                let symbol = match action {
                                    "+" => "+",
                                    "-" => "−",
                                    "*" => "×",
                                    "/" => "÷",
                                    _ => "",
                                };
                                state.expression = format!("{} {}", val, symbol);
                                state.current_input = "0".to_string();
                                println!("    [Calculator Logic] Pending op: {} (Accumulator: {})", action, val);
                            }
                            "=" => {
                                if state.pending_op != CalcOp::None {
                                    let b: i64 = state.current_input.parse().unwrap_or(0);
                                    let a = state.accumulator;

                                    // Real Dalvik Bytecode Execution via DexVm!
                                    // Dalvik opcodes:
                                    // 0x90 = add-int v2, v0, v1
                                    // 0x91 = sub-int v2, v0, v1
                                    // 0x92 = mul-int v2, v0, v1
                                    // 0x93 = div-int v2, v0, v1
                                    let (opcode_code, op_name) = match state.pending_op {
                                        CalcOp::Add => (0x90u8, "add-int"),
                                        CalcOp::Sub => (0x91u8, "sub-int"),
                                        CalcOp::Mul => (0x92u8, "mul-int"),
                                        CalcOp::Div => (0x93u8, "div-int"),
                                        CalcOp::None => (0x90u8, "add-int"),
                                    };

                                    // Build Dalvik bytecode:
                                    // const/16 v0, <a>      -> 0x0013, a
                                    // const/16 v1, <b>      -> 0x0113, b
                                    // <op> v2, v0, v1       -> (opcode), 0x0100 (src1=0, src2=1, dst=2)
                                    // return v2             -> 0x020f
                                    let mut insns = Vec::new();
                                    insns.push(0x0013);
                                    insns.push(a as u16);
                                    insns.push(0x0113);
                                    insns.push(b as u16);
                                    let op_u16 = (opcode_code as u16) | (0x0200); // opcode + dst=2
                                    insns.push(op_u16);
                                    insns.push(0x0100); // src1=0, src2=1
                                    insns.push(0x020f); // return v2

                                    println!("[DexVm Execution] Invoking com.android.calculator2.Calculator.calculate()");
                                    println!("    Executing Dalvik Opcode: {} v2, v0 (={}), v1 (={})", op_name, a, b);

                                    let res = state.vm.execute_bytecode(&insns, 3, 0, &[]);
                                    if let ExecutionResult::Returned(Some(Value::Int(res_val))) = res {
                                        println!("    [DexVm Result] v2 = {} (Computation Succeeded!)", res_val);
                                        let symbol = match state.pending_op {
                                            CalcOp::Add => "+",
                                            CalcOp::Sub => "−",
                                            CalcOp::Mul => "×",
                                            CalcOp::Div => "÷",
                                            CalcOp::None => "",
                                        };
                                        state.expression = format!("{} {} {} =", a, symbol, b);
                                        state.current_input = res_val.to_string();
                                        state.accumulator = res_val as i64;
                                        state.pending_op = CalcOp::None;
                                        state.has_result = true;
                                    } else {
                                        state.current_input = "Error".to_string();
                                        state.has_result = true;
                                    }
                                }
                            }
                            _ => {}
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
            println!("\n[Android Activity] onDestroy() called -> Android Calculator closed.");
            PostQuitMessage(0);
            0
        }
        _ => DefWindowProcW(hwnd, msg, w_param, l_param),
    }
}

pub fn run_calculator_app() -> Result<(), String> {
    unsafe {
        let class_name = to_wide_chars("DroidBridgeCalculatorClass");
        let window_title = to_wide_chars("DroidBridge — Android Calculator (com.android.calculator2)");

        let h_instance = GetModuleHandleW(null());
        let cursor = LoadCursorW(null_mut(), IDC_ARROW);

        let wc = WNDCLASSEXW {
            cb_size: std::mem::size_of::<WNDCLASSEXW>() as u32,
            style: 0x0002 | 0x0001,
            lpfn_wnd_proc: Some(calc_window_proc),
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

        RegisterClassExW(&wc);

        let width = 380;
        let height = 580;
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
            return Err("Failed to create Calculator Window HWND".to_string());
        }

        // Layout button grid (4 cols x 5 rows)
        let mut buttons = Vec::new();
        let grid = [
            [("C", "C", 0x00453838, 0x0084DC3D), ("DEL", "DEL", 0x00453838, 0x0084DC3D), ("%", "%", 0x00453838, 0x0084DC3D), ("÷", "/", 0x000A9FFF, 0x00FFFFFF)],
            [("7", "7", 0x003A3030, 0x00FFFFFF), ("8", "8", 0x003A3030, 0x00FFFFFF), ("9", "9", 0x003A3030, 0x00FFFFFF), ("×", "*", 0x000A9FFF, 0x00FFFFFF)],
            [("4", "4", 0x003A3030, 0x00FFFFFF), ("5", "5", 0x003A3030, 0x00FFFFFF), ("6", "6", 0x003A3030, 0x00FFFFFF), ("−", "-", 0x000A9FFF, 0x00FFFFFF)],
            [("1", "1", 0x003A3030, 0x00FFFFFF), ("2", "2", 0x003A3030, 0x00FFFFFF), ("3", "3", 0x003A3030, 0x00FFFFFF), ("+", "+", 0x000A9FFF, 0x00FFFFFF)],
            [("±", "±", 0x003A3030, 0x00FFFFFF), ("0", "0", 0x003A3030, 0x00FFFFFF), (".", ".", 0x003A3030, 0x00FFFFFF), ("=", "=", 0x003DDC84, 0x00000000)],
        ];

        let start_x = 18;
        let start_y = 180;
        let btn_w = 75;
        let btn_h = 62;
        let gap = 12;

        for (r, row) in grid.iter().enumerate() {
            for (c, (label, action, bg, txt)) in row.iter().enumerate() {
                let bx = start_x + (c as i32) * (btn_w + gap);
                let by = start_y + (r as i32) * (btn_h + gap);
                buttons.push(CalcButton {
                    label,
                    action,
                    color_bg: *bg,
                    color_text: *txt,
                    rect: RECT {
                        left: bx,
                        top: by,
                        right: bx + btn_w,
                        bottom: by + btn_h,
                    },
                });
            }
        }

        CALC_STATE.with(|cell| {
            *cell.borrow_mut() = Some(CalculatorState {
                expression: String::new(),
                current_input: "0".to_string(),
                accumulator: 0,
                pending_op: CalcOp::None,
                has_result: false,
                vm: DexVm::new(),
                buttons,
            });
        });

        println!("\n[+] Android Calculator Activity Window created (HWND: {:?})", hwnd);
        println!("[*] Window centered on Desktop: {}x{} at ({}, {})", width, height, pos_x, pos_y);

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
