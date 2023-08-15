use std::ffi::c_void;
use std::mem;
use std::ptr::null_mut;
use windows::Win32::System::Memory::{
    VirtualAlloc, VirtualFree, MEM_COMMIT, MEM_RELEASE, PAGE_READWRITE,
};
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
    };
}

//TODO(voided): This is a global for now.
global_mut!(RUN_STATE: RunState = RunState::Starting);
global_mut!(GLOBAL_BACK_BUFFER: OffscreenBuffer = OffscreenBuffer {
    info: BITMAPINFO {
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
        bmiColors: [RGBQUAD{
            rgbBlue: 0,
            rgbGreen: 0,
            rgbRed: 0,
            rgbReserved: 0,
        }],
    },
    memory: null_mut(),
    bytes_per_pixel: 0,
    width: 0,
    height: 0,
});

struct OffscreenBuffer {
    info: BITMAPINFO,
    memory: *mut c_void,
    width: i32,
    height: i32,
    bytes_per_pixel: i32,
}

impl OffscreenBuffer {
    pub fn pitch(&self) -> isize {
        (self.width * self.bytes_per_pixel) as isize
    }

    pub fn memory_size(&self) -> i32 {
        self.bytes_per_pixel * self.width * self.height
    }
}

#[derive(Copy, Clone)]
struct WindowDimensions {
    width: i32,
    height: i32,
}

unsafe fn window_dimension(window: HWND) -> (i32, i32) {
    let mut client_rect = RECT::default();
    GetClientRect(window, &mut client_rect).expect("Failed to get drawing window rect!");

    let width = client_rect.right - client_rect.left;
    let height = client_rect.bottom - client_rect.top;

    (width, height)
}

unsafe fn render_weird_gradient(buffer: &OffscreenBuffer, x_offset: i32, y_offest: i32) {
    // windows rgb order is actually padding-bgr
    // on lil endian we need to load xxRRGGBB
    let mut row = buffer.memory.cast::<u8>();
    let pitch = buffer.pitch();

    for y in 0..buffer.height {
        let mut pixel = row.cast::<u32>();
        for x in 0..buffer.width {
            let blue = (x + x_offset) as u32 & 0xFF;
            let green = (y + y_offest) as u32 & 0xFF;
            let a = green << 8 | blue;
            *pixel = a;
            pixel = pixel.offset(1);
        }
        row = row.offset(pitch);
    }
}

unsafe fn resize_dib_section(buffer: &mut OffscreenBuffer, width: i32, height: i32) {
    //TODO(voided): bulletproof this.
    //maybe don't free first, free after, then free first if that fails.

    if !buffer.memory.is_null() {
        VirtualFree(buffer.memory, 0, MEM_RELEASE).ok();
        buffer.memory = null_mut();
    }

    buffer.width = width;
    buffer.height = height;
    buffer.bytes_per_pixel = 4;

    buffer.info.bmiHeader.biSize = mem::size_of::<BITMAPINFOHEADER>() as u32;
    buffer.info.bmiHeader.biWidth = buffer.width;
    buffer.info.bmiHeader.biHeight = -buffer.height; // sets origin to top left
    buffer.info.bmiHeader.biPlanes = 1;
    buffer.info.bmiHeader.biBitCount = 32;
    buffer.info.bmiHeader.biCompression = BI_RGB.0;

    buffer.memory = VirtualAlloc(
        None,
        buffer.memory_size() as usize,
        MEM_COMMIT,
        PAGE_READWRITE,
    );
}

unsafe fn display_buffer_in_window(
    device_context: HDC,
    x: i32,
    y: i32,
    window_width: i32,
    window_height: i32,
    buffer: &OffscreenBuffer,
) {
    //TODO(voided): Aspect ratio correction
    StretchDIBits(
        device_context,
        x,
        y,
        window_width,
        window_height,
        0,
        0,
        buffer.width,
        buffer.height,
        Some(buffer.memory),
        &buffer.info,
        DIB_RGB_COLORS,
        SRCCOPY,
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

                let (window_width, window_height) = window_dimension(window);

                let mut paint: PAINTSTRUCT = PAINTSTRUCT::default();
                let hdc = BeginPaint(window, &mut paint);

                let x = paint.rcPaint.left;
                let y = paint.rcPaint.top;

                display_buffer_in_window(
                    hdc,
                    x,
                    y,
                    window_width,
                    window_height,
                    &GLOBAL_BACK_BUFFER,
                );
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
        resize_dib_section(&mut GLOBAL_BACK_BUFFER, 1280, 720);

        let instance = GetModuleHandleW(None)?;
        debug_assert!(instance.0 != 0);

        let window_class = w!("VoidedsHandmadeClass");

        let wc = WNDCLASSW {
            hCursor: Default::default(),
            hInstance: instance.into(),
            lpszClassName: window_class,

            style: CS_HREDRAW | CS_VREDRAW,
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

            render_weird_gradient(&GLOBAL_BACK_BUFFER, x_offset, y_offset);

            let device_context = GetDC(window);
            let (window_width, window_height) = window_dimension(window);

            display_buffer_in_window(
                device_context,
                0,
                0,
                window_width,
                window_height,
                &GLOBAL_BACK_BUFFER,
            );

            ReleaseDC(window, device_context);

            x_offset += 2;
            y_offset += 1;
        }

        Ok(())
    }
}
