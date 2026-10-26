// A `;` list run in process (menus, hooks, key bindings) keeps tmux's order
// when one of its commands is forwarded to the server: that command and every
// command after it reach the server as ONE list, which it runs in order.
// Sent one by one, `new-window ; split-window -h ; select-pane -L ;
// split-window -v` ran the select before the splits existed and split the
// right pane instead of the left.

use super::*;

use std::io::Read;
use std::net::TcpListener;
use std::time::Duration;

/// Run `cmd` with a stand-in server on a local port and return every line
/// it was sent, the AUTH line excepted.
fn forwarded_lines(cmd: &str) -> Vec<String> {
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let mut app = AppState::new("chain_forward".to_string());
    app.control_port = Some(listener.local_addr().unwrap().port());
    app.session_key = "k".to_string();
    execute_command_string(&mut app, cmd).unwrap();

    listener.set_nonblocking(true).unwrap();
    let mut lines = Vec::new();
    while let Ok((mut stream, _)) = listener.accept() {
        stream.set_nonblocking(false).unwrap();
        stream.set_read_timeout(Some(Duration::from_millis(200))).unwrap();
        let mut text = String::new();
        let _ = stream.read_to_string(&mut text);
        lines.extend(text.lines().filter(|l| !l.starts_with("AUTH ")).map(str::to_string));
    }
    lines
}

#[test]
fn a_forwarded_command_takes_the_rest_of_its_list_with_it() {
    let lines = forwarded_lines(r#"new-window ; split-window -h -l "25%" ; select-pane -L ; split-window -v -l "20%" ; select-pane -U"#);
    assert_eq!(lines.len(), 1, "one message, not one per command: {lines:?}");
    let sent = crate::config::split_chained_commands_pub(&lines[0]);
    assert_eq!(sent, vec![
        "new-window",
        r#"split-window -h -l "25%""#,
        "select-pane -L",
        r#"split-window -v -l "20%""#,
        "select-pane -U",
    ]);
}

#[test]
fn a_single_forwarded_command_is_sent_as_it_was() {
    assert_eq!(forwarded_lines("new-window"), vec!["new-window".to_string()]);
}
