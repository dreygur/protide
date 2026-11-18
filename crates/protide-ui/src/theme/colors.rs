use gpui::{Hsla, rgb};

/// Color palette for the application
#[derive(Clone)]
pub struct Colors {
    // Backgrounds
    pub bg_primary: Hsla,
    pub bg_secondary: Hsla,
    pub bg_tertiary: Hsla,
    pub bg_elevated: Hsla,

    // Text
    pub text_primary: Hsla,
    pub text_secondary: Hsla,
    pub text_muted: Hsla,

    // Borders
    pub border: Hsla,
    pub border_focused: Hsla,

    // Accent
    pub accent: Hsla,
    pub accent_hover: Hsla,

    // HTTP Methods
    pub method_get: Hsla,
    pub method_post: Hsla,
    pub method_put: Hsla,
    pub method_patch: Hsla,
    pub method_delete: Hsla,
    pub method_head: Hsla,
    pub method_options: Hsla,

    // Protocol colors
    pub protocol_ws: Hsla,
    pub protocol_grpc: Hsla,
    pub protocol_graphql: Hsla,

    // Status codes
    pub status_success: Hsla,
    pub status_redirect: Hsla,
    pub status_client_error: Hsla,
    pub status_server_error: Hsla,

    // Semantic colors
    pub success: Hsla,
    pub warning: Hsla,
    pub error: Hsla,
    pub info: Hsla,

    // Interactive states
    pub hover_overlay: Hsla,
    pub active_overlay: Hsla,
    pub selected_bg: Hsla,

    // Focus indicator
    pub focus_ring: Hsla,
    pub focus_ring_error: Hsla,

    // Collaboration / sync
    pub team_accent: Hsla,
    pub sync_active: Hsla,

    // Modal backdrop
    pub overlay: Hsla,
}

impl Colors {
    pub fn dark() -> Self {
        Self {
            // Backgrounds - Zed-style ultra-dark IDE palette
            bg_primary: rgb(0x0d0d0f).into(),   // app bg
            bg_secondary: rgb(0x111113).into(), // sidebar bg
            bg_tertiary: rgb(0x131315).into(),  // panel bg (request/response)
            bg_elevated: rgb(0x1b1b1e).into(),  // input/elevated bg

            // Text
            text_primary: rgb(0xe4e4ed).into(),
            text_secondary: rgb(0x7f7f92).into(),
            text_muted: rgb(0x3e3e4a).into(),

            // Borders
            border: rgb(0x252529).into(),
            border_focused: rgb(0x4ade80).into(),

            // Accent - green like Zed
            accent: rgb(0x4ade80).into(),
            accent_hover: rgb(0x6ee7a0).into(),

            // HTTP Methods - design spec colors
            method_get: rgb(0x4ade80).into(),     // green
            method_post: rgb(0x60a5fa).into(),    // blue
            method_put: rgb(0xfbbf24).into(),     // yellow
            method_patch: rgb(0xfb923c).into(),   // orange
            method_delete: rgb(0xf87171).into(),  // red
            method_head: rgb(0xa78bfa).into(),    // purple
            method_options: rgb(0x94a3b8).into(), // slate

            // Protocol colors
            protocol_ws: rgb(0x34d399).into(),      // emerald
            protocol_grpc: rgb(0x818cf8).into(),    // indigo
            protocol_graphql: rgb(0xf472b6).into(), // pink

            // Status codes
            status_success: rgb(0x4ade80).into(),
            status_redirect: rgb(0xfbbf24).into(),
            status_client_error: rgb(0xfb923c).into(),
            status_server_error: rgb(0xf87171).into(),

            // Semantic colors
            success: rgb(0x4ade80).into(),
            warning: rgb(0xfbbf24).into(),
            error: rgb(0xf87171).into(),
            info: rgb(0x60a5fa).into(),

            // Interactive states (white overlays for dark theme)
            hover_overlay: Hsla {
                h: 0.0,
                s: 0.0,
                l: 1.0,
                a: 0.035,
            },
            active_overlay: Hsla {
                h: 0.0,
                s: 0.0,
                l: 1.0,
                a: 0.055,
            },
            selected_bg: Hsla {
                h: 0.0,
                s: 0.0,
                l: 1.0,
                a: 0.055,
            },

            // Focus indicator
            focus_ring: rgb(0x4ade80).into(),
            focus_ring_error: rgb(0xf87171).into(),

            // Collaboration / sync
            team_accent: rgb(0x4ade80).into(), // green, matches accent
            sync_active: rgb(0x22c55e).into(), // brighter green for active state

            // Modal backdrop
            overlay: Hsla {
                h: 0.0,
                s: 0.0,
                l: 0.0,
                a: 0.5,
            },
        }
    }

    pub fn light() -> Self {
        Self {
            // Backgrounds
            bg_primary: rgb(0xffffff).into(),
            bg_secondary: rgb(0xf3f3f3).into(),
            bg_tertiary: rgb(0xe8e8e8).into(),
            bg_elevated: rgb(0xffffff).into(),

            // Text
            text_primary: rgb(0x1e1e1e).into(),
            text_secondary: rgb(0x616161).into(),
            text_muted: rgb(0x9e9e9e).into(),

            // Borders
            border: rgb(0xd4d4d4).into(),
            border_focused: rgb(0x007acc).into(),

            // Accent
            accent: rgb(0x007acc).into(),
            accent_hover: rgb(0x0066b8).into(),

            // HTTP Methods: the dark palette's hues, darkened to 4.5:1 on white
            method_get: rgb(0x188640).into(),
            method_post: rgb(0x0871f2).into(),
            method_put: rgb(0x966d03).into(),
            method_patch: rgb(0xbe5804).into(),
            method_delete: rgb(0xe90c0c).into(),
            method_head: rgb(0x460bf4).into(),
            method_options: rgb(0x627693).into(),

            // Protocol colors
            protocol_ws: rgb(0x059669).into(),
            protocol_grpc: rgb(0x4338ca).into(),
            protocol_graphql: rgb(0xdb2777).into(),

            // Status codes
            status_success: rgb(0x2e7d32).into(),
            status_redirect: rgb(0xef6c00).into(),
            status_client_error: rgb(0xc62828).into(),
            status_server_error: rgb(0xc62828).into(),

            // Semantic colors
            success: rgb(0x2e7d32).into(),
            warning: rgb(0xef6c00).into(),
            error: rgb(0xc62828).into(),
            info: rgb(0x007acc).into(),

            // Interactive states (black overlays for light theme)
            hover_overlay: Hsla {
                h: 0.0,
                s: 0.0,
                l: 0.0,
                a: 0.04,
            },
            active_overlay: Hsla {
                h: 0.0,
                s: 0.0,
                l: 0.0,
                a: 0.08,
            },
            selected_bg: Hsla {
                h: 203.0 / 360.0,
                s: 1.0,
                l: 0.4,
                a: 0.15,
            },

            // Focus indicator
            focus_ring: rgb(0x007acc).into(),
            focus_ring_error: rgb(0xc62828).into(),

            // Collaboration / sync
            team_accent: rgb(0x007acc).into(), // blue, matches accent
            sync_active: rgb(0x008000).into(), // darker green for active

            // Modal backdrop
            overlay: Hsla {
                h: 0.0,
                s: 0.0,
                l: 0.0,
                a: 0.4,
            },
        }
    }
}
