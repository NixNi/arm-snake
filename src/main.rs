#![deny(unsafe_code)]
#![no_main]
#![no_std]

use panic_halt as _;
use stm32f4xx_hal::{self as hal};

use rand::{Rng, SeedableRng};
use rand_pcg::Pcg32;

use cortex_m_rt::entry;
use hal::{gpio::NoPin, pac, prelude::*};
use smart_leds::{brightness, SmartLedsWrite, RGB8};
use ws2812_spi as ws2812;

#[entry]
fn main() -> ! {
    let dp = pac::Peripherals::take().expect("cannot take peripherals");

    // Configure APB bus clock to 48 MHz, cause ws2812b requires 3 Mbps SPI
    let rcc = dp.RCC.constrain();
    let clocks = rcc.cfgr.use_hse(25.MHz()).sysclk(48.MHz()).freeze();

    let mut delay = dp.TIM1.delay_us(&clocks);
    let gpioa = dp.GPIOA.split();
    let gpiob = dp.GPIOB.split();
    let b_up = gpiob.pb0.into_pull_up_input();
    let b_down = gpiob.pb1.into_pull_up_input();
    let b_right = gpiob.pb2.into_pull_up_input();
    let b_left = gpiob.pb3.into_pull_up_input();
    let mut b_start = gpiob.pb4.into_pull_up_input();
    let b_pause = gpiob.pb5.into_pull_up_input();
    let b_stop = gpiob.pb6.into_pull_up_input();

    let spi = dp.SPI1.spi(
        (NoPin::new(), NoPin::new(), gpioa.pa7),
        ws2812::MODE,
        3000.kHz(),
        &clocks,
    );

    const NUM_LEDS: usize = 64;
    const RED: RGB8 = RGB8 {
        r: 207,
        g: 71,
        b: 71,
    };
    const GREEN: RGB8 = RGB8 {
        r: 110,
        g: 163,
        b: 55,
    };
    const BLACK: RGB8 = RGB8 {
        r: 19,
        g: 12,
        b: 28,
    };
    const YELLOW: RGB8 = RGB8 {
        r: 217,
        g: 210,
        b: 94,
    };
    const WHITE: RGB8 = RGB8 {
        r: 250,
        g: 250,
        b: 250,
    };
    let mut buffer = [0; NUM_LEDS * 12 + 20];
    let mut snake_buffer = [65u8; NUM_LEDS];
    #[derive(PartialEq)]
    enum DirectionEnum {
        UP,
        DOWN,
        RIGHT,
        LEFT,
    }
    let mut direction: DirectionEnum = DirectionEnum::LEFT;
    let mut ws = ws2812::prerendered::Ws2812::new(spi, buffer.as_mut_slice());
    let mut rng = Pcg32::seed_from_u64(0);
    let mut apple_pos: u8 = rng.gen_range(0..NUM_LEDS as u8);

    // Wait before start write for syncronization
    delay.delay(200.micros());

    pre_game(
        &mut ws,
        &mut delay,
        &mut b_start,
        &mut snake_buffer,
        &mut direction,
    );

    loop {
        //change snakes direction
        if b_pause.is_low() {
            pause(&mut ws, &mut delay, &mut b_start);
        }
        if b_stop.is_low() {
            pre_game(
                &mut ws,
                &mut delay,
                &mut b_start,
                &mut snake_buffer,
                &mut direction,
            );
        }
        if b_up.is_low() && !(direction == DirectionEnum::DOWN) {
            direction = DirectionEnum::UP;
        }
        if b_down.is_low() && !(direction == DirectionEnum::UP) {
            direction = DirectionEnum::DOWN;
        }
        if b_right.is_low() && !(direction == DirectionEnum::LEFT) {
            direction = DirectionEnum::RIGHT;
        }
        if b_left.is_low() && !(direction == DirectionEnum::RIGHT) {
            direction = DirectionEnum::LEFT;
        }

        if (((snake_buffer[0] % 8) == 0) && (direction == DirectionEnum::LEFT))
            || (((snake_buffer[0] % 8) == 7) && (direction == DirectionEnum::RIGHT))
            || ((snake_buffer[0] <= 7) && (direction == DirectionEnum::UP))
            || ((snake_buffer[0] >= 56) && (direction == DirectionEnum::DOWN))
        {
            pre_game(
                &mut ws,
                &mut delay,
                &mut b_start,
                &mut snake_buffer,
                &mut direction,
            );
        }

        let snake_end_index = snake_buffer.iter().position(|&i| i == 65).unwrap() - 1;
        let snake_tail_save = snake_buffer[snake_end_index];
        snake_buffer[snake_end_index] = 65;
        for i in (1..NUM_LEDS).rev() {
            snake_buffer[i] = snake_buffer[i - 1];
        }

        match direction {
            DirectionEnum::UP => {
                snake_buffer[0] -= 8;
            }
            DirectionEnum::DOWN => {
                snake_buffer[0] += 8;
            }
            DirectionEnum::LEFT => {
                snake_buffer[0] -= 1;
            }
            DirectionEnum::RIGHT => {
                snake_buffer[0] += 1;
            }
        }

        for scale in &snake_buffer[1..] {
            if *scale == snake_buffer[0] {
                pre_game(
                    &mut ws,
                    &mut delay,
                    &mut b_start,
                    &mut snake_buffer,
                    &mut direction,
                );
                break;
            }
        }

        if snake_buffer[0] == apple_pos {
            snake_buffer[snake_end_index + 1] = snake_tail_save;
            apple_pos = rng.gen_range(0..NUM_LEDS as u8);
            while snake_buffer.contains(&apple_pos) {
                apple_pos = rng.gen_range(0..NUM_LEDS as u8);
            }
        }

        let data = (0..NUM_LEDS).map(|i| {
            if snake_buffer.contains(&(i as u8)) {
                GREEN
            } else {
                if apple_pos == i as u8 {
                    RED
                } else {
                    BLACK
                }
            }
        });
        ws.write(brightness(data, 255)).unwrap();
        delay.delay(100.millis());
    }

    fn pre_game(
        ws: &mut ws2812_spi::prerendered::Ws2812<'_, stm32f4xx_hal::spi::Spi<pac::SPI1>>,
        delay: &mut stm32f4xx_hal::timer::Delay<pac::TIM1, 1000000>,
        b_start: &mut stm32f4xx_hal::gpio::Pin<'B', 4>,
        snake_buffer: &mut [u8; NUM_LEDS],
        direction: &mut DirectionEnum,
    ) {
        let s_array = [
            12u8, 13, 18, 19, 25, 34, 35, 36, 37, 40, 46, 49, 50, 51, 52, 53, 6, 14, 15,
        ];
        loop {
            let data = (0..NUM_LEDS).map(|i| {
                if s_array.contains(&(i as u8)) {
                    return GREEN;
                }
                if i == 7 || i == 5 {
                    return YELLOW;
                }
                if i == 23 {
                    return RED;
                }

                return BLACK;
            });
            ws.write(brightness(data, 255)).unwrap();
            delay.delay(100.millis());
            if b_start.is_low() {
                break;
            }
        }
        for elem in snake_buffer.iter_mut() {
            *elem = 65;
        }
        snake_buffer[0] = 28;
        snake_buffer[1] = 29;
        snake_buffer[2] = 30;
        // snake_buffer[3] = 31;
        // snake_buffer[4] = 39;
        // snake_buffer[5] = 38;
        *direction = DirectionEnum::LEFT;
    }

    fn pause(
        ws: &mut ws2812_spi::prerendered::Ws2812<'_, stm32f4xx_hal::spi::Spi<pac::SPI1>>,
        delay: &mut stm32f4xx_hal::timer::Delay<pac::TIM1, 1000000>,
        b_start: &mut stm32f4xx_hal::gpio::Pin<'B', 4>,
    ) {
        loop {
            let s_array = [10, 18, 26, 34, 42, 50, 13, 21, 29, 37, 45, 53];
            let data = (0..NUM_LEDS).map(|i| {
                if s_array.contains(&(i as u8)) {
                    return WHITE;
                }
                return BLACK;
            });
            ws.write(brightness(data, 255)).unwrap();
            delay.delay(100.millis());
            if b_start.is_low() {
                break;
            }
        }
    }
}
