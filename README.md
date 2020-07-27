# rtest

A testing GUI for Rust.

# Skills needed
* Learn Inkscape so I can create the icons for the buttons

# Things to create

- [ ] The main rtest window
  - [ ] Ability to flip between views using a tab at the top of the page
  - [ ] Save and restore position on a per-project basis

- [ ] The main window 'Tests' tab
  - [ ] Tab strip along the top to allow selection of main functions. Fixed size.
  - [ ] Left-hand side: Treeview showing the tests
  - [ ] Right-hand side: Details about the test output from `println` and `log`
        Also possibly the source code.
  - [ ] Config: The hierarchy in which the tests should be shown (module, file, flat)
  - [ ] Config: Module abbreviation system
  - [ ] Ability to click a test and open it in favourite editor
  - [ ] Config: specify the editor
  - [ ] Test filtering
    - [ ] Text box: allow typing in of filter expressions
    - [ ] By passing/failing/ignored - are there other categories
    - [ ] By module
  - [ ] Toolbar
    - [ ] Buttons to control the overall process

- [ ] Secondary tabs
  - [ ] The execution Queue display
  - [ ] Test coverage display
  - [ ] Statistics (lines of code etc.?)
  - [ ] (NTH) Settings (if we add a GUI editor)

- [ ] Widgets
  - [ ] A generic treeview widget
        https://docs.microsoft.com/en-us/dotnet/api/system.windows.forms.treeview?view=netcore-3.1
  - [ ] A generic TabStrip widget
  - [ ] A green/red percentage bar chart widget

- [ ] Assets and Theming
  - [ ] Icons for the toolbar. We want these to be SVG so they can be scaled to any size.
  - [ ] Light and dark default themes

- [ ] Shadow copy feature
  - [X] Config: specify a temp dir or a known dir (~/.rtest/...)
  - [ ] Config: concept of 'destination' which can be on another machine (build server) or local
        Tests are run in the destination dir. Initially, destination can be the same as the source
        and shadow copy can be a no-op
  - [X] Minimal copy on local disk
  - [X] Directory watcher, need to determine changed files
  - [X] Should trigger further work (e.g. sync, build, test) by placing items into the execution queue.
  - [ ] Config: Things to ignore (default to .gitignore)
  - [ ] Config: Things to include especially
  - [ ] Config: Ability to poll every N seconds instead of file watching
  - [ ] Better handling of errors in shadw_copy_destination.rs.

- [ ] Build engine
  - [ ] Config: Compile in Debug mode by default
  - [ ] Config: allow specification of features and flags and things?

- [ ] Test execution engine
  - [X] We need to discover all the tests
  - [X] Run all tests
  - [ ] Run specific tests
  - [ ] Calculate only the *affected* tests and run them automatically
  - [ ] Capture test output from `println` and `log`
  - [ ] Unit tests, integration tests, what types of tests are there?
  - [ ] Config: Ability to blacklist some tests, maybe by regex or module or type.
  - [ ] Ability to extract data and write it to a JSON or CSV file for further reporting.

- [ ] The execution Queue
  - [X] Define the types of things that can be put in the queue (sync, build, analyze, run tests)
  - [ ] Emission of events so progress can be monitored?
  - [X] Ability to pause and clear the queue
  - [ ] "Start again" feature: recreate a new shadow copy

- [ ] Configuration system
  - [ ] System, user, project, folder overrides
  - [ ] Q: Use environment variables too?

- [ ] Possible sub-crates
  - [ ] rsync replacement (from the Shadow Copy function)
  - [ ] The widgets, can go into Druid eventually
  - [ ] The core module, design as a library to allow multiple front-ends, e.g. a TUI one.
  - [ ] A graphing widget/library, similar to d3.js?

- [ ] Misc
  - [ ] Sounds for when a build or test run completes successfully or fails?

It is possible to link against cargo as a crate: https://github.com/roblabla/cargo-travis
But this has downsides (see the README above) and https://users.rust-lang.org/t/how-stable-is-cargos-message-format-json/15662
