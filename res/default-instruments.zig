const std = @import("std");
const math = std.math;
const gba = @import("gba.zig");

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

fn arpeggio(freq: u32, frame: u32, semitones: []const u8) u32 {
    const r = semitone_ratios[semitones[frame % semitones.len]];
    return r.apply(freq);
}

fn vibrato(delay: u32, p: u16, freq: u32, frame: u32) u32 {
    // Use almost half a semitone (0.475) amplitude for the delta triangle wave.
    // This fixed ratio is smaller than one so use the inverse ratio to avoid floating points.
    const inv_ratio = comptime @floatToInt(u32, math.round(1 / (math.pow(f32, 1.0594630943592953, 0.475) - 1)));
    const a = freq / inv_ratio;
    const delta = 1 + 4 * a / p * math.absCast(@mod((@mod(@intCast(i32, frame - delay) - p / 4, p) + p), p) - p / 2) - a;
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

    export fn square1_1p(freq: u32, _: u32) void {
        base_env_duty.write(gba.square1);
        gba.CtrlFreq.init()
                .withTrigger(1)
                .withSquareFreq(freq)
            .write(gba.square1);
        square1_released_at = null;
    }
    export fn square1_1r(_: u32, _: u32, frame: u32) void {
        square1_released_at = frame;
    }
    export fn square1_1f(_: u32, _: u32, frame: u32) void {
        if (square1_released_at) |decay_frame| {
            if (frame - decay_frame < explicit_env_dec_frames.len)
                explicit_env_dec_frames[frame - decay_frame].write(gba.square1);
        }
    }

    fn register() void {
        gba.setInstrument("□", 0, .{
            .press = "square1_1p",
            .release = "square1_1r",
            .frame = "square1_1f",
            .frames_after_release = 8,
            });
    }
};

const square1_2 = struct {

    export fn square1_2p(freq: u32, _: u32) void {
        base_env_duty_1_4.write(gba.square1);
        gba.CtrlFreq.init()
            .withSquareFreq(freq)
            .withTrigger(1)
            .write(gba.square1);
        square1_released_at = null;
    }
    export fn square1_2r(_: u32, _: u32, frame: u32) void {
        square1_released_at = frame;
    }
    export fn square1_2f(freq: u32, _: u32, frame: u32) void {
        const delay = 36;
        if (frame > delay)
            gba.CtrlFreq.init()
                .withSquareFreq(vibrato(delay, 8, freq, frame))
                .write(gba.square1);
        if (square1_released_at) |decay_frame| {
            if (frame - decay_frame < explicit_env_dec_1_4_frames.len)
                explicit_env_dec_1_4_frames[frame - decay_frame].write(gba.square1);
        }
    }

    fn register() void {
        gba.setInstrument("◰", 0, .{
            .press = "square1_2p",
            .release = "square1_2r",
            .frame = "square1_2f",
            .frames_after_release = 8,
            });
    }
};


const square1_3 = struct {
    export fn square1_3p(freq: u32, _: u32) void {
        // 2:bipp e:a:d:1 f:0:d:2 g
        gba.EnvDutyLen.init()
            .withDuty(gba.dut_1_8)
            .withEnvDir(gba.env_dec)
            .withEnvStart(0xa)
            .withEnvInterval(1)
            .write(gba.square1);
        gba.CtrlFreq.init()
            .withSquareFreq(freq)
            .withTrigger(1)
            .write(gba.square1);
    }
    export fn square1_3f(freq: u32, _: u32, frame: u32) void {
        if (frame == 2) {
            gba.EnvDutyLen.init()
                .write(gba.square1);
            gba.CtrlFreq.init()
                .withSquareFreq(freq)
                .withTrigger(1)
                .write(gba.square1);
        }
    }

    fn register() void {
        gba.setInstrument("🞍", 0, .{
            .press = "square1_3p",
            .frame = "square1_3f",
            });
    }
};

const square1_4 = struct {
    export fn square1_4p(freq: u32, _: u32) void {
        base_env_duty_1_4.write(gba.square1);
        gba.CtrlFreq.init()
            .withSquareFreq(freq)
            .withTrigger(1)
            .write(gba.square1);
        square1_released_at = null;
    }
    export fn square1_4r(_: u32, _: u32, frame: u32) void {
        square1_released_at = frame;
    }
    export fn square1_4f(_: u32, _: u32, frame: u32) void {
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
            if (frame - decay_frame < explicit_env_dec_1_4_frames.len)
                v = explicit_env_dec_1_4_frames[frame - decay_frame];
        }
        v.withDuty(duties[frame % duties.len])
            .write(gba.square1);
    }

    fn register() void {
        gba.setInstrument("▦", 0, .{
            .press = "square1_4p",
            .release = "square1_4r",
            .frame = "square1_4f",
            .frames_after_release = 8,
            });
    }
};

const noise_1 = struct {
    export fn noise_1p(freq: u32, _: u32) void {
        gba.EnvDutyLen.init()
            .withEnvStart(0xf)
            .withEnvDir(gba.env_dec)
            .withEnvInterval(1)
            .write(gba.noise);
        // Use the frequency as input for now just so that different
        // keys produce different sounds.
        const gb_freq = gba.CtrlFreq.squareFreqToFreq(freq);
        gba.NoiseCtrlFreq.init()
            .withClockShift(@truncate(u4, gb_freq >> 4))
            .withCounterWidth(gba.wid_15)
            .withClockDivisor(@truncate(u3, gb_freq))
            .withTrigger(1)
            .write(gba.noise);
    }

    fn register() void {
        gba.setInstrument("🟕", 1, .{ .press = "noise_1p" });
    }
};

const noise_2 = struct {
    var env_frames: []const ?gba.EnvDutyLen = &.{};
    var ctrl_frames: []const ?gba.NoiseCtrlFreq = &.{};
    export fn noise_2f(_: u32, _: u32, frame: u32) void {
        if (frame < env_frames.len)
            if (env_frames[frame]) |reg|
                reg.write(gba.noise);
        if (frame < ctrl_frames.len)
            if (ctrl_frames[frame]) |reg|
                reg.write(gba.noise);
    }
    export fn noise_2p(_: u32, note: u32) void {
        switch (note % 12) {
            0 => {
                const Static = struct {
                    const env = .{
                        .{.env_start = 7, .env_dir = gba.env_dec, .env_interval = 1},
                    };
                    const ctrl = .{
                        .{.s = 1, .width = gba.wid_15, .r = gba.div_8, .trigger = 1},
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
                        .{.s = 7, .width = gba.wid_7, .r = gba.div_16, .trigger = 1},
                        .{.s = 6, .width = gba.wid_7, .r = gba.div_16},
                        .{.s = 5, .width = gba.wid_7, .r = gba.div_16},
                        .{.s = 5, .width = gba.wid_15, .r = gba.div_16},
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
                        .{.s = 1, .width = gba.wid_15, .r = gba.div_16, .trigger = 1},
                        .{.s = 1, .width = gba.wid_15, .r = gba.div_32},
                        .{.s = 1, .width = gba.wid_15, .r = gba.div_48},
                        .{.s = 1, .width = gba.wid_15, .r = gba.div_64},
                        .{.s = 1, .width = gba.wid_15, .r = gba.div_80},
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
                        .{.s = 5, .width = gba.wid_7, .r = gba.div_16, .trigger = 1},
                        .{.s = 5, .width = gba.wid_7, .r = gba.div_48},
                        .{.s = 5, .width = gba.wid_7, .r = gba.div_48},
                        .{.s = 5, .width = gba.wid_7, .r = gba.div_80},
                        .{.s = 5, .width = gba.wid_7, .r = gba.div_112},
                        .{.s = 6, .width = gba.wid_15, .r = gba.div_8},
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
                        .{.s = 5, .width = gba.wid_7, .r = gba.div_16, .trigger = 1},
                        .{.s = 7, .width = gba.wid_7, .r = gba.div_16},
                        .{.s = 6, .width = gba.wid_7, .r = gba.div_16},
                        .{.s = 5, .width = gba.wid_15, .r = gba.div_8},
                        .{.s = 5, .width = gba.wid_15, .r = gba.div_8},
                        .{.s = 5, .width = gba.wid_15, .r = gba.div_16},
                        .{.s = 4, .width = gba.wid_15, .r = gba.div_16},
                        .{.s = 5, .width = gba.wid_15, .r = gba.div_16},
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
                        .{.s = 0, .width = gba.wid_15, .r = 4, .trigger = 1},
                        .{.s = 0, .width = gba.wid_15, .r = 2, .trigger = 1},
                        .{.s = 0, .width = gba.wid_15, .r = 6, .trigger = 1},
                        .{.s = 0, .width = gba.wid_15, .r = 3, .trigger = 1},
                        null,
                        null,
                        null,
                        .{.s = 0, .width = gba.wid_15, .r = 4, .trigger = 1},
                        .{.s = 0, .width = gba.wid_15, .r = 2, .trigger = 1},
                        .{.s = 0, .width = gba.wid_15, .r = 1, .trigger = 1},
                        .{.s = 0, .width = gba.wid_15, .r = 1, .trigger = 1},
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
                        .{.s = 0, .width = gba.wid_15, .r = 2, .trigger = 1},
                        .{.s = 5, .width = gba.wid_15, .r = 1, .trigger = 1},
                        .{.s = 6, .width = gba.wid_15, .r = 1, .trigger = 1},
                        .{.s = 7, .width = gba.wid_15, .r = 1, .trigger = 1},
                        .{.s = 9, .width = gba.wid_15, .r = 1, .trigger = 1},
                        .{.s = 7, .width = gba.wid_15, .r = 1, .trigger = 1},
                        .{.s = 6, .width = gba.wid_15, .r = 0, .trigger = 1},
                        .{.s = 0, .width = gba.wid_15, .r = 4, .trigger = 1},
                        .{.s = 0, .width = gba.wid_15, .r = 2, .trigger = 1},
                        .{.s = 0, .width = gba.wid_15, .r = 1, .trigger = 1},
                        .{.s = 0, .width = gba.wid_15, .r = 1, .trigger = 1},
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
                        .{.s = 6, .width = gba.wid_15, .r = 0, .trigger = 1},
                        .{.s = 5, .width = gba.wid_15, .r = 2, .trigger = 1},
                        .{.s = 4, .width = gba.wid_15, .r = 2, .trigger = 1},
                        .{.s = 4, .width = gba.wid_15, .r = 1, .trigger = 1},
                        .{.s = 2, .width = gba.wid_15, .r = 2, .trigger = 1},
                        .{.s = 1, .width = gba.wid_15, .r = 1, .trigger = 1},
                        .{.s = 0, .width = gba.wid_15, .r = 4, .trigger = 1},
                        .{.s = 0, .width = gba.wid_15, .r = 4, .trigger = 1},
                        .{.s = 0, .width = gba.wid_15, .r = 2, .trigger = 1},
                        .{.s = 0, .width = gba.wid_15, .r = 1, .trigger = 1},
                        .{.s = 0, .width = gba.wid_15, .r = 1, .trigger = 1},
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

    fn register() void {
        gba.setInstrument("🟗", 1, .{
            .press = "noise_2p",
            .frame = "noise_2f",
            .frames_after_release = 15,
            });
    }
};

const square1_5 = struct {
    export fn square1_5p(freq: u32, _: u32) void {
        // 1:superdrum e:d:d:2 f:2:d:2 g e
        gba.Sweep.init()
            .withTime(2)
            .withDir(gba.swe_dec)
            .withShift(2)
            .write(gba.square1);
        gba.EnvDutyLen.init()
            .withDuty(gba.dut_2_4)
            .withEnvStart(0xd)
            .withEnvDir(gba.env_dec)
            .withEnvInterval(2)
            .write(gba.square1);
        gba.CtrlFreq.init()
            .withSquareFreq(freq)
            .withTrigger(1)
            .write(gba.square1);
    }

    fn register() void {
        gba.setInstrument("◎", 1, .{ .press = "square1_5p" });
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
        .write(gba.wave);
    gba.CtrlFreq.init()
        .withWaveFreq(freq)
        .withTrigger(1)
        .write(gba.wave);
    wave_decay_at = null;
}
export fn wave_env_r(_: u32, _: u32, frame: u32) void {
    wave_decay_at = frame;
}
export fn wave_env_f(_: u32, _: u32, frame: u32) void {
    if (wave_decay_at) |decay_frame| {
        if (frame - decay_frame < wave_env_frames.len)
            wave_env_frames[frame - decay_frame].write(gba.wave);
    }
}

const wave_1 = struct {
    const table = gba.wav(0x0123456789abcdeffedcba9876543210);
    export fn wave_1p(freq: u32, _: u32) void {
        wave_p(freq, &table);
    }

    fn register() void {
        gba.setInstrument("🛆", 2, .{
            .press = "wave_1p",
            .release = "wave_env_r",
            .frame = "wave_env_f",
            .frames_after_release = 4,
            });
    }
};

const wave_2 = struct {
    const table = gba.wav(0x11235678999876679adffec985421131);
    export fn wave_2p(freq: u32, _: u32) void {
        wave_p(freq, &table);
    }

    fn register() void {
        gba.setInstrument("◉", 2, .{
            .press = "wave_2p",
            .release = "wave_env_r",
            .frame = "wave_env_f",
            .frames_after_release = 4,
            });
    }
};

const wave_3 = struct {
    const table = gba.wav(0xdedcba98765432100000000011111111);
    export fn wave_3p(freq: u32, _: u32) void {
        wave_p(freq, &table);
    }
    export fn wave_3f(freq: u32, _: u32, frame: u32) void {
        const Static = struct {
            const semitones =  [_]u8{0, 3, 5, 12};
        };
        gba.CtrlFreq.init()
            .withWaveFreq(arpeggio(freq, frame, &Static.semitones))
            .write(gba.wave);
        if (wave_decay_at) |decay_frame| {
            if (frame - decay_frame < wave_env_frames.len)
                wave_env_frames[frame - decay_frame].write(gba.wave);
        }
    }

    fn register() void {
        gba.setInstrument("▻", 2, .{
            .press = "wave_3p",
            .release = "wave_env_r",
            .frame = "wave_3f",
            .frames_after_release = 4,
            });
    }
};

const wave_4 = struct {
    const table = gba.wav(0xf0f0f0f0f0f0f0f0ff00ff00ff00ff00);
    export fn wave_4p(freq: u32, _: u32) void {
        wave_p(freq, &table);
    }

    fn register() void {
        gba.setInstrument("🞠", 2, .{
            .press = "wave_4p",
            .release = "wave_env_r",
            .frame = "wave_env_f",
            .frames_after_release = 4,
            });
    }
};

const wave_5 = struct {
    const table = gba.wav(0x0234679acdffffeeeeffffdca9764310);
    var current_step_freq: u32 = 0;
    export fn wave_5p(freq: u32, _: u32) void {
        gba.WaveRam.setTable(&table);
        gba.WaveVolLen.init()
            .withVolume(gba.vol_100)
            .write(gba.wave);
        gba.CtrlFreq.init()
            .withWaveFreq(freq)
            .withTrigger(1)
            .write(gba.wave);
        wave_decay_at = 12;
        current_step_freq = freq;
    }
    export fn wave_5f(_: u32, _: u32, frame: u32) void {
        gba.CtrlFreq.init()
            .withWaveFreq(semitones_steps(3, &current_step_freq))
            .write(gba.wave);
        if (wave_decay_at) |decay_frame| {
            if (frame - decay_frame < wave_env_frames.len)
                wave_env_frames[frame - decay_frame].write(gba.wave);
        }
    }

    fn register() void {
        gba.setInstrument("◺", 3, .{
            .press = "wave_5p",
            .frame = "wave_5f",
            .frames_after_release = 16,
            });
    }
};

pub fn main() void {
    square1_1.register();
    square1_2.register();
    square1_3.register();
    square1_4.register();
    noise_1.register();
    noise_2.register();
    square1_5.register();
    wave_1.register();
    wave_2.register();
    wave_3.register();
    wave_4.register();
    wave_5.register();
}

