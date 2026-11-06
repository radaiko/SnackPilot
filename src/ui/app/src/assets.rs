//! Bundled font assets (SIL OFL, see assets/fonts/OFL.txt).
//! Embedded so the spike needs no APK asset-loading path.

pub const INTER_REGULAR: &[u8] = include_bytes!("../assets/fonts/Inter-Regular.ttf");
pub const INTER_SEMIBOLD: &[u8] = include_bytes!("../assets/fonts/Inter-SemiBold.ttf");
pub const INTER_BOLD: &[u8] = include_bytes!("../assets/fonts/Inter-Bold.ttf");
