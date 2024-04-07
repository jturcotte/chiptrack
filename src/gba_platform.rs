// Copyright © SixtyFPS GmbH <info@slint-ui.com>
// SPDX-License-Identifier: GPL-3.0-only OR LicenseRef-Slint-commercial

extern crate alloc;

pub mod renderer;

use crate::elog;
use crate::gba_platform::renderer::MainScreen;
use crate::log;
use crate::sound_renderer::SoundRenderer;
use crate::MainWindow;

use alloc::boxed::Box;
use alloc::rc::Rc;
use core::cell::RefCell;
use core::fmt::Write;

use embedded_alloc::Heap;
use gba::{
    mgba::{MgbaBufferedLogger, MgbaMessageLevel},
    prelude::*,
};
use slint::platform::software_renderer::MinimalSoftwareWindow;
use slint::platform::WindowEvent;
use slint::PlatformError;
use slint::SharedString;

#[alloc_error_handler]
fn oom(layout: core::alloc::Layout) -> ! {
    panic!("Out of memory {:?}", layout);
}

#[panic_handler]
fn panic_handler(info: &core::panic::PanicInfo) -> ! {
    elog!(
        "PANIC! {}:{}: {:?}",
        info.location().map_or("", |l| l.file()),
        info.location().map_or(0, |l| l.line()),
        info.message()
    );

    loop {}
}

const HEAP_SIZE: usize = 256 * 1024;

#[global_allocator]
static ALLOCATOR: Heap = Heap::empty();

const KEY_A: u16 = 0b00_00000001;
const KEY_B: u16 = 0b00_00000010;
const KEY_SELECT: u16 = 0b00_00000100;
const KEY_START: u16 = 0b00_00001000;
const KEY_RIGHT: u16 = 0b00_00010000;
const KEY_LEFT: u16 = 0b00_00100000;
const KEY_UP: u16 = 0b00_01000000;
const KEY_DOWN: u16 = 0b00_10000000;
const KEY_R: u16 = 0b01_00000000;
const KEY_L: u16 = 0b10_00000000;
const KEYS_ALL: u16 = 0b11_11111111;
const KEYS_REPEATABLE: u16 = 0b00_11110000;

// This is a type alias for the enabled `restore-state-*` feature.
// For example, it is `bool` if you enable `restore-state-bool`.
use critical_section::RawRestoreState;

struct GbaCriticalSection;
critical_section::set_impl!(GbaCriticalSection);

unsafe impl critical_section::Impl for GbaCriticalSection {
    unsafe fn acquire() -> RawRestoreState {
        true
    }
    unsafe fn release(_token: RawRestoreState) {}
}

struct GbaPlatform {
    main_screen: MainScreen,
    window: Rc<MinimalSoftwareWindow>,
}

static mut LAST_TIMER3_READ: u16 = 0;
static mut BASE_MILLIS_SINCE_START: u32 = 0;
// FIXME: Use GbaCell just to avoid unsafe?
static mut SOUND_RENDERER: Option<Rc<RefCell<SoundRenderer>>> = None;
static mut WINDOW: Option<slint::Weak<MainWindow>> = None;

static TRIGGERED_IRQS: GbaCell<u16> = GbaCell::new(0);

#[link_section = ".iwram"]
extern "C" fn irq_handler(b: IrqBits) {
    // IntrWait won't tell us which interrupts made it return from sleep,
    // so gather this information in the interrupt handler where this is told to us.
    TRIGGERED_IRQS.write(TRIGGERED_IRQS.read() | b.to_u16());
}

pub fn init() {
    unsafe { ALLOCATOR.init(0x02000000, HEAP_SIZE) }

    RUST_IRQ_HANDLER.write(Some(irq_handler));
    DISPSTAT.write(DisplayStatus::new().with_irq_vblank(true));
    KEYCNT.write(
        KeyControl::new()
            .with_a(true)
            .with_b(true)
            .with_select(true)
            .with_start(true)
            .with_right(true)
            .with_left(true)
            .with_up(true)
            .with_down(true)
            .with_r(true)
            .with_l(true)
            .with_irq_enabled(true),
    );
    IE.write(IrqBits::VBLANK.with_keypad(true));
    IME.write(true);

    // 16.78 MHz / (16*1024) = 1024 overflows per second
    // This means that each overflow will increment the cascaded TIMER3 each ~1ms.
    TIMER2_RELOAD.write(0xffff - 16);
    TIMER2_CONTROL.write(TimerControl::new().with_enabled(true).with_scale(TimerScale::_1024));
    TIMER3_CONTROL.write(TimerControl::new().with_enabled(true).with_cascade(true));

    let main_screen = MainScreen::new();
    let window = MinimalSoftwareWindow::new(Default::default());
    slint::platform::set_platform(Box::new(GbaPlatform { main_screen, window })).expect("backend already initialized");
}

pub fn set_sound_renderer(sound_renderer: Rc<RefCell<SoundRenderer>>) {
    unsafe { SOUND_RENDERER = Some(sound_renderer) }
}

// FIXME: Move as a platform method and attach here.
pub fn set_main_window(main_window: slint::Weak<MainWindow>) {
    unsafe { WINDOW = Some(main_window) }
}

impl slint::platform::Platform for GbaPlatform {
    fn create_window_adapter(&self) -> Result<Rc<dyn slint::platform::WindowAdapter>, PlatformError> {
        Ok(self.window.clone())
    }

    fn duration_since_start(&self) -> core::time::Duration {
        let timer3_read = TIMER3_COUNT.read();
        // FIXME: Don't use static mut
        let ms = unsafe {
            if timer3_read < LAST_TIMER3_READ {
                BASE_MILLIS_SINCE_START += 0xffff;
            }
            LAST_TIMER3_READ = timer3_read;
            BASE_MILLIS_SINCE_START + LAST_TIMER3_READ as u32
        };

        core::time::Duration::new(0, ms.wrapping_mul(1_000_000))
    }

    fn debug_log(&self, arguments: core::fmt::Arguments) {
        if let Ok(mut logger) = MgbaBufferedLogger::try_new(MgbaMessageLevel::Info) {
            write!(logger, "{}", arguments).ok();
        }
    }

    fn run_event_loop(&self) -> Result<(), PlatformError> {
        // FIXME: Those take iwram space by being put on the stack and could probably be used for something better.
        let slint_key_a: SharedString = 'x'.into();
        let slint_key_a_capital: SharedString = 'X'.into();
        let slint_key_b: SharedString = 'z'.into();
        let slint_key_b_capital: SharedString = 'Z'.into();
        let slint_key_select: SharedString = slint::platform::Key::Control.into();
        let slint_key_start: SharedString = slint::platform::Key::Return.into();
        let slint_key_right: SharedString = slint::platform::Key::RightArrow.into();
        let slint_key_left: SharedString = slint::platform::Key::LeftArrow.into();
        let slint_key_up: SharedString = slint::platform::Key::UpArrow.into();
        let slint_key_down: SharedString = slint::platform::Key::DownArrow.into();
        let slint_key_r: SharedString = slint::platform::Key::Shift.into();
        let slint_key_l: SharedString = slint::platform::Key::Alt.into();

        let main_screen = &self.main_screen;
        let window = self.window.clone();
        main_screen.attach_trackers();

        log!("--- Memory used before loop: {}kb", ALLOCATOR.used());

        let mut prev_keys = 0u16;
        let mut repeating_key_mask = 0u16;
        let mut prev_used = 0;
        let mut frames_until_repeat: Option<u16> = None;
        // IntrWait seems to never halt if ignore_existing_interrupts is false (which I need to process a possible
        // pending vblank after processing keys), and this somehow causes multiple missed frames.
        // So just assume that we're in mgba if mgba_logging_available() is true and skip just one frame
        // if keys are being handled while vblank is requested.
        let ignore_existing_interrupts = mgba_logging_available();

        loop {
            IntrWait(
                ignore_existing_interrupts,
                IrqBits::new().with_vblank(true).with_keypad(true),
            );
            let process_vblank = TRIGGERED_IRQS.read() & IrqBits::VBLANK.to_u16() != 0;
            // Processing the keys only after redrawing and then waiting for the next vblank would
            // mean that all key handlers would be delayed by one frame, including their effect on
            // the audio through instruments.
            // So also unblock the loop to process press and releases on keypad interupts so that we
            // can process key changes after we checked and went back to sleep.
            // Also check if KEYINPUT changed on vblank since mgba only seems to register key
            // releases during that time.
            let process_keys = process_vblank || TRIGGERED_IRQS.read() & IrqBits::KEYPAD.to_u16() != 0;
            TRIGGERED_IRQS.write(0);

            let cps = 16 * 1024 * 1024 / 1024;
            // Run main_screen.draw() before key handling to avoid missing the vblank window due to the heaving
            // processing happening in key handlers.
            if process_vblank {
                slint::platform::update_timers_and_animations();

                TIMER0_CONTROL.write(TimerControl::new().with_enabled(false));
                TIMER0_RELOAD.write(0);
                TIMER0_CONTROL.write(TimerControl::new().with_scale(TimerScale::_1024).with_enabled(true));
                main_screen.draw();
                let time = TIMER0_COUNT.read() as u32 * 1000 / cps;
                if time > 0 {
                    log!("--- main_screen.draw(ms) {}", time);
                }

                TIMER0_CONTROL.write(TimerControl::new().with_enabled(false));
                TIMER0_RELOAD.write(0);
                TIMER0_CONTROL.write(TimerControl::new().with_scale(TimerScale::_1024).with_enabled(true));
                unsafe {
                    SOUND_RENDERER
                        .as_ref()
                        .unwrap()
                        .borrow_mut()
                        .sound_engine
                        .advance_frame();
                }
                let time = TIMER0_COUNT.read() as u32 * 1000 / cps;
                if time > 0 {
                    log!("--- sound_engine.advance_frame(ms) {}", time);
                }

                if prev_used != ALLOCATOR.used() {
                    log!("--- Memory used: {}kb", ALLOCATOR.used());
                    prev_used = ALLOCATOR.used();
                }

                if let Some(frames) = frames_until_repeat.as_mut() {
                    *frames -= 1
                }
            }

            if process_keys {
                TIMER0_CONTROL.write(TimerControl::new().with_enabled(false));
                TIMER0_RELOAD.write(0);
                TIMER0_CONTROL.write(TimerControl::new().with_scale(TimerScale::_1024).with_enabled(true));

                let released_keys = KEYINPUT.read().to_u16();
                let switched_keys = released_keys ^ prev_keys;
                if switched_keys != 0 || frames_until_repeat == Some(0) {
                    let mut process_key = |key_mask: u16, out_key: &SharedString, capital_out_key: &SharedString| {
                        if switched_keys & key_mask != 0 {
                            if released_keys & key_mask == 0 {
                                // log!("PRESS {}", out_key.chars().next().unwrap() as u8);
                                // This isn't ideal but KEY_R is mapped to shift, and other platforms pass the key text capitalized
                                // when shift is pressed, which is taken into account in the event handling.
                                let text = if (released_keys & KEY_R) == 0 {
                                    capital_out_key.clone()
                                } else {
                                    out_key.clone()
                                };
                                window.dispatch_event(WindowEvent::KeyPressed { text });
                                if key_mask & KEYS_REPEATABLE != 0 {
                                    repeating_key_mask = key_mask;
                                    frames_until_repeat = Some(10);
                                }
                            } else {
                                // log!("RELEASE {}", out_key.chars().next().unwrap() as u8);
                                let text = if (released_keys & KEY_R) == 0 {
                                    capital_out_key.clone()
                                } else {
                                    out_key.clone()
                                };
                                window.dispatch_event(WindowEvent::KeyReleased { text });
                            }
                        }

                        if frames_until_repeat == Some(0)
                            && released_keys & key_mask == 0
                            && repeating_key_mask == key_mask
                        {
                            // log!("REPEAT {}", out_key.chars().next().unwrap() as u8);
                            let text = if (released_keys & KEY_R) == 0 {
                                capital_out_key.clone()
                            } else {
                                out_key.clone()
                            };
                            window.dispatch_event(WindowEvent::KeyPressed { text });
                            frames_until_repeat = Some(2);
                        }
                    };

                    process_key(KEY_A, &slint_key_a, &slint_key_a_capital);
                    process_key(KEY_B, &slint_key_b, &slint_key_b_capital);
                    process_key(KEY_SELECT, &slint_key_select, &slint_key_select);
                    process_key(KEY_START, &slint_key_start, &slint_key_start);
                    process_key(KEY_RIGHT, &slint_key_right, &slint_key_right);
                    process_key(KEY_LEFT, &slint_key_left, &slint_key_left);
                    process_key(KEY_UP, &slint_key_up, &slint_key_up);
                    process_key(KEY_DOWN, &slint_key_down, &slint_key_down);
                    process_key(KEY_R, &slint_key_r, &slint_key_r);
                    process_key(KEY_L, &slint_key_l, &slint_key_l);
                    prev_keys = released_keys;

                    if released_keys == KEYS_ALL {
                        frames_until_repeat = None;
                    }
                }

                let time = TIMER0_COUNT.read() as u32 * 1000 / cps;
                if time > 0 {
                    log!("--- process_key(ms) {}", time);
                }
            }
        }
    }
}
