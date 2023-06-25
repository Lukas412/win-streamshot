use std::mem::size_of;
use std::ops::Not;
use windows::core::{Error, IntoParam};
use windows::Win32::Foundation::{ERROR_INVALID_PARAMETER, E_FAIL, HWND, RECT};
use windows::Win32::Graphics::Gdi::{
    CreateCompatibleBitmap, CreateCompatibleDC, CreatedHDC, DeleteDC, DeleteObject, GetDC,
    GetDIBits, ReleaseDC, SelectObject, BITMAPINFO, BITMAPINFOHEADER, BI_RGB, DIB_RGB_COLORS,
    HBITMAP, HDC,
};
use windows::Win32::Storage::Xps::{PrintWindow, PRINT_WINDOW_FLAGS};
use windows::Win32::UI::HiDpi::{SetProcessDpiAwareness, PROCESS_PER_MONITOR_DPI_AWARE};
use windows::Win32::UI::WindowsAndMessaging::{GetClientRect, GetWindowRect, PW_RENDERFULLCONTENT};

#[derive(Clone)]
pub(crate) struct Hdc {
    pub(crate) hdc: HDC,
}

impl Hdc {
    pub(crate) fn get_dc<P0>(hwnd: P0) -> Result<Hdc, Error>
    where
        P0: Into<HWND>,
    {
        unsafe {
            match GetDC(hwnd.into()) {
                e if e.is_invalid() => Err(Error::from_win32()),
                hdc => Ok(Hdc { hdc }),
            }
        }
    }
}

impl Drop for Hdc {
    fn drop(&mut self) {
        unsafe {
            ReleaseDC(HWND::default(), self.hdc);
        }
    }
}

impl From<&Hdc> for HDC {
    fn from(item: &Hdc) -> Self {
        item.hdc
    }
}

impl From<Hdc> for HDC {
    fn from(item: Hdc) -> Self {
        item.hdc
    }
}

#[allow(dead_code)]
#[derive(Debug)]
pub(crate) struct Rect {
    //pub(crate) rect: RECT,
    pub(crate) left: i32,
    pub(crate) top: i32,
    pub(crate) right: i32,
    pub(crate) bottom: i32,
    pub(crate) width: i32,
    pub(crate) height: i32,
}

impl Rect {
    pub(crate) fn get_client_rect<P0>(hwnd: P0) -> Result<Rect, Error>
    where
        P0: Into<HWND>,
    {
        let mut rect = RECT::default();
        unsafe {
            match GetClientRect(hwnd.into(), &mut rect).as_bool() {
                true => Ok(Rect {
                    left: rect.left,
                    top: rect.top,
                    right: rect.right,
                    bottom: rect.bottom,
                    width: rect.right - rect.left,
                    height: rect.bottom - rect.top,
                }),
                false => Err(Error::from_win32()),
            }
        }
    }
}

pub(crate) struct CreatedHdc {
    pub(crate) hdc: CreatedHDC,
}

impl CreatedHdc {
    pub(crate) fn create_compatible_dc<P0>(hdc: P0) -> Result<CreatedHdc, Error>
    where
        P0: IntoParam<HDC>,
    {
        unsafe {
            match CreateCompatibleDC(hdc) {
                e if e.is_invalid() => Err(Error::from_win32()),
                hdc => Ok(CreatedHdc { hdc }),
            }
        }
    }
}

impl From<&CreatedHdc> for HDC {
    fn from(item: &CreatedHdc) -> Self {
        HDC(item.hdc.0)
    }
}

impl From<CreatedHdc> for HDC {
    fn from(item: CreatedHdc) -> Self {
        HDC(item.hdc.0)
    }
}

impl Drop for CreatedHdc {
    fn drop(&mut self) {
        unsafe {
            DeleteDC(self.hdc);
        }
    }
}

pub(crate) struct Hbitmap {
    pub(crate) hbitmap: HBITMAP,
}

impl Hbitmap {
    pub(crate) fn create_compatible_bitmap<P0>(hdc: P0, w: i32, h: i32) -> Result<Hbitmap, Error>
    where
        P0: IntoParam<HDC>,
    {
        unsafe {
            match CreateCompatibleBitmap(hdc, w, h) {
                e if e.is_invalid() => Err(Error::from_win32()),
                hbitmap => Ok(Hbitmap { hbitmap }),
            }
        }
    }
}

impl Drop for Hbitmap {
    fn drop(&mut self) {
        unsafe {
            DeleteObject(self.hbitmap);
        }
    }
}

impl From<Hbitmap> for HBITMAP {
    fn from(item: Hbitmap) -> Self {
        item.hbitmap
    }
}

pub struct WindowBGRBuffer {
    handle: isize,
    width: u32,
    height: u32,
    buffer: Vec<u8>,
}

impl WindowBGRBuffer {
    pub fn read(&mut self) -> windows::core::Result<()> {
        self.buffer.clear();
        let hwnd = HWND(self.handle);

        unsafe {
            let _ = SetProcessDpiAwareness(PROCESS_PER_MONITOR_DPI_AWARE);
        }
        let hdc_screen = Hdc::get_dc(hwnd)?;

        let (rect_width, rect_height) = {
            let mut rect = RECT::default();
            unsafe {
                if GetWindowRect(hwnd, &mut rect).as_bool().not() {
                    return Err(Error::from_win32());
                };
            }
            (rect.right - rect.left, rect.bottom - rect.top)
        };

        let hdc = CreatedHdc::create_compatible_dc(hdc_screen.hdc)?;
        let hbitmap = Hbitmap::create_compatible_bitmap(hdc_screen.hdc, rect_width, rect_height)?;

        unsafe {
            if SelectObject(hdc.hdc, hbitmap.hbitmap).is_invalid() {
                return Err(Error::from_win32());
            }
        }

        let flags = PRINT_WINDOW_FLAGS(PW_RENDERFULLCONTENT);
        unsafe {
            if PrintWindow(hwnd, hdc.hdc, flags) == false {
                return Err(Error::from_win32());
            }
        }

        let bitmap_info_header = BITMAPINFOHEADER {
            biSize: size_of::<BITMAPINFOHEADER>() as u32,
            biPlanes: 1,
            biBitCount: 32,
            biWidth: rect_width,
            biHeight: -rect_height,
            biCompression: BI_RGB.0 as u32,
            ..Default::default()
        };
        let mut bit_map_info = BITMAPINFO {
            bmiHeader: bitmap_info_header,
            ..Default::default()
        };
        self.buffer.reserve((4 * rect_width * rect_height) as usize);

        unsafe {
            get_di_bits_into(
                &mut self.buffer,
                &mut bit_map_info,
                hdc.into(),
                hbitmap.hbitmap,
                rect_height as u32,
            )
        }
    }
}

pub unsafe fn get_di_bits_into(
    buffer: &mut Vec<u8>,
    bit_map_info: &mut BITMAPINFO,
    hdc: HDC,
    hbitmap: HBITMAP,
    height: u32,
) -> windows::core::Result<()> {
    let gdb = GetDIBits(
        hdc,
        hbitmap,
        0,
        height,
        Some(buffer.as_mut_ptr() as *mut core::ffi::c_void),
        bit_map_info,
        DIB_RGB_COLORS,
    );
    if gdb == 0 || gdb == ERROR_INVALID_PARAMETER.0 as i32 {
        return Err(Error::new(E_FAIL, "GetDIBits error".into()));
    }
    Ok(())
}
