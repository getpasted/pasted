use std::cell::RefCell;
use std::ffi::c_void;
use windows::Win32::Foundation::HWND;
use windows::Win32::System::Com::{
    CoCreateInstance, CoInitializeEx, CLSCTX_INPROC_SERVER, COINIT_MULTITHREADED,
};
use windows::Win32::UI::Accessibility::{CUIAutomation, IUIAutomation};

thread_local! {
    static AUTOMATION: RefCell<Option<IUIAutomation>> = const { RefCell::new(None) };
}

fn with_automation<T>(read: impl FnOnce(&IUIAutomation) -> Option<T>) -> Option<T> {
    unsafe {
        let _ = CoInitializeEx(None, COINIT_MULTITHREADED);
        AUTOMATION.with(|slot| {
            if slot.borrow().is_none() {
                *slot.borrow_mut() =
                    CoCreateInstance(&CUIAutomation, None, CLSCTX_INPROC_SERVER).ok();
            }
            let slot = slot.borrow();
            read(slot.as_ref()?)
        })
    }
}

fn text(value: windows::core::BSTR) -> Option<String> {
    let value = String::from_utf16_lossy(&value);
    (!value.trim().is_empty()).then_some(value)
}

pub(super) fn window_title(handle: u64) -> Option<String> {
    with_automation(|automation| unsafe {
        let element = automation
            .ElementFromHandle(HWND(handle as *mut c_void))
            .ok()?;
        element.CurrentName().ok().and_then(text)
    })
}

pub(super) fn focused_context(
    application: String,
) -> Result<crate::smart_paste::SmartPasteContext, String> {
    with_automation(|automation| unsafe {
        let element = automation.GetFocusedElement().ok()?;
        Some(crate::smart_paste::SmartPasteContext {
            application,
            role: element.CurrentLocalizedControlType().ok().and_then(text),
            label: element.CurrentName().ok().and_then(text),
            description: element.CurrentAutomationId().ok().and_then(text),
            help: element.CurrentHelpText().ok().and_then(text),
            placeholder: None,
        })
    })
    .ok_or_else(|| {
        "The focused field does not expose context to Windows UI Automation.".to_string()
    })
}
