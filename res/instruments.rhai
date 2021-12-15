fn instrument_id_1() { "🎹" }
fn instrument_1(freq) {
    dmg.square1.freq = freq;
    dmg.square1.sweep_time = 0;
    dmg.square1.sweep_dir = dmg::INC;
    dmg.square1.sweep_shift = 0;

    dmg.square1.duty = dmg::DUTY_2_4;
    dmg.square1.env_start = 0xf;
    dmg.square1.env_dir = dmg::INC;
    dmg.square1.env_period = 0;
    dmg.square1.trigger();

    dmg.wait_frames(1);
    dmg.square1.env_start = 0xd;

    dmg.wait_frames(4);
    dmg.square1.env_start = 0x6;

    dmg.wait_frames(2);
    dmg.square1.env_start = 0x1;

    dmg.wait_frames(1);
    dmg.square1.env_start = 0x0;

}

fn instrument_id_2() { "🎸" }
fn instrument_2(freq) {
    dmg.square1.freq = freq;
    dmg.square1.sweep_time = 0;
    dmg.square1.sweep_dir = dmg::INC;
    dmg.square1.sweep_shift = 0;

    dmg.square1.duty = dmg::DUTY_1_4;
    dmg.square1.env_start = 0xf;
    dmg.square1.env_dir = dmg::DEC;
    dmg.square1.env_period = 0;
    dmg.square1.trigger_with_length(64);
}

fn instrument_id_3() { "✨" }
fn instrument_3(freq) {
    dmg.square1.freq = freq;
    // 2:bipp e:a:d:1 f:0:d:2 g
    dmg.square1.duty = dmg::DUTY_1_8;
    dmg.square1.env_start = 0xa;
    dmg.square1.env_dir = dmg::DEC;
    dmg.square1.env_period = 1;
    dmg.square1.sweep_time = 0;
    dmg.square1.sweep_dir = dmg::DEC;
    dmg.square1.sweep_shift = 2;
    dmg.square1.trigger();

    dmg.wait_frames(2);
    // e:0:D:0 g e
    dmg.square1.env_start = 0x0;
    dmg.square1.env_period = 0;
}

fn instrument_id_4() { "🌊" }
fn instrument_4(freq) {
    dmg.square1.freq = freq;
    // r:1 e:f:d:0 f:0:d:0 g
    dmg.square1.duty = dmg::DUTY_1_4;
    dmg.square1.env_start = 0xf;
    dmg.square1.env_dir = dmg::DEC;
    dmg.square1.env_period = 0;
    dmg.square1.sweep_time = 0;
    dmg.square1.sweep_dir = dmg::DEC;
    dmg.square1.sweep_shift = 0;
    dmg.square1.trigger();

    // r:3
    dmg.wait_frames(2);
    dmg.square1.duty = dmg::DUTY_3_4;
    // r:0
    dmg.wait_frames(2);
    dmg.square1.duty = dmg::DUTY_1_8;
    // r:3
    dmg.wait_frames(2);
    dmg.square1.duty = dmg::DUTY_3_4;
    // r:1
    dmg.wait_frames(2);
    dmg.square1.duty = dmg::DUTY_1_4;
    // r:3
    dmg.wait_frames(2);
    dmg.square1.duty = dmg::DUTY_3_4;
    // r:1
    dmg.wait_frames(2);
    dmg.square1.duty = dmg::DUTY_1_4;
    // r:3
    dmg.wait_frames(2);
    dmg.square1.duty = dmg::DUTY_3_4;
    // r:0
    dmg.wait_frames(2);
    dmg.square1.duty = dmg::DUTY_1_8;

    // e:0:d:0 g
    dmg.square1.env_start = 0x0;
}

fn instrument_id_5() { "🥁" }
fn instrument_5(freq) {
    dmg.noise.env_start = 0xf;
    dmg.noise.env_dir = dmg::DEC;
    dmg.noise.env_period = 1;
    // Use the frequency as input for now just so that different
    // keys produce different sounds.
    let gb_freq = freq.to_square_gb_freq();
    dmg.noise.clock_shift = (gb_freq >> 4) & 0xf;
    dmg.noise.counter_width = dmg::WIDTH_15;
    dmg.noise.clock_divisor = gb_freq & 0x7;
    dmg.noise.trigger();
}

fn instrument_id_6() { "🛢️" }
fn instrument_6(freq) {
    dmg.square1.freq = freq;
    // 1:superdrum e:d:d:2 f:2:d:2 g e
    dmg.square1.duty = dmg::DUTY_2_4;
    dmg.square1.env_start = 0xd;
    dmg.square1.env_dir = dmg::DEC;
    dmg.square1.env_period = 2;
    dmg.square1.sweep_time = 2;
    dmg.square1.sweep_dir = dmg::DEC;
    dmg.square1.sweep_shift = 2;
    dmg.square1.trigger();
}

fn instrument_id_7() { "🔈" }
fn instrument_7(freq) {
    dmg.wave.freq = freq;
    dmg.wave.playing = false;
    dmg.wave.table = "0123456789abcdeffedcba9876543210";
    dmg.wave.playing = true;
    dmg.wave.volume = dmg::VOLUME_100;
    dmg.wave.trigger_with_length(64);
}
