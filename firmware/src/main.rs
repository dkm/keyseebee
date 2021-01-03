#![no_main]
#![no_std]

// set the panic handler
use panic_halt as _;

extern crate smart_leds;
extern crate ws2812_spi;

use smart_leds::{brightness, colors, SmartLedsWrite, RGB8};

use ws2812_spi as ws2812;

use core::convert::Infallible;

use embedded_hal::digital::v2::{InputPin, OutputPin};

use generic_array::typenum::{U12, U5};

use hal::gpio::{gpioa, gpiob, Alternate, Input, Output, PullUp, PushPull, AF0};

use hal::prelude::*;

use embedded_hal::spi::FullDuplex;

use hal::usb;
use hal::{
    spi::{EightBit, Mode, Phase, Polarity},
    stm32, timers,
};

use keyberon::action::{k, l, m, Action, Action::*};
use keyberon::debounce::Debouncer;
use keyberon::impl_heterogenous_array;
use keyberon::key_code::KbHidReport;
use keyberon::key_code::KeyCode;
use keyberon::key_code::KeyCode::*;
use keyberon::layout::Layout;
use keyberon::matrix::{Matrix, PressedKeys};

use rtic::app;

use stm32f0xx_hal as hal;

use usb_device::bus::UsbBusAllocator;
use usb_device::class::UsbClass as _;
type Spi = hal::spi::Spi<
    stm32::SPI1,
    gpioa::PA5<Alternate<AF0>>,
    gpioa::PA6<Alternate<AF0>>,
    gpioa::PA7<Alternate<AF0>>,
    EightBit,
>;

type UsbClass = keyberon::Class<'static, usb::UsbBusType, Leds<Spi>>;

type UsbDevice = usb_device::device::UsbDevice<'static, usb::UsbBusType>;

trait ResultExt<T> {
    fn get(self) -> T;
}
impl<T> ResultExt<T> for Result<T, Infallible> {
    fn get(self) -> T {
        match self {
            Ok(v) => v,
            Err(e) => match e {},
        }
    }
}

pub struct Cols(
    gpioa::PA0<Input<PullUp>>,  // 12
    gpioa::PA1<Input<PullUp>>,  // 11
    gpiob::PB13<Input<PullUp>>, // 10
    gpiob::PB12<Input<PullUp>>, // 9
    gpiob::PB14<Input<PullUp>>, // 8
    gpiob::PB15<Input<PullUp>>, // 7
    gpioa::PA15<Input<PullUp>>, // 6
    gpiob::PB3<Input<PullUp>>,  // 5
    gpiob::PB4<Input<PullUp>>,  // 4
    gpiob::PB5<Input<PullUp>>,  // 3
    gpiob::PB8<Input<PullUp>>,  // 2
    gpiob::PB9<Input<PullUp>>,  // 1
);
impl_heterogenous_array! {
    Cols,
    dyn InputPin<Error = Infallible>,
    U12,
    [0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11]
}

pub struct Rows(
    gpiob::PB0<Output<PushPull>>,
    gpiob::PB1<Output<PushPull>>,
    gpiob::PB2<Output<PushPull>>,
    gpiob::PB10<Output<PushPull>>,
    gpiob::PB11<Output<PushPull>>,
);
impl_heterogenous_array! {
    Rows,
    dyn OutputPin<Error = Infallible>,
    U5,
    [0, 1, 2, 3, 4]
}

#[derive(Debug, Clone, Copy, Eq, PartialEq)]
pub enum CustomActions {
    LightUp,
    LightDown,

    ModeCycle,
    ColorCycle,
}

#[rustfmt::skip]
pub static LAYERS: keyberon::layout::Layers<CustomActions> = &[
    &[
        &[k(Kb1),     k(Kb2),     k(Kb3),    k(Kb4),     k(Kb5),  k(Grave),  k(Kb6),    k(Kb7),      k(Kb8),   k(Kb9),    k(Kb0),    k(Minus)],
        &[k(Q),       k(W),       k(E),      k(R),       k(T),    k(Tab),    k(Y),      k(U),        k(I),     k(O),      k(P),      k(LBracket)],
        &[k(A),       k(S),       k(D),      k(F),       k(G),    k(BSpace), k(H),      k(J),        k(K),     k(L),      k(SColon), k(Quote)],
        &[k(Z),       k(X),       k(C),      k(V),       k(B),    k(Enter),  k(N),      k(M),        k(Comma), k(Dot),    k(Slash),  k(Bslash)  ],
        &[k(LCtrl),   l(1),       k(LGui),   k(LShift),  k(LAlt), k(Space),  k(RAlt),   k(RBracket), k(Equal), k(Delete), k(RShift), k(RCtrl)],

    ], &[
        &[k(F1),          k(F2),      k(F3), k(F4),  k(F5),  k(F6),     k(F7),     k(F8),   k(F9),     k(F10), k(F11), k(F12)],
        &[k(SysReq),      k(NumLock), Trans, Trans,  Trans,  k(Escape), k(Insert), k(PgUp), k(PgDown), Trans,  Trans,  Trans ],
        &[Trans    ,      Trans     , Trans, Trans,  Trans,  Trans,     k(Home),   k(Up),   k(End),    Trans,  Trans,  Trans ],
        &[k(NonUsBslash), Action::Custom(CustomActions::ColorCycle),      Trans, Trans,  Trans,  Trans,     k(Left),   k(Down), k(Right),  Trans,  Trans,  k(PgUp) ],
        &[Action::Custom(CustomActions::LightUp),          Trans,      Action::Custom(CustomActions::LightDown), Action::Custom(CustomActions::ModeCycle),  Trans,  Trans,     Trans,     Trans,   Trans,     Trans,  Trans, Trans],
    ],
];

pub struct Leds<SPI> {
    ws: ws2812::Ws2812<SPI>,
    leds: [RGB8; 10],
}

impl<SPI, E> keyberon::keyboard::Leds for Leds<SPI>
where
    SPI: FullDuplex<u8, Error = E>,
{
    fn caps_lock(&mut self, status: bool) {
        if status {
            self.leds[0] = colors::BLUE;
        } else {
            self.leds[0] = colors::BLACK;
        }
        self.ws.write(brightness(self.leds.iter().cloned(), 10));
    }

    fn num_lock(&mut self, status: bool) {
        if status {
            self.leds[1] = colors::GREEN;
        } else {
            self.leds[1] = colors::BLACK;
        }
        self.ws.write(brightness(self.leds.iter().cloned(), 10));
    }

    fn compose(&mut self, status: bool) {
        if status {
            self.leds[3] = colors::VIOLET;
        } else {
            self.leds[3] = colors::BLACK;
        }
        self.ws.write(brightness(self.leds.iter().cloned(), 10));
    }
}

#[derive(Debug, Clone, Copy, Eq, PartialEq)]
pub enum BacklightMode {
    Off,
    Solid(RGB8),
    K2000(RGB8, usize, usize, bool),
    Fire,
    Breath(RGB8, bool),
}

pub struct Backlight {
    mode: BacklightMode,
    brightness: u8,
}

trait ColorSeq {
    fn next_color(&self) -> RGB8;
}

const COLORS_SEQ: [RGB8; 5] = [
    colors::RED,
    colors::GREEN,
    colors::BLUE,
    colors::VIOLET,
    colors::YELLOW,
];

impl ColorSeq for RGB8 {
    fn next_color(&self) -> RGB8 {
        let mut next = false;

        for c in &COLORS_SEQ {
            if next {
                return *c;
            }

            if *self == *c {
                next = true;
            }
        }

        COLORS_SEQ[0]
    }
}

impl Backlight {
    pub fn next_mode(&mut self) {
        self.mode = match self.mode {
            BacklightMode::Off => BacklightMode::Solid(colors::RED),
            BacklightMode::Solid(_) => BacklightMode::K2000(colors::RED, 0, 0, true),
            BacklightMode::K2000(_, _, _, _) => BacklightMode::Fire,
            BacklightMode::Fire => BacklightMode::Breath(colors::RED, true),
            BacklightMode::Breath(_, _) => BacklightMode::Off,
        }
    }

    pub fn next_color(&mut self) {
        self.mode = match self.mode {
            BacklightMode::Solid(c) => BacklightMode::Solid(c.next_color()),
            BacklightMode::Breath(c, dir) => BacklightMode::Breath(c.next_color(), dir),
            BacklightMode::K2000(c, s, i, dir) => BacklightMode::K2000(c.next_color(), s, i, dir),
            any => any,
        }
    }

    pub fn refresh_leds(&mut self, leds: &mut Leds<Spi>) {
        self.mode = match self.mode {
            BacklightMode::Off => {
                for l in leds.leds[4..].iter_mut() {
                    *l = colors::BLACK;
                }
                BacklightMode::Off
            }

            BacklightMode::Solid(c) => {
                for l in leds.leds[4..].iter_mut() {
                    *l = c;
                }
                BacklightMode::Solid(c)
            }

            BacklightMode::Breath(c, dir) => {
                for l in leds.leds[4..].iter_mut() {
                    *l = c;
                }

                let mut new_dir = dir;
                if dir {
                    if self.brightness == 100 {
                        self.brightness -= 1;
                        new_dir = false;
                    }
                    self.brightness += 1;
                } else {
                    if self.brightness == 0 {
                        self.brightness += 1;
                        new_dir = true;
                    }
                    self.brightness -= 1;
                }
                BacklightMode::Breath(c, new_dir)
            }

            BacklightMode::K2000(c, step, index, dir) => {
                let mut new_dir = dir;
                let mut new_index = index;

                let mut step = step + 1;

                if step == 100 {
                    step = 0;

                    if new_index == 0 && !dir {
                        new_index = 0;
                        new_dir = true;
                    } else if new_index == 5 && dir {
                        new_index = 5;
                        new_dir = false;
                    } else {
                        new_index = if dir { index + 1 } else { index - 1 };
                    }
                }

                for (i, l) in leds.leds[4..].iter_mut().enumerate() {
                    if i == new_index {
                        *l = c;
                    } else {
                        *l = colors::BLACK;
                    }
                }
                BacklightMode::K2000(c, step, new_index as usize, new_dir)
            }
            any => any,
        };

        leds.ws
            .write(brightness(leds.leds.iter().cloned(), self.brightness));
    }
}

#[app(device = crate::hal::pac, peripherals = true)]
const APP: () = {
    struct Resources {
        usb_dev: UsbDevice,
        usb_class: UsbClass,
        matrix: Matrix<Cols, Rows>,
        debouncer: Debouncer<PressedKeys<U5, U12>>,
        layout: Layout<CustomActions>,
        timer: timers::Timer<stm32::TIM3>,

        backlight: Backlight,
    }

    #[init]
    fn init(mut c: init::Context) -> init::LateResources {
        static mut USB_BUS: Option<UsbBusAllocator<usb::UsbBusType>> = None;

        let mut rcc = c
            .device
            .RCC
            .configure()
            .hsi48()
            .enable_crs(c.device.CRS)
            .sysclk(48.mhz())
            .pclk(24.mhz())
            .freeze(&mut c.device.FLASH);

        let gpioa = c.device.GPIOA.split(&mut rcc);
        let gpiob = c.device.GPIOB.split(&mut rcc);

        let usb = usb::Peripheral {
            usb: c.device.USB,
            pin_dm: gpioa.pa11,
            pin_dp: gpioa.pa12,
        };
        *USB_BUS = Some(usb::UsbBusType::new(usb));
        let usb_bus = USB_BUS.as_ref().unwrap();

        // Handling of ws2812 leds

        let pa5 = gpioa.pa5; // sck
        let pa6 = gpioa.pa6; // miso
        let pa7 = gpioa.pa7; // mosi

        // Configure pins for SPI
        let (sck, miso, mosi) = cortex_m::interrupt::free(move |cs| {
            (
                pa5.into_alternate_af0(cs),
                pa6.into_alternate_af0(cs),
                pa7.into_alternate_af0(cs),
            )
        });

        const MODE: Mode = Mode {
            polarity: Polarity::IdleHigh,
            phase: Phase::CaptureOnSecondTransition,
        };
        let spi = Spi::spi1(
            c.device.SPI1,
            (sck, miso, mosi),
            MODE,
            3_000_000.hz(),
            &mut rcc,
        );

        // ws2812
        let ws = ws2812::Ws2812::new(spi);

        let mut leds = Leds {
            ws,
            leds: [colors::BLACK; 10],
        };
        leds.ws.write(leds.leds.iter().cloned()).unwrap();

        let usb_class = keyberon::new_class(usb_bus, leds);
        let usb_dev = keyberon::new_device(usb_bus);

        let mut timer = timers::Timer::tim3(c.device.TIM3, 1.khz(), &mut rcc);
        timer.listen(timers::Event::TimeOut);

        let pa15 = gpioa.pa15;
        let pa1 = gpioa.pa1;
        let pa0 = gpioa.pa0;

        let matrix = cortex_m::interrupt::free(move |cs| {
            Matrix::new(
                Cols(
                    pa0.into_pull_up_input(cs),
                    pa1.into_pull_up_input(cs),
                    gpiob.pb13.into_pull_up_input(cs),
                    gpiob.pb12.into_pull_up_input(cs),
                    gpiob.pb14.into_pull_up_input(cs),
                    gpiob.pb15.into_pull_up_input(cs),
                    pa15.into_pull_up_input(cs),
                    gpiob.pb3.into_pull_up_input(cs),
                    gpiob.pb4.into_pull_up_input(cs),
                    gpiob.pb5.into_pull_up_input(cs),
                    gpiob.pb8.into_pull_up_input(cs),
                    gpiob.pb9.into_pull_up_input(cs),
                ),
                Rows(
                    gpiob.pb0.into_push_pull_output(cs),
                    gpiob.pb1.into_push_pull_output(cs),
                    gpiob.pb2.into_push_pull_output(cs),
                    gpiob.pb10.into_push_pull_output(cs),
                    gpiob.pb11.into_push_pull_output(cs),
                ),
            )
        });

        init::LateResources {
            usb_dev,
            usb_class,
            timer,
            debouncer: Debouncer::new(PressedKeys::default(), PressedKeys::default(), 5),
            matrix: matrix.get(),
            layout: Layout::new(LAYERS),

            backlight: Backlight {
                mode: BacklightMode::Off,
                brightness: 8,
            },
        }
    }

    #[task(binds = USB, priority = 4, resources = [usb_dev, usb_class])]
    fn usb_rx(c: usb_rx::Context) {
        if c.resources.usb_dev.poll(&mut [c.resources.usb_class]) {
            c.resources.usb_class.poll();
        }
    }

    #[task(
        binds = TIM3,
        priority = 2,
        resources = [matrix, debouncer, timer, layout, usb_class, backlight],
    )]
    fn tick(c: tick::Context) {
        c.resources.timer.wait().ok();

        for event in c
            .resources
            .debouncer
            .events(c.resources.matrix.get().unwrap())
        {
            c.resources.layout.event(event);
        }
        let mut usb_class = c.resources.usb_class;
        let backlight = c.resources.backlight;

        match c.resources.layout.tick() {
            keyberon::layout::CustomEvent::Release(CustomActions::LightUp) => {
                let bl_val = &mut backlight.brightness;
                *bl_val = if *bl_val == 100 { 100 } else { *bl_val + 1 };
                usb_class.lock(|k| {
                    let leds = k.device_mut().leds();
                    leds.ws
                        .write(brightness(leds.leds.iter().cloned(), *bl_val));
                });
            }
            keyberon::layout::CustomEvent::Release(CustomActions::LightDown) => {
                let bl_val = &mut backlight.brightness;
                *bl_val = if *bl_val == 0 { 0 } else { *bl_val - 1 };
                usb_class.lock(|k| {
                    let leds = k.device_mut().leds();
                    leds.ws
                        .write(brightness(leds.leds.iter().cloned(), *bl_val));
                });
            }
            keyberon::layout::CustomEvent::Release(CustomActions::ColorCycle) => {
                backlight.next_color();
            }
            keyberon::layout::CustomEvent::Release(CustomActions::ModeCycle) => {
                backlight.next_mode();
            }

            _ => (),
        }

        usb_class.lock(|k| {
            backlight.refresh_leds(&mut k.device_mut().leds());
        });

        c.resources.layout.tick();
        send_report(c.resources.layout.keycodes(), &mut usb_class);
    }

    extern "C" {
        fn CEC_CAN();
    }
};

fn send_report(iter: impl Iterator<Item = KeyCode>, usb_class: &mut resources::usb_class<'_>) {
    use rtic::Mutex;
    let report: KbHidReport = iter.collect();
    if usb_class.lock(|k| k.device_mut().set_keyboard_report(report.clone())) {
        while let Ok(0) = usb_class.lock(|k| k.write(report.as_bytes())) {}
    }
}
