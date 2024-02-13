// Copyright © 2024 Jocelyn Turcotte <turcotte.j@gmail.com>
// SPDX-License-Identifier: CC0-1.0
//
// After modifying the instruments Zig source code, you must re-compile the WebAssembly file that can be executed by Chiptrack.
// This file instructs the Zig compiler how to compile it and for this you need
//  - The Zig compiler, available at https://ziglang.org/download/
//  - ct.zig, the Chiptrack instruments support module, available at https://raw.githubusercontent.com/jturcotte/chiptrack/v0.3/instruments/ct.zig
//  - wasm2wat in your PATH, available at https://github.com/WebAssembly/wabt/releases
//
// Then to rebuild the instruments .wat file from modified source code:
//  zig build
//
// The compiled instruments.wat file will be in the source folder, beside the zig file.
// Chiptrack will reload it automatically if the song is currently loaded.

const std = @import("std");

pub fn build(b: *std.Build) void {
    // Build options, "zig build --help" will show their usage.
    const source_file =
        b.option([]const u8, "source", "Instruments source file to compile (default: instruments.zig).") orelse "instruments.zig";
    const ct_zig_path =
        b.option([]const u8, "ct.zig", "Path to ct.zig (default: ct.zig).") orelse "ct.zig";
    const wat_out =
        b.option(bool, "wat", "Compile the instruments to WAT (Web Assembly Text Format) instead of binary WASM. Chiptrack can load either format but this is better for GitHub gist uploads and requires wasm2wat in PATH (default: true).") orelse true;

    if (std.fs.cwd().access(ct_zig_path, .{}) == std.fs.Dir.AccessError.FileNotFound)
        @panic("ct.zig was not found but is necessary to build. You can download it from https://raw.githubusercontent.com/jturcotte/chiptrack/v0.3/instruments/ct.zig .");

    // Listing the module here also allows editors like VSCode with Zig language support to discover it and provide code completion.
    const ct_module = b.addModule("ct", .{ .source_file = .{ .path = ct_zig_path } });

    // Build the wasm file.
    const wasm = b.addExecutable(.{
        .name = std.fs.path.stem(source_file),
        .root_source_file = .{ .path = source_file },
        .target = .{
            .cpu_arch = .wasm32,
            .os_tag = .freestanding,
        },
        .optimize = .ReleaseFast,
    });
    wasm.rdynamic = true;
    wasm.export_table = true;
    wasm.max_memory = 65536;
    wasm.stack_size = 8192;
    wasm.strip = !wat_out;
    wasm.addModule("ct", ct_module);

    const wf = b.addWriteFiles();
    b.getInstallStep().dependOn(&wf.step);
    if (wat_out) {
        // Generate the wat file by calling wasm2wat on the compiled wasm.
        const wasm2wat = b.addSystemCommand(&.{ "wasm2wat", "-f" });
        wasm2wat.addFileArg(wasm.getEmittedBin());
        const final_url = std.fmt.allocPrint(b.allocator, "{s}.wat", .{wasm.name}) catch unreachable;
        // Copy the output wat file into the source folder, beside the zig file.
        wf.addCopyFileToSource(wasm2wat.captureStdOut(), final_url);
    } else {
        // Copy the wasm file into the source folder, beside the zig file.
        wf.addCopyFileToSource(wasm.getEmittedBin(), std.fmt.allocPrint(b.allocator, "{s}.wasm", .{wasm.name}) catch unreachable);
    }
}
