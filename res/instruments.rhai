fn instrument_id_1() { "🎹" }
fn instrument_1(freq) {
    gb.square1.freq = freq;
    gb.square1.sweep_time = 0;
    gb.square1.sweep_dir = gb::INC;
    gb.square1.sweep_shift = 0;

    gb.square1.duty = gb::DUTY_2_4;
    let t =                          [0,   1,   1,   3,   2,   1];
    gb.square1.env_start = t.frames([0xf, 0xd, 0xd, 0x6, 0x1, 0x0]);
    gb.square1.env_dir = gb::INC;
    gb.square1.env_period = 0;
    gb.square1.initialize = true;
}

fn instrument_id_2() { "🎸" }
fn instrument_2(freq) {
    gb.square1.freq = freq;
    gb.square1.sweep_time = 0;
    gb.square1.sweep_dir = gb::INC;
    gb.square1.sweep_shift = 0;

    gb.square1.duty = gb::DUTY_1_4;
    gb.square1.env_start = 0xf;
    gb.square1.env_dir = gb::DEC;
    gb.square1.env_period = 0;
    gb.square1.trigger_with_length(64);
}

fn instrument_id_3() { "✨" }
fn instrument_3(freq) {
    gb.square1.freq = freq;
    // 2:bipp e:a:d:1 f:0:d:2 g
    gb.square1.duty = gb::DUTY_1_8;
    gb.square1.env_dir = gb::DEC;
    let t =                           [0,   2];
    gb.square1.env_start =  t.frames([0xa, 0]);
    gb.square1.env_period = t.frames([1,   0]);
    gb.square1.sweep_time = 0;
    gb.square1.sweep_dir = gb::DEC;
    gb.square1.sweep_shift = 2;
    gb.square1.trigger();
    gb.wait_frames(2);
    gb.square1.trigger();
}

fn instrument_id_4() { "🌊" }
fn instrument_4(freq) {
    gb.square1.freq = freq;

    let t = [0, 2, 2, 2, 2, 2, 2, 2, 2];
    gb.square1.duty = t.frames([
        gb::DUTY_1_4,
        gb::DUTY_3_4,
        gb::DUTY_1_8,
        gb::DUTY_3_4,
        gb::DUTY_1_4,
        gb::DUTY_3_4,
        gb::DUTY_1_4,
        gb::DUTY_3_4,
        gb::DUTY_1_8]);

    gb.square1.env_start = [0, 16].frames([0xf, 0x0]);
    gb.square1.env_dir = gb::DEC;
    gb.square1.env_period = 0;
    gb.square1.sweep_time = 0;
    gb.square1.sweep_dir = gb::DEC;
    gb.square1.sweep_shift = 0;
    gb.square1.trigger();
}

fn instrument_id_5() { "🥁" }
fn instrument_5(freq) {
    gb.noise.env_start = 0xf;
    gb.noise.env_dir = gb::DEC;
    gb.noise.env_period = 1;
    // Use the frequency as input for now just so that different
    // keys produce different sounds.
    let gb_freq = freq.to_square_gb_freq();
    gb.noise.clock_shift = (gb_freq >> 4) & 0xf;
    gb.noise.counter_width = gb::WIDTH_15;
    gb.noise.clock_divisor = gb_freq & 0x7;
    gb.noise.trigger();
}

fn instrument_id_6() { "🛢️" }
fn instrument_6(freq) {
    gb.square1.freq = freq;
    // 1:superdrum e:d:d:2 f:2:d:2 g e
    gb.square1.duty = gb::DUTY_2_4;
    gb.square1.env_start = 0xd;
    gb.square1.env_dir = gb::DEC;
    gb.square1.env_period = 2;
    gb.square1.sweep_time = 2;
    gb.square1.sweep_dir = gb::DEC;
    gb.square1.sweep_shift = 2;
    gb.square1.trigger();
}

fn instrument_id_7() { "🔈" }
fn instrument_7(freq) {
    gb.wave.freq = freq;
    gb.wave.playing = false;
    gb.wave.table = "0123456789abcdeffedcba9876543210";
    gb.wave.playing = true;
    gb.wave.volume = gb::VOLUME_100;
    gb.wave.trigger_with_length(64);
}
