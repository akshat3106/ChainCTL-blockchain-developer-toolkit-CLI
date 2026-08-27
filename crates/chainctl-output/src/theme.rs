/// Color/icon capability, resolved once at startup from `--no-color` and the
/// `NO_COLOR` convention (ARCHITECTURE.md §4 — `chainctl-output`).
#[derive(Debug, Clone, Copy)]
pub struct Theme {
    pub color: bool,
}

impl Theme {
    pub fn detect(no_color_flag: bool) -> Self {
        let color = !no_color_flag && std::env::var_os("NO_COLOR").is_none();
        Self { color }
    }
}
