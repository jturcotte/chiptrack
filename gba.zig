const std = @import("std");
const fmt = std.fmt;

extern fn print([*:0]const u8) void;
pub extern fn gba_set_sound_reg(addr: u32, value: u32) void;
pub extern fn gba_set_wave_table(table: [*]const u8, table_len: u32) void;
pub extern fn set_instrument_at_column(id: [*:0]const u8, col: i32, press_symbol: [*:0]const u8, release_symbol: [*:0]const u8, frame_symbol: [*:0]const u8) void;

pub fn debug(comptime f: []const u8, args: anytype) void {
    var b: [128]u8 = undefined;
    _ = fmt.bufPrint(&b, f, args) catch unreachable;
    print(@ptrCast([*:0]u8, &b));
}

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

pub const Channel = enum {
    square1,
    square2,
    wave,
    noise,
};
pub const square1 = Channel.square1;
pub const square2 = Channel.square2;
pub const wave = Channel.wave;
pub const noise = Channel.noise;

pub const swe_inc = 0;
pub const swe_dec = 1;
pub const env_dec = 0;
pub const env_inc = 1;
pub const dut_1_8 = 0;
pub const dut_1_4 = 1;
pub const dut_2_4 = 2;
pub const dut_3_4 = 3;
pub const vol_0 = 0;
pub const vol_100 = 1;
pub const vol_50 = 2;
pub const vol_25 = 3;
pub const vol_75 = 4;
pub const wid_15 = 0;
pub const wid_7 = 1;
pub const div_8 = 0;
pub const div_16 = 1;
pub const div_32 = 2;
pub const div_48 = 3;
pub const div_64 = 4;
pub const div_80 = 5;
pub const div_96 = 6;
pub const div_112 = 7;

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
    pub fn write(self: EnvDutyLen, channel: Channel) void {
        const address: u32 = switch (channel) {
            Channel.square1 => nr11_12,
            Channel.square2 => nr21_22,
            Channel.noise => nr41_42,
            else => unreachable,
        };
        gba_set_sound_reg(address, @bitCast(u16, self));
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
    pub fn withFreq(self: CtrlFreq, v: u11) CtrlFreq {
        var copy = self;
        copy.freq = v;
        return copy;
    }
    pub fn squareFreqToFreq(freq: u32) u11 {
        return @truncate(u11, 2048 - ((131072 * 1024) / freq));
    }
    pub fn waveFreqToFreq(freq: u32) u11 {
        return @truncate(u11, 2048 - ((65536 * 1024) / freq));
    }
    pub fn withSquareFreq(self: CtrlFreq, freq: u32) CtrlFreq {
        return self.withFreq(squareFreqToFreq(freq));
    }
    pub fn withWaveFreq(self: CtrlFreq, freq: u32) CtrlFreq {
        return self.withFreq(waveFreqToFreq(freq));
    }

    pub fn write(self: CtrlFreq, channel: Channel) void {
        const address: u32 = switch (channel) {
            Channel.square1 => nr13_14,
            Channel.square2 => nr23_24,
            Channel.wave => nr33_34,
            else => unreachable,
        };
        gba_set_sound_reg(address, @bitCast(u16, self));
    }
};

pub const WavTable = struct { v: [16]u8 };
pub fn wav(comptime t: u128) WavTable {
    // A u128 literal will be stored in memory as little-endian but we
    // need them in the same order in memory to be passed as a u8 slice.
    return WavTable{ .v = @bitCast([16]u8, @byteSwap(t)) };
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

    pub fn write(self: WaveRam, comptime channel: Channel) void {
        const address: u32 = switch (channel) {
            Channel.wave => nr30,
            else => unreachable,
        };
        gba_set_sound_reg(address, @bitCast(u16, self));
    }
    pub fn setTable(table: *const WavTable) void {
        (WaveRam{ .playing = 0 }).write(wave);
        gba_set_wave_table(&table.v, table.v.len);
        (WaveRam{ .playing = 1 }).write(wave);
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

    pub fn write(self: WaveVolLen, comptime channel: Channel) void {
        const address: u32 = switch (channel) {
            Channel.wave => nr31_32,
            else => unreachable,
        };
        gba_set_sound_reg(address, @bitCast(u16, self));
    }
};

/// (NR43, NR44) - Channel 4 Frequency/Control (R/W)
pub const NoiseCtrlFreq = packed struct {
    ///  Bit        Expl.
    ///  0-2   R/W  Dividing Ratio of Frequencies (r)
    r: u3 = 0,
    ///  3     R/W  Counter Step/Width (0=15 bits, 1=7 bits)
    width: u1 = 0,
    ///  4-7   R/W  Shift Clock Frequency (s)
    s: u4 = 0,
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
    pub fn withClockShift(self: NoiseCtrlFreq, v: u4) NoiseCtrlFreq {
        var copy = self;
        copy.s = v;
        return copy;
    }
    pub fn withCounterWidth(self: NoiseCtrlFreq, v: u1) NoiseCtrlFreq {
        var copy = self;
        copy.width = v;
        return copy;
    }
    pub fn withClockDivisor(self: NoiseCtrlFreq, v: u3) NoiseCtrlFreq {
        var copy = self;
        copy.r = v;
        return copy;
    }

    pub fn write(self: NoiseCtrlFreq, comptime channel: Channel) void {
        const address: u32 = switch (channel) {
            Channel.noise => nr43_44,
            else => unreachable,
        };
        gba_set_sound_reg(address, @bitCast(u16, self));
    }
};
