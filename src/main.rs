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

    let spi = dp.SPI1.spi(
        (NoPin::new(), NoPin::new(), gpioa.pa7),
        ws2812::MODE,
        3000.kHz(),
        &clocks,
    );


    
    const NUM_LEDS: usize = 64;
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
    snake_buffer[0] = 28;
    snake_buffer[1] = 29;
    snake_buffer[2] = 30;
    // snake_buffer[3] = 31;
    // snake_buffer[4] = 39;
    // snake_buffer[5] = 38;
    // snake_buffer[6] = 37;
    // snake_buffer[7] = 36;
    // snake_buffer[8] = 35;
    // snake_buffer[9] = 34;
    // snake_buffer[10] = 33;
    // snake_buffer[11] = 32;
    

    let mut ws = ws2812::prerendered::Ws2812::new(spi, buffer.as_mut_slice());
    let mut rng = Pcg32::seed_from_u64(0);
    let mut apple_pos: u8 = rng.gen_range(0..NUM_LEDS as u8);

    // Wait before start write for syncronization
    delay.delay(200.micros());

    'main_loop:loop {
        //change snakes direction
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
        {
            break;
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

        if snake_buffer[0] > 64 {
            break;
        }

        for scale in &snake_buffer[1..] {
            if *scale == snake_buffer[0] {break 'main_loop;}
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
                RGB8 { r: 0, g: 255, b: 0 }
            } else {
                if apple_pos == i as u8 {
                    RGB8 { r: 255, g: 0, b: 0 }
                } else {
                    RGB8 { r: 0, g: 0, b: 0 }
                }
            }
        });
        ws.write(brightness(data, 255)).unwrap();
        delay.delay(100.millis());
    }

    loop {
        let data = (0..NUM_LEDS).map(|_| RGB8 { r: 0, g: 0, b: 255 });
        ws.write(brightness(data, 255)).unwrap();
        delay.delay(100.millis());
    }
}
