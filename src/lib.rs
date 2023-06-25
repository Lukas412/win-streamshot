use {
  crate::wrappers::{CreatedHdcWrapper, HbitmapWrapper, HdcWrapper},
  std::{mem::size_of, ops::Not},
  windows::{
    core::Error,
    Win32::{
      Foundation::{ERROR_INVALID_PARAMETER, E_FAIL, HWND, RECT},
      Graphics::Gdi::{
        GetDIBits, SelectObject, BITMAPINFO, BITMAPINFOHEADER, BI_RGB, DIB_RGB_COLORS,
      },
      Storage::Xps::{PrintWindow, PRINT_WINDOW_FLAGS},
      UI::{
        HiDpi::{SetProcessDpiAwareness, PROCESS_PER_MONITOR_DPI_AWARE},
        WindowsAndMessaging::{GetWindowRect, PW_RENDERFULLCONTENT},
      },
    },
  },
};

mod wrappers;

pub struct WindowBGRBuffer {
  handle: HWND,
  width: i32,
  height: i32,
  hdc: CreatedHdcWrapper,
  hbitmap: HbitmapWrapper,
  buffer: Vec<u8>,
}

impl WindowBGRBuffer {
  pub fn new(handle: HWND) -> windows::core::Result<Self> {
    unsafe {
      let _ = SetProcessDpiAwareness(PROCESS_PER_MONITOR_DPI_AWARE);
    }

    let mut rect = RECT::default();
    unsafe {
      if GetWindowRect(handle, &mut rect).as_bool().not() {
        return Err(Error::from_win32());
      };
    }
    let width = rect.right - rect.left;
    let height = rect.bottom - rect.top;

    let hdc_screen = HdcWrapper::get_dc(handle)?;

    let hdc = CreatedHdcWrapper::create_compatible_dc(hdc_screen.inner())?;
    let hbitmap = HbitmapWrapper::create_compatible_bitmap(hdc_screen.inner(), width, height)?;

    unsafe {
      if SelectObject(hdc.inner(), hbitmap.inner()).is_invalid() {
        return Err(Error::from_win32());
      }
    }

    let flags = PRINT_WINDOW_FLAGS(PW_RENDERFULLCONTENT);
    unsafe {
      if PrintWindow(handle, hdc.inner(), flags) == false {
        return Err(Error::from_win32());
      }
    }

    Ok(Self {
      handle,
      width,
      height,
      hdc,
      hbitmap,
      buffer: vec![0; (4 * width * height) as usize],
    })
  }

  pub fn height(&self) -> i32 {
    self.height
  }

  pub fn width(&self) -> i32 {
    self.width
  }

  pub fn buffer(&self) -> &Vec<u8> {
    &self.buffer
  }

  pub fn read(&mut self) -> windows::core::Result<()> {
    let bitmap_info_header = BITMAPINFOHEADER {
      biSize: size_of::<BITMAPINFOHEADER>() as u32,
      biPlanes: 1,
      biBitCount: 32,
      biWidth: self.width,
      biHeight: -self.height,
      biCompression: BI_RGB.0 as u32,
      ..Default::default()
    };
    let bit_map_info = BITMAPINFO {
      bmiHeader: bitmap_info_header,
      ..Default::default()
    };

    unsafe {
      let gdb = GetDIBits(
        self.hdc.inner(),
        self.hbitmap.inner(),
        0,
        self.height as u32,
        Some(self.buffer.as_mut_ptr() as *mut core::ffi::c_void),
        &mut bit_map_info.clone(),
        DIB_RGB_COLORS,
      );
      if gdb == 0 || gdb == ERROR_INVALID_PARAMETER.0 as i32 {
        return Err(Error::new(E_FAIL, "GetDIBits error".into()));
      }
      Ok(())
    }
  }
}
