use log::info;
use druid::piet::Color;
use druid::widget::{Label, Button, Split, Flex, CrossAxisAlignment, MainAxisAlignment,
    SizedBox};
use druid::{Widget, WidgetExt};

type STATE = ();
const TABSTRIP_HEIGHT: f64 = 50.0;

fn build_tabstrip_button(text: &str) -> impl Widget<STATE> {
    let msg = format!("Clicked {}", text);

    let button = Button::new(text)
        .on_click(move |_event, _data: &mut STATE, _env| {
            info!("{}", msg);
        })
        .center()
        .border(Color::WHITE, 1.0)
        .padding(4.0);

    SizedBox::new(button)
        .height(TABSTRIP_HEIGHT)
        .width(100.0)
}

/// Construct the tabstrip at the top of the main window
fn build_tabstrip() -> impl Widget<STATE> {
    let tabstrip = Flex::row()
        .with_flex_child(build_tabstrip_button("Tests"), 1.0)
        .with_flex_child(build_tabstrip_button("Coverage"), 1.0)
        .with_flex_child(build_tabstrip_button("Stats"), 1.0)
        .with_flex_child(build_tabstrip_button("Queue"), 1.0)
        .with_flex_child(build_tabstrip_button("Settings"), 1.0);

    SizedBox::new(tabstrip).height(TABSTRIP_HEIGHT)
}

/// Construct the 'test panel'. This is the entire set of controls that
/// is displayed when the TESTS tab is selected.
fn build_test_panel() -> impl Widget<STATE> {
    // This is the toolbar at the top of the panel.
    let test_toolbar = Button::new("TEST TOOLBAR")
        .border(Color::WHITE, 1.0)
        .expand_width()
        .padding(4.0);
    let test_toolbar = SizedBox::new(test_toolbar).height(50.0);

    // This splitter contains the treeview on the LHS and the results on the RHS.
    let test_tree_splitter = Split::columns(
        Label::new("TEST TREE"),
        Label::new("TEST RESULTS"))
        .split_point(0.35)
        .draggable(true)
        .min_size(120.0)
        .border(Color::WHITE, 1.0)
        .expand()
        .padding(4.0);

    // This constructs the actual panel containing those two controls.
    Flex::column()
        .with_child(test_toolbar)
        .with_flex_child(test_tree_splitter, 1.0)
        .background(Color::rgb8(128,128,128))
        .expand()
}

/// Constructs the main window of the application.
pub fn build_main_window() -> impl Widget<STATE> {
    Flex::column()
        .cross_axis_alignment(CrossAxisAlignment::Center)
        .with_child(build_tabstrip())
        .with_flex_child(build_test_panel(), 1.0)
}