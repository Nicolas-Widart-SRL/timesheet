//! Fonts embedded in the binary, shared by the UI and the PDF exports (SIL Open Font License, see assets/fonts).

pub const SANS_REGULAR: &[u8] = include_bytes!("../assets/fonts/InstrumentSans-Regular.ttf");
pub const SANS_MEDIUM: &[u8] = include_bytes!("../assets/fonts/InstrumentSans-Medium.ttf");
pub const SANS_SEMIBOLD: &[u8] = include_bytes!("../assets/fonts/InstrumentSans-SemiBold.ttf");
pub const SANS_BOLD: &[u8] = include_bytes!("../assets/fonts/InstrumentSans-Bold.ttf");
pub const MONO_REGULAR: &[u8] = include_bytes!("../assets/fonts/JetBrainsMono-Regular.ttf");
pub const MONO_SEMIBOLD: &[u8] = include_bytes!("../assets/fonts/JetBrainsMono-SemiBold.ttf");
