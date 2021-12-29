fn three_semitones_steps(timeline, start) {
    let next = start;
    timeline.frames(timeline.map(|w, i| {
        let n = next;
        next = n * 0.840896415253715;
        n
    }))
}

fn instrument_id_1() { "□" }
fn instrument_1(note, freq) {
    gb.square1.freq = freq;
    gb.square1.sweep_time = 0;
    gb.square1.sweep_dir = gb::SWE_INC;
    gb.square1.sweep_shift = 0;

    gb.square1.duty = gb::DUT_2_4;
    let t =                          [0,   1,   1,   3,   2,   1];
    gb.square1.env_start = t.frames([0xf, 0xd, 0xd, 0x6, 0x1, 0x0]);
    gb.square1.env_dir = gb::ENV_INC;
    gb.square1.env_period = 0;
    gb.square1.initialize = true;
}

fn instrument_id_2() { "◰" }
fn instrument_2(note, freq) {
    gb.square1.freq = freq;
    gb.square1.sweep_time = 0;
    gb.square1.sweep_dir = gb::SWE_INC;
    gb.square1.sweep_shift = 0;

    gb.square1.duty = gb::DUT_1_4;
    gb.square1.env_start = 0xf;
    gb.square1.env_dir = gb::ENV_DEC;
    gb.square1.env_period = 0;
    gb.square1.trigger_with_length(64);
}

fn instrument_id_3() { "🞍" }
fn instrument_3(note, freq) {
    gb.square1.freq = freq;
    // 2:bipp e:a:d:1 f:0:d:2 g
    gb.square1.duty = gb::DUT_1_8;
    gb.square1.env_dir = gb::ENV_DEC;
    let t =                           [0,   2];
    gb.square1.env_start =  t.frames([0xa, 0]);
    gb.square1.env_period = t.frames([1,   0]);
    gb.square1.sweep_time = 0;
    gb.square1.sweep_dir = gb::SWE_DEC;
    gb.square1.sweep_shift = 2;
    gb.square1.trigger();
    gb.wait_frames(2);
    gb.square1.trigger();
}

fn instrument_id_4() { "▦" }
fn instrument_4(note, freq) {
    gb.square1.freq = freq;

    let t = [0, 2, 2, 2, 2, 2, 2, 2, 2];
    gb.square1.duty = t.frames([
        gb::DUT_1_4,
        gb::DUT_3_4,
        gb::DUT_1_8,
        gb::DUT_3_4,
        gb::DUT_1_4,
        gb::DUT_3_4,
        gb::DUT_1_4,
        gb::DUT_3_4,
        gb::DUT_1_8]);

    gb.square1.env_start = [0, 16].frames([0xf, 0x0]);
    gb.square1.env_dir = gb::ENV_DEC;
    gb.square1.env_period = 0;
    gb.square1.sweep_time = 0;
    gb.square1.sweep_dir = gb::SWE_DEC;
    gb.square1.sweep_shift = 0;
    gb.square1.trigger();
}

fn instrument_id_5() { "🟕" }
fn instrument_5(note, freq) {
    gb.noise.env_start = 0xf;
    gb.noise.env_dir = gb::ENV_DEC;
    gb.noise.env_period = 1;
    // Use the frequency as input for now just so that different
    // keys produce different sounds.
    let gb_freq = freq.to_square_gb_freq();
    gb.noise.clock_shift = (gb_freq >> 4) & 0xf;
    gb.noise.counter_width = gb::WID_15;
    gb.noise.clock_divisor = gb_freq & 0x7;
    gb.noise.trigger();
}

fn instrument_id_6() { "🟗" }
fn instrument_6(note, freq) {
    switch note % 12 {
        0 => {
            gb.noise.env_start = 7;
            gb.noise.env_dir = gb::ENV_DEC;
            gb.noise.env_period = 1;
            gb.noise.clock_shift = 1;
            gb.noise.counter_width = gb::WID_15;
            gb.noise.clock_divisor = gb::DIV_8;
            gb.noise.trigger();

        },
        1 => {
            gb.noise.env_start = 10;
            gb.noise.env_dir = gb::ENV_DEC;
            gb.noise.env_period = 1;

            let t =                           [0,         1,         1,         1];
            gb.noise.clock_shift =   t.frames([7,         6,         5,         5]);
            gb.noise.counter_width = t.frames([gb::WID_7, gb::WID_7, gb::WID_7, gb::WID_15]);
            gb.noise.clock_divisor = gb::DIV_16;
            gb.noise.trigger();
        },
        2 => {
            gb.noise.env_start = 7;
            gb.noise.env_dir = gb::ENV_DEC;
            gb.noise.env_period = 2;
            gb.noise.clock_shift = 1;
            let t =                           [0,          1,          1,          1,          1];
            gb.noise.counter_width = t.frames([gb::WID_15, gb::WID_15, gb::WID_15, gb::WID_15, gb::WID_15]);
            gb.noise.clock_divisor = t.frames([gb::DIV_16, gb::DIV_32, gb::DIV_48, gb::DIV_64, gb::DIV_80]);
            gb.noise.trigger();
        },
        4 => {
            gb.noise.env_start = 10;
            gb.noise.env_dir = gb::ENV_DEC;
            gb.noise.env_period = 1;
            let t =                           [0,          1,          1,          1,          1,           1];
            gb.noise.clock_shift =   t.frames([5,          5,          5,          5,          5,           6]);
            gb.noise.counter_width = t.frames([gb::WID_7,  gb::WID_7,  gb::WID_7,  gb::WID_7,  gb::WID_7,   gb::WID_15]);
            gb.noise.clock_divisor = t.frames([gb::DIV_16, gb::DIV_48, gb::DIV_48, gb::DIV_80, gb::DIV_112, gb::DIV_8]);
            gb.noise.trigger();
        },
        5 => {
            gb.noise.env_start = 10;
            gb.noise.env_dir = gb::ENV_DEC;
            gb.noise.env_period = 2;
            let t =                           [0,          1,          1,          1,          1,          1,          1,          1];
            gb.noise.clock_shift =   t.frames([5,          7,          6,          5,          5,          5,          4,          5]);
            gb.noise.counter_width = t.frames([gb::WID_7,  gb::WID_7,  gb::WID_7,  gb::WID_15, gb::WID_15, gb::WID_15, gb::WID_15, gb::WID_15]);
            gb.noise.clock_divisor = t.frames([gb::DIV_16, gb::DIV_16, gb::DIV_16, gb::DIV_8,  gb::DIV_8,  gb::DIV_16, gb::DIV_16, gb::DIV_16]);
            gb.noise.trigger();
        },
        7 => {
            gb.noise.counter_width = gb::WID_15;
            gb.noise.env_dir = gb::ENV_DEC;

            let t =                                 [0,  1,  1,  1,  4,  1,  1,  1];
            gb.noise.clock_shift =         t.frames([0,  0,  0,  0,  0,  0,  0,  0]);
            gb.noise.clock_divisor =       t.frames([4,  2,  6,  3,  4,  2,  1,  1]);
            gb.noise.env_start =           t.frames([9,  8,  3,  3,  6,  4,  2,  0]);
            gb.noise.env_period =          t.frames([0,  0,  0,  4,  0,  0,  0,  3]);
            gb.noise.initialize =          t.frames([1,  1,  1,  1,  1,  1,  1,  1]);
        },
        9 => {
            gb.noise.counter_width = gb::WID_15;
            gb.noise.env_dir = gb::ENV_DEC;

            let t =                                 [  0,  1,  1,  1,  1,  1,  1,  1,  1,  1,  1];
            gb.noise.clock_shift =         t.frames([  0,  5,  6,  7,  9,  7,  6,  0,  0,  0,  0]);
            gb.noise.clock_divisor =       t.frames([  2,  1,  1,  1,  1,  1,  0,  4,  2,  1,  1]);
            gb.noise.env_start =           t.frames([ 13, 13, 11,  7,  5,  3,  2,  6,  4,  2,  0]);
            gb.noise.env_period =          t.frames([  0,  0,  0,  0,  0,  0,  1,  0,  0,  0,  3]);
            gb.noise.initialize =          t.frames([  1,  1,  1,  1,  1,  1,  1,  1,  1,  1,  1]);
        },
        11 => {
            gb.noise.counter_width = gb::WID_15;
            gb.noise.env_dir = gb::ENV_DEC;

            let t =                                 [  0,  1,  1,  1,  1,  1,  1,  1,  1,  1,  1];
            gb.noise.clock_shift =         t.frames([  6,  5,  4,  4,  2,  1,  0,  0,  0,  0,  0]);
            gb.noise.clock_divisor =       t.frames([  0,  2,  2,  1,  2,  1,  4,  4,  2,  1,  1]);
            gb.noise.env_start =           t.frames([ 13, 13, 13,  8,  1,  2,  3,  6,  4,  2,  0]);
            gb.noise.env_period =          t.frames([  0,  0,  0,  0,  0,  0,  3,  0,  0,  0,  3]);
            gb.noise.initialize =          t.frames([  1,  1,  1,  1,  1,  1,  1,  1,  1,  1,  1]);

        }
    }
}

fn instrument_id_7() { "◎" }
fn instrument_7(note, freq) {
    gb.square1.freq = freq;
    // 1:superdrum e:d:d:2 f:2:d:2 g e
    gb.square1.duty = gb::DUT_2_4;
    gb.square1.env_start = 0xd;
    gb.square1.env_dir = gb::ENV_DEC;
    gb.square1.env_period = 2;
    gb.square1.sweep_time = 2;
    gb.square1.sweep_dir = gb::SWE_DEC;
    gb.square1.sweep_shift = 2;
    gb.square1.trigger();
}

fn instrument_id_9() { "🛆" }
fn instrument_9(note, freq) {
    gb.wave.freq = freq;
    gb.wave.playing = false;
    gb.wave.table = "0123456789abcdeffedcba9876543210";
    gb.wave.playing = true;
    let t = [0, 4, 1, 1];
    gb.wave.volume = t.frames([gb::VOL_100, gb::VOL_50, gb::VOL_25, gb::VOL_0]);
    gb.wave.trigger();
}

fn instrument_id_10() { "◉" }
fn instrument_10(note, freq) {
    gb.wave.playing = false;
    gb.wave.table = "11235678999876679adffec985421131";
    gb.wave.playing = true;
    gb.wave.freq = freq;
    let t = [0, 4, 1, 1];
    gb.wave.volume = t.frames([gb::VOL_100, gb::VOL_50, gb::VOL_25, gb::VOL_0]);
    gb.wave.trigger();
}

fn instrument_id_11() { "▻" }
fn instrument_11(note, freq) {
    gb.wave.playing = false;
    gb.wave.table = "dedcba98765432100000000011111111";
    gb.wave.playing = true;
    gb.wave.freq = freq;
    let t = [0, 4, 1, 1];
    gb.wave.volume = t.frames([gb::VOL_100, gb::VOL_50, gb::VOL_25, gb::VOL_0]);
    gb.wave.trigger();
}

fn instrument_id_12() { "🞠" }
fn instrument_12(note, freq) {
    gb.wave.playing = false;
    gb.wave.table = "f0f0f0f0f0f0f0f0ff00ff00ff00ff00";
    gb.wave.playing = true;
    gb.wave.freq = freq;
    gb.wave.trigger();                // C#1 gate=11
    let t = [0, 4, 1, 1];
    gb.wave.volume = t.frames([gb::VOL_100, gb::VOL_50, gb::VOL_25, gb::VOL_0]);
    gb.wave.trigger();
}

fn instrument_id_13() { "◺" }
fn instrument_13(note, freq) {
    gb.wave.playing = false;
    gb.wave.table = "0234679acdffffeeeeffffdca9764310";
    gb.wave.playing = true;
    let t = [0, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1];
    gb.wave.freq = three_semitones_steps(t, freq);                  // A-5 +11.0 ct gb_freq=0x04a
    gb.wave.volume = gb::VOL_100;
    gb.wave.trigger();
    gb.wave.volume = [9, 1, 1].frames([gb::VOL_50, gb::VOL_25, gb::VOL_0]);
}
