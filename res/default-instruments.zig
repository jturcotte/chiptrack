const std = @import("std");
const math = std.math;
const ct = @import("ct");
const gba = ct.gba;

const Fraction = struct {
    num: u8,
    de: u8,
    fn apply(self: Fraction, freq: u32) u32 {
        return freq * self.num / self.de;
    }
};

// Approximation of semitone frequency ratios using integer fractions
// to avoid floating point operations on the GBA.
const semitone_ratios = [_]Fraction{
    .{.num = 1, .de = 1},
    .{.num = 107, .de = 101},
    .{.num = 55, .de = 49},
    .{.num = 44, .de = 37},
    .{.num = 160, .de = 127},
    .{.num = 227, .de = 170},
    .{.num = 239, .de = 169},
    .{.num = 253, .de = 169},
    .{.num = 227, .de = 143},
    .{.num = 37, .de = 22},
    .{.num = 98, .de = 55},
    .{.num = 185, .de = 98},
    .{.num = 2, .de = 1},
};

fn semitones_steps(comptime semitones: u32, accum: *u32) u32 {
        const freq: u32 = accum.*;
        accum.* = semitone_ratios[semitones].apply(freq);
    return freq;
}

fn arpeggio(freq: u32, t: u32, semitones: []const u8) u32 {
    const r = semitone_ratios[semitones[t % semitones.len]];
    return r.apply(freq);
}

fn vibrato(delay: u32, p: u16, freq: u32, t: u32) u32 {
    // Use almost half a semitone (0.475) amplitude for the delta triangle wave.
    // This fixed ratio is smaller than one so use the inverse ratio to avoid floating points.
    const inv_ratio = comptime @as(u32, @intFromFloat(math.round(1 / (math.pow(f32, 1.0594630943592953, 0.475) - 1))));
    const a = freq / inv_ratio;
    const delta = 1 + 4 * a / p * math.absCast(@mod((@mod(@as(i32, @intCast(t - delay)) - p / 4, p) + p), p) - p / 2) - a;
    return freq + delta;
}

const base_env_duty_1_4 = gba.EnvDutyLen.init().withEnvStart(0xf).withDuty(gba.dut_1_4);
const explicit_env_dec_1_4_frames = [_]gba.EnvDutyLen{
    base_env_duty_1_4.withEnvStart(0xd),
    base_env_duty_1_4.withEnvStart(0xd),
    base_env_duty_1_4.withEnvStart(0xd),
    base_env_duty_1_4.withEnvStart(0xd),
    base_env_duty_1_4.withEnvStart(0x6),
    base_env_duty_1_4.withEnvStart(0x6),
    base_env_duty_1_4.withEnvStart(0x1),
    base_env_duty_1_4.withEnvStart(0x0),
};
var square1_released_at: ?u32 = null;

const square1_1 = struct {
    pub const id: [*:0]const u8 = "□";
    pub const frames_after_release: u32 = 8;

    const base_env_duty = gba.EnvDutyLen.init().withEnvStart(0xf).withDuty(gba.dut_2_4);
    const explicit_env_dec_frames = [_]gba.EnvDutyLen{
        base_env_duty.withEnvStart(0xd),
        base_env_duty.withEnvStart(0xd),
        base_env_duty.withEnvStart(0xd),
        base_env_duty.withEnvStart(0xd),
        base_env_duty.withEnvStart(0x6),
        base_env_duty.withEnvStart(0x6),
        base_env_duty.withEnvStart(0x1),
        base_env_duty.withEnvStart(0x0),
    };

    pub fn press(freq: u32, _: u8, _: i8, _: i8) callconv(.C) void {
        base_env_duty.writeTo(gba.square1);
        gba.CtrlFreq.init()
            .withTrigger(1)
            .withSquareFreq(freq)
            .writeTo(gba.square1);
        square1_released_at = null;
    }
    pub fn release(_: u32, _: u8, t: u32) callconv(.C) void {
        square1_released_at = t;
    }
    pub fn frame(_: u32, _: u8, t: u32) callconv(.C) void {
        if (square1_released_at) |decay_frame| {
            if (t - decay_frame < explicit_env_dec_frames.len)
                explicit_env_dec_frames[t - decay_frame].writeTo(gba.square1);
        }
    }
};

const square1_2 = struct {
    pub const id: [*:0]const u8 = "◰";
    pub const frames_after_release: u32 = 8;

    var p: u16 = 8;
    var duty: u2 = 0;
    fn set_duty(val: i8) void {
        duty = @intCast(val);
        base_env_duty_1_4
            .withDuty(duty)
            .writeTo(gba.square1);
    }
    fn set_p(val: i8) void {
        p = @max(1, @as(u16, @intCast(val)));
    }

    pub fn press(freq: u32, _: u8, duty_val: i8, p_val: i8) callconv(.C) void {
        set_p(p_val);
        set_duty(duty_val);
        gba.CtrlFreq.init()
            .withSquareFreq(freq)
            .withTrigger(1)
            .writeTo(gba.square1);
        square1_released_at = null;
    }
    pub fn release(_: u32, _: u8, t: u32) callconv(.C) void {
        square1_released_at = t;
    }

    pub fn frame(freq: u32, _: u8, t: u32) callconv(.C) void {
        const delay = 36;
        if (t > delay)
            gba.CtrlFreq.init()
                .withSquareFreq(vibrato(delay, p, freq, t))
                .writeTo(gba.square1);
        if (square1_released_at) |decay_frame| {
            if (t - decay_frame < explicit_env_dec_1_4_frames.len)
                explicit_env_dec_1_4_frames[t - decay_frame].withDuty(duty).writeTo(gba.square1);
        }
    }
    pub fn set_param(param_num: u8, val: i8) callconv(.C) void {
        if (param_num == 0)
            set_duty(val)
        else
            set_p(val);
    }
};


const square1_3 = struct {
    pub const id: [*:0]const u8 = "🞍";

    pub fn press(freq: u32, _: u8, _: i8, _: i8) callconv(.C) void {
        // 2:bipp e:a:d:1 f:0:d:2 g
        gba.EnvDutyLen.init()
            .withDuty(gba.dut_1_8)
            .withEnvDir(gba.env_dec)
            .withEnvStart(0xa)
            .withEnvInterval(1)
            .writeTo(gba.square1);
        gba.CtrlFreq.init()
            .withSquareFreq(freq)
            .withTrigger(1)
            .writeTo(gba.square1);
    }
    pub fn frame(freq: u32, _: u8, t: u32) callconv(.C) void {
        if (t == 2) {
            gba.EnvDutyLen.init()
                .writeTo(gba.square1);
            gba.CtrlFreq.init()
                .withSquareFreq(freq)
                .withTrigger(1)
                .writeTo(gba.square1);
        }
    }
};

const square1_4 = struct {
    pub const id: [*:0]const u8 = "▦";
    pub const frames_after_release: u32 = 8;

    pub fn press(freq: u32, _: u8, _: i8, _: i8) callconv(.C) void {
        base_env_duty_1_4.writeTo(gba.square1);
        gba.CtrlFreq.init()
            .withSquareFreq(freq)
            .withTrigger(1)
            .writeTo(gba.square1);
        square1_released_at = null;
    }
    pub fn release(_: u32, _: u8, t: u32) callconv(.C) void {
        square1_released_at = t;
    }
    pub fn frame(_: u32, _: u8, t: u32) callconv(.C) void {
        const duties = [_]u2{
            gba.dut_1_4,
            gba.dut_3_4,
            gba.dut_1_8,
            gba.dut_3_4,
            gba.dut_1_4,
            gba.dut_3_4,
            gba.dut_1_4,
            gba.dut_3_4,
            gba.dut_1_8};

        var v = base_env_duty_1_4;
        if (square1_released_at) |decay_frame| {
            if (t - decay_frame < explicit_env_dec_1_4_frames.len)
                v = explicit_env_dec_1_4_frames[t - decay_frame];
        }
        v.withDuty(duties[t % duties.len])
            .writeTo(gba.square1);
    }
};

const noise_1 = struct {
    pub const id: [*:0]const u8 = "🟕";

    pub fn press(freq: u32, _: u8, _: i8, _: i8) callconv(.C) void {
        gba.EnvDutyLen.init()
            .withEnvStart(0xf)
            .withEnvDir(gba.env_dec)
            .withEnvInterval(1)
            .writeTo(gba.noise);
        // Use the frequency as input for now just so that different
        // keys produce different sounds.
        const gb_freq = gba.encodeSquareFreq(freq);
        gba.NoiseCtrlFreq.init()
            .withFreq(@truncate(gb_freq >> 4))
            .withCounterWidth(gba.wid_15)
            .withFreqDiv(@truncate(gb_freq))
            .withTrigger(1)
            .writeTo(gba.noise);
    }
};

const noise_2 = struct {
    pub const id: [*:0]const u8 = "🟗";
    pub const frames_after_release: u32 = 15;

    var env_frames: []const ?gba.EnvDutyLen = &.{};
    var ctrl_frames: []const ?gba.NoiseCtrlFreq = &.{};
    pub fn frame(_: u32, _: u8, t: u32) callconv(.C) void {
        if (t < env_frames.len)
            if (env_frames[t]) |reg|
                reg.writeTo(gba.noise);
        if (t < ctrl_frames.len)
            if (ctrl_frames[t]) |reg|
                reg.writeTo(gba.noise);
    }
    pub fn press(_: u32, note: u8, _: i8, _: i8) callconv(.C) void {
        switch (note % 12) {
            0 => {
                const Static = struct {
                    const env = .{
                        .{.env_start = 7, .env_dir = gba.env_dec, .env_interval = 1},
                    };
                    const ctrl = .{
                        .{.freq = 1, .width = gba.wid_15, .freq_div = gba.div_8, .trigger = 1},
                    };
                };
                env_frames = &Static.env;
                ctrl_frames = &Static.ctrl;
            },
            1 => {
                const Static = struct {
                    const env = .{
                        .{.env_start = 10, .env_dir = gba.env_dec, .env_interval = 1},
                    };
                    const ctrl = .{
                        .{.freq = 7, .width = gba.wid_7, .freq_div = gba.div_16, .trigger = 1},
                        .{.freq = 6, .width = gba.wid_7, .freq_div = gba.div_16},
                        .{.freq = 5, .width = gba.wid_7, .freq_div = gba.div_16},
                        .{.freq = 5, .width = gba.wid_15, .freq_div = gba.div_16},
                    };
                };
                env_frames = &Static.env;
                ctrl_frames = &Static.ctrl;
            },
            2 => {
                const Static = struct {
                    const env = .{
                        .{.env_start = 7, .env_dir = gba.env_dec, .env_interval = 2},
                    };
                    const ctrl = .{
                        .{.freq = 1, .width = gba.wid_15, .freq_div = gba.div_16, .trigger = 1},
                        .{.freq = 1, .width = gba.wid_15, .freq_div = gba.div_32},
                        .{.freq = 1, .width = gba.wid_15, .freq_div = gba.div_48},
                        .{.freq = 1, .width = gba.wid_15, .freq_div = gba.div_64},
                        .{.freq = 1, .width = gba.wid_15, .freq_div = gba.div_80},
                    };
                };
                env_frames = &Static.env;
                ctrl_frames = &Static.ctrl;
            },
            4 => {
                const Static = struct {
                    const env = .{
                        .{.env_start = 10, .env_dir = gba.env_dec, .env_interval = 1},
                    };
                    const ctrl = .{
                        .{.freq = 5, .width = gba.wid_7, .freq_div = gba.div_16, .trigger = 1},
                        .{.freq = 5, .width = gba.wid_7, .freq_div = gba.div_48},
                        .{.freq = 5, .width = gba.wid_7, .freq_div = gba.div_48},
                        .{.freq = 5, .width = gba.wid_7, .freq_div = gba.div_80},
                        .{.freq = 5, .width = gba.wid_7, .freq_div = gba.div_112},
                        .{.freq = 6, .width = gba.wid_15, .freq_div = gba.div_8},
                    };
                };
                env_frames = &Static.env;
                ctrl_frames = &Static.ctrl;
            },
            5 => {
                const Static = struct {
                    const env = .{
                        .{.env_start = 10, .env_dir = gba.env_dec, .env_interval = 2},
                    };
                    const ctrl = .{
                        .{.freq = 5, .width = gba.wid_7, .freq_div = gba.div_16, .trigger = 1},
                        .{.freq = 7, .width = gba.wid_7, .freq_div = gba.div_16},
                        .{.freq = 6, .width = gba.wid_7, .freq_div = gba.div_16},
                        .{.freq = 5, .width = gba.wid_15, .freq_div = gba.div_8},
                        .{.freq = 5, .width = gba.wid_15, .freq_div = gba.div_8},
                        .{.freq = 5, .width = gba.wid_15, .freq_div = gba.div_16},
                        .{.freq = 4, .width = gba.wid_15, .freq_div = gba.div_16},
                        .{.freq = 5, .width = gba.wid_15, .freq_div = gba.div_16},
                    };
                };
                env_frames = &Static.env;
                ctrl_frames = &Static.ctrl;
            },
            7 => {
                const Static = struct {
                    const env = .{
                        .{.env_start = 9, .env_dir = gba.env_dec, .env_interval = 0},
                        .{.env_start = 8, .env_dir = gba.env_dec, .env_interval = 0},
                        .{.env_start = 3, .env_dir = gba.env_dec, .env_interval = 0},
                        .{.env_start = 3, .env_dir = gba.env_dec, .env_interval = 4},
                        null,
                        null,
                        null,
                        .{.env_start = 6, .env_dir = gba.env_dec, .env_interval = 0},
                        .{.env_start = 4, .env_dir = gba.env_dec, .env_interval = 0},
                        .{.env_start = 2, .env_dir = gba.env_dec, .env_interval = 0},
                        .{.env_start = 0, .env_dir = gba.env_dec, .env_interval = 3},
                    };
                    const ctrl = .{
                        .{.freq = 0, .width = gba.wid_15, .freq_div = 4, .trigger = 1},
                        .{.freq = 0, .width = gba.wid_15, .freq_div = 2, .trigger = 1},
                        .{.freq = 0, .width = gba.wid_15, .freq_div = 6, .trigger = 1},
                        .{.freq = 0, .width = gba.wid_15, .freq_div = 3, .trigger = 1},
                        null,
                        null,
                        null,
                        .{.freq = 0, .width = gba.wid_15, .freq_div = 4, .trigger = 1},
                        .{.freq = 0, .width = gba.wid_15, .freq_div = 2, .trigger = 1},
                        .{.freq = 0, .width = gba.wid_15, .freq_div = 1, .trigger = 1},
                        .{.freq = 0, .width = gba.wid_15, .freq_div = 1, .trigger = 1},
                    };
                };
                env_frames = &Static.env;
                ctrl_frames = &Static.ctrl;
            },
            9 => {
                const Static = struct {
                    const env = .{
                        .{.env_start = 13, .env_dir = gba.env_dec, .env_interval = 0},
                        .{.env_start = 13, .env_dir = gba.env_dec, .env_interval = 0},
                        .{.env_start = 11, .env_dir = gba.env_dec, .env_interval = 0},
                        .{.env_start = 7, .env_dir = gba.env_dec, .env_interval = 0},
                        .{.env_start = 5, .env_dir = gba.env_dec, .env_interval = 0},
                        .{.env_start = 3, .env_dir = gba.env_dec, .env_interval = 0},
                        .{.env_start = 2, .env_dir = gba.env_dec, .env_interval = 1},
                        .{.env_start = 6, .env_dir = gba.env_dec, .env_interval = 0},
                        .{.env_start = 4, .env_dir = gba.env_dec, .env_interval = 0},
                        .{.env_start = 2, .env_dir = gba.env_dec, .env_interval = 0},
                        .{.env_start = 0, .env_dir = gba.env_dec, .env_interval = 3},
                    };
                    const ctrl = .{
                        .{.freq = 0, .width = gba.wid_15, .freq_div = 2, .trigger = 1},
                        .{.freq = 5, .width = gba.wid_15, .freq_div = 1, .trigger = 1},
                        .{.freq = 6, .width = gba.wid_15, .freq_div = 1, .trigger = 1},
                        .{.freq = 7, .width = gba.wid_15, .freq_div = 1, .trigger = 1},
                        .{.freq = 9, .width = gba.wid_15, .freq_div = 1, .trigger = 1},
                        .{.freq = 7, .width = gba.wid_15, .freq_div = 1, .trigger = 1},
                        .{.freq = 6, .width = gba.wid_15, .freq_div = 0, .trigger = 1},
                        .{.freq = 0, .width = gba.wid_15, .freq_div = 4, .trigger = 1},
                        .{.freq = 0, .width = gba.wid_15, .freq_div = 2, .trigger = 1},
                        .{.freq = 0, .width = gba.wid_15, .freq_div = 1, .trigger = 1},
                        .{.freq = 0, .width = gba.wid_15, .freq_div = 1, .trigger = 1},
                    };
                };
                env_frames = &Static.env;
                ctrl_frames = &Static.ctrl;
            },
            11 => {
                const Static = struct {
                    const env = .{
                        .{.env_start = 13, .env_dir = gba.env_dec, .env_interval = 0},
                        .{.env_start = 13, .env_dir = gba.env_dec, .env_interval = 0},
                        .{.env_start = 13, .env_dir = gba.env_dec, .env_interval = 0},
                        .{.env_start = 8, .env_dir = gba.env_dec, .env_interval = 0},
                        .{.env_start = 1, .env_dir = gba.env_dec, .env_interval = 0},
                        .{.env_start = 2, .env_dir = gba.env_dec, .env_interval = 0},
                        .{.env_start = 3, .env_dir = gba.env_dec, .env_interval = 3},
                        .{.env_start = 6, .env_dir = gba.env_dec, .env_interval = 0},
                        .{.env_start = 4, .env_dir = gba.env_dec, .env_interval = 0},
                        .{.env_start = 2, .env_dir = gba.env_dec, .env_interval = 0},
                        .{.env_start = 0, .env_dir = gba.env_dec, .env_interval = 3},
                    };
                    const ctrl = .{
                        .{.freq = 6, .width = gba.wid_15, .freq_div = 0, .trigger = 1},
                        .{.freq = 5, .width = gba.wid_15, .freq_div = 2, .trigger = 1},
                        .{.freq = 4, .width = gba.wid_15, .freq_div = 2, .trigger = 1},
                        .{.freq = 4, .width = gba.wid_15, .freq_div = 1, .trigger = 1},
                        .{.freq = 2, .width = gba.wid_15, .freq_div = 2, .trigger = 1},
                        .{.freq = 1, .width = gba.wid_15, .freq_div = 1, .trigger = 1},
                        .{.freq = 0, .width = gba.wid_15, .freq_div = 4, .trigger = 1},
                        .{.freq = 0, .width = gba.wid_15, .freq_div = 4, .trigger = 1},
                        .{.freq = 0, .width = gba.wid_15, .freq_div = 2, .trigger = 1},
                        .{.freq = 0, .width = gba.wid_15, .freq_div = 1, .trigger = 1},
                        .{.freq = 0, .width = gba.wid_15, .freq_div = 1, .trigger = 1},
                    };
                };
                env_frames = &Static.env;
                ctrl_frames = &Static.ctrl;
            },
            else => {
                env_frames = &.{};
                ctrl_frames = &.{};
            },
        }
    }
};

const square1_5 = struct {
    pub const id: [*:0]const u8 = "◎";

    pub fn press(freq: u32, _: u8, _: i8, _: i8) callconv(.C) void {
        // 1:superdrum e:d:d:2 f:2:d:2 g e
        gba.Sweep.init()
            .withTime(2)
            .withDir(gba.swe_dec)
            .withShift(2)
            .writeTo(gba.square1);
        gba.EnvDutyLen.init()
            .withDuty(gba.dut_2_4)
            .withEnvStart(0xd)
            .withEnvDir(gba.env_dec)
            .withEnvInterval(2)
            .writeTo(gba.square1);
        gba.CtrlFreq.init()
            .withSquareFreq(freq)
            .withTrigger(1)
            .writeTo(gba.square1);
    }
};

const wave_env_frames = [_]gba.WaveVolLen{
    .{.volume = gba.vol_75},
    .{.volume = gba.vol_50},
    .{.volume = gba.vol_25},
    .{.volume = gba.vol_0},
};
var wave_decay_at: ?u32 = null;
fn wave_p(freq: u32, table: *const gba.WavTable) void {
    gba.WaveRam.setTable(table);
    gba.WaveVolLen.init()
        .withVolume(gba.vol_100)
        .writeTo(gba.wave);
    gba.CtrlFreq.init()
        .withWaveFreq(freq)
        .withTrigger(1)
        .writeTo(gba.wave);
    wave_decay_at = null;
}
fn wave_env_r(_: u32, _: u8, t: u32) callconv(.C) void {
    wave_decay_at = t;
}
fn wave_env_f(_: u32, _: u8, t: u32) callconv(.C) void {
    if (wave_decay_at) |decay_frame| {
        if (t - decay_frame < wave_env_frames.len)
            wave_env_frames[t - decay_frame].writeTo(gba.wave);
    }
}

const wave_1 = struct {
    pub const id: [*:0]const u8 = "🛆";

    const table = gba.wav(0x0123456789abcdeffedcba9876543210);
    pub fn press(freq: u32, _: u8, _: i8, _: i8) callconv(.C) void {
        wave_p(freq, &table);
    }
    pub const release = wave_env_r;
    pub const frame = wave_env_f;
    pub const frames_after_release: u32 = 4;
};

const wave_2 = struct {
    pub const id: [*:0]const u8 = "◉";

    const table = gba.wav(0x11235678999876679adffec985421131);
    pub fn press(freq: u32, _: u8, _: i8, _: i8) callconv(.C) void {
        wave_p(freq, &table);
    }

    pub const release = wave_env_r;
    pub const frame = wave_env_f;
    pub const frames_after_release: u32 = 4;
};

const wave_3 = struct {
    pub const id: [*:0]const u8 = "▻";
    pub const frames_after_release: u32 = 4;

    const table = gba.wav(0xdedcba98765432100000000011111111);
    pub fn press(freq: u32, _: u8, _: i8, _: i8) callconv(.C) void {
        wave_p(freq, &table);
    }
    pub fn frame(freq: u32, _: u8, t: u32) callconv(.C) void {
        const Static = struct {
            const semitones =  [_]u8{0, 3, 5, 12};
        };
        gba.CtrlFreq.init()
            .withWaveFreq(arpeggio(freq, t, &Static.semitones))
            .writeTo(gba.wave);
        if (wave_decay_at) |decay_frame| {
            if (t - decay_frame < wave_env_frames.len)
                wave_env_frames[t - decay_frame].writeTo(gba.wave);
        }
    }
    pub const release = wave_env_r;
};

const wave_4 = struct {
    pub const id: [*:0]const u8 = "🞠";

    const table = gba.wav(0xf0f0f0f0f0f0f0f0ff00ff00ff00ff00);
    pub fn press(freq: u32, _: u8, _: i8, _: i8) callconv(.C) void {
        wave_p(freq, &table);
    }
    pub const release = wave_env_r;
    pub const frame = wave_env_f;
    pub const frames_after_release: u32 = 4;
};

const wave_5 = struct {
    pub const id: [*:0]const u8 = "◺";
    pub const frames_after_release: u32 = 16;

    const table = gba.wav(0x0234679acdffffeeeeffffdca9764310);
    var current_step_freq: u32 = 0;
    pub fn press(freq: u32, _: u8, _: i8, _: i8) callconv(.C) void {
        gba.WaveRam.setTable(&table);
        gba.WaveVolLen.init()
            .withVolume(gba.vol_100)
            .writeTo(gba.wave);
        gba.CtrlFreq.init()
            .withWaveFreq(freq)
            .withTrigger(1)
            .writeTo(gba.wave);
        wave_decay_at = 12;
        current_step_freq = freq;
    }
    pub fn frame(_: u32, _: u8, t: u32) callconv(.C) void {
        gba.CtrlFreq.init()
            .withWaveFreq(semitones_steps(3, &current_step_freq))
            .writeTo(gba.wave);
        if (wave_decay_at) |decay_frame| {
            if (t - decay_frame < wave_env_frames.len)
                wave_env_frames[t - decay_frame].writeTo(gba.wave);
        }
    }
};

pub fn main() void {
    ct.registerInstrument(square1_1, 0);
    ct.registerInstrument(square1_2, 0);
    ct.registerInstrument(square1_3, 0);
    ct.registerInstrument(square1_4, 0);
    ct.registerInstrument(noise_1, 1);
    ct.registerInstrument(noise_2, 1);
    ct.registerInstrument(square1_5, 1);
    ct.registerInstrument(wave_1, 2);
    ct.registerInstrument(wave_2, 2);
    ct.registerInstrument(wave_3, 2);
    ct.registerInstrument(wave_4, 2);
    ct.registerInstrument(wave_5, 3);
}

