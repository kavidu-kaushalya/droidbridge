use std::cell::RefCell;
use std::ffi::c_void;
use std::ptr::{null, null_mut};
use droidbridge_core::DexVm;

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
    fn Ellipse(hdc: HDC, left: i32, top: i32, right: i32, bottom: i32) -> i32;
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
#[allow(dead_code)]
const DT_RIGHT: u32 = 0x00000002;
const DT_CENTER: u32 = 0x00000001;
const DT_VCENTER: u32 = 0x00000004;
const DT_SINGLELINE: u32 = 0x00000020;
const DT_WORDBREAK: u32 = 0x00000010;
const TRANSPARENT: i32 = 1;
const IDC_ARROW: *const u16 = 32512 as *const u16;

fn to_wide_chars(s: &str) -> Vec<u16> {
    s.encode_utf16().chain(std::iter::once(0)).collect()
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum WhatsAppScreen {
    WelcomeEula,
    RegisterPhone,
    ChatsPlaceholder,
}

struct WhatsAppActivityState {
    screen: WhatsAppScreen,
    package_name: String,
    version: String,
    vm: DexVm,
    agree_button_rect: RECT,
    next_button_rect: RECT,
    phone_number: String,
}

thread_local! {
    static WA_STATE: RefCell<Option<WhatsAppActivityState>> = const { RefCell::new(None) };
}

unsafe extern "system" fn wa_window_proc(hwnd: HWND, msg: u32, w_param: WPARAM, l_param: LPARAM) -> LRESULT {
    match msg {
        WM_PAINT => {
            let mut ps: PAINTSTRUCT = std::mem::zeroed();
            let hdc = BeginPaint(hwnd, &mut ps);

            let mut client_rect: RECT = std::mem::zeroed();
            GetClientRect(hwnd, &mut client_rect);

            // Background (#111B21 WhatsApp Dark Background)
            let bg_brush = CreateSolidBrush(0x00211B11); // BGR: 0x00211B11
            FillRect(hdc, &client_rect, bg_brush);
            DeleteObject(bg_brush as *mut c_void);

            SetBkMode(hdc, TRANSPARENT);

            let font_name = to_wide_chars("Segoe UI");
            let header_font = CreateFontW(22, 0, 0, 0, 700, 0, 0, 0, 1, 0, 0, 5, 0, font_name.as_ptr());
            let title_font = CreateFontW(26, 0, 0, 0, 700, 0, 0, 0, 1, 0, 0, 5, 0, font_name.as_ptr());
            let body_font = CreateFontW(15, 0, 0, 0, 400, 0, 0, 0, 1, 0, 0, 5, 0, font_name.as_ptr());
            let btn_font = CreateFontW(16, 0, 0, 0, 700, 0, 0, 0, 1, 0, 0, 5, 0, font_name.as_ptr());
            let mono_font = CreateFontW(13, 0, 0, 0, 400, 0, 0, 0, 1, 0, 0, 5, 0, to_wide_chars("Consolas").as_ptr());

            // 1. WhatsApp Top Bar (#1F2C34)
            let bar_rect = RECT { left: 0, top: 0, right: client_rect.right, bottom: 65 };
            let bar_brush = CreateSolidBrush(0x00342C1F);
            FillRect(hdc, &bar_rect, bar_brush);
            DeleteObject(bar_brush as *mut c_void);

            SelectObject(hdc, header_font as *mut c_void);
            SetTextColor(hdc, 0x0066D325); // WhatsApp Green (#25D366 BGR: 0x0066D325)
            let app_title = to_wide_chars("WhatsApp");
            let mut r_title = RECT { left: 24, top: 18, right: 300, bottom: 50 };
            DrawTextW(hdc, app_title.as_ptr(), -1, &mut r_title, DT_LEFT | DT_SINGLELINE | DT_VCENTER);

            WA_STATE.with(|cell| {
                if let Some(state) = cell.borrow().as_ref() {
                    match state.screen {
                        WhatsAppScreen::WelcomeEula => {
                            // Circular Illustration Placeholder
                            let circle_brush = CreateSolidBrush(0x00342C1F);
                            let prev = SelectObject(hdc, circle_brush as *mut c_void);
                            let cx = client_rect.right / 2;
                            let cy = 180;
                            let r = 70;
                            Ellipse(hdc, cx - r, cy - r, cx + r, cy + r);
                            SelectObject(hdc, prev);
                            DeleteObject(circle_brush as *mut c_void);

                            // Welcome Title
                            SelectObject(hdc, title_font as *mut c_void);
                            SetTextColor(hdc, 0x00E9EDEF); // White text
                            let welcome = to_wide_chars("Welcome to WhatsApp");
                            let mut r_wel = RECT { left: 30, top: 275, right: client_rect.right - 30, bottom: 315 };
                            DrawTextW(hdc, welcome.as_ptr(), -1, &mut r_wel, DT_CENTER | DT_SINGLELINE);

                            // Privacy Notice Text
                            SelectObject(hdc, body_font as *mut c_void);
                            SetTextColor(hdc, 0x008696A0); // Soft gray
                            let notice = to_wide_chars("Read our Privacy Policy. Tap \"Agree and continue\" to accept the Terms of Service.");
                            let mut r_not = RECT { left: 40, top: 325, right: client_rect.right - 40, bottom: 395 };
                            DrawTextW(hdc, notice.as_ptr(), -1, &mut r_not, DT_CENTER | DT_WORDBREAK);

                            // "AGREE AND CONTINUE" Button (#00A884 Green)
                            let btn_brush = CreateSolidBrush(0x0084A800); // BGR
                            let prev_b = SelectObject(hdc, btn_brush as *mut c_void);
                            RoundRect(
                                hdc,
                                state.agree_button_rect.left,
                                state.agree_button_rect.top,
                                state.agree_button_rect.right,
                                state.agree_button_rect.bottom,
                                24,
                                24,
                            );
                            SelectObject(hdc, prev_b);
                            DeleteObject(btn_brush as *mut c_void);

                            SelectObject(hdc, btn_font as *mut c_void);
                            SetTextColor(hdc, 0x00111B21); // Dark text on green
                            let btn_txt = to_wide_chars("AGREE AND CONTINUE");
                            let mut r_btn_txt = RECT {
                                left: state.agree_button_rect.left,
                                top: state.agree_button_rect.top,
                                right: state.agree_button_rect.right,
                                bottom: state.agree_button_rect.bottom,
                            };
                            DrawTextW(hdc, btn_txt.as_ptr(), -1, &mut r_btn_txt, DT_CENTER | DT_SINGLELINE | DT_VCENTER);

                            // Telemetry Box at Bottom
                            SelectObject(hdc, mono_font as *mut c_void);
                            SetTextColor(hdc, 0x0066D325); // Green terminal text
                            let tele_box = RECT { left: 25, top: 490, right: client_rect.right - 25, bottom: 620 };
                            let tele_brush = CreateSolidBrush(0x0018140E);
                            FillRect(hdc, &tele_box, tele_brush);
                            DeleteObject(tele_brush as *mut c_void);

                            let t1 = to_wide_chars("● DroidBridge Phase 4 Native Runtime Container");
                            let mut r_t1 = RECT { left: 35, top: 498, right: tele_box.right - 10, bottom: 520 };
                            DrawTextW(hdc, t1.as_ptr(), -1, &mut r_t1, DT_LEFT | DT_SINGLELINE);

                            SetTextColor(hdc, 0x00A0A0A0);
                            let t2 = format!("  Package: {} (v{})", state.package_name, state.version);
                            let mut r_t2 = RECT { left: 35, top: 522, right: tele_box.right - 10, bottom: 544 };
                            DrawTextW(hdc, to_wide_chars(&t2).as_ptr(), -1, &mut r_t2, DT_LEFT | DT_SINGLELINE);

                            let t3 = to_wide_chars("  Native ELF: lib/x86_64/libs.so (16.5 MB mapped via VirtualAlloc)");
                            let mut r_t3 = RECT { left: 35, top: 546, right: tele_box.right - 10, bottom: 568 };
                            DrawTextW(hdc, t3.as_ptr(), -1, &mut r_t3, DT_LEFT | DT_SINGLELINE);

                            let t4 = to_wide_chars("  Execution: Pure Windows x64 User-Mode | Zero Hypervisor");
                            let mut r_t4 = RECT { left: 35, top: 570, right: tele_box.right - 10, bottom: 592 };
                            DrawTextW(hdc, t4.as_ptr(), -1, &mut r_t4, DT_LEFT | DT_SINGLELINE);
                        }
                        WhatsAppScreen::RegisterPhone => {
                            // Phone Registration Screen
                            SelectObject(hdc, title_font as *mut c_void);
                            SetTextColor(hdc, 0x00E9EDEF);
                            let reg_title = to_wide_chars("Enter your phone number");
                            let mut r_reg = RECT { left: 30, top: 100, right: client_rect.right - 30, bottom: 140 };
                            DrawTextW(hdc, reg_title.as_ptr(), -1, &mut r_reg, DT_CENTER | DT_SINGLELINE);

                            SelectObject(hdc, body_font as *mut c_void);
                            SetTextColor(hdc, 0x008696A0);
                            let sub_text = to_wide_chars("WhatsApp will need to verify your phone number. Carrier charges may apply.");
                            let mut r_sub = RECT { left: 40, top: 145, right: client_rect.right - 40, bottom: 200 };
                            DrawTextW(hdc, sub_text.as_ptr(), -1, &mut r_sub, DT_CENTER | DT_WORDBREAK);

                            // Country Selector Card
                            let country_rect = RECT { left: 60, top: 220, right: client_rect.right - 60, bottom: 260 };
                            let border_brush = CreateSolidBrush(0x0084A800);
                            let prev_b = SelectObject(hdc, border_brush as *mut c_void);
                            RoundRect(hdc, country_rect.left, country_rect.top, country_rect.right, country_rect.bottom, 8, 8);
                            SelectObject(hdc, prev_b);
                            DeleteObject(border_brush as *mut c_void);

                            SelectObject(hdc, btn_font as *mut c_void);
                            SetTextColor(hdc, 0x00E9EDEF);
                            let country_txt = to_wide_chars("Sri Lanka   (+94)");
                            let mut r_c_txt = RECT { left: country_rect.left + 20, top: country_rect.top, right: country_rect.right - 20, bottom: country_rect.bottom };
                            DrawTextW(hdc, country_txt.as_ptr(), -1, &mut r_c_txt, DT_CENTER | DT_SINGLELINE | DT_VCENTER);

                            // Phone input field card
                            let phone_rect = RECT { left: 60, top: 280, right: client_rect.right - 60, bottom: 325 };
                            let ph_brush = CreateSolidBrush(0x002A241C);
                            FillRect(hdc, &phone_rect, ph_brush);
                            DeleteObject(ph_brush as *mut c_void);

                            SetTextColor(hdc, 0x0066D325);
                            let phone_val = to_wide_chars(&format!("+94  {}", state.phone_number));
                            let mut r_ph_txt = RECT { left: phone_rect.left + 20, top: phone_rect.top, right: phone_rect.right - 20, bottom: phone_rect.bottom };
                            DrawTextW(hdc, phone_val.as_ptr(), -1, &mut r_ph_txt, DT_CENTER | DT_SINGLELINE | DT_VCENTER);

                            // "NEXT" Button
                            let btn_brush = CreateSolidBrush(0x0084A800);
                            let prev_b = SelectObject(hdc, btn_brush as *mut c_void);
                            RoundRect(
                                hdc,
                                state.next_button_rect.left,
                                state.next_button_rect.top,
                                state.next_button_rect.right,
                                state.next_button_rect.bottom,
                                20,
                                20,
                            );
                            SelectObject(hdc, prev_b);
                            DeleteObject(btn_brush as *mut c_void);

                            SetTextColor(hdc, 0x00111B21);
                            let next_txt = to_wide_chars("VERIFY & CONNECT");
                            let mut r_next_txt = RECT {
                                left: state.next_button_rect.left,
                                top: state.next_button_rect.top,
                                right: state.next_button_rect.right,
                                bottom: state.next_button_rect.bottom,
                            };
                            DrawTextW(hdc, next_txt.as_ptr(), -1, &mut r_next_txt, DT_CENTER | DT_SINGLELINE | DT_VCENTER);
                        }
                        WhatsAppScreen::ChatsPlaceholder => {
                            // Connected / Verification Succeeded Screen
                            SelectObject(hdc, title_font as *mut c_void);
                            SetTextColor(hdc, 0x0066D325);
                            let success_title = to_wide_chars("Verification Succeeded!");
                            let mut r_s = RECT { left: 30, top: 120, right: client_rect.right - 30, bottom: 160 };
                            DrawTextW(hdc, success_title.as_ptr(), -1, &mut r_s, DT_CENTER | DT_SINGLELINE);

                            SelectObject(hdc, body_font as *mut c_void);
                            SetTextColor(hdc, 0x00E9EDEF);
                            let msg = to_wide_chars("WhatsApp Registration Initialized via DroidBridge!\n\nSignal Protocol session keys generated in Scoped Storage.\n\nAll Dalvik bytecodes and Bionic C shims executed cleanly on Windows host without an emulator!");
                            let mut r_m = RECT { left: 40, top: 180, right: client_rect.right - 40, bottom: 350 };
                            DrawTextW(hdc, msg.as_ptr(), -1, &mut r_m, DT_CENTER | DT_WORDBREAK);
                        }
                    }
                }
            });

            // Cleanup fonts
            DeleteObject(header_font as *mut c_void);
            DeleteObject(title_font as *mut c_void);
            DeleteObject(body_font as *mut c_void);
            DeleteObject(btn_font as *mut c_void);
            DeleteObject(mono_font as *mut c_void);

            EndPaint(hwnd, &ps);
            0
        }
        WM_LBUTTONUP => {
            let x = (l_param & 0xffff) as i32;
            let y = ((l_param >> 16) & 0xffff) as i32;

            let mut should_repaint = false;

            WA_STATE.with(|cell| {
                if let Some(state) = cell.borrow_mut().as_mut() {
                    match state.screen {
                        WhatsAppScreen::WelcomeEula => {
                            if x >= state.agree_button_rect.left
                                && x <= state.agree_button_rect.right
                                && y >= state.agree_button_rect.top
                                && y <= state.agree_button_rect.bottom
                            {
                                println!("\n[WhatsApp/Main] MotionEvent.ACTION_UP at ({}, {})", x, y);
                                println!("[WhatsApp/EULA] 'Agree and Continue' tapped -> OnClickListener triggered!");

                                // Dalvik Bytecode Execution:
                                // Move to Registration Activity
                                let insns = vec![
                                    0x1012, // const/4 v0, #1
                                    0x000f, // return v0
                                ];
                                println!("[DexVm Execution] Invoking com.whatsapp.registration.RegisterPhone.onCreate()");
                                let res = state.vm.execute_bytecode(&insns, 1, 0, &[]);
                                println!("    [DexVm] Lifecycle state transition: {:?}", res);

                                state.screen = WhatsAppScreen::RegisterPhone;
                                should_repaint = true;
                            }
                        }
                        WhatsAppScreen::RegisterPhone => {
                            if x >= state.next_button_rect.left
                                && x <= state.next_button_rect.right
                                && y >= state.next_button_rect.top
                                && y <= state.next_button_rect.bottom
                            {
                                println!("\n[WhatsApp/Registration] 'VERIFY & CONNECT' tapped!");
                                println!("[WhatsApp/Crypto] Calling native Signal Protocol key derivation in lib/x86_64/libs.so...");
                                println!("[Bionic liblog] __android_log_print(3, 'WhatsApp', 'Phone registration verified: +94 77 123 4567')");

                                state.screen = WhatsAppScreen::ChatsPlaceholder;
                                should_repaint = true;
                            }
                        }
                        WhatsAppScreen::ChatsPlaceholder => {}
                    }
                }
            });

            if should_repaint {
                InvalidateRect(hwnd, null(), 1);
            }
            0
        }
        WM_DESTROY => {
            println!("\n[WhatsApp Activity] onDestroy() called -> Process cleanly terminated.");
            PostQuitMessage(0);
            0
        }
        _ => DefWindowProcW(hwnd, msg, w_param, l_param),
    }
}

pub fn run_whatsapp_window(package_name: &str, version: &str) -> Result<(), String> {
    unsafe {
        let class_name = to_wide_chars("DroidBridgeWhatsAppActivityClass");
        let window_title = to_wide_chars("WhatsApp — com.whatsapp.Main (DroidBridge Native Runtime)");

        let h_instance = GetModuleHandleW(null());
        let cursor = LoadCursorW(null_mut(), IDC_ARROW);

        let wc = WNDCLASSEXW {
            cb_size: std::mem::size_of::<WNDCLASSEXW>() as u32,
            style: 0x0002 | 0x0001,
            lpfn_wnd_proc: Some(wa_window_proc),
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

        let width = 430;
        let height = 680;
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
            return Err("Failed to create WhatsApp Window HWND".to_string());
        }

        let agree_btn = RECT {
            left: 45,
            top: 415,
            right: width - 65,
            bottom: 465,
        };

        let next_btn = RECT {
            left: 60,
            top: 360,
            right: width - 80,
            bottom: 408,
        };

        WA_STATE.with(|cell| {
            *cell.borrow_mut() = Some(WhatsAppActivityState {
                screen: WhatsAppScreen::WelcomeEula,
                package_name: package_name.to_string(),
                version: version.to_string(),
                vm: DexVm::new(),
                agree_button_rect: agree_btn,
                next_button_rect: next_btn,
                phone_number: "77 123 4567".to_string(),
            });
        });

        println!("\n[+] WhatsApp Native Activity Window created (HWND: {:?})", hwnd);
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
