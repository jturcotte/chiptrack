// Copyright © 2024 Jocelyn Turcotte <turcotte.j@gmail.com>
// SPDX-License-Identifier: CC0-1.0

const std = @import("std");
const math = std.math;
const ct = @import("ct");
const gba = ct.gba;

// We can't read the current state from the sound chip, so we have to keep a static copy
// here and update it every time before writing it to the sound chip so that instruments
// can update channels independently.
var sound_ctrl = gba.SoundCtrl.init();

const Fraction = struct {
    num: u16,
    de: u16,
    fn apply(self: Fraction, freq: u32) u32 {
        return freq * self.num / self.de;
    }
    fn reverse(self: Fraction) Fraction {
        return .{ .num = self.de, .de = self.num };
    }
};

// Approximation of semitone frequency ratios using integer fractions
// to avoid floating point operations on the GBA.
const semitone_ratios = [_]Fraction{
    .{ .num = 1, .de = 1 },
    .{ .num = 107, .de = 101 },
    .{ .num = 55, .de = 49 },
    .{ .num = 44, .de = 37 },
    .{ .num = 160, .de = 127 },
    .{ .num = 227, .de = 170 },
    .{ .num = 239, .de = 169 },
    .{ .num = 253, .de = 169 },
    .{ .num = 227, .de = 143 },
    .{ .num = 37, .de = 22 },
    .{ .num = 98, .de = 55 },
    .{ .num = 185, .de = 98 },
    .{ .num = 2, .de = 1 },
};

fn semitones_steps(semitones: u32, accum: *u32) u32 {
    const freq: u32 = accum.*;
    accum.* = semitone_ratios[semitones].apply(freq);
    return freq;
}

fn apply_semitone(freq: u32, semitone: i8) u32 {
    const abs_semitone = @abs(semitone);
    var r = semitone_ratios[abs_semitone % 12];
    // Multiply the numerator by 2^(semitone/12) for octaves
    r.num *= @shlExact(@as(u16, 1), @intCast(abs_semitone / 12));

    if (semitone < 0) {
        r = r.reverse();
    }
    return r.apply(freq);
}

fn arpeggio(freq: u32, t: u32, semitones: []const i8) u32 {
    const semitone = semitones[t % semitones.len];
    return apply_semitone(freq, semitone);
}

fn vibrato(delay: u32, p: u16, freq: u32, t: u32) u32 {
    // Use almost half a semitone (0.475) amplitude for the delta triangle wave.
    // This fixed ratio is smaller than one so use the inverse ratio to avoid floating points.
    const inv_ratio = comptime @as(u32, @intFromFloat(math.round(1 / (math.pow(f32, 1.0594630943592953, 0.475) - 1))));
    const a = freq / inv_ratio;
    const delta = 1 + 4 * a / p * @abs(@mod((@mod(@as(i32, @intCast(t - delay)) - p / 4, p) + p), p) - p / 2) - a;
    return freq + delta;
}

const ADSR = struct {
    const State = enum {
        attack,
        decay,
        sustain,
        release,
    };
    level: i8 = 0,
    state: State = State.attack,
    attack_step: i8,
    decay_step: i8,
    sustain_level: i8,
    release_step: i8,

    /// Returns a new ADSR in the attack state using the provided envelope parameters:
    /// `attack_step` is the increment per `frame` call from 0 to 15 during the attack state.
    /// `decay_step` is the decrement from 15 to `sustain_level` during the decay state.
    /// `sustain_level` is volume during the sustain state.
    /// `release_step` is the decrement from `sustain_level` to 0 during the release state.
    pub fn init(attack_step: i8, decay_step: i8, sustain_level: i8, release_step: i8) ADSR {
        return ADSR{
            .attack_step = attack_step,
            .decay_step = decay_step,
            .sustain_level = sustain_level,
            .release_step = release_step,
        };
    }
    /// Call this once per instrument frame
    pub fn frame(self: *ADSR) u4 {
        switch (self.state) {
            .attack => {
                self.level += self.attack_step;
                if (self.level >= 15) {
                    self.state = State.decay;
                    self.level = 15;
                }
            },
            .decay => {
                self.level -= self.decay_step;
                if (self.level <= self.sustain_level) {
                    self.state = State.sustain;
                    self.level = self.sustain_level;
                }
            },
            .sustain => {},
            .release => {
                self.level -= self.release_step;
                if (self.level < 0) {
                    // Re-use the state
                    self.state = State.sustain;
                    self.level = 0;
                }
            },
        }
        return @intCast(self.level);
    }
    /// Call this when the instrument is released
    pub fn release(self: *ADSR) void {
        self.state = State.release;
        self.level = self.sustain_level;
    }

    /// Returns how many frames are needed to finish the release state after `release` is called.
    pub fn frames_after_release(self: ADSR) u32 {
        return self.sustain_level / self.release_step + 1;
    }
};

const adsr_template = ADSR.init(0x8, 0x5, 0xa, 0x3);
var square1_adsr = ADSR{
    .attack_step = 0,
    .decay_step = 0,
    .sustain_level = 0,
    .release_step = 0,
};
var square2_adsr = ADSR{
    .attack_step = 0,
    .decay_step = 0,
    .sustain_level = 0,
    .release_step = 0,
};
var wave_adsr = ADSR{
    .attack_step = 0,
    .decay_step = 0,
    .sustain_level = 0,
    .release_step = 0,
};

//=== The instruments definition starts here ===//

/// This is a very basic instruments that holds the note until it's released.
const square1_1 = struct {
    pub const id: [*:0]const u8 = "S1";

    pub fn press(freq: u32, _: u8, _: i8, _: i8) callconv(.C) void {
        // Reset the Sweep here since another instrument might have set it.
        gba.Sweep.init().writeTo(gba.square1);
        // Set the volume to 0xa out of 0xf (4 bits).
        gba.EnvDutyLen.init()
            .withEnvStart(0xa)
            .withDuty(gba.dut_2_4)
            .writeTo(gba.square1);
        // Set the frequency and trigger the note, this also applies the envelope.
        gba.CtrlFreq.init()
            .withSquareFreq(freq)
            .withTrigger(1)
            .writeTo(gba.square1);
    }
    pub fn release(freq: u32, _: u8, _: u32) callconv(.C) void {
        // Set a decreasing envelope from 0xa down to 0x0
        // with a decrease of 1 per env clock (64Hz).
        gba.EnvDutyLen.init()
            .withEnvDir(gba.env_dec)
            .withEnvStart(0xa)
            .withEnvInterval(1)
            .withDuty(gba.dut_2_4)
            .writeTo(gba.square1);
        gba.CtrlFreq.init()
            .withSquareFreq(freq)
            .withTrigger(1)
            .writeTo(gba.square1);
    }
};

/// A square instrument with a vibrato effect.
/// `p0`: duty (0-3)
/// `p1`: vibrato period
const square1_2 = struct {
    pub const id: [*:0]const u8 = "S2";
    pub const frames_after_release: u32 = adsr_template.frames_after_release();

    var env_duty = gba.EnvDutyLen{ .duty = gba.dut_1_4 };
    var p: u16 = 8;
    fn set_duty(val: i8) void {
        env_duty.duty = @intCast(val);
    }
    fn set_p(val: i8) void {
        p = @max(1, @as(u16, @intCast(val)));
    }

    pub fn press(_: u32, _: u8, duty_val: i8, p_val: i8) callconv(.C) void {
        set_duty(duty_val);
        set_p(if (p_val != 0) p_val else 12);
        square1_adsr = adsr_template;

        gba.Sweep.init().writeTo(gba.square1);
    }
    pub fn release(_: u32, _: u8, _: u32) callconv(.C) void {
        square1_adsr.release();
    }

    pub fn frame(freq: u32, _: u8, t: u32) callconv(.C) void {
        const delay = 21;
        env_duty
            .withEnvStart(square1_adsr.frame())
            .writeTo(gba.square1);

        gba.CtrlFreq.init()
            .withSquareFreq(if (t > delay) vibrato(delay, p, freq, t) else freq)
            .withTrigger(1)
            .writeTo(gba.square1);
    }
    pub fn set_param(param_num: u8, val: i8) callconv(.C) void {
        if (param_num == 0)
            set_duty(val)
        else
            set_p(if (val != 0) val else 12);
    }
};

/// Using the length counter for a short bleep.
/// `p0`: duty (0-3)
const square1_3 = struct {
    pub const id: [*:0]const u8 = "S3";

    pub fn press(freq: u32, _: u8, p0: i8, _: i8) callconv(.C) void {
        gba.Sweep.init().writeTo(gba.square1);
        gba.EnvDutyLen.init()
            .withDuty(@intCast(p0))
            .withEnvDir(gba.env_dec)
            .withEnvStart(0xa)
            .withEnvInterval(1)
            .withLength(48)
            .writeTo(gba.square1);
        gba.CtrlFreq.init()
            .withSquareFreq(freq)
            .withTrigger(1)
            .withLengthEnabled(1)
            .writeTo(gba.square1);
    }
};

/// An instrument alternating the duty cycle every 2 frames.
const square1_4 = struct {
    pub const id: [*:0]const u8 = "S4";
    pub const frames_after_release: u32 = adsr_template.frames_after_release();

    pub fn press(_: u32, _: u8, _: i8, _: i8) callconv(.C) void {
        square1_adsr = adsr_template;
        gba.Sweep.init().writeTo(gba.square1);
    }
    pub fn release(_: u32, _: u8, _: u32) callconv(.C) void {
        square1_adsr.release();
    }
    pub fn frame(freq: u32, _: u8, t: u32) callconv(.C) void {
        const duties = [_]u2{
            gba.dut_1_4,
            gba.dut_2_4,
            gba.dut_3_4,
            gba.dut_2_4,
        };

        gba.EnvDutyLen.init()
            .withDuty(duties[(t / 2) % duties.len])
            .withEnvStart(square1_adsr.frame())
            .writeTo(gba.square1);
        gba.CtrlFreq.init()
            .withSquareFreq(freq)
            .withTrigger(1)
            .writeTo(gba.square1);
    }
};

/// Sweep the frequency down with an automatic envelope.
const square1_5 = struct {
    pub const id: [*:0]const u8 = "S5";

    pub fn press(freq: u32, _: u8, _: i8, _: i8) callconv(.C) void {
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

/// Example of an instrument that uses both square channels and applies a vibrato effect to both.
/// `p0`: semitones detune
const square2_1 = struct {
    pub const id: [*:0]const u8 = "T1";
    // Keep calling frame until the envelope is finished
    pub const frames_after_release: u32 = 13;

    var steps: i8 = 0;
    pub fn press(freq: u32, _: u8, p0: i8, _: i8) callconv(.C) void {
        steps = if (p0 == 0) 4 else p0;

        gba.Sweep.init().writeTo(gba.square1);
        (gba.EnvDutyLen{ .duty = gba.dut_3_4, .env_start = 10 })
            .writeTo(gba.square1);
        (gba.EnvDutyLen{ .duty = gba.dut_2_4, .env_start = 13 })
            .writeTo(gba.square2);
        gba.CtrlFreq.init()
            .withTrigger(1)
            .withSquareFreq(freq)
            .writeTo(gba.square1);
        gba.CtrlFreq.init()
            .withTrigger(1)
            .withSquareFreq(apply_semitone(freq, steps))
            .writeTo(gba.square2);
    }
    pub fn frame(freq: u32, _: u8, t: u32) callconv(.C) void {
        const delay = 14;
        const p = 12;
        if (t > delay) {
            gba.CtrlFreq.init()
                .withSquareFreq(vibrato(delay, p, freq, t))
                .writeTo(gba.square1);
            // Same vibrato parameters for the second square channel but phase it so that it's opposite.
            gba.CtrlFreq.init()
                .withSquareFreq(vibrato(delay + p / 2, p, apply_semitone(freq, steps), t))
                .writeTo(gba.square2);
        }
    }
    pub fn release(freq: u32, _: u8, _: u32) callconv(.C) void {
        (gba.EnvDutyLen{ .duty = gba.dut_3_4, .env_interval = 1, .env_dir = gba.env_dec, .env_start = 10 })
            .writeTo(gba.square1);
        (gba.EnvDutyLen{ .duty = gba.dut_2_4, .env_interval = 1, .env_dir = gba.env_dec, .env_start = 13 })
            .writeTo(gba.square2);
        gba.CtrlFreq.init()
            .withTrigger(1)
            .withSquareFreq(freq)
            .writeTo(gba.square1);
        gba.CtrlFreq.init()
            .withTrigger(1)
            .withSquareFreq(apply_semitone(freq, steps))
            .writeTo(gba.square2);
    }
};

/// Arpeggio effect alternating between 3 tones based on the sequenced note.
/// `p0`: arpeggio note 1 semitones
/// `p1`: arpeggio note 2 semitones
const square2_2 = struct {
    pub const id: [*:0]const u8 = "T2";
    pub const frames_after_release: u32 = 24;
    var semitones = [_]i8{ 0, 4, 7, 12 };

    pub fn press(_: u32, _: u8, p0: i8, p1: i8) callconv(.C) void {
        square2_adsr = adsr_template;

        semitones[1] = if (p0 == 0) 4 else p0;
        semitones[2] = if (p1 == 0) 7 else p1;
    }
    pub fn frame(freq: u32, _: u8, t: u32) callconv(.C) void {
        gba.EnvDutyLen.init()
            .withDuty(gba.dut_2_4)
            .withEnvStart(square2_adsr.frame())
            .writeTo(gba.square2);
        gba.CtrlFreq.init()
            .withSquareFreq(arpeggio(freq, t, &semitones))
            .withTrigger(1)
            .writeTo(gba.square2);
    }
    pub fn release(_: u32, _: u8, _: u32) callconv(.C) void {
        square2_adsr.release();
    }
};

/// Square instrument with a switch effect between the left and right channels.
/// `p0`: left channel switch period
/// `p1`: right channel switch period
const square2_3 = struct {
    pub const id: [*:0]const u8 = "T3";

    var left_p: u7 = 0;
    var right_p: u7 = 0;
    pub fn press(freq: u32, _: u8, p0: i8, p1: i8) callconv(.C) void {
        left_p = if (p0 == 0) 4 else @intCast(p0);
        right_p = if (p1 == 0) 5 else @intCast(p1);

        (gba.EnvDutyLen{ .duty = gba.dut_2_4, .env_start = 13 })
            .writeTo(gba.square2);
        gba.CtrlFreq.init()
            .withTrigger(1)
            .withSquareFreq(freq)
            .writeTo(gba.square2);
    }
    pub fn frame(_: u32, _: u8, t: u32) callconv(.C) void {
        const u7t: u7 = @intCast(t);
        // Every p0 frames, switch the square2 left channel.
        if (u7t % left_p == 0)
            sound_ctrl.square2_l ^= 1;
        // Every p1 frames for the right.
        if (u7t % right_p == 0)
            sound_ctrl.square2_r ^= 1;
        sound_ctrl.write();
    }
    pub fn release(freq: u32, _: u8, _: u32) callconv(.C) void {
        (gba.EnvDutyLen{ .duty = gba.dut_2_4, .env_interval = 1, .env_dir = gba.env_dec, .env_start = 13 })
            .writeTo(gba.square2);
        gba.CtrlFreq.init()
            .withTrigger(1)
            .withSquareFreq(freq)
            .writeTo(gba.square2);

        // Re-enable left+right channels.
        sound_ctrl.square2_l = 1;
        sound_ctrl.square2_r = 1;
        sound_ctrl.write();
    }
};

/// Basic square instrument with a manual per frame envelope instead of using EnvDutyLen's automatic envelope.
const square2_4 = struct {
    pub const id: [*:0]const u8 = "T4";
    pub const frames_after_release: u32 = adsr_template.frames_after_release();

    pub fn press(_: u32, _: u8, _: i8, _: i8) callconv(.C) void {
        square2_adsr = adsr_template;
    }
    pub fn release(_: u32, _: u8, _: u32) callconv(.C) void {
        square2_adsr.release();
    }
    pub fn frame(freq: u32, _: u8, _: u32) callconv(.C) void {
        gba.EnvDutyLen.init()
            .withDuty(gba.dut_3_4)
            .withEnvStart(square2_adsr.frame())
            .writeTo(gba.square2);
        gba.CtrlFreq.init()
            .withTrigger(1)
            .withSquareFreq(freq)
            .writeTo(gba.square2);
    }
};

const wave_env_frames = [_]gba.WaveVolLen{
    .{ .volume = gba.vol_75 },
    .{ .volume = gba.vol_50 },
    .{ .volume = gba.vol_25 },
    .{ .volume = gba.vol_0 },
};
var wave_released_at: ?u32 = null;
fn wave_p(freq: u32, table: *const gba.WavTable) void {
    gba.WaveRam.setTable(table);
    gba.WaveVolLen.init()
        .withVolume(gba.vol_100)
        .writeTo(gba.wave);
    gba.CtrlFreq.init()
        .withWaveFreq(freq)
        .withTrigger(1)
        .writeTo(gba.wave);
    wave_released_at = null;
}
fn wave_env_r(_: u32, _: u8, t: u32) callconv(.C) void {
    wave_released_at = t;
}
fn wave_env_f(_: u32, _: u8, t: u32) callconv(.C) void {
    if (wave_released_at) |r_frame| {
        if (t - r_frame < wave_env_frames.len)
            wave_env_frames[t - r_frame].writeTo(gba.wave);
    }
}

/// Triangle wave
const wave_1 = struct {
    pub const id: [*:0]const u8 = "W1";

    const table = gba.wav(0x0123456789abcdeffedcba9876543210);
    pub fn press(freq: u32, _: u8, _: i8, _: i8) callconv(.C) void {
        wave_p(freq, &table);
    }
    pub const release = wave_env_r;
    pub const frame = wave_env_f;
    pub const frames_after_release: u32 = 4;
};

/// Bass-like wave sound when played at lower frequencies.
const wave_2 = struct {
    pub const id: [*:0]const u8 = "W2";

    const table = gba.wav(0x11235678999876679adffec985421131);
    pub fn press(freq: u32, _: u8, _: i8, _: i8) callconv(.C) void {
        wave_p(freq, &table);
    }

    pub const release = wave_env_r;
    pub const frame = wave_env_f;
    pub const frames_after_release: u32 = 4;
};

/// Arpeggio effect on a ramp-up wave shape.
const wave_3 = struct {
    pub const id: [*:0]const u8 = "W3";
    pub const frames_after_release: u32 = 4;
    var semitones = [_]i8{ 0, 4, 7, 12 };

    const table = gba.wav(0xdedcba98765432100000000011111111);
    pub fn press(freq: u32, _: u8, p0: i8, p1: i8) callconv(.C) void {
        wave_p(freq, &table);
        semitones[1] = if (p0 == 0) 4 else p0;
        semitones[2] = if (p1 == 0) 7 else p1;
    }
    pub fn frame(freq: u32, _: u8, t: u32) callconv(.C) void {
        gba.CtrlFreq.init()
            .withWaveFreq(arpeggio(freq, t, &semitones))
            .writeTo(gba.wave);
        if (wave_released_at) |r_frame| {
            if (t - r_frame < wave_env_frames.len)
                wave_env_frames[t - r_frame].writeTo(gba.wave);
        }
    }
    pub const release = wave_env_r;
};

const wave_4 = struct {
    pub const id: [*:0]const u8 = "W4";

    const table = gba.wav(0xf0f0f0f0f0f0f0f0ff00ff00ff00ff00);
    pub fn press(freq: u32, _: u8, _: i8, _: i8) callconv(.C) void {
        wave_p(freq, &table);
    }
    pub const release = wave_env_r;
    pub const frame = wave_env_f;
    pub const frames_after_release: u32 = 4;
};

/// Sweep up a by number of semitones each frame.
/// `p0`: number of semitones to sweep up
const wave_5 = struct {
    pub const id: [*:0]const u8 = "W5";
    pub const frames_after_release: u32 = 16;

    const table = gba.wav(0x0234679acdffffeeeeffffdca9764310);
    var steps: u32 = 4;
    var current_step_freq: u32 = 0;
    pub fn press(freq: u32, _: u8, p0: i8, _: i8) callconv(.C) void {
        gba.WaveRam.setTable(&table);
        gba.WaveVolLen.init()
            .withVolume(gba.vol_100)
            .writeTo(gba.wave);
        gba.CtrlFreq.init()
            .withWaveFreq(freq)
            .withTrigger(1)
            .writeTo(gba.wave);
        wave_released_at = 12;
        steps = if (p0 == 0) 4 else @as(u32, @intCast(p0)) % 12;
        current_step_freq = freq;
    }
    pub fn frame(_: u32, _: u8, t: u32) callconv(.C) void {
        gba.CtrlFreq.init()
            .withWaveFreq(semitones_steps(steps, &current_step_freq))
            .writeTo(gba.wave);
        if (wave_released_at) |r_frame| {
            if (t - r_frame < wave_env_frames.len)
                wave_env_frames[t - r_frame].writeTo(gba.wave);
        }
    }
};

/// A noise instrument with different pre-defined sounds per note.
const noise_1 = struct {
    pub const id: [*:0]const u8 = "N1";
    pub const frames_after_release: u32 = 15;

    // Different sounds must update the sound chip over multiple frames but the sound is selected
    // on press. So keep a slice to the selected static lifetime tables of register values so
    // that the frame function can use it.
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
        // Ignore the frequency but use the MIDI note number to select which sound to play.
        switch (note % 12) {
            0 => {
                const Static = struct {
                    const env = .{
                        .{ .env_start = 7, .env_dir = gba.env_dec, .env_interval = 1 },
                    };
                    const ctrl = .{
                        .{ .freq = 1, .width = gba.wid_15, .freq_div = gba.div_8, .trigger = 1 },
                    };
                };
                env_frames = &Static.env;
                ctrl_frames = &Static.ctrl;
            },
            1 => {
                const Static = struct {
                    const env = .{
                        .{ .env_start = 10, .env_dir = gba.env_dec, .env_interval = 1 },
                    };
                    const ctrl = .{
                        .{ .freq = 7, .width = gba.wid_7, .freq_div = gba.div_16, .trigger = 1 },
                        .{ .freq = 6, .width = gba.wid_7, .freq_div = gba.div_16 },
                        .{ .freq = 5, .width = gba.wid_7, .freq_div = gba.div_16 },
                        .{ .freq = 5, .width = gba.wid_15, .freq_div = gba.div_16 },
                    };
                };
                env_frames = &Static.env;
                ctrl_frames = &Static.ctrl;
            },
            2 => {
                const Static = struct {
                    const env = .{
                        .{ .env_start = 7, .env_dir = gba.env_dec, .env_interval = 2 },
                    };
                    const ctrl = .{
                        .{ .freq = 1, .width = gba.wid_15, .freq_div = gba.div_16, .trigger = 1 },
                        .{ .freq = 1, .width = gba.wid_15, .freq_div = gba.div_32 },
                        .{ .freq = 1, .width = gba.wid_15, .freq_div = gba.div_48 },
                        .{ .freq = 1, .width = gba.wid_15, .freq_div = gba.div_64 },
                        .{ .freq = 1, .width = gba.wid_15, .freq_div = gba.div_80 },
                    };
                };
                env_frames = &Static.env;
                ctrl_frames = &Static.ctrl;
            },
            3 => {
                const Static = struct {
                    const env = .{
                        .{ .env_start = 10, .env_dir = gba.env_dec, .env_interval = 1 },
                    };
                    const ctrl = .{
                        .{ .freq = 5, .width = gba.wid_7, .freq_div = gba.div_16, .trigger = 1 },
                        .{ .freq = 5, .width = gba.wid_7, .freq_div = gba.div_48 },
                        .{ .freq = 5, .width = gba.wid_7, .freq_div = gba.div_48 },
                        .{ .freq = 5, .width = gba.wid_7, .freq_div = gba.div_80 },
                        .{ .freq = 5, .width = gba.wid_7, .freq_div = gba.div_112 },
                        .{ .freq = 6, .width = gba.wid_15, .freq_div = gba.div_8 },
                    };
                };
                env_frames = &Static.env;
                ctrl_frames = &Static.ctrl;
            },
            4 => {
                const Static = struct {
                    const env = .{
                        .{ .env_start = 10, .env_dir = gba.env_dec, .env_interval = 2 },
                    };
                    const ctrl = .{
                        .{ .freq = 5, .width = gba.wid_7, .freq_div = gba.div_16, .trigger = 1 },
                        .{ .freq = 7, .width = gba.wid_7, .freq_div = gba.div_16 },
                        .{ .freq = 6, .width = gba.wid_7, .freq_div = gba.div_16 },
                        .{ .freq = 5, .width = gba.wid_15, .freq_div = gba.div_8 },
                        .{ .freq = 5, .width = gba.wid_15, .freq_div = gba.div_8 },
                        .{ .freq = 5, .width = gba.wid_15, .freq_div = gba.div_16 },
                        .{ .freq = 4, .width = gba.wid_15, .freq_div = gba.div_16 },
                        .{ .freq = 5, .width = gba.wid_15, .freq_div = gba.div_16 },
                    };
                };
                env_frames = &Static.env;
                ctrl_frames = &Static.ctrl;
            },
            5 => {
                const Static = struct {
                    const env = .{
                        .{ .env_start = 9, .env_dir = gba.env_dec, .env_interval = 0 },
                        .{ .env_start = 8, .env_dir = gba.env_dec, .env_interval = 0 },
                        .{ .env_start = 3, .env_dir = gba.env_dec, .env_interval = 0 },
                        .{ .env_start = 3, .env_dir = gba.env_dec, .env_interval = 4 },
                        null,
                        null,
                        null,
                        .{ .env_start = 6, .env_dir = gba.env_dec, .env_interval = 0 },
                        .{ .env_start = 4, .env_dir = gba.env_dec, .env_interval = 0 },
                        .{ .env_start = 2, .env_dir = gba.env_dec, .env_interval = 0 },
                        .{ .env_start = 0, .env_dir = gba.env_dec, .env_interval = 3 },
                    };
                    const ctrl = .{
                        .{ .freq = 0, .width = gba.wid_15, .freq_div = 4, .trigger = 1 },
                        .{ .freq = 0, .width = gba.wid_15, .freq_div = 2, .trigger = 1 },
                        .{ .freq = 0, .width = gba.wid_15, .freq_div = 6, .trigger = 1 },
                        .{ .freq = 0, .width = gba.wid_15, .freq_div = 3, .trigger = 1 },
                        null,
                        null,
                        null,
                        .{ .freq = 0, .width = gba.wid_15, .freq_div = 4, .trigger = 1 },
                        .{ .freq = 0, .width = gba.wid_15, .freq_div = 2, .trigger = 1 },
                        .{ .freq = 0, .width = gba.wid_15, .freq_div = 1, .trigger = 1 },
                        .{ .freq = 0, .width = gba.wid_15, .freq_div = 1, .trigger = 1 },
                    };
                };
                env_frames = &Static.env;
                ctrl_frames = &Static.ctrl;
            },
            6 => {
                const Static = struct {
                    const env = .{
                        .{ .env_start = 13, .env_dir = gba.env_dec, .env_interval = 0 },
                        .{ .env_start = 13, .env_dir = gba.env_dec, .env_interval = 0 },
                        .{ .env_start = 11, .env_dir = gba.env_dec, .env_interval = 0 },
                        .{ .env_start = 7, .env_dir = gba.env_dec, .env_interval = 0 },
                        .{ .env_start = 5, .env_dir = gba.env_dec, .env_interval = 0 },
                        .{ .env_start = 3, .env_dir = gba.env_dec, .env_interval = 0 },
                        .{ .env_start = 2, .env_dir = gba.env_dec, .env_interval = 1 },
                        .{ .env_start = 6, .env_dir = gba.env_dec, .env_interval = 0 },
                        .{ .env_start = 4, .env_dir = gba.env_dec, .env_interval = 0 },
                        .{ .env_start = 2, .env_dir = gba.env_dec, .env_interval = 0 },
                        .{ .env_start = 0, .env_dir = gba.env_dec, .env_interval = 3 },
                    };
                    const ctrl = .{
                        .{ .freq = 0, .width = gba.wid_15, .freq_div = 2, .trigger = 1 },
                        .{ .freq = 5, .width = gba.wid_15, .freq_div = 1, .trigger = 1 },
                        .{ .freq = 6, .width = gba.wid_15, .freq_div = 1, .trigger = 1 },
                        .{ .freq = 7, .width = gba.wid_15, .freq_div = 1, .trigger = 1 },
                        .{ .freq = 9, .width = gba.wid_15, .freq_div = 1, .trigger = 1 },
                        .{ .freq = 7, .width = gba.wid_15, .freq_div = 1, .trigger = 1 },
                        .{ .freq = 6, .width = gba.wid_15, .freq_div = 0, .trigger = 1 },
                        .{ .freq = 0, .width = gba.wid_15, .freq_div = 4, .trigger = 1 },
                        .{ .freq = 0, .width = gba.wid_15, .freq_div = 2, .trigger = 1 },
                        .{ .freq = 0, .width = gba.wid_15, .freq_div = 1, .trigger = 1 },
                        .{ .freq = 0, .width = gba.wid_15, .freq_div = 1, .trigger = 1 },
                    };
                };
                env_frames = &Static.env;
                ctrl_frames = &Static.ctrl;
            },
            7 => {
                const Static = struct {
                    const env = .{
                        .{ .env_start = 13, .env_dir = gba.env_dec, .env_interval = 0 },
                        .{ .env_start = 13, .env_dir = gba.env_dec, .env_interval = 0 },
                        .{ .env_start = 13, .env_dir = gba.env_dec, .env_interval = 0 },
                        .{ .env_start = 8, .env_dir = gba.env_dec, .env_interval = 0 },
                        .{ .env_start = 1, .env_dir = gba.env_dec, .env_interval = 0 },
                        .{ .env_start = 2, .env_dir = gba.env_dec, .env_interval = 0 },
                        .{ .env_start = 3, .env_dir = gba.env_dec, .env_interval = 3 },
                        .{ .env_start = 6, .env_dir = gba.env_dec, .env_interval = 0 },
                        .{ .env_start = 4, .env_dir = gba.env_dec, .env_interval = 0 },
                        .{ .env_start = 2, .env_dir = gba.env_dec, .env_interval = 0 },
                        .{ .env_start = 0, .env_dir = gba.env_dec, .env_interval = 3 },
                    };
                    const ctrl = .{
                        .{ .freq = 6, .width = gba.wid_15, .freq_div = 0, .trigger = 1 },
                        .{ .freq = 5, .width = gba.wid_15, .freq_div = 2, .trigger = 1 },
                        .{ .freq = 4, .width = gba.wid_15, .freq_div = 2, .trigger = 1 },
                        .{ .freq = 4, .width = gba.wid_15, .freq_div = 1, .trigger = 1 },
                        .{ .freq = 2, .width = gba.wid_15, .freq_div = 2, .trigger = 1 },
                        .{ .freq = 1, .width = gba.wid_15, .freq_div = 1, .trigger = 1 },
                        .{ .freq = 0, .width = gba.wid_15, .freq_div = 4, .trigger = 1 },
                        .{ .freq = 0, .width = gba.wid_15, .freq_div = 4, .trigger = 1 },
                        .{ .freq = 0, .width = gba.wid_15, .freq_div = 2, .trigger = 1 },
                        .{ .freq = 0, .width = gba.wid_15, .freq_div = 1, .trigger = 1 },
                        .{ .freq = 0, .width = gba.wid_15, .freq_div = 1, .trigger = 1 },
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

/// A noise instrument that derives the sound parameters arbitrarily from the frequency.
const noise_2 = struct {
    pub const id: [*:0]const u8 = "N2";

    pub fn press(freq: u32, _: u8, _: i8, _: i8) callconv(.C) void {
        gba.EnvDutyLen.init()
            .withEnvStart(0xf)
            .withEnvDir(gba.env_dec)
            .withEnvInterval(1)
            .writeTo(gba.noise);
        // Use the frequency as input just so that different keys produce different sounds.
        const gb_freq = gba.encodeSquareFreq(freq);
        gba.NoiseCtrlFreq.init()
            .withFreq(@truncate(gb_freq >> 4))
            .withCounterWidth(gba.wid_15)
            .withFreqDiv(@truncate(gb_freq))
            .withTrigger(1)
            .writeTo(gba.noise);
    }
};

/// Instruments are compiled as executables but this is actually going to be executed like a library
/// entry point. Instruments structs register their functions as callback and they will
/// be called when needed after this function returns.
/// Instruments that are not registered are not visible to the application.
pub fn main() void {
    ct.registerInstrument(square1_1, 0);
    ct.registerInstrument(square1_2, 0);
    ct.registerInstrument(square1_3, 0);
    ct.registerInstrument(square1_4, 0);
    ct.registerInstrument(square1_5, 0);
    ct.registerInstrument(square2_1, 1);
    ct.registerInstrument(square2_2, 1);
    ct.registerInstrument(square2_3, 1);
    ct.registerInstrument(square2_4, 1);
    ct.registerInstrument(wave_1, 2);
    ct.registerInstrument(wave_2, 2);
    ct.registerInstrument(wave_3, 2);
    ct.registerInstrument(wave_4, 2);
    ct.registerInstrument(wave_5, 2);
    ct.registerInstrument(noise_1, 3);
    ct.registerInstrument(noise_2, 3);
}
