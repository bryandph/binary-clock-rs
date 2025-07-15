use embassy_rp::{
    clocks::clk_ref_freq,
    pac::{self, TIMER1 as TIMER},
};

pub fn now() -> u64 {
    loop {
        let hi = TIMER.timehr().read();
        let lo = TIMER.timelr().read();
        let hi2 = TIMER.timehr().read();
        if hi == hi2 {
            return (hi as u64) << 32 | (lo as u64);
        }
    }
}

pub fn set_time(cur_time: i64) -> () {
    let usigned: u64 = cur_time.try_into().unwrap();
    let hi = (usigned >> 32) as u32;
    let lo = usigned as u32;
    TIMER.timelw().write_value(lo);
    TIMER.timehw().write_value(hi);
    pac::TICKS
        .timer1_cycles()
        .write(|w| w.0 = clk_ref_freq() / 1_000_000);
    pac::TICKS.timer1_ctrl().write(|w| w.set_enable(true))
}
