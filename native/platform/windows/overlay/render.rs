//! Direct2D on a tiny premultiplied BGRA DIB; no web or backdrop blur.
use super::geometry::{Frame, BODY_HEIGHT, BODY_WIDTH, ERROR_HEIGHT, ERROR_WIDTH, PAD};
use crate::locale::text as tr;
use windows::core::w;
use windows::Win32::Foundation::*;
use windows::Win32::Graphics::{
    Direct2D::Common::*, Direct2D::*, DirectWrite::*, Dxgi::Common::*, Gdi::*,
};
use windows::Win32::UI::WindowsAndMessaging::*;

fn color(r: f32, g: f32, b: f32, a: f32) -> D2D1_COLOR_F {
    D2D1_COLOR_F { r, g, b, a }
}
fn system_color(index: SYS_COLOR_INDEX) -> D2D1_COLOR_F {
    let value = unsafe { GetSysColor(index) };
    color(
        (value & 255) as f32 / 255.0,
        ((value >> 8) & 255) as f32 / 255.0,
        ((value >> 16) & 255) as f32 / 255.0,
        1.0,
    )
}
fn rounded(x: f32, y: f32, width: f32, height: f32, radius: f32) -> D2D1_ROUNDED_RECT {
    D2D1_ROUNDED_RECT {
        rect: D2D_RECT_F {
            left: x,
            top: y,
            right: x + width,
            bottom: y + height,
        },
        radiusX: radius,
        radiusY: radius,
    }
}
fn ellipse(x: f32, y: f32, radius: f32) -> D2D1_ELLIPSE {
    let mut value = D2D1_ELLIPSE::default();
    value.point.X = x;
    value.point.Y = y;
    value.radiusX = radius;
    value.radiusY = radius;
    value
}
struct Dib {
    dc: HDC,
    bitmap: HBITMAP,
    previous: HGDIOBJ,
}
impl Dib {
    unsafe fn new(frame: Frame) -> windows::core::Result<Self> {
        let dc = CreateCompatibleDC(None);
        if dc.0.is_null() {
            return Err(windows::core::Error::from_win32());
        }
        let info = BITMAPINFO {
            bmiHeader: BITMAPINFOHEADER {
                biSize: std::mem::size_of::<BITMAPINFOHEADER>() as u32,
                biWidth: frame.width,
                biHeight: -frame.height,
                biPlanes: 1,
                biBitCount: 32,
                biCompression: BI_RGB.0,
                ..Default::default()
            },
            ..Default::default()
        };
        let mut pixels = std::ptr::null_mut();
        let bitmap = match CreateDIBSection(Some(dc), &info, DIB_RGB_COLORS, &mut pixels, None, 0) {
            Ok(bitmap) => bitmap,
            Err(error) => {
                let _ = DeleteDC(dc);
                return Err(error);
            }
        };
        let previous = SelectObject(dc, HGDIOBJ(bitmap.0));
        Ok(Self {
            dc,
            bitmap,
            previous,
        })
    }
}
impl Drop for Dib {
    fn drop(&mut self) {
        unsafe {
            SelectObject(self.dc, self.previous);
            let _ = DeleteObject(HGDIOBJ(self.bitmap.0));
            let _ = DeleteDC(self.dc);
        }
    }
}
pub enum Content<'a> {
    Listening(&'a [f32; 9]),
    Processing(f32),
    Success,
    Error(&'a str, bool),
}
pub struct Painter {
    target: ID2D1DCRenderTarget,
    ink: ID2D1SolidColorBrush,
    body: ID2D1LinearGradientBrush,
    glow: ID2D1RadialGradientBrush,
    pearl: ID2D1RadialGradientBrush,
    text: IDWriteTextFormat,
    dib: Dib,
    pub frame: Frame,
    high_contrast: bool,
}
impl Painter {
    pub fn new(frame: Frame, high_contrast: bool) -> windows::core::Result<Self> {
        unsafe {
            let dib = Dib::new(frame)?;
            let factory: ID2D1Factory = D2D1CreateFactory(D2D1_FACTORY_TYPE_SINGLE_THREADED, None)?;
            // Software rasterization of 128x50 avoids an extra D3D device/VRAM pool.
            let target = factory.CreateDCRenderTarget(&D2D1_RENDER_TARGET_PROPERTIES {
                r#type: D2D1_RENDER_TARGET_TYPE_SOFTWARE,
                pixelFormat: D2D1_PIXEL_FORMAT {
                    format: DXGI_FORMAT_B8G8R8A8_UNORM,
                    alphaMode: D2D1_ALPHA_MODE_PREMULTIPLIED,
                },
                dpiX: frame.dpi as f32,
                dpiY: frame.dpi as f32,
                ..Default::default()
            })?;
            target.BindDC(
                dib.dc,
                &RECT {
                    left: 0,
                    top: 0,
                    right: frame.width,
                    bottom: frame.height,
                },
            )?;
            let ink = target.CreateSolidColorBrush(&color(1.0, 1.0, 1.0, 1.0), None)?;
            let stops = target.CreateGradientStopCollection(
                &[
                    D2D1_GRADIENT_STOP {
                        position: 0.0,
                        color: color(0.125, 0.132, 0.148, 1.0),
                    },
                    D2D1_GRADIENT_STOP {
                        position: 1.0,
                        color: color(0.047, 0.052, 0.063, 1.0),
                    },
                ],
                D2D1_GAMMA_2_2,
                D2D1_EXTEND_MODE_CLAMP,
            )?;
            let mut linear = D2D1_LINEAR_GRADIENT_BRUSH_PROPERTIES::default();
            linear.startPoint.Y = PAD;
            linear.endPoint.Y = PAD + BODY_HEIGHT;
            let body = target.CreateLinearGradientBrush(&linear, None, &stops)?;
            let radial = |radius,
                          stops: &[D2D1_GRADIENT_STOP]|
             -> windows::core::Result<ID2D1RadialGradientBrush> {
                let stops = target.CreateGradientStopCollection(
                    stops,
                    D2D1_GAMMA_2_2,
                    D2D1_EXTEND_MODE_CLAMP,
                )?;
                let mut props = D2D1_RADIAL_GRADIENT_BRUSH_PROPERTIES::default();
                props.center.X = PAD + 25.0;
                props.center.Y = PAD + 17.0;
                props.radiusX = radius;
                props.radiusY = radius;
                target.CreateRadialGradientBrush(&props, None, &stops)
            };
            // Brushes/stop collections are cached for the visible session.
            let glow = radial(
                15.0,
                &[
                    D2D1_GRADIENT_STOP {
                        position: 0.0,
                        color: color(0.73, 0.72, 1.0, 0.45),
                    },
                    D2D1_GRADIENT_STOP {
                        position: 0.42,
                        color: color(0.45, 0.83, 0.96, 0.13),
                    },
                    D2D1_GRADIENT_STOP {
                        position: 1.0,
                        color: color(0.55, 0.7, 0.95, 0.0),
                    },
                ],
            )?;
            let pearl = radial(
                3.5,
                &[
                    D2D1_GRADIENT_STOP {
                        position: 0.0,
                        color: color(1.0, 1.0, 1.0, 1.0),
                    },
                    D2D1_GRADIENT_STOP {
                        position: 0.45,
                        color: color(0.88, 0.96, 1.0, 1.0),
                    },
                    D2D1_GRADIENT_STOP {
                        position: 1.0,
                        color: color(0.64, 0.72, 0.93, 1.0),
                    },
                ],
            )?;
            let write: IDWriteFactory = DWriteCreateFactory(DWRITE_FACTORY_TYPE_SHARED)?;
            let text = write.CreateTextFormat(
                w!("Segoe UI"),
                None,
                DWRITE_FONT_WEIGHT_NORMAL,
                DWRITE_FONT_STYLE_NORMAL,
                DWRITE_FONT_STRETCH_NORMAL,
                13.0,
                windows::core::PCWSTR(super::wide(tr("es", "en")).as_ptr()),
            )?;
            text.SetWordWrapping(DWRITE_WORD_WRAPPING_WRAP)?;
            target.SetTextAntialiasMode(D2D1_TEXT_ANTIALIAS_MODE_GRAYSCALE);
            Ok(Self {
                target,
                ink,
                body,
                glow,
                pearl,
                text,
                dib,
                frame,
                high_contrast,
            })
        }
    }
    unsafe fn fill(&self, rect: D2D1_ROUNDED_RECT, value: D2D1_COLOR_F) {
        self.ink.SetColor(&value);
        self.target.FillRoundedRectangle(&rect, &self.ink);
    }
    unsafe fn text(&self, value: &str, rect: D2D_RECT_F, value_color: D2D1_COLOR_F) {
        self.ink.SetColor(&value_color);
        self.target.DrawText(
            &value.encode_utf16().collect::<Vec<_>>(),
            &self.text,
            &rect,
            &self.ink,
            D2D1_DRAW_TEXT_OPTIONS_CLIP,
            DWRITE_MEASURING_MODE_NATURAL,
        );
    }
    pub fn draw(
        &self,
        hwnd: HWND,
        content: Content<'_>,
        opacity: f32,
    ) -> windows::core::Result<()> {
        unsafe {
            let (width, height) = if self.frame.error {
                (ERROR_WIDTH, ERROR_HEIGHT)
            } else {
                (BODY_WIDTH, BODY_HEIGHT)
            };
            self.target.BeginDraw();
            self.target.Clear(Some(&color(0.0, 0.0, 0.0, 0.0)));
            if !self.high_contrast {
                for i in (1..=5).rev() {
                    let spread = i as f32;
                    self.fill(
                        rounded(
                            PAD - spread,
                            PAD - spread + 1.6,
                            width + spread * 2.0,
                            height + spread * 2.0,
                            (if self.frame.error { 16.0 } else { 17.0 }) + spread,
                        ),
                        color(0.0, 0.0, 0.0, 0.023),
                    );
                }
                self.target.FillRoundedRectangle(
                    &rounded(
                        PAD,
                        PAD,
                        width,
                        height,
                        if self.frame.error { 16.0 } else { 17.0 },
                    ),
                    &self.body,
                );
                self.ink.SetColor(&color(0.82, 0.85, 0.93, 0.14));
            } else {
                self.fill(
                    rounded(PAD, PAD, width, height, 17.0),
                    system_color(COLOR_WINDOW),
                );
                self.ink.SetColor(&system_color(COLOR_WINDOWTEXT));
            }
            self.target.DrawRoundedRectangle(
                &rounded(
                    PAD + 0.5,
                    PAD + 0.5,
                    width - 1.0,
                    height - 1.0,
                    if self.frame.error { 15.5 } else { 16.5 },
                ),
                &self.ink,
                if self.high_contrast { 1.5 } else { 0.65 },
                None,
            );
            match content {
                Content::Error(message, recoverable) => {
                    let foreground = if self.high_contrast {
                        system_color(COLOR_WINDOWTEXT)
                    } else {
                        color(0.91, 0.93, 0.98, 1.0)
                    };
                    self.text(
                        message,
                        D2D_RECT_F {
                            left: PAD + 16.0,
                            top: PAD + 13.0,
                            right: PAD + width - 16.0,
                            bottom: PAD + 48.0,
                        },
                        foreground,
                    );
                    for (label, left, right) in [
                        (
                            if recoverable {
                                tr("Copiar", "Copy")
                            } else {
                                tr("Opciones", "Options")
                            },
                            16.0,
                            99.0,
                        ),
                        (tr("Reintentar", "Retry"), 109.0, 202.0),
                        (tr("Cerrar", "Close"), 212.0, 270.0),
                    ] {
                        self.fill(
                            rounded(PAD + left, PAD + 55.0, right - left, 24.0, 7.0),
                            if self.high_contrast {
                                system_color(COLOR_BTNFACE)
                            } else {
                                color(0.19, 0.20, 0.24, 1.0)
                            },
                        );
                        self.text(
                            label,
                            D2D_RECT_F {
                                left: PAD + left + 9.0,
                                top: PAD + 58.0,
                                right: PAD + right,
                                bottom: PAD + 80.0,
                            },
                            if self.high_contrast {
                                system_color(COLOR_BTNTEXT)
                            } else {
                                foreground
                            },
                        );
                    }
                }
                other => {
                    let (energy, pulse, bars) = match other {
                        Content::Listening(levels) => (
                            levels.iter().copied().fold(0.0_f32, f32::max),
                            1.0,
                            Some(levels),
                        ),
                        Content::Processing(pulse) => (0.0, pulse, None),
                        _ => (0.0, 1.0, None),
                    };
                    let x = PAD
                        + if bars.is_some() {
                            25.0
                        } else {
                            BODY_WIDTH / 2.0
                        };
                    let dot = ellipse(x, PAD + 17.0, 3.2);
                    if !self.high_contrast {
                        self.glow.SetCenter(dot.point);
                        self.pearl.SetCenter(dot.point);
                        self.glow
                            .SetOpacity((0.55 + energy * 2.0).clamp(0.55, 1.0) * pulse);
                        self.target
                            .FillEllipse(&ellipse(x, PAD + 17.0, 15.0), &self.glow);
                        self.pearl.SetOpacity(pulse);
                        self.target.FillEllipse(&dot, &self.pearl);
                    } else {
                        self.ink.SetColor(&system_color(COLOR_WINDOWTEXT));
                        self.target.FillEllipse(&dot, &self.ink);
                    }
                    if let Some(levels) = bars {
                        for (i, level) in levels.iter().enumerate() {
                            let height = 2.6 + 19.0 * (1.0 - (-level.max(0.0) * 24.0).exp());
                            self.fill(
                                rounded(
                                    PAD + 45.0 + i as f32 * 4.5,
                                    PAD + 17.0 - height / 2.0,
                                    2.2,
                                    height,
                                    1.1,
                                ),
                                if self.high_contrast {
                                    system_color(COLOR_WINDOWTEXT)
                                } else {
                                    color(0.83, 0.87, 0.95, 0.90)
                                },
                            );
                        }
                    }
                }
            }
            self.target.EndDraw(None, None)?;
            let blend = BLENDFUNCTION {
                BlendOp: AC_SRC_OVER as u8,
                SourceConstantAlpha: (opacity.clamp(0.0, 1.0) * 255.0).round() as u8,
                AlphaFormat: AC_SRC_ALPHA as u8,
                ..Default::default()
            };
            UpdateLayeredWindow(
                hwnd,
                None,
                Some(&POINT {
                    x: self.frame.x,
                    y: self.frame.y,
                }),
                Some(&SIZE {
                    cx: self.frame.width,
                    cy: self.frame.height,
                }),
                Some(self.dib.dc),
                Some(&POINT { x: 0, y: 0 }),
                COLORREF(0),
                Some(&blend),
                ULW_ALPHA,
            )
        }
    }
}
