/// 屏幕物理像素边界
#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
pub struct PhysicalBounds {
    pub x: i32,
    pub y: i32,
    pub width: i32,
    pub height: i32,
}

impl PhysicalBounds {
    pub fn new(x: i32, y: i32, width: i32, height: i32) -> Self {
        Self { x, y, width, height }
    }

    /// 根据 DPI 缩放系数由逻辑像素计算物理像素
    pub fn from_logical(x: f32, y: f32, width: f32, height: f32, scale_factor: f32) -> Self {
        let scale = if scale_factor > 0.0 { scale_factor } else { 1.0 };
        Self {
            x: (x * scale).round() as i32,
            y: (y * scale).round() as i32,
            width: (width * scale).round() as i32,
            height: (height * scale).round() as i32,
        }
    }
}
