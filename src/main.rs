use std::ffi::c_void;
use std::mem;
use std::ptr::null_mut;
use windows::{
    core::*, Win32::Foundation::*, Win32::Graphics::Gdi::*,
    Win32::System::LibraryLoader::GetModuleHandleW, Win32::UI::WindowsAndMessaging::*,
};
use windows::Win32::System::Memory::{MEM_COMMIT, MEM_RELEASE, PAGE_READWRITE, VirtualAlloc, VirtualFree};

#[derive(Eq, PartialEq, Copy, Clone, Debug)]
enum RunState {
    Starting,
    Running,
    Stopping,
}

///Declares a static mut! Allows to search for specifically global muts
macro_rules! global_mut {
    ($variable:ident : $t:ty = $e:expr) => {
        static mut $variable: $t = $e;
    }
}

//TODO(voided): This is a global for now.
global_mut!(RUN_STATE: RunState = RunState::Starting);
global_mut!(BITMAP_INFO: BITMAPINFO = BITMAPINFO {
        bmiHeader: BITMAPINFOHEADER {
            biSize: 0,
            biWidth: 0,
            biHeight: 0,
            biPlanes: 0,
            biBitCount: 0,
            biCompression: 0,
            biSizeImage: 0,
            biXPelsPerMeter: 0,
            biYPelsPerMeter: 0,
            biClrUsed: 0,
            biClrImportant: 0,
        },
        bmiColors: [RGBQUAD {
            rgbBlue: 0,
            rgbGreen: 0,
            rgbRed: 0,
            rgbReserved: 0,
        }],
    });
global_mut!(BITMAP_MEMORY: *mut c_void = null_mut());
global_mut!(BITMAP_WIDTH: i32 = 0);
global_mut!(BITMAP_HEIGTH: i32 = 0);

const BITS_PER_PIXEL: i32 = 4;

unsafe fn render_weird_gradient(x_offset: i32, y_offest: i32) {
    // windows rgb order is actually bgr
    let mut row = BITMAP_MEMORY.cast::<u8>();
    let pitch = (BITMAP_WIDTH * BITS_PER_PIXEL) as isize;
    for y in 0..BITMAP_HEIGTH {
        let mut pixel = row;
        for x in 0..BITMAP_WIDTH {
            *pixel = (x + x_offset) as u8;
            pixel = pixel.offset(1);

            *pixel = (y + y_offest) as u8;
            pixel = pixel.offset(1);

            *pixel = 0;
            pixel = pixel.offset(1);

            *pixel = 0;
            pixel = pixel.offset(1);
        }
        row = row.offset(pitch);
    }
}

unsafe fn resize_dib_section(width: i32, height: i32) {
    //TODO(voided): bulletproof this.
    //maybe don't free first, free after, then free first if that fails.

    if !BITMAP_MEMORY.is_null() {
        VirtualFree(BITMAP_MEMORY, 0, MEM_RELEASE).ok();
        BITMAP_MEMORY = null_mut();
    }

    BITMAP_WIDTH = width;
    BITMAP_HEIGTH = height;

    BITMAP_INFO.bmiHeader.biSize = mem::size_of::<BITMAPINFOHEADER>() as u32;
    BITMAP_INFO.bmiHeader.biWidth = BITMAP_WIDTH;
    BITMAP_INFO.bmiHeader.biHeight = -BITMAP_HEIGTH; // sets origin to top left
    BITMAP_INFO.bmiHeader.biPlanes = 1;
    BITMAP_INFO.bmiHeader.biBitCount = 32;
    BITMAP_INFO.bmiHeader.biCompression = BI_RGB.0;

    let bitmap_memory_size = BITS_PER_PIXEL * BITMAP_WIDTH * BITMAP_HEIGTH;

    BITMAP_MEMORY = VirtualAlloc(None, bitmap_memory_size as usize, MEM_COMMIT, PAGE_READWRITE);
}

unsafe fn update_window(device_context: HDC, client_rect: &RECT, x: i32, y: i32, width: i32, height: i32) {
    let window_width = client_rect.right - client_rect.left;
    let window_height = client_rect.bottom - client_rect.top;
    StretchDIBits(
        device_context,
        // x, y, width, height,
        0, 0, BITMAP_WIDTH, BITMAP_HEIGTH,
        0, 0, window_width, window_height,
        Some(BITMAP_MEMORY),
        &BITMAP_INFO,
        DIB_RGB_COLORS, SRCCOPY,
    );
}

pub unsafe extern "system" fn window_procedure(
    window: HWND,
    message: u32,
    w_param: WPARAM,
    l_param: LPARAM,
) -> LRESULT {
    unsafe {
        let mut result: LRESULT = LRESULT(0);
        match message {
            WM_ACTIVATEAPP => {
                println!("WM_ACTIVATEAPP");
            }
            WM_SIZE => {
                println!("WM_SIZE");
                let mut client_rect = RECT::default();
                GetClientRect(window, &mut client_rect).expect("Failed to get drawing window!");

                let width = client_rect.right - client_rect.left;
                let height = client_rect.bottom - client_rect.top;

                resize_dib_section(width, height);
            }
            WM_CLOSE => {
                println!("WM_CLOSE");
                //TODO(voided): Handle with message to the user?
                RUN_STATE = RunState::Stopping;
            }
            WM_DESTROY => {
                println!("WM_DESTROY");
                //TODO(voided): Handle this as error - recreate the window?
                RUN_STATE = RunState::Stopping;
            }
            WM_PAINT => {
                println!("WM_PAINT");
                let mut client_rect = RECT::default();
                GetClientRect(window, &mut client_rect).expect("Failed to get drawing window!");

                let mut paint: PAINTSTRUCT = PAINTSTRUCT::default();
                let hdc = BeginPaint(window, &mut paint);

                let x = paint.rcPaint.left;
                let y = paint.rcPaint.top;
                let width = paint.rcPaint.right - paint.rcPaint.left;
                let height = paint.rcPaint.bottom - paint.rcPaint.top;

                update_window(hdc, &client_rect, x, y, width, height);
                EndPaint(window, &paint);
            }
            _ => {
                result = DefWindowProcW(window, message, w_param, l_param);
            }
        }

        result
    }
}

fn main() -> Result<()> {
    unsafe {
        let instance = GetModuleHandleW(None)?;
        debug_assert!(instance.0 != 0);

        let window_class = w!("VoidedsHandmadeClass");

        let wc = WNDCLASSW {
            hCursor: Default::default(),
            hInstance: instance.into(),
            lpszClassName: window_class,

            style: CS_HREDRAW | CS_VREDRAW | CS_OWNDC,
            lpfnWndProc: Some(window_procedure),
            ..Default::default()
        };

        let atom = RegisterClassW(&wc);
        debug_assert!(atom != 0);

        let window = CreateWindowExW(
            WINDOW_EX_STYLE::default(),
            window_class,
            w!("Voideds Handmade?"),
            WS_OVERLAPPEDWINDOW | WS_VISIBLE,
            CW_USEDEFAULT,
            CW_USEDEFAULT,
            CW_USEDEFAULT,
            CW_USEDEFAULT,
            None,
            None,
            instance,
            None,
        );

        RUN_STATE = RunState::Running;
        let mut message = MSG::default();
        let mut x_offset = 0;
        let mut y_offset = 0;

        while RUN_STATE != RunState::Stopping {
            while PeekMessageW(&mut message, None, 0, 0, PM_REMOVE).as_bool() {
                if message.message == WM_QUIT {
                    RUN_STATE = RunState::Stopping;
                }

                TranslateMessage(&message);
                DispatchMessageW(&message);
            }

            render_weird_gradient(x_offset, y_offset);

            let device_context = GetDC(window);
            let mut client_rect = RECT::default();
            GetClientRect(window, &mut client_rect).expect("Failed to get drawing window!");

            let window_width = client_rect.right - client_rect.left;
            let window_height = client_rect.bottom - client_rect.top;

            update_window(device_context, &client_rect, 0, 0, window_width, window_height);
            ReleaseDC(window, device_context);

            x_offset += 2;
            y_offset += 1;
        }

        Ok(())
    }
}
