// display-menu expands formats in its title, item names and item commands
// when the menu opens, as tmux's menu_add_item does (menu.c). Without it a
// label showed `#{@option}` literally and its command split with the
// unexpanded string as the size.

use super::*;

fn app_with_options() -> AppState {
    let mut app = AppState::new("menu_formats".to_string());
    app.user_options.insert("@right_pct".to_string(), "25".to_string());
    app.user_options.insert("@bottom_pct".to_string(), "20".to_string());
    app
}

#[test]
fn menu_names_and_commands_show_option_values() {
    let app = app_with_options();
    let mut menu = Menu {
        title: "Layout #{@right_pct}".to_string(),
        items: vec![
            MenuItem {
                name: "3: Split (R #{@right_pct}%, lower #{@bottom_pct}%)".to_string(),
                key: Some('3'),
                command: r##"split-window -h -l "#{@right_pct}%"; split-window -v -l "#{@bottom_pct}%""##.to_string(),
                is_separator: false,
            },
            MenuItem { name: String::new(), key: None, command: String::new(), is_separator: true },
        ],
        selected: 0,
        x: None,
        y: None,
    };
    expand_menu_formats(&mut menu, &app);
    assert_eq!(menu.title, "Layout 25");
    assert_eq!(menu.items[0].name, "3: Split (R 25%, lower 20%)");
    // `%` stays: the expansion is untimed, not strftime.
    assert_eq!(menu.items[0].command, r##"split-window -h -l "25%"; split-window -v -l "20%""##);
    assert!(menu.items[1].is_separator && menu.items[1].name.is_empty());
}

#[test]
fn display_menu_command_opens_an_expanded_menu() {
    let mut app = app_with_options();
    // One word per field: this path still splits menu items at whitespace.
    execute_command_string(&mut app, "display-menu -T Layouts R#{@right_pct}% r split-window").unwrap();
    let Mode::MenuMode { menu } = &app.mode else { panic!("menu did not open") };
    assert_eq!(menu.items[0].name, "R25%");
}
