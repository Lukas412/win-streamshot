use {
  crate::wrappers::{CreatedHdcWrapper, HbitmapWrapper, HdcWrapper},
  std::{
    marker::PhantomData,
    mem::size_of,
    ops::{Deref, Not},
  },
  windows::{
    core::Error,
    Win32::{
      Foundation::{BOOL, ERROR_INVALID_PARAMETER, E_FAIL, HWND, LPARAM, RECT},
      Graphics::Gdi::{
        GetDIBits, SelectObject, BITMAPINFO, BITMAPINFOHEADER, BI_RGB, DIB_RGB_COLORS,
      },
      Storage::Xps::{PrintWindow, PRINT_WINDOW_FLAGS},
      UI::{
        HiDpi::{SetProcessDpiAwareness, PROCESS_PER_MONITOR_DPI_AWARE},
        WindowsAndMessaging::{
          EnumWindows, GetWindowRect, GetWindowTextLengthW, GetWindowTextW, IsWindowVisible,
          PW_RENDERFULLCONTENT,
        },
      },
    },
  },
};

pub struct WindowFinder {
  windows: Vec<Window>,
}

impl WindowFinder {
  pub fn new() -> windows::core::Result<Self> {
    Ok(Self {
      windows: get_windows()?,
    })
  }

  pub fn find(&self, name: &str) -> Option<windows::core::Result<WindowScreenshotBuffer>> {
    self
      .windows
      .iter()
      .filter(|window| window.name.contains(name))
      .next()
      .map(|window| WindowScreenshotBuffer::new(window.handle))
  }

  pub fn find_exact(&self, name: &str) -> Option<windows::core::Result<WindowScreenshotBuffer>> {
    self
      .windows
      .iter()
      .filter(|window| window.name == name)
      .next()
      .map(|window| WindowScreenshotBuffer::new(window.handle))
  }
}

struct Window {
  handle: HWND,
  name: String,
}

fn get_windows() -> windows::core::Result<Vec<Window>> {
  let mut windows = Vec::new();
  unsafe {
    let result = EnumWindows(
      Some(wl_callback),
      LPARAM(&mut windows as *mut Vec<Window> as isize),
    );
    if result == false {
      return Err(Error::from_win32());
    }
  }
  Ok(windows)
}

unsafe extern "system" fn wl_callback(hwnd: HWND, lparam: LPARAM) -> BOOL {
  let windows = lparam.0 as *mut Vec<Window>;

  if IsWindowVisible(hwnd) == false {
    return BOOL::from(true);
  }

  let window_text_length = GetWindowTextLengthW(hwnd);
  if window_text_length == 0 {
    return BOOL::from(true);
  }

  let mut name_buf: Vec<u16> = vec![0; (window_text_length + 1) as usize];
  if GetWindowTextW(hwnd, &mut name_buf) == 0 {
    return BOOL::from(true);
  }

  let name_buf = match name_buf.split_last() {
    Some((_, last)) => last,
    None => return BOOL::from(true),
  };

  let name = String::from_utf16_lossy(name_buf);
  (*windows).push(Window { handle: hwnd, name });

  BOOL::from(true)
}

mod wrappers;

pub struct WindowScreenshotBuffer {
  handle: HWND,
  width: i32,
  height: i32,
  buffer: Vec<u8>,
}

impl WindowScreenshotBuffer {
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

    Ok(Self {
      handle,
      width,
      height,
      buffer: vec![0; (4 * width * height) as usize],
    })
  }

  pub fn get_bgr_screenshot(&mut self) -> windows::core::Result<Screenshot<BGRA>> {
    self.read()?;
    Ok(Screenshot {
      width: self.width as u32,
      height: self.height as u32,
      image: &self.buffer,
      marker: PhantomData::default(),
    })
  }

  pub fn get_rgb_screenshot(&mut self) -> windows::core::Result<Screenshot<RGBA>> {
    self.read()?;
    self
      .buffer
      .chunks_exact_mut(4)
      .for_each(|pixel| pixel.swap(0, 2));
    Ok(Screenshot {
      width: self.width as u32,
      height: self.height as u32,
      image: &self.buffer,
      marker: PhantomData::default(),
    })
  }

  fn read(&mut self) -> windows::core::Result<()> {
    let hdc_screen = HdcWrapper::get_dc(self.handle)?;

    let hdc = CreatedHdcWrapper::create_compatible_dc(hdc_screen.inner())?;
    let hbitmap =
      HbitmapWrapper::create_compatible_bitmap(hdc_screen.inner(), self.width, self.height)?;

    unsafe {
      if SelectObject(hdc.inner(), hbitmap.inner()).is_invalid() {
        return Err(Error::from_win32());
      }
    }

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
        hdc.inner(),
        hbitmap.inner(),
        0,
        self.height as u32,
        Some(self.buffer.as_mut_ptr() as *mut core::ffi::c_void),
        &mut bit_map_info.clone(),
        DIB_RGB_COLORS,
      );
      if gdb == 0 || gdb == ERROR_INVALID_PARAMETER.0 as i32 {
        return Err(Error::new(E_FAIL, "GetDIBits error".into()));
      }
    }
    Ok(())
  }
}

struct BGRA;
struct RGBA;

pub struct Screenshot<'a, Color> {
  width: u32,
  height: u32,
  image: &'a Vec<u8>,
  marker: PhantomData<Color>,
}

impl<'a, Color> Screenshot<'a, Color> {
  pub fn width(&self) -> u32 {
    self.width
  }

  pub fn height(&self) -> u32 {
    self.height
  }

  pub fn total_pixels(&self) -> u32 {
    self.height * self.width
  }
}

impl<'a, Color> Deref for Screenshot<'a, Color> {
  type Target = &'a Vec<u8>;

  fn deref(&self) -> &Self::Target {
    &self.image
  }
}
