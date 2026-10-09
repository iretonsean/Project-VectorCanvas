//! Text-input overlay (G6 §6, Flutter precedent G7 S.3).
//!
//! A real `<input>` is created for every text-input node and placed (by `mirror.rs`) at the
//! runtime's `text_input_rect`. It is transparent: the *canvas* draws text, caret and preedit.
//! The browser owns IME, autofill and the accessible text field; this module forwards
//! keyboard and composition events to the runtime (as `HostEvent`s) and never edits the DOM
//! value itself — the runtime's value flows back through the mirror.
use std::cell::Cell;
use std::rc::Rc;

use accesskit::NodeId;
use wasm_bindgen::prelude::*;
use wasm_bindgen::JsCast;
use web_sys::{CompositionEvent, Event, HtmlInputElement, InputEvent, KeyboardEvent};

use crate::mirror::EventSink;
use crate::HostEvent;

pub struct OverlayHandle {
    pub input: HtmlInputElement,
    /// True between compositionstart and compositionend.
    pub composing: Rc<Cell<bool>>,
    _listeners: Vec<Closure<dyn FnMut(Event)>>,
}

/// Keys the runtime edits with; the browser default is suppressed so the DOM value only ever
/// changes through the runtime.
fn is_edit_key(k: &str) -> bool {
    matches!(
        k,
        "Backspace" | "Delete" | "ArrowLeft" | "ArrowRight" | "ArrowUp" | "ArrowDown" | "Home" | "End" | "Enter" | "Escape"
    )
}

pub fn attach(
    input: &HtmlInputElement,
    id: NodeId,
    sink: Rc<dyn EventSink>,
    programmatic_focus: Rc<Cell<bool>>,
) -> Result<OverlayHandle, JsValue> {
    let composing = Rc::new(Cell::new(false));
    let mut ls: Vec<Closure<dyn FnMut(Event)>> = Vec::new();
    let mut add = |name: &str, cb: Closure<dyn FnMut(Event)>| -> Result<(), JsValue> {
        input.add_event_listener_with_callback(name, cb.as_ref().unchecked_ref())?;
        ls.push(cb);
        Ok(())
    };

    // keydown: named editing keys go to the runtime. IME-owned keydowns (keyCode 229 /
    // isComposing) are left alone.
    {
        let sink = sink.clone();
        add(
            "keydown",
            Closure::new(move |e: Event| {
                let ke: KeyboardEvent = e.clone().unchecked_into();
                if ke.is_composing() || ke.key_code() == 229 {
                    return;
                }
                let key = ke.key();
                let (shift, ctrl, alt, meta) = (ke.shift_key(), ke.ctrl_key(), ke.alt_key(), ke.meta_key());
                if key == "Tab" && !ctrl && !meta && !alt {
                    // Synchronous: if focus leaves the app the browser default must run (no keyboard trap).
                    if sink.tab(shift) {
                        e.prevent_default();
                    }
                } else if is_edit_key(&key) && !ctrl && !meta {
                    e.prevent_default();
                    sink.push(HostEvent::Key { key, shift, ctrl, alt });
                } else if ctrl || meta {
                    // Shortcuts (Ctrl+A, …): the runtime sees them; the browser default is not suppressed.
                    sink.push(HostEvent::Key { key, shift, ctrl: ctrl || meta, alt });
                }
            }),
        )?;
    }

    // beforeinput: printable text / paste -> runtime, default suppressed.
    {
        let sink = sink.clone();
        let composing = composing.clone();
        add(
            "beforeinput",
            Closure::new(move |e: Event| {
                let ie: InputEvent = e.clone().unchecked_into();
                let t = ie.input_type();
                if composing.get() || ie.is_composing() || t == "insertCompositionText" {
                    return; // composition events own this path
                }
                match t.as_str() {
                    "insertText" | "insertFromPaste" | "insertReplacementText" | "insertFromDrop" => {
                        if let Some(d) = ie.data() {
                            e.prevent_default();
                            sink.push(HostEvent::Text(d));
                        }
                    }
                    "deleteContentBackward" => {
                        e.prevent_default();
                        sink.push(HostEvent::Key { key: "Backspace".into(), shift: false, ctrl: false, alt: false });
                    }
                    "deleteContentForward" => {
                        e.prevent_default();
                        sink.push(HostEvent::Key { key: "Delete".into(), shift: false, ctrl: false, alt: false });
                    }
                    _ => {}
                }
                // The DOM value must track the runtime, not the browser edit.
                sink.push(HostEvent::ResyncInput(id));
            }),
        )?;
    }

    // Composition: preedit -> runtime while composing; commit on end.
    {
        let c = composing.clone();
        let sink = sink.clone();
        add(
            "compositionstart",
            Closure::new(move |_e: Event| {
                c.set(true);
                sink.push(HostEvent::Preedit { text: String::new(), cursor: None });
            }),
        )?;
    }
    {
        let sink = sink.clone();
        add(
            "compositionupdate",
            Closure::new(move |e: Event| {
                let ce: CompositionEvent = e.unchecked_into();
                let text = ce.data().unwrap_or_default();
                let n = text.chars().count();
                sink.push(HostEvent::Preedit { text, cursor: Some((n, n)) });
            }),
        )?;
    }
    {
        let c = composing.clone();
        let sink = sink.clone();
        add(
            "compositionend",
            Closure::new(move |e: Event| {
                c.set(false);
                let ce: CompositionEvent = e.unchecked_into();
                let text = ce.data().unwrap_or_default();
                sink.push(HostEvent::Preedit { text: String::new(), cursor: None });
                if !text.is_empty() {
                    sink.push(HostEvent::Commit(text));
                }
                sink.push(HostEvent::ResyncInput(id));
            }),
        )?;
    }
    // Native pointer selection inside the overlay -> runtime selection (UTF-16 units).
    for ev in ["pointerup", "select"] {
        let sink = sink.clone();
        let inp = input.clone();
        add(
            ev,
            Closure::new(move |_e: Event| {
                let s = inp.selection_start().ok().flatten().unwrap_or(0) as usize;
                let e = inp.selection_end().ok().flatten().unwrap_or(0) as usize;
                sink.push(HostEvent::Selection(id, s, e));
            }),
        )?;
    }
    // Focus (pointer click on the overlay, or AT) -> runtime focus.
    {
        let sink = sink.clone();
        add(
            "focus",
            Closure::new(move |_e: Event| {
                if !programmatic_focus.get() {
                    sink.push(HostEvent::Focus(id));
                }
            }),
        )?;
    }

    drop(add);
    Ok(OverlayHandle { input: input.clone(), composing, _listeners: ls })
}
