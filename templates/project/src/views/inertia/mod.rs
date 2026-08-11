pub mod auth;
pub mod dashboard;
pub mod shared;

use serde::Serialize;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum InertiaRenderMode {
    Client,
    Ssr,
}

pub trait InertiaPage: Serialize {
    const COMPONENT: &'static str;
    const RENDER_MODE: InertiaRenderMode = InertiaRenderMode::Client;
}

pub const PAGE_COMPONENTS: &[(&str, &str)] = &[
    (
        dashboard::DashboardProps::COMPONENT,
        "resources/js/Pages/Dashboard.tsx",
    ),
    (
        auth::LoginProps::COMPONENT,
        "resources/js/Pages/Auth/Login.tsx",
    ),
    (
        auth::RegisterProps::COMPONENT,
        "resources/js/Pages/Auth/Register.tsx",
    ),
];

#[cfg(test)]
mod tests {
    use std::path::Path;

    use ts_rs::TS;

    use super::{
        InertiaPage, InertiaRenderMode, PAGE_COMPONENTS,
        auth::{LoginProps, RegisterProps},
        dashboard::DashboardProps,
        shared::{AuthProps, FlashProps, SafeUser, SharedProps},
    };

    #[test]
    fn export_bindings() {
        DashboardProps::export().unwrap();
        LoginProps::export().unwrap();
        RegisterProps::export().unwrap();
        SafeUser::export().unwrap();
        AuthProps::export().unwrap();
        FlashProps::export().unwrap();
        SharedProps::export().unwrap();
    }

    #[test]
    fn committed_bindings_are_current() {
        let snapshots = [
            (
                "resources/js/generated/DashboardProps.ts",
                include_str!("../../../resources/js/generated/DashboardProps.ts"),
            ),
            (
                "resources/js/generated/LoginProps.ts",
                include_str!("../../../resources/js/generated/LoginProps.ts"),
            ),
            (
                "resources/js/generated/RegisterProps.ts",
                include_str!("../../../resources/js/generated/RegisterProps.ts"),
            ),
            (
                "resources/js/generated/SafeUser.ts",
                include_str!("../../../resources/js/generated/SafeUser.ts"),
            ),
            (
                "resources/js/generated/AuthProps.ts",
                include_str!("../../../resources/js/generated/AuthProps.ts"),
            ),
            (
                "resources/js/generated/FlashProps.ts",
                include_str!("../../../resources/js/generated/FlashProps.ts"),
            ),
            (
                "resources/js/generated/SharedProps.ts",
                include_str!("../../../resources/js/generated/SharedProps.ts"),
            ),
        ];
        export_bindings();
        for (path, snapshot) in snapshots {
            let generated = std::fs::read_to_string(path).unwrap();
            assert_eq!(generated, snapshot, "stale TypeScript binding: {path}");
        }
    }

    #[test]
    fn every_registered_component_exists() {
        for (_, path) in PAGE_COMPONENTS {
            assert!(
                Path::new(env!("CARGO_MANIFEST_DIR")).join(path).is_file(),
                "missing {path}"
            );
        }
    }

    #[test]
    fn rendering_mode_is_selected_per_page() {
        assert_eq!(DashboardProps::RENDER_MODE, InertiaRenderMode::Ssr);
        assert_eq!(LoginProps::RENDER_MODE, InertiaRenderMode::Client);
        assert_eq!(RegisterProps::RENDER_MODE, InertiaRenderMode::Client);
    }
}
