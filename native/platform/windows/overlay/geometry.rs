//! Physical placement derives from logical body size and the target work area.
pub const PAD: f32 = 8.0;
pub const BODY_WIDTH: f32 = 112.0;
pub const BODY_HEIGHT: f32 = 34.0;
pub const ERROR_WIDTH: f32 = 286.0;
pub const ERROR_HEIGHT: f32 = 88.0;
#[derive(Clone, Copy, PartialEq, Debug)]
pub struct Frame {
    pub x: i32,
    pub y: i32,
    pub width: i32,
    pub height: i32,
    pub dpi: u32,
    pub error: bool,
}
impl Frame {
    pub fn place(work: [i32; 4], dpi: u32, error: bool) -> Self {
        let dpi = dpi.clamp(96, 768);
        let scale = dpi as f32 / 96.0;
        let width =
            ((if error { ERROR_WIDTH } else { BODY_WIDTH } + PAD * 2.0) * scale).round() as i32;
        let height =
            ((if error { ERROR_HEIGHT } else { BODY_HEIGHT } + PAD * 2.0) * scale).round() as i32;
        Self {
            x: (work[0] + (work[2] - work[0] - width) / 2).max(work[0]),
            y: (work[3] - height - (24.0 * scale) as i32).max(work[1]),
            width,
            height,
            dpi,
            error,
        }
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn scaled_pill_stays_inside_negative_coordinate_work_area() {
        for dpi in [96, 144, 192] {
            for error in [false, true] {
                let frame = Frame::place([-2560, -200, 0, 1240], dpi, error);
                assert!(frame.x >= -2560 && frame.x + frame.width <= 0);
                assert!(frame.y >= -200 && frame.y + frame.height < 1240);
                if !error {
                    assert_eq!(frame.width, 128 * dpi as i32 / 96);
                    assert_eq!(frame.height, 50 * dpi as i32 / 96);
                }
            }
        }
    }
}
