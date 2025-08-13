//! Input protocol bindings for UEFI to Slint.
//!
//! This will expose printable keys as well as a subset of special keys to Slint, as well
//! as the state of the mouse. In addition, it also provides a helper method
//! [`MouseState::draw_cursor`].

use core::time::Duration;

use bootmgr_rs_core::{
    BootResult,
    system::helper::{create_timer, locate_protocol},
};
use slint::{
    LogicalPosition,
    platform::{Key as SlintKey, PointerEventButton},
};
use uefi::{
    Event, ResultExt,
    boot::{self, ScopedProtocol, TimerTrigger},
    proto::console::{
        gop::BltPixel,
        pointer::{Pointer, PointerMode},
        text::{Key as UefiKey, ScanCode},
    },
};

use crate::app::App;

/// The size of the cursor.
const CURSOR_SIZE: usize = 5;

/// The main storage of mouse state.
pub struct MouseState {
    /// The pointer.
    pointer: ScopedProtocol<Pointer>,

    /// The current mode of the pointer.
    mode: PointerMode,

    /// The current position of the pointer.
    position: LogicalPosition,

    /// The mouse button that is currently being pressed
    button: PointerEventButton,

    /// If the pointer is disabled or not.
    disabled: bool,
}

impl MouseState {
    /// Get a new [`MouseState`].
    pub fn new() -> BootResult<Self> {
        let mut pointer = locate_protocol::<Pointer>()?;
        let mode = *pointer.mode();
        let position = LogicalPosition::new(0.0, 0.0);

        let disabled =
            pointer.reset(false).is_err() || mode.resolution[0] == 0 || mode.resolution[1] == 0;

        Ok(Self {
            pointer,
            mode,
            position,
            button: PointerEventButton::Other,
            disabled,
        })
    }

    /// Get the current state of the mouse, if there was any.
    ///
    /// This will record both mouse buttons being pressed as middle mouse button.
    #[allow(
        clippy::cast_precision_loss,
        reason = "f64 is exactly precise up to 2^53, which is more than enough"
    )]
    pub fn get_state(&mut self) -> Option<(LogicalPosition, PointerEventButton)> {
        if !self.disabled
            && let Ok(Some(state)) = self.pointer.read_state()
        {
            self.position.x += state.relative_movement[0] as f32 / self.mode.resolution[0] as f32;
            self.position.y += state.relative_movement[1] as f32 / self.mode.resolution[1] as f32;

            self.button = match state.button {
                [true, true] => PointerEventButton::Middle,
                [true, false] => PointerEventButton::Left,
                [false, true] => PointerEventButton::Right,
                [false, false] => PointerEventButton::Other,
            };

            Some((self.position, self.button))
        } else {
            None
        }
    }

    /// Get the color of the cursor.
    pub const fn color(&self) -> BltPixel {
        let _ = self;
        BltPixel::new(255, 255, 255)
    }

    /// Get the current position of the cursor.
    #[allow(
        clippy::cast_possible_truncation,
        reason = "The value of the position is unlikely to be high enough to matter"
    )]
    #[allow(
        clippy::cast_sign_loss,
        reason = "position.x and position.y are clamped to be always greater than 0.0 beforehand"
    )]
    pub const fn position(&self) -> (usize, usize) {
        (self.position.x as usize, self.position.y as usize)
    }

    /// Get the size of the cursor in dimensions.
    pub const fn dims(&self) -> (usize, usize) {
        let _ = self;
        (CURSOR_SIZE, CURSOR_SIZE)
    }

    /// Check if the cursor is enabled or not.
    pub const fn enabled(&self) -> bool {
        !self.disabled
    }

    /// Return an event that waits for the pointer to move.
    ///
    /// This simply delegates to the inner `pointer`.
    pub fn wait_for_input_event(&self) -> Option<Event> {
        self.pointer.wait_for_input_event()
    }
}

impl App {
    /// Handle a particular key, if there is any that is currently pressed.
    ///
    /// This is slightly different from how the ratatui frontend does it, because
    /// of how different slint's paradigm is. Animations and timers must be updated
    /// even while waiting for keys. Because of that reason, waiting for keys
    /// is separate from handling them.
    pub fn handle_key(&mut self) -> Option<char> {
        match self.input.read_key() {
            Ok(Some(UefiKey::Printable(char))) if char == '\r' => Some('\n'),
            Ok(Some(UefiKey::Printable(char))) => Some(char::from(char)),
            Ok(Some(UefiKey::Special(char))) => Some(
                match char {
                    ScanCode::LEFT => SlintKey::LeftArrow,
                    ScanCode::RIGHT => SlintKey::RightArrow,
                    ScanCode::ESCAPE => SlintKey::Escape,
                    _ => return None,
                }
                .into(),
            ),
            _ => None,
        }
    }

    /// Wait for an event.
    ///
    /// This will also clear the event queue every time it is called, because the duration may be different between calls.
    pub fn wait_for_events(&mut self, duration: Option<Duration>) -> BootResult<()> {
        if let Some(duration) = duration {
            let duration_time = u64::try_from(duration.as_nanos() / 100).unwrap_or(u64::MAX);
            let timer = create_timer(TimerTrigger::Relative(duration_time))?;
            let _ = self.events.push(timer);
        }

        boot::wait_for_event(&mut self.events).discard_errdata()?;
        self.events.clear();

        Ok(())
    }

    /// Create the input wait for key events.
    pub fn create_events(&mut self) {
        if let Some(event) = self.input.wait_for_key_event() {
            let _ = self.events.push(event);
        }

        if let Some(event) = self.mouse.wait_for_input_event() {
            let _ = self.events.push(event);
        }
    }
}
