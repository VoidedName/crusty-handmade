use std::ffi::c_void;
use std::mem;
use std::ptr::null_mut;
use windows::{
    core::*, Win32::Foundation::*, Win32::Graphics::Gdi::*,
    Win32::System::LibraryLoader::GetModuleHandleW, Win32::UI::WindowsAndMessaging::*,
};

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
global_mut!(BITMAP_HANDLE: HBITMAP = HBITMAP(0));
global_mut!(BITMAP_DEVICE_CONTEXT: HDC = HDC(0));

unsafe fn resize_dib_section(width: i32, height: i32) {
    //TODO(voided): bulletproof this.
    //maybe don't free first, free after, then free first if that fails.

    if BITMAP_HANDLE.is_invalid() {
        //TODO(voided): should we recreate these under certain special circumstances?
        BITMAP_DEVICE_CONTEXT = CreateCompatibleDC(HDC::default());
    }

    if !BITMAP_HANDLE.is_invalid() {
        DeleteObject(BITMAP_HANDLE);
    }

    BITMAP_INFO.bmiHeader.biSize = mem::size_of::<BITMAPINFOHEADER>() as u32;
    BITMAP_INFO.bmiHeader.biWidth = width;
    BITMAP_INFO.bmiHeader.biHeight = height;
    BITMAP_INFO.bmiHeader.biPlanes = 1;
    BITMAP_INFO.bmiHeader.biBitCount = 32;
    BITMAP_INFO.bmiHeader.biCompression = BI_RGB.0;

    BITMAP_HANDLE = CreateDIBSection(
        BITMAP_DEVICE_CONTEXT,
        &BITMAP_INFO,
        DIB_RGB_COLORS,
        &mut BITMAP_MEMORY,
        HANDLE::default(),
        0,
    ).unwrap();
}

unsafe fn update_window(device_context: HDC, x: i32, y: i32, width: i32, height: i32) {
    StretchDIBits(
        device_context,
        x, y, width, height,
        x, y, width, height,
        None,
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
                let mut client_rect = RECT::default();
                GetClientRect(window, &mut client_rect).expect("Failed to get drawing window!");
                println!("{:?}", &BITMAP_MEMORY);

                let width = client_rect.right - client_rect.left;
                let height = client_rect.bottom - client_rect.top;

                resize_dib_section(width, height);
                println!("WM_SIZE");
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
                let mut paint: PAINTSTRUCT = PAINTSTRUCT::default();
                let hdc = BeginPaint(window, &mut paint);

                let x = paint.rcPaint.left;
                let y = paint.rcPaint.top;
                let width = paint.rcPaint.right - paint.rcPaint.left;
                let height = paint.rcPaint.bottom - paint.rcPaint.top;

                update_window(hdc, x, y, width, height);
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

        CreateWindowExW(
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

        while RUN_STATE != RunState::Stopping {
            let BOOL(result) = GetMessageW(&mut message, None, 0, 0);
            if result > 0 {
                DispatchMessageW(&message);
            } else {
                if result == 1 {
                    GetLastError()?;
                }
                RUN_STATE = RunState::Stopping;
            }
        }

        Ok(())
    }
}
