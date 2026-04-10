/// Screen Capture
/// ===============
/// Captures the full display as a JPEG base64 string.
/// Mac: uses Core Graphics CGDisplayCreateImage
/// Windows: uses GDI BitBlt
///
/// JPEG quality 55 = good balance of quality vs Gemini token cost.
/// Lower quality = fewer tokens = cheaper + faster Gemini response.

use anyhow::Result;
use base64::{Engine as _, engine::general_purpose::STANDARD as BASE64};

pub struct ScreenCapture;

impl ScreenCapture {
    pub fn new() -> Self { Self }

    pub fn capture_jpeg_base64(&self, quality: u8) -> Result<String> {
        let image = self.capture_raw()?;
        let jpeg = self.encode_jpeg(&image, quality)?;
        Ok(BASE64.encode(&jpeg))
    }

    #[cfg(target_os = "macos")]
    fn capture_raw(&self) -> Result<RawImage> {
        use core_graphics::display::{CGDisplay, CGPoint, CGRect, CGSize};

        let display_id = CGDisplay::main().id;

        // Use CGWindowListCreateImage for screenshot
        let cg_image = unsafe {
            core_graphics::sys::CGWindowListCreateImage(
                core_graphics::display::CGRect::new(
                    &CGPoint::new(0.0, 0.0),
                    &CGSize::new(99999.0, 99999.0),
                ),
                core_graphics::display::kCGWindowListOptionOnScreenOnly,
                core_graphics::display::kCGNullWindowID,
                core_graphics::display::kCGWindowImageDefault,
            )
        };

        if cg_image.is_null() {
            anyhow::bail!("CGWindowListCreateImage returned null. Screen recording permission may not be granted.");
        }

        let cg_img = unsafe { core_graphics::image::CGImage::from_ptr(cg_image) };
        let width = cg_img.width() as u32;
        let height = cg_img.height() as u32;
        let bytes_per_row = cg_img.bytes_per_row();
        let raw_data = cg_img.data();
        let raw_bytes = raw_data.bytes();

        // Convert BGRA to RGB
        let mut rgb = Vec::with_capacity((width * height * 3) as usize);
        for y in 0..height as usize {
            for x in 0..width as usize {
                let offset = y * bytes_per_row + x * 4;
                if offset + 2 < raw_bytes.len() {
                    rgb.push(raw_bytes[offset + 2]); // R
                    rgb.push(raw_bytes[offset + 1]); // G
                    rgb.push(raw_bytes[offset]);     // B
                }
            }
        }

        Ok(RawImage { data: rgb, width, height })
    }

    #[cfg(target_os = "windows")]
    fn capture_raw(&self) -> Result<RawImage> {
        use windows::Win32::Graphics::Gdi::{
            BitBlt, CreateCompatibleBitmap, CreateCompatibleDC, DeleteDC,
            DeleteObject, GetDC, GetDIBits, ReleaseDC, SelectObject,
            BITMAPINFO, BITMAPINFOHEADER, BI_RGB, DIB_RGB_COLORS, SRCCOPY,
        };
        use windows::Win32::UI::WindowsAndMessaging::{GetDesktopWindow, GetSystemMetrics, SM_CXSCREEN, SM_CYSCREEN};

        unsafe {
            let width = GetSystemMetrics(SM_CXSCREEN) as u32;
            let height = GetSystemMetrics(SM_CYSCREEN) as u32;
            let hwnd = GetDesktopWindow();
            let hdc = GetDC(hwnd);
            let hdc_mem = CreateCompatibleDC(hdc);
            let hbmp = CreateCompatibleBitmap(hdc, width as i32, height as i32);
            SelectObject(hdc_mem, hbmp);
            BitBlt(hdc_mem, 0, 0, width as i32, height as i32, hdc, 0, 0, SRCCOPY);

            let row_size = ((width * 3 + 3) & !3) as usize;
            let mut buf = vec![0u8; row_size * height as usize];

            let mut bmi = BITMAPINFO {
                bmiHeader: BITMAPINFOHEADER {
                    biSize: std::mem::size_of::<BITMAPINFOHEADER>() as u32,
                    biWidth: width as i32,
                    biHeight: -(height as i32),
                    biPlanes: 1,
                    biBitCount: 24,
                    biCompression: BI_RGB.0,
                    ..Default::default()
                },
                ..Default::default()
            };

            GetDIBits(hdc_mem, hbmp, 0, height, Some(buf.as_mut_ptr() as *mut _), &mut bmi, DIB_RGB_COLORS);

            // BGR → RGB, remove row padding
            let mut rgb = Vec::with_capacity((width * height * 3) as usize);
            for y in 0..height as usize {
                for x in 0..width as usize {
                    let o = y * row_size + x * 3;
                    rgb.push(buf[o + 2]);
                    rgb.push(buf[o + 1]);
                    rgb.push(buf[o]);
                }
            }

            DeleteObject(hbmp);
            DeleteDC(hdc_mem);
            ReleaseDC(hwnd, hdc);

            Ok(RawImage { data: rgb, width, height })
        }
    }

    fn encode_jpeg(&self, image: &RawImage, quality: u8) -> Result<Vec<u8>> {
        use image::{ImageBuffer, Rgb};
        use std::io::Cursor;

        let img = ImageBuffer::<Rgb<u8>, _>::from_raw(
            image.width, image.height, image.data.clone()
        ).ok_or_else(|| anyhow::anyhow!("Failed to create image buffer"))?;

        let mut output = Cursor::new(Vec::new());
        let mut encoder = image::codecs::jpeg::JpegEncoder::new_with_quality(&mut output, quality);
        encoder.encode_image(&img)?;
        Ok(output.into_inner())
    }
}

struct RawImage {
    data: Vec<u8>,
    width: u32,
    height: u32,
}
