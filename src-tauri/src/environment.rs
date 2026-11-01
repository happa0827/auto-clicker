#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Os {
    Windows,
    MacOs,
    Linux,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EnvInfo {
    pub os: Os,
    pub xdg_session_type: Option<String>,
    pub wayland_display: Option<String>,
    pub mac_accessibility_trusted: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Warning {
    WaylandUnsupported,
    MacAccessibilityMissing,
}

fn is_wayland(env: &EnvInfo) -> bool {
    match env.xdg_session_type.as_deref() {
        Some(kind) => kind.eq_ignore_ascii_case("wayland"),
        None => env.wayland_display.is_some(),
    }
}

pub fn check(env: &EnvInfo) -> Vec<Warning> {
    let mut warnings = vec![];
    if env.os == Os::Linux && is_wayland(env) {
        warnings.push(Warning::WaylandUnsupported);
    }
    if env.os == Os::MacOs && !env.mac_accessibility_trusted {
        warnings.push(Warning::MacAccessibilityMissing);
    }
    warnings
}

#[cfg(test)]
mod tests {
    use super::*;

    fn linux(session: Option<&str>, wayland_display: Option<&str>) -> EnvInfo {
        EnvInfo {
            os: Os::Linux,
            xdg_session_type: session.map(String::from),
            wayland_display: wayland_display.map(String::from),
            mac_accessibility_trusted: true,
        }
    }

    #[test]
    fn wayland_session_is_flagged_as_unsupported() {
        assert_eq!(check(&linux(Some("wayland"), Some("wayland-0"))), vec![Warning::WaylandUnsupported]);
        assert_eq!(check(&linux(Some("Wayland"), None)), vec![Warning::WaylandUnsupported]);
    }

    #[test]
    fn x11_session_has_no_warnings() {
        assert!(check(&linux(Some("x11"), None)).is_empty());
    }

    #[test]
    fn unknown_session_type_falls_back_to_the_wayland_display_variable() {
        assert_eq!(check(&linux(None, Some("wayland-0"))), vec![Warning::WaylandUnsupported]);
        assert!(check(&linux(None, None)).is_empty());
    }

    #[test]
    fn wayland_variables_are_ignored_off_linux() {
        let mut env = linux(Some("wayland"), Some("wayland-0"));
        env.os = Os::Windows;
        assert!(check(&env).is_empty());
    }

    #[test]
    fn macos_without_accessibility_permission_is_flagged() {
        let env = EnvInfo { os: Os::MacOs, xdg_session_type: None, wayland_display: None, mac_accessibility_trusted: false };
        assert_eq!(check(&env), vec![Warning::MacAccessibilityMissing]);
        let trusted = EnvInfo { mac_accessibility_trusted: true, ..env };
        assert!(check(&trusted).is_empty());
    }

    #[test]
    fn the_accessibility_flag_only_matters_on_macos() {
        let mut env = linux(Some("x11"), None);
        env.mac_accessibility_trusted = false;
        assert!(check(&env).is_empty());
    }
}
