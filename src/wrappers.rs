use windows::{
  core::Error,
  Win32::{
    Foundation::HWND,
    Graphics::Gdi::{
      CreateCompatibleBitmap, CreateCompatibleDC, CreatedHDC, DeleteDC, DeleteObject, GetDC,
      ReleaseDC, HBITMAP, HDC,
    },
  },
};

pub(crate) struct HdcWrapper {
  inner: HDC,
}

impl HdcWrapper {
  pub(crate) fn get_dc(hwnd: HWND) -> Result<HdcWrapper, Error> {
    unsafe {
      match GetDC(hwnd) {
        e if e.is_invalid() => Err(Error::from_win32()),
        hdc => Ok(HdcWrapper { inner: hdc }),
      }
    }
  }

  pub(crate) fn inner(&self) -> HDC {
    self.inner
  }
}

impl Drop for HdcWrapper {
  fn drop(&mut self) {
    unsafe {
      ReleaseDC(HWND::default(), self.inner);
    }
  }
}

pub(crate) struct CreatedHdcWrapper {
  inner: CreatedHDC,
}

impl CreatedHdcWrapper {
  pub(crate) fn create_compatible_dc(hdc: HDC) -> Result<CreatedHdcWrapper, Error> {
    unsafe {
      match CreateCompatibleDC(hdc) {
        error if error.is_invalid() => Err(Error::from_win32()),
        hdc => Ok(CreatedHdcWrapper { inner: hdc }),
      }
    }
  }

  pub(crate) fn inner(&self) -> HDC {
    HDC(self.inner.0)
  }
}

impl Drop for CreatedHdcWrapper {
  fn drop(&mut self) {
    unsafe {
      DeleteDC(self.inner);
    }
  }
}

pub(crate) struct HbitmapWrapper {
  inner: HBITMAP,
}

impl HbitmapWrapper {
  pub(crate) fn create_compatible_bitmap(
    hdc: HDC,
    w: i32,
    h: i32,
  ) -> Result<HbitmapWrapper, Error> {
    unsafe {
      match CreateCompatibleBitmap(hdc, w, h) {
        e if e.is_invalid() => Err(Error::from_win32()),
        hbitmap => Ok(HbitmapWrapper { inner: hbitmap }),
      }
    }
  }

  pub(crate) fn inner(&self) -> HBITMAP {
    self.inner
  }
}

impl Drop for HbitmapWrapper {
  fn drop(&mut self) {
    unsafe {
      DeleteObject(self.inner);
    }
  }
}
