// Copyright © 2024 Jocelyn Turcotte <turcotte.j@gmail.com>
// SPDX-License-Identifier: CC0-1.0
//
// This file contains convenience structs to write commands to the GBA's Programmable Sound Generator (PSG) registers.
// This is compiled together with the instruments implementation into a WebAssembly file that sits together with
// the song containing the sequencer patterns, and Chiptrack will execute those function each time an instrument
// note is pressed, release or held (on each frame they are).

const std = @import("std");
const fmt = std.fmt;

pub const press_fn = *const fn (freq: u32, note: u8, param0: i8, param1: i8) callconv(.C) void;
pub const release_fn = *const fn (freq: u32, note: u8, t: u32) callconv(.C) void;
pub const frame_fn = *const fn (freq: u32, note: u8, t: u32) callconv(.C) void;
pub const set_param_fn = *const fn (param_num: u8, value: i8) callconv(.C) void;

// These few functions defines the WebAssembly interface between the guest (instruments) and the host (Chiptrack).
extern fn print([*:0]const u8) void;
extern fn gba_set_sound_reg(addr: u32, value: u32) void;
extern fn gba_set_wave_table(table: [*]const u8, table_len: u32) void;
extern fn set_instrument_at_column(id: [*:0]const u8, col: u32, frames_after_release: u32, press: ?press_fn, release: ?release_fn, frame: ?frame_fn, set_param: ?set_param_fn) void;

/// Instructs Chiptrack to log a message to the console during an instrument's callback function.
/// This is useful for debugging the instrument's behavior and can be used like this:
///   ct.debug("The note is {}.", .{ note });
pub fn debug(comptime f: []const u8, args: anytype) void {
    var b: [256]u8 = undefined;
    const r = fmt.bufPrint(&b, f, args) catch unreachable;
    b[r.len] = 0;
    print(@ptrCast(&b));
}

const Instrument = struct {
    press: ?press_fn = null,
    release: ?release_fn = null,
    frame: ?frame_fn = null,
    set_param: ?set_param_fn = null,
    frames_after_release: u32 = 0,
};

/// Registers an instrument using parameters and function pointers provided through an Instrument struct instance.
pub fn setInstrument(id: [*:0]const u8, col: u32, instrument: Instrument) void {
    set_instrument_at_column(id, col, instrument.frames_after_release, instrument.press, instrument.release, instrument.frame, instrument.set_param);
}

/// Registers an instrument struct that has the following mandatory public static declaration (not field):
/// - id: A null-terminated string identifying the instrument in the song's pattern definitions
/// And the following optional public static declarations (not fields):
/// - press: a function called at the start of each sequencer press step
/// - release: a function called at the end of each sequencer release step
/// - frame: a function called on every frame between press and release
/// - set_param: a function called on every step between press and release (exclusively) where the instrument parameters are changed
/// - frames_after_release: a u32 that can extend the number of frames for which the frame function is called after the release step
/// If any optional declaration is mis-spelled or non-public, it will be silently ignored.
pub fn registerInstrument(comptime instrument: anytype, col: u32) void {
    const press: ?press_fn = if (@hasDecl(instrument, "press")) instrument.press else null;
    const release: ?release_fn = if (@hasDecl(instrument, "release")) instrument.release else null;
    const frame: ?frame_fn = if (@hasDecl(instrument, "frame")) instrument.frame else null;
    const set_param: ?set_param_fn = if (@hasDecl(instrument, "set_param")) instrument.set_param else null;
    const far: u32 = if (@hasDecl(instrument, "frames_after_release")) instrument.frames_after_release else 0;
    set_instrument_at_column(instrument.id, col, far, press, release, frame, set_param);
}

/// See the following resources for more information on the GB's and GBA's PSG
/// that can be referred to when implementing instruments:
/// https://rust-console.github.io/gbatek-gbaonly/#gbasoundcontroller
/// http://belogic.com/gba/
/// https://www.copetti.org/writings/consoles/game-boy/#audio
/// https://www.coranac.com/tonc/text/sndsqr.htm
/// https://gbdev.io/pandocs/Audio.html
/// https://gbdev.gg8.se/wiki/articles/Gameboy_sound_hardware
pub const gba = struct {
    pub const nr10 = 0x4000060;
    pub const nr11_12 = 0x4000062;
    pub const nr13_14 = 0x4000064;
    pub const nr21_22 = 0x4000068;
    pub const nr23_24 = 0x400006C;
    pub const nr30 = 0x4000070;
    pub const nr31_32 = 0x4000072;
    pub const nr33_34 = 0x4000074;
    pub const nr41_42 = 0x4000078;
    pub const nr43_44 = 0x400007C;
    pub const nr50_51 = 0x4000080;

    pub fn encodeSquareFreq(freq: u32) u11 {
        return @truncate(2048 - ((131072 * 256) / freq));
    }
    pub fn encodeWaveFreq(freq: u32) u11 {
        return @truncate(2048 - ((65536 * 256) / freq));
    }

    pub const Channel = enum {
        square1,
        square2,
        wave,
        noise,

        pub fn encodeFreq(channel: Channel, freq: u32) u11 {
            return switch (channel) {
                Channel.square1 => encodeSquareFreq(freq),
                Channel.square2 => encodeSquareFreq(freq),
                Channel.noise => encodeWaveFreq(freq),
                else => unreachable,
            };
        }
    };
    pub const square1 = Channel.square1;
    pub const square2 = Channel.square2;
    pub const wave = Channel.wave;
    pub const noise = Channel.noise;

    pub const swe_inc: u1 = 0;
    pub const swe_dec: u1 = 1;
    pub const env_dec: u1 = 0;
    pub const env_inc: u1 = 1;
    pub const dut_1_8: u2 = 0;
    pub const dut_1_4: u2 = 1;
    pub const dut_2_4: u2 = 2;
    pub const dut_3_4: u2 = 3;
    pub const vol_0: u3 = 0;
    pub const vol_100: u3 = 1;
    pub const vol_50: u3 = 2;
    pub const vol_25: u3 = 3;
    pub const vol_75: u3 = 5;
    pub const wid_15: u1 = 0;
    pub const wid_7: u1 = 1;
    pub const div_8: u3 = 0;
    pub const div_16: u3 = 1;
    pub const div_32: u3 = 2;
    pub const div_48: u3 = 3;
    pub const div_64: u3 = 4;
    pub const div_80: u3 = 5;
    pub const div_96: u3 = 6;
    pub const div_112: u3 = 7;

    /// (NR10) - Channel 1 Sweep register (R/W)
    pub const Sweep = packed struct {
        // Bit        Expl.
        // 0-2   R/W  Number of sweep shift      (n=0-7)
        shift: u3 = 0,
        // 3     R/W  Sweep Frequency Direction  (0=Increase, 1=Decrease)
        dir: u1 = 1,
        // 4-6   R/W  Sweep Time; units of 7.8ms (0-7, min=7.8ms, max=54.7ms)
        time: u3 = 0,
        // 7-15  -    Not used
        _: u9 = 0,

        pub fn init() Sweep {
            return Sweep{};
        }
        pub fn withShift(self: Sweep, v: u3) Sweep {
            var copy = self;
            copy.shift = v;
            return copy;
        }
        pub fn withDir(self: Sweep, v: u1) Sweep {
            var copy = self;
            copy.dir = v;
            return copy;
        }
        pub fn withTime(self: Sweep, v: u3) Sweep {
            var copy = self;
            copy.time = v;
            return copy;
        }
        pub fn address(channel: Channel) u32 {
            return switch (channel) {
                Channel.square1 => nr10,
                else => unreachable,
            };
        }
        pub fn writeTo(self: Sweep, channel: Channel) void {
            gba_set_sound_reg(address(channel), @as(u16, @bitCast(self)));
        }
    };

    /// (NRx1, NRx2) - Duty/Len/Envelope (R/W)
    pub const EnvDutyLen = packed struct {
        ///  Bit        Expl.
        ///  0-5   W    Sound length; units of (64-n)/256s  (0-63)
        length: u6 = 0,
        ///  6-7   R/W  Wave Pattern Duty                   (0-3)
        duty: u2 = 0,
        ///  8-10  R/W  Envelope Step-Time; units of n/64s  (1-7, 0=No Envelope)
        env_interval: u3 = 0,
        ///  11    R/W  Envelope Direction                  (0=Decrease, 1=Increase)
        env_dir: u1 = 0,
        ///  12-15 R/W  Initial Volume of envelope          (1-15, 0=No Sound)
        env_start: u4 = 0,

        pub fn init() EnvDutyLen {
            return EnvDutyLen{};
        }
        pub fn withEnvStart(self: EnvDutyLen, v: u4) EnvDutyLen {
            var copy = self;
            copy.env_start = v;
            return copy;
        }
        pub fn withEnvDir(self: EnvDutyLen, v: u1) EnvDutyLen {
            var copy = self;
            copy.env_dir = v;
            return copy;
        }
        pub fn withEnvInterval(self: EnvDutyLen, v: u3) EnvDutyLen {
            var copy = self;
            copy.env_interval = v;
            return copy;
        }
        pub fn withDuty(self: EnvDutyLen, v: u2) EnvDutyLen {
            var copy = self;
            copy.duty = v;
            return copy;
        }
        pub fn withLength(self: EnvDutyLen, v: u6) EnvDutyLen {
            var copy = self;
            copy.length = v;
            return copy;
        }
        pub fn address(channel: Channel) u32 {
            return switch (channel) {
                Channel.square1 => nr11_12,
                Channel.square2 => nr21_22,
                Channel.noise => nr41_42,
                else => unreachable,
            };
        }
        pub fn writeTo(self: EnvDutyLen, channel: Channel) void {
            gba_set_sound_reg(address(channel), @as(u16, @bitCast(self)));
        }
    };

    /// (NRx3, NRx4) - Frequency/Control (R/W)
    pub const CtrlFreq = packed struct {
        ///  Bit        Expl.
        ///  0-10  W    Frequency; 131072/(2048-n)Hz  (0-2047)
        freq: u11 = 0,
        ///  11-13 -    Not used
        _: u3 = 0,
        ///  14    R/W  Length Flag  (1=Stop output when length in NR11 expires)
        length_enabled: u1 = 0,
        ///  15    W    Initial      (1=Restart Sound)
        trigger: u1 = 0,

        pub fn init() CtrlFreq {
            return CtrlFreq{};
        }
        pub fn withTrigger(self: CtrlFreq, v: u1) CtrlFreq {
            var copy = self;
            copy.trigger = v;
            return copy;
        }
        pub fn withLengthEnabled(self: CtrlFreq, v: u1) CtrlFreq {
            var copy = self;
            copy.length_enabled = v;
            return copy;
        }
        /// Sets an 11 bits native GBA freq value
        pub fn withFreq(self: CtrlFreq, v: u11) CtrlFreq {
            var copy = self;
            copy.freq = v;
            return copy;
        }
        /// Sets an 8 bits fixed point frequency, will be converted to a square native freq and set.
        pub fn withSquareFreq(self: CtrlFreq, freq: u32) CtrlFreq {
            return self.withFreq(encodeSquareFreq(freq));
        }
        /// Sets an 8 bits fixed point frequency, will be converted to a wave native freq and set.
        pub fn withWaveFreq(self: CtrlFreq, freq: u32) CtrlFreq {
            return self.withFreq(encodeWaveFreq(freq));
        }

        pub fn address(channel: Channel) u32 {
            return switch (channel) {
                Channel.square1 => nr13_14,
                Channel.square2 => nr23_24,
                Channel.wave => nr33_34,
                else => unreachable,
            };
        }
        pub fn writeTo(self: CtrlFreq, channel: Channel) void {
            gba_set_sound_reg(address(channel), @as(u16, @bitCast(self)));
        }
    };

    pub const WavTable = struct { v: [16]u8 };
    pub fn wav(comptime t: u128) WavTable {
        // A u128 literal will be stored in memory as little-endian but we
        // need them in the same order in memory to be passed as a u8 slice.
        return WavTable{ .v = @bitCast(@byteSwap(t)) };
    }

    /// (NR30) - Channel 3 Stop/Wave RAM select (R/W)
    pub const WaveRam = packed struct {
        ///  Bit        Expl.
        ///  0-4   -    Not used
        _: u5 = 0,
        ///  5     R/W  Wave RAM Dimension   (0=One bank/32 digits, 1=Two banks/64 digits)
        dimension: u1 = 0,
        ///  6     R/W  Wave RAM Bank Number (0-1, see below)
        bank: u1 = 0,
        ///  7     R/W  Sound Channel 3 Off  (0=Stop, 1=Playback)
        playing: u1 = 0,
        ///  8-15  -    Not used
        _2: u8 = 0,

        pub fn init() WaveRam {
            return WaveRam{};
        }
        pub fn withPlaying(self: WaveRam, v: u1) WaveRam {
            var copy = self;
            copy.playing = v;
            return copy;
        }

        pub fn address(channel: Channel) u32 {
            return switch (channel) {
                Channel.wave => nr30,
                else => unreachable,
            };
        }
        pub fn writeTo(self: WaveRam, channel: Channel) void {
            gba_set_sound_reg(address(channel), @as(u16, @bitCast(self)));
        }
        pub fn setTable(table: *const WavTable) void {
            (WaveRam{ .playing = 0 }).writeTo(wave);
            gba_set_wave_table(&table.v, table.v.len);
            (WaveRam{ .playing = 1 }).writeTo(wave);
        }
    };

    /// (NR31, NR32) - Channel 3 Length/Volume (R/W)
    pub const WaveVolLen = packed struct {
        /// Bit        Expl.
        /// 0-7   W    Sound length; units of (256-n)/256s  (0-255)
        length: u8 = 0,
        /// 8-12  -    Not used.
        _: u5 = 0,
        /// 13-14 R/W  Sound Volume  (0=Mute/Zero, 1=100%, 2=50%, 3=25%)
        /// 15    R/W  Force Volume  (0=Use above, 1=Force 75% regardless of above)
        volume: u3 = 0,

        pub fn init() WaveVolLen {
            return WaveVolLen{};
        }
        pub fn withVolume(self: WaveVolLen, v: u3) WaveVolLen {
            var copy = self;
            copy.volume = v;
            return copy;
        }
        pub fn withLength(self: WaveVolLen, v: u8) WaveVolLen {
            var copy = self;
            copy.length = v;
            return copy;
        }

        pub fn address(channel: Channel) u32 {
            return switch (channel) {
                Channel.wave => nr31_32,
                else => unreachable,
            };
        }
        pub fn writeTo(self: WaveVolLen, channel: Channel) void {
            gba_set_sound_reg(address(channel), @as(u16, @bitCast(self)));
        }
    };

    /// (NR43, NR44) - Channel 4 Frequency/Control (R/W)
    pub const NoiseCtrlFreq = packed struct {
        ///  Bit        Expl.
        ///  0-2   R/W  Dividing Ratio of Frequencies
        freq_div: u3 = 0,
        ///  3     R/W  Counter Step/Width (0=15 bits, 1=7 bits)
        width: u1 = 0,
        ///  4-7   R/W  Shift Clock Frequency
        freq: u4 = 0,
        ///  8-13  -    Not used
        _: u6 = 0,
        ///  14    R/W  Length Flag  (1=Stop output when length in NR41 expires)
        length_enabled: u1 = 0,
        ///  15    W    Initial      (1=Restart Sound)
        trigger: u1 = 0,

        pub fn init() NoiseCtrlFreq {
            return NoiseCtrlFreq{};
        }
        pub fn withTrigger(self: NoiseCtrlFreq, v: u1) NoiseCtrlFreq {
            var copy = self;
            copy.trigger = v;
            return copy;
        }
        pub fn withLengthEnabled(self: NoiseCtrlFreq, v: u1) NoiseCtrlFreq {
            var copy = self;
            copy.length_enabled = v;
            return copy;
        }
        pub fn withFreq(self: NoiseCtrlFreq, v: u4) NoiseCtrlFreq {
            var copy = self;
            copy.freq = v;
            return copy;
        }
        pub fn withCounterWidth(self: NoiseCtrlFreq, v: u1) NoiseCtrlFreq {
            var copy = self;
            copy.width = v;
            return copy;
        }
        pub fn withFreqDiv(self: NoiseCtrlFreq, v: u3) NoiseCtrlFreq {
            var copy = self;
            copy.freq_div = v;
            return copy;
        }

        pub fn address(channel: Channel) u32 {
            return switch (channel) {
                Channel.noise => nr43_44,
                else => unreachable,
            };
        }
        pub fn writeTo(self: NoiseCtrlFreq, channel: Channel) void {
            gba_set_sound_reg(address(channel), @as(u16, @bitCast(self)));
        }
    };

    /// (NR50, NR51) - Channel L/R Volume/Enable (R/W)
    pub const SoundCtrl = packed struct {
        ///  Bit        Expl.
        ///  0-2   R/W  Sound 1-4 Master Volume RIGHT (0-7)
        master_r: u3 = 7,
        ///  3     -    Not used
        _: u1 = 1,
        ///  4-6   R/W  Sound 1-4 Master Volume LEFT (0-7)
        master_l: u3 = 7,
        ///  7     -    Not used
        _2: u1 = 1,
        ///  8-11  R/W  Sound 1-4 Enable Flags RIGHT (each Bit 8-11, 0=Disable, 1=Enable)
        square1_r: u1 = 1,
        square2_r: u1 = 1,
        wave_r: u1 = 1,
        noise_r: u1 = 1,
        ///  12-15 R/W  Sound 1-4 Enable Flags LEFT (each Bit 12-15, 0=Disable, 1=Enable)
        square1_l: u1 = 1,
        square2_l: u1 = 1,
        wave_l: u1 = 1,
        noise_l: u1 = 1,

        pub fn init() SoundCtrl {
            return SoundCtrl{};
        }
        pub fn withMasterR(self: SoundCtrl, v: u3) SoundCtrl {
            var copy = self;
            copy.master_r = v;
            return copy;
        }
        pub fn withMasterL(self: SoundCtrl, v: u3) SoundCtrl {
            var copy = self;
            copy.master_l = v;
            return copy;
        }
        pub fn withSquare1R(self: SoundCtrl, v: u1) SoundCtrl {
            var copy = self;
            copy.square1_r = v;
            return copy;
        }
        pub fn withSquare2R(self: SoundCtrl, v: u1) SoundCtrl {
            var copy = self;
            copy.square2_r = v;
            return copy;
        }
        pub fn withWaveR(self: SoundCtrl, v: u1) SoundCtrl {
            var copy = self;
            copy.wave_r = v;
            return copy;
        }
        pub fn withNoiseR(self: SoundCtrl, v: u1) SoundCtrl {
            var copy = self;
            copy.noise_r = v;
            return copy;
        }
        pub fn withSquare1L(self: SoundCtrl, v: u1) SoundCtrl {
            var copy = self;
            copy.square1_l = v;
            return copy;
        }
        pub fn withSquare2L(self: SoundCtrl, v: u1) SoundCtrl {
            var copy = self;
            copy.square2_l = v;
            return copy;
        }
        pub fn withWaveL(self: SoundCtrl, v: u1) SoundCtrl {
            var copy = self;
            copy.wave_l = v;
            return copy;
        }
        pub fn withNoiseL(self: SoundCtrl, v: u1) SoundCtrl {
            var copy = self;
            copy.chan_l = v;
            return copy;
        }
        pub fn write(self: SoundCtrl) void {
            gba_set_sound_reg(nr50_51, @as(u16, @bitCast(self)));
        }
    };
};
