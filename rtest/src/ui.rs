use gio::prelude::*;
use gtk::prelude::*;
use gtk::*;
use log::info;

pub fn show_main_window() {
    let application = gtk::Application::new(Some("philipdaniels.com.rtest"), Default::default())
        .expect("Initialization failed...");

    application.connect_activate(|app| {
        // Load the compiled resource bundle
        let resources_bytes = include_bytes!("../resources/resources.gresource");
        let resource_data = glib::Bytes::from(&resources_bytes[..]);
        let res = gio::Resource::from_data(&resource_data).unwrap();
        gio::resources_register(&res);

        // Load the window UI
        let builder = Builder::from_resource("/rtest/main_window.glade");
        connect_callbacks(&builder);

        // Get a reference to the window
        let window: ApplicationWindow = builder
            .get_object("main_window")
            .expect("Couldn't get window");
        window.set_application(Some(app));

        // Show the UI
        window.show_all();
    });

    let args = vec![];
    application.run(&args);
}

fn connect_callbacks(builder: &Builder) {
    let menu = builder
        .get_object::<MenuItem>("mnuRedo")
        .expect("Couldn't get mnuRedo");
    menu.connect_activate(on_mnu_redo_activated);

    let menu = builder
        .get_object::<MenuItem>("mnuRefresh")
        .expect("Couldn't get mnuRefresh");
    menu.connect_activate(on_mnu_refresh_activated);
}

fn on_mnu_redo_activated(_menu_item: &MenuItem) {
    info!("on_mnu_redo_activated free function")
}

fn on_mnu_refresh_activated(_menu_item: &MenuItem) {
    info!("on_mnu_refresh_activated free function")
}
