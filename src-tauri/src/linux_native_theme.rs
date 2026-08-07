use gtk::prelude::*;
use std::cell::RefCell;

thread_local! {
    static MENU_STYLE_PROVIDER: RefCell<Option<gtk::CssProvider>> = const { RefCell::new(None) };
}

pub fn apply_menu_theme(dark: bool) -> Result<(), String> {
    let Some(screen) = gtk::gdk::Screen::default() else {
        return Err("GTK did not provide a screen for native menu styling".into());
    };

    let css = if dark {
        r#"
            menubar, menubar > menuitem {
                background-color: #202124;
                color: #f2f2f2;
            }
            menubar > menuitem:hover,
            menubar > menuitem:focus {
                background-color: #34363a;
                color: #ffffff;
            }
            menu, menu menuitem {
                background-color: #252629;
                color: #f2f2f2;
            }
            menu menuitem:hover,
            menu menuitem:focus {
                background-color: #3a3c41;
                color: #ffffff;
            }
            menu separator { background-color: #45474c; }
        "#
    } else {
        r#"
            menubar, menubar > menuitem {
                background-color: #f6f4ef;
                color: #292724;
            }
            menubar > menuitem:hover,
            menubar > menuitem:focus {
                background-color: #e4e0d8;
                color: #171614;
            }
            menu, menu menuitem {
                background-color: #faf8f3;
                color: #292724;
            }
            menu menuitem:hover,
            menu menuitem:focus {
                background-color: #e7e2d9;
                color: #171614;
            }
            menu separator { background-color: #d1cbc0; }
        "#
    };

    let provider = gtk::CssProvider::new();
    provider
        .load_from_data(css.as_bytes())
        .map_err(|error| format!("Could not load native GTK menu theme: {error}"))?;

    MENU_STYLE_PROVIDER.with(|slot| {
        if let Some(previous) = slot.borrow_mut().take() {
            gtk::StyleContext::remove_provider_for_screen(&screen, &previous);
        }
        gtk::StyleContext::add_provider_for_screen(
            &screen,
            &provider,
            gtk::STYLE_PROVIDER_PRIORITY_APPLICATION,
        );
        *slot.borrow_mut() = Some(provider);
    });

    Ok(())
}
