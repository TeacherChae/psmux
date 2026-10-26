// pane-border-format is expanded per pane on the server (tmux
// screen-redraw.c expands it with each pane as the format target), and the
// result ships to the client as `border_label` from both layout serialisers.
// The client cannot expand it: conditionals and pane variables live in the
// server's state, so it used to draw everything but `#P` literally.
//
// No psmux server and no session: stub panes on a stub pty. Registered from
// src/layout.rs.

use super::*;

use std::sync::atomic::{AtomicBool, AtomicU64, AtomicU8};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

use crate::types::Node;

const ROWS: u16 = 10;
const COLS: u16 = 40;

fn make_pane(id: usize, title: &str) -> crate::types::Pane {
    let (master, writer) = crate::util::stub_pane_pty(portable_pty::PtySize {
        rows: ROWS,
        cols: COLS,
        pixel_width: 0,
        pixel_height: 0,
    });
    let epoch = Instant::now() - Duration::from_secs(2);
    crate::types::Pane {
        master,
        writer,
        child: crate::util::StubChild::exited(),
        term: Arc::new(Mutex::new(vt100::Parser::new(ROWS, COLS, 0))),
        last_rows: ROWS,
        last_cols: COLS,
        id,
        title: title.to_string(),
        title_locked: true,
        child_pid: None,
        data_version: Arc::new(AtomicU64::new(0)),
        last_title_check: epoch,
        last_infer_title: epoch,
        dead: false,
        last_text_input: None,
        last_special_key: None,
        vt_bridge_cache: None,
        vti_mode_cache: None,
        mouse_input_cache: None, win32_input_latched: false, input_off: false,
        scroll_fg_cache: None,
        mouse_proto_owner: None,
        wheel_auth: None,
        cursor_shape: Arc::new(AtomicU8::new(0)),
        bell_pending: Arc::new(AtomicBool::new(false)),
        cpr_pending: Arc::new(AtomicBool::new(false)),
        color_query_pending: Arc::new(std::sync::atomic::AtomicU32::new(0)),
        copy_state: None, live_term: None,
        pane_style: None,
        pane_options: Default::default(),
        squelch_until: None,
        output_ring: Arc::new(Mutex::new(std::collections::VecDeque::new())),
        spawned_at: None,
        start_command: String::new(),
        cwd_hint: None,
    }
}

/// Two side-by-side panes, %0 "left" (active) and %1 "right".
fn two_pane_app(border_status: Option<&str>) -> AppState {
    let mut app = AppState::new("borderlabel".to_string());
    app.window_base_index = 0;
    app.pane_base_index = 0;
    app.windows.push(crate::types::Window {
        root: Node::Split {
            kind: LayoutKind::Horizontal,
            sizes: vec![50, 50],
            children: vec![Node::Leaf(make_pane(0, "left")), Node::Leaf(make_pane(1, "right"))],
        },
        active_path: vec![0],
        name: "w".to_string(),
        id: 0,
        area: ratatui::layout::Rect::new(0, 0, COLS * 2, ROWS),
        window_size: None,
        window_options: Default::default(),
        activity_flag: false,
        bell_flag: false,
        silence_flag: false,
        last_output_time: Instant::now(),
        last_seen_version: 0,
        manual_rename: false,
        layout_index: 0,
        pane_mru: vec![],
        zoom_saved: None,
        linked_from: None,
        floating: Vec::new(),
        floating_focus: None,
    });
    app.active_idx = 0;
    if let Some(status) = border_status {
        app.user_options.insert("pane-border-status".to_string(), status.to_string());
    }
    app.user_options.insert(
        "pane-border-format".to_string(),
        "#[bold]#{?pane_active,ACTIVE ,}#P #D #{pane_title}".to_string(),
    );
    app
}

fn labels(layout: &LayoutJson) -> Vec<Option<String>> {
    match layout {
        LayoutJson::Leaf { border_label, .. } => vec![border_label.clone()],
        LayoutJson::Split { children, .. } => children.iter().flat_map(labels).collect(),
    }
}

fn parse(json: &str) -> LayoutJson {
    serde_json::from_str(json).expect("layout JSON parses")
}

#[test]
fn pane_shorthands_follow_the_format_target_pane() {
    let app = two_pane_app(Some("top"));
    // tmux aliases #P/#D to pane_index/pane_id of the target, not the
    // window's active pane (list-panes -F, per-pane border labels).
    assert_eq!(crate::format::expand_format_for_pane_by_id("#P #D", &app, 0), "0 %0");
    assert_eq!(crate::format::expand_format_for_pane_by_id("#P #D", &app, 1), "1 %1");
    // Without a target pane they still describe the active pane.
    assert_eq!(crate::format::expand_format("#P #D", &app), "0 %0");
}

#[test]
fn both_serialisers_ship_per_pane_expanded_labels() {
    let expected = vec![
        Some("#[bold]ACTIVE 0 %0 left".to_string()),
        Some("#[bold]1 %1 right".to_string()),
    ];
    let mut app = two_pane_app(Some("top"));
    assert_eq!(labels(&parse(&dump_layout_json(&mut app).unwrap())), expected);
    let mut app = two_pane_app(Some("top"));
    assert_eq!(labels(&parse(&dump_layout_json_fast(&mut app).unwrap())), expected);
}

#[test]
fn hidden_label_row_ships_no_labels() {
    for status in [None, Some("off")] {
        let mut app = two_pane_app(status);
        assert_eq!(labels(&parse(&dump_layout_json(&mut app).unwrap())), vec![None, None]);
        let mut app = two_pane_app(status);
        assert_eq!(labels(&parse(&dump_layout_json_fast(&mut app).unwrap())), vec![None, None]);
    }
}
