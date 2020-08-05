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
    let button: Button = builder.get_object("btnMain").expect("Couldn't get btnMain");
    button.connect_clicked(on_btn_main_clicked);
}

fn on_btn_main_clicked(_btn: &Button) {
    info!("on_btn_main_clicked free function")
}
