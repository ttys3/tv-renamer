use backend::common::{self, Arguments};
use gtk;
use gdk::enums::key;
use gtk::prelude::*;
use gtk::{
    Builder, Button, CheckButton, Entry, FileChooserDialog, ListBox, ListBoxRow, ListStore,
    SpinButton, TreeView, TreeViewColumn, Type, Window, WindowType
};
use std::fs;
use std::path::PathBuf;

// TODO:
// - Implement GTK3 notifications for when errors occur, and when renaming has completed successfully
// - Add a tab for checking the rename log.
// - Add support for a configuration file.
// - Reduce source code redundancy with macros and functions.

pub fn launch() {
    gtk::init().unwrap_or_else(|_| panic!("tv-renamer: failed to initialize GTK."));

    // Open the Glade GTK UI and import key GTK objects from the UI.
    let builder = Builder::new_from_string(include_str!("gtk_interface.glade"));
    let window: Window                  = builder.get_object("main_window").unwrap();
    let preview_button: Button          = builder.get_object("preview_button").unwrap();
    let rename_button: Button           = builder.get_object("rename_button").unwrap();
    let input_list: ListBox             = builder.get_object("input_list").unwrap();
    let series_name_entry: Entry        = builder.get_object("series_name_entry").unwrap();
    let series_directory_entry: Entry   = builder.get_object("series_directory_entry").unwrap();
    let series_directory_button: Button = builder.get_object("series_directory_button").unwrap();
    let episode_spin_button: SpinButton = builder.get_object("episode_spin_button").unwrap();
    let season_spin_button: SpinButton  = builder.get_object("season_spin_button").unwrap();
    let preview_tree: TreeView          = builder.get_object("preview_tree").unwrap();
    let info_bar: gtk::InfoBar          = builder.get_object("info_bar").unwrap();
    let info_button: Button             = builder.get_object("info_close").unwrap();
    let notification_label: gtk::Label  = builder.get_object("notification_label").unwrap();

    // Create rows for the input_list
    let automatic_row   = ListBoxRow::new();
    let log_changes_row = ListBoxRow::new();
    let series_name_row = ListBoxRow::new();
    let tvdb_row        = ListBoxRow::new();
    automatic_row.set_selectable(false);
    log_changes_row.set_selectable(false);
    series_name_row.set_selectable(false);
    tvdb_row.set_selectable(false);

    // Create check boxes for the rows
    let automatic_check   = CheckButton::new_with_label("Automatic");
    let no_name_check     = CheckButton::new_with_label("No Series Name");
    let tvdb_check        = CheckButton::new_with_label("TVDB Titles");
    let log_changes_check = CheckButton::new_with_label("Log Changes");

    // Add the check boxes to the rows
    automatic_row.add(&automatic_check);
    series_name_row.add(&no_name_check);
    tvdb_row.add(&tvdb_check);
    log_changes_row.add(&log_changes_check);

    // Add the rows to the list box
    input_list.insert(&automatic_row, -1);
    input_list.insert(&series_name_row, -1);
    input_list.insert(&tvdb_row, -1);
    input_list.insert(&log_changes_row, -1);

    // TreeView's List Store
    // Link these up to the preview_tree and then start renaming
    let preview_list = ListStore::new(&[
        Type::String, // Before
        Type::String, // After
    ]);

    // Create and append the Before column to the preview tree
    let before_column = TreeViewColumn::new();
    let renderer      = gtk::CellRendererText::new();
    before_column.set_title("Before");
    before_column.set_resizable(true);
    before_column.pack_start(&renderer, true);
    before_column.add_attribute(&renderer, "text", 0);
    preview_tree.append_column(&before_column);

    // Create and append the After column to the preview tree
    let after_column = TreeViewColumn::new();
    let renderer     = gtk::CellRendererText::new();
    after_column.set_title("After");
    after_column.set_resizable(true);
    after_column.pack_start(&renderer, true);
    after_column.add_attribute(&renderer, "text", 1);
    preview_tree.append_column(&after_column);

    // Connect the preview_list to the preview tree
    preview_tree.set_model(Some(&preview_list));
    preview_tree.set_headers_visible(true);

    { // Hide the Info Bar when the Info Bar is closed
        let info_bar = info_bar.clone();
        info_button.connect_clicked(move |_| {
            info_bar.hide();
        });
    }

    { // NOTE: Update the preview when the Automatic checkbutton is modified
        let auto                = automatic_check.clone();
        let no_name             = no_name_check.clone();
        let tvdb                = tvdb_check.clone();
        let log_changes         = log_changes_check.clone();
        let season_spin_button  = season_spin_button.clone();
        let episode_spin_button = episode_spin_button.clone();
        let series_entry        = series_name_entry.clone();
        let directory_entry     = series_directory_entry.clone();
        let preview_list        = preview_list.clone();
        let info_bar            = info_bar.clone();
        let notification_label  = notification_label.clone();
        automatic_check.connect_clicked(move |_| {
            if let Some(directory) = directory_entry.get_text() {
                let mut program = &mut Arguments {
                    automatic:     auto.get_active(),
                    dry_run:       false,
                    log_changes:   log_changes.get_active(),
                    no_name:       no_name.get_active(),
                    tvdb:          tvdb.get_active(),
                    verbose:       false,
                    directory:     directory,
                    series_name:   series_entry.get_text().unwrap_or_default(),
                    season_number: season_spin_button.get_value_as_int() as usize,
                    episode_count: episode_spin_button.get_value_as_int() as usize,
                    pad_length:    2,
                };

                if !program.directory.is_empty() { program.update_preview(&preview_list, &info_bar, &notification_label); }
            }
        });
    }

    { // NOTE: Update the preview when the TVDB checkbutton is modified
        let auto                = automatic_check.clone();
        let no_name             = no_name_check.clone();
        let tvdb                = tvdb_check.clone();
        let log_changes         = log_changes_check.clone();
        let season_spin_button  = season_spin_button.clone();
        let episode_spin_button = episode_spin_button.clone();
        let series_entry        = series_name_entry.clone();
        let directory_entry     = series_directory_entry.clone();
        let preview_list        = preview_list.clone();
        let info_bar            = info_bar.clone();
        let notification_label  = notification_label.clone();
        tvdb_check.connect_clicked(move |_| {
            if let Some(directory) = directory_entry.get_text() {
                let mut program = &mut Arguments {
                    automatic:     auto.get_active(),
                    dry_run:       false,
                    log_changes:   log_changes.get_active(),
                    no_name:       no_name.get_active(),
                    tvdb:          tvdb.get_active(),
                    verbose:       false,
                    directory:     directory,
                    series_name:   series_entry.get_text().unwrap_or_default(),
                    season_number: season_spin_button.get_value_as_int() as usize,
                    episode_count: episode_spin_button.get_value_as_int() as usize,
                    pad_length:    2,
                };

                if !program.directory.is_empty() { program.update_preview(&preview_list, &info_bar, &notification_label); }
            }
        });
    }

    { // NOTE: Update the preview when the "No Name In Series" checkbutton is modified
        let auto                = automatic_check.clone();
        let no_name             = no_name_check.clone();
        let tvdb                = tvdb_check.clone();
        let log_changes         = log_changes_check.clone();
        let season_spin_button  = season_spin_button.clone();
        let episode_spin_button = episode_spin_button.clone();
        let series_entry        = series_name_entry.clone();
        let directory_entry     = series_directory_entry.clone();
        let preview_list        = preview_list.clone();
        let info_bar            = info_bar.clone();
        let notification_label  = notification_label.clone();
        no_name_check.connect_clicked(move |_| {
            if let Some(directory) = directory_entry.get_text() {
                let mut program = &mut Arguments {
                    automatic:     auto.get_active(),
                    dry_run:       false,
                    log_changes:   log_changes.get_active(),
                    no_name:       no_name.get_active(),
                    tvdb:          tvdb.get_active(),
                    verbose:       false,
                    directory:     directory,
                    series_name:   series_entry.get_text().unwrap_or_default(),
                    season_number: season_spin_button.get_value_as_int() as usize,
                    episode_count: episode_spin_button.get_value_as_int() as usize,
                    pad_length:    2,
                };

                if !program.directory.is_empty() { program.update_preview(&preview_list, &info_bar, &notification_label); }
            }
        });
    }

    { // NOTE: Programs the Choose Directory button with a File Chooser Dialog.
        let auto                = automatic_check.clone();
        let no_name             = no_name_check.clone();
        let tvdb                = tvdb_check.clone();
        let log_changes         = log_changes_check.clone();
        let season_spin_button  = season_spin_button.clone();
        let episode_spin_button = episode_spin_button.clone();
        let series_entry        = series_name_entry.clone();
        let directory_entry     = series_directory_entry.clone();
        let preview_list        = preview_list.clone();
        let info_bar            = info_bar.clone();
        let notification_label  = notification_label.clone();
        series_directory_button.connect_clicked(move |_| {
            // Open file chooser dialog to modify series_directory_entry.
            let dialog = FileChooserDialog::new (
                Some("Choose Directory"),
                Some(&Window::new(WindowType::Popup)),
                gtk::FileChooserAction::SelectFolder,
            );
            dialog.add_button("Cancel", gtk::ResponseType::Cancel as i32);
            dialog.add_button("OK", gtk::ResponseType::Ok as i32);

            if dialog.run() == gtk::ResponseType::Ok as i32 {
                if let Some(path) = dialog.get_filename() {
                    if let Some(text) = path.to_str() {
                        directory_entry.set_text(text);
                    }
                }
            }
            dialog.destroy();

            if let Some(directory) = directory_entry.get_text() {
                let mut program = &mut Arguments {
                    automatic:     auto.get_active(),
                    dry_run:       false,
                    log_changes:   log_changes.get_active(),
                    no_name:       no_name.get_active(),
                    tvdb:          tvdb.get_active(),
                    verbose:       false,
                    directory:     directory,
                    series_name:   series_entry.get_text().unwrap_or_default(),
                    season_number: season_spin_button.get_value_as_int() as usize,
                    episode_count: episode_spin_button.get_value_as_int() as usize,
                    pad_length:    2,
                };

                if !program.directory.is_empty() { program.update_preview(&preview_list, &info_bar, &notification_label); }
            }
        });
    }

    { // NOTE: Controls what happens when the preview button is pressed
        let button              = preview_button.clone();
        let auto                = automatic_check.clone();
        let no_name             = no_name_check.clone();
        let tvdb                = tvdb_check.clone();
        let log_changes         = log_changes_check.clone();
        let season_spin_button  = season_spin_button.clone();
        let episode_spin_button = episode_spin_button.clone();
        let series_entry        = series_name_entry.clone();
        let directory_entry     = series_directory_entry.clone();
        let preview_list        = preview_list.clone();
        let info_bar            = info_bar.clone();
        let notification_label  = notification_label.clone();
        button.connect_clicked(move |_| {
            if let Some(directory) = directory_entry.get_text() {
                let mut program = &mut Arguments {
                    automatic:     auto.get_active(),
                    dry_run:       false,
                    log_changes:   log_changes.get_active(),
                    no_name:       no_name.get_active(),
                    tvdb:          tvdb.get_active(),
                    verbose:       false,
                    directory:     directory,
                    series_name:   series_entry.get_text().unwrap_or_default(),
                    season_number: season_spin_button.get_value_as_int() as usize,
                    episode_count: episode_spin_button.get_value_as_int() as usize,
                    pad_length:    2,
                };

                if !program.directory.is_empty() { program.update_preview(&preview_list, &info_bar, &notification_label); }
            }
        });
    }

    { // NOTE: Controls what happens when rename button is pressed
        let button              = rename_button.clone();
        let auto                = automatic_check.clone();
        let no_name             = no_name_check.clone();
        let tvdb                = tvdb_check.clone();
        let log_changes         = log_changes_check.clone();
        let season_spin_button  = season_spin_button.clone();
        let episode_spin_button = episode_spin_button.clone();
        let series_entry        = series_name_entry.clone();
        let directory_entry     = series_directory_entry.clone();
        let preview_list        = preview_list.clone();
        let info_bar            = info_bar.clone();
        let notification_label  = notification_label.clone();
        button.connect_clicked(move |_| {
            if let Some(directory) = directory_entry.get_text() {
                let mut program = &mut Arguments {
                    automatic:     auto.get_active(),
                    dry_run:       false,
                    log_changes:   log_changes.get_active(),
                    no_name:       no_name.get_active(),
                    tvdb:          tvdb.get_active(),
                    verbose:       false,
                    directory:     directory,
                    series_name:   series_entry.get_text().unwrap_or_default(),
                    season_number: season_spin_button.get_value_as_int() as usize,
                    episode_count: episode_spin_button.get_value_as_int() as usize,
                    pad_length:    2,
                };

                if !program.directory.is_empty() {
                    program.rename_series(&preview_list, &info_bar, &notification_label);
                }
            }
        });
    }

    window.show_all();
    info_bar.hide();


    // Quit the program when the program has been exited
    window.connect_delete_event(|_, _| {
        gtk::main_quit();
        Inhibit(false)
    });

    // Define custom actions on keypress
    window.connect_key_press_event(move |_, key| {
        if let key::Escape = key.get_keyval() { gtk::main_quit() }
        gtk::Inhibit(false)
    });

    gtk::main();

}

impl Arguments {
    fn update_preview(&mut self, preview_list: &ListStore, info_bar: &gtk::InfoBar, notification_label: &gtk::Label) {
        preview_list.clear();
        if self.automatic {
            let series = PathBuf::from(&self.directory);
            self.series_name = series.components().last().unwrap().as_os_str().to_str().unwrap().to_string();
            match common::get_seasons(&self.directory) {
                Ok(seasons) => {
                    for season in seasons {
                        match common::derive_season_number(&season) {
                            Some(number) => self.season_number = number,
                            None         => continue
                        }
                        if let Some(error) = self.rename_episodes(season.as_os_str().to_str().unwrap(), preview_list, true) {
                            info_bar.set_message_type(gtk::MessageType::Error);
                            notification_label.set_text(&error);
                            info_bar.show();
                        }
                    }
                },
                Err(err) => {
                    info_bar.set_message_type(gtk::MessageType::Error);
                    notification_label.set_text(err);
                    info_bar.show();
                }
            }
        } else if let Some(error) = self.rename_episodes(&self.directory, preview_list, true) {
            info_bar.set_message_type(gtk::MessageType::Error);
            notification_label.set_text(&error);
            info_bar.show();
        }
    }

    fn rename_series(&mut self, preview_list: &ListStore, info_bar: &gtk::InfoBar, notification_label: &gtk::Label) {
        preview_list.clear();
        if self.automatic {
            let series = PathBuf::from(&self.directory);
            self.series_name = series.components().last().unwrap().as_os_str().to_str().unwrap().to_string();
            match common::get_seasons(&self.directory) {
                Ok(seasons) => {
                    for season in seasons {
                        match common::derive_season_number(&season) {
                            Some(number) => self.season_number = number,
                            None         => continue
                        }
                        if let Some(error) = self.rename_episodes(season.as_os_str().to_str().unwrap(), preview_list, false) {
                            info_bar.set_message_type(gtk::MessageType::Error);
                            notification_label.set_text(&error);
                        } else {
                            info_bar.set_message_type(gtk::MessageType::Info);
                            notification_label.set_text("Rename Success");
                        }
                    }
                },
                Err(err) => {
                    info_bar.set_message_type(gtk::MessageType::Error);
                    notification_label.set_text(err);
                }
            }
        } else if let Some(error) = self.rename_episodes(&self.directory, preview_list, false) {
            info_bar.set_message_type(gtk::MessageType::Error);
            notification_label.set_text(&error);
        } else {
            info_bar.set_message_type(gtk::MessageType::Info);
            notification_label.set_text("Rename Success");
        }
        info_bar.show();
    }

    fn rename_episodes(&self, directory: &str, preview_list: &ListStore, dry_run: bool) -> Option<String> {
        match common::get_episodes(directory) {
            Ok(episodes) => {
                match self.get_targets(directory, &episodes, self.episode_count) {
                    Ok(targets) => {
                        if self.log_changes { common::log_append_time(); }
                        let mut error_occurred = false;
                        for (source, target) in episodes.iter().zip(targets) {
                            if !dry_run {
                                if fs::rename(&source, &target).is_err() { error_occurred = true; };
                                if self.log_changes { common::log_append_change(source.as_path(), target.as_path()); }
                            }

                            // Update the preview
                            let source = source.components().last().unwrap().as_os_str().to_str().unwrap().to_string();
                            let target = target.components().last().unwrap().as_os_str().to_str().unwrap().to_string();
                            preview_list.insert_with_values(None, &[0, 1], &[&source, &target]);
                        }
                        if error_occurred { Some(String::from("Rename Failed")) } else { None }
                    },
                    Err(err) => Some(err)
                }
            },
            Err(err) => Some(String::from(err))
        }
    }
}
