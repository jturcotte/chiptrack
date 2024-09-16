(module $default-instruments.wasm
  (type (;0;) (func (param i32 i32 i32 i32 i32 i32) (result i32)))
  (type (;1;) (func (param i32 i32 i32 i32 i32 i32 i32)))
  (type (;2;) (func (param i32 i32)))
  (type (;3;) (func))
  (type (;4;) (func (param i32 i32 i32 i32)))
  (type (;5;) (func (param i32 i32 i32)))
  (type (;6;) (func (param i32)))
  (import "env" "set_instrument_at_column" (func $set_instrument_at_column (type 0)))
  (import "env" "define_param" (func $define_param (type 1)))
  (import "env" "gba_set_sound_reg" (func $gba_set_sound_reg (type 2)))
  (import "env" "gba_set_wave_table" (func $gba_set_wave_table (type 2)))
  (func $_start (type 3)
    (local i32)
    (block  ;; label = @1
      (br_if 0 (;@1;)
        (i32.eq
          (local.tee 0
            (i32.and
              (call $set_instrument_at_column
                (i32.const 8218)
                (i32.const 0)
                (i32.const 15)
                (i32.const 1)
                (i32.const 2)
                (i32.const 3))
              (i32.const 255)))
          (i32.const 255)))
      (call $define_param
        (local.get 0)
        (i32.const 0)
        (i32.const 8192)
        (i32.const 2)
        (i32.const 0)
        (i32.const 3)
        (i32.const 4))
      (call $define_param
        (local.get 0)
        (i32.const 1)
        (i32.const 8197)
        (i32.const 12)
        (i32.const 2)
        (i32.const 127)
        (i32.const 5)))
    (block  ;; label = @1
      (br_if 0 (;@1;)
        (i32.eq
          (i32.and
            (local.tee 0
              (call $set_instrument_at_column
                (i32.const 8257)
                (i32.const 0)
                (i32.const 0)
                (i32.const 6)
                (i32.const 0)
                (i32.const 0)))
            (i32.const 255))
          (i32.const 255)))
      (call $define_param
        (i32.and
          (local.get 0)
          (i32.const 255))
        (i32.const 0)
        (i32.const 8192)
        (i32.const 2)
        (i32.const 0)
        (i32.const 3)
        (i32.const 0)))
    (block  ;; label = @1
      (br_if 0 (;@1;)
        (i32.eq
          (local.tee 0
            (i32.and
              (call $set_instrument_at_column
                (i32.const 8245)
                (i32.const 0)
                (i32.const 15)
                (i32.const 7)
                (i32.const 2)
                (i32.const 8))
              (i32.const 255)))
          (i32.const 255)))
      (call $define_param
        (local.get 0)
        (i32.const 0)
        (i32.const 8248)
        (i32.const -123)
        (i32.const -128)
        (i32.const 127)
        (i32.const 0))
      (call $define_param
        (local.get 0)
        (i32.const 1)
        (i32.const 8227)
        (i32.const -93)
        (i32.const -128)
        (i32.const 127)
        (i32.const 0)))
    (block  ;; label = @1
      (br_if 0 (;@1;)
        (i32.eq
          (i32.and
            (local.tee 0
              (call $set_instrument_at_column
                (i32.const 8215)
                (i32.const 0)
                (i32.const 0)
                (i32.const 9)
                (i32.const 0)
                (i32.const 0)))
            (i32.const 255))
          (i32.const 255)))
      (call $define_param
        (i32.and
          (local.get 0)
          (i32.const 255))
        (i32.const 0)
        (i32.const 8192)
        (i32.const 2)
        (i32.const 0)
        (i32.const 3)
        (i32.const 0)))
    (block  ;; label = @1
      (br_if 0 (;@1;)
        (i32.eq
          (local.tee 0
            (i32.and
              (call $set_instrument_at_column
                (i32.const 8287)
                (i32.const 0)
                (i32.const 15)
                (i32.const 10)
                (i32.const 2)
                (i32.const 11))
              (i32.const 255)))
          (i32.const 255)))
      (call $define_param
        (local.get 0)
        (i32.const 0)
        (i32.const 8192)
        (i32.const 2)
        (i32.const 0)
        (i32.const 3)
        (i32.const 12))
      (call $define_param
        (local.get 0)
        (i32.const 1)
        (i32.const 8227)
        (i32.const -93)
        (i32.const -128)
        (i32.const 127)
        (i32.const 0)))
    (block  ;; label = @1
      (br_if 0 (;@1;)
        (i32.eq
          (i32.and
            (local.tee 0
              (call $set_instrument_at_column
                (i32.const 8281)
                (i32.const 0)
                (i32.const 15)
                (i32.const 7)
                (i32.const 2)
                (i32.const 13)))
            (i32.const 255))
          (i32.const 255)))
      (call $define_param
        (local.tee 0
          (i32.and
            (local.get 0)
            (i32.const 255)))
        (i32.const 0)
        (i32.const 8248)
        (i32.const -123)
        (i32.const -128)
        (i32.const 127)
        (i32.const 0))
      (call $define_param
        (local.get 0)
        (i32.const 1)
        (i32.const 8227)
        (i32.const -93)
        (i32.const -128)
        (i32.const 127)
        (i32.const 0)))
    (block  ;; label = @1
      (br_if 0 (;@1;)
        (i32.eq
          (local.tee 0
            (i32.and
              (call $set_instrument_at_column
                (i32.const 8275)
                (i32.const 0)
                (i32.const 15)
                (i32.const 7)
                (i32.const 2)
                (i32.const 14))
              (i32.const 255)))
          (i32.const 255)))
      (call $define_param
        (local.get 0)
        (i32.const 0)
        (i32.const 8248)
        (i32.const -123)
        (i32.const -128)
        (i32.const 127)
        (i32.const 0))
      (call $define_param
        (local.get 0)
        (i32.const 1)
        (i32.const 8227)
        (i32.const -93)
        (i32.const -128)
        (i32.const 127)
        (i32.const 0)))
    (block  ;; label = @1
      (br_if 0 (;@1;)
        (i32.eq
          (i32.and
            (local.tee 0
              (call $set_instrument_at_column
                (i32.const 8269)
                (i32.const 0)
                (i32.const 15)
                (i32.const 7)
                (i32.const 2)
                (i32.const 15)))
            (i32.const 255))
          (i32.const 255)))
      (call $define_param
        (local.tee 0
          (i32.and
            (local.get 0)
            (i32.const 255)))
        (i32.const 0)
        (i32.const 8248)
        (i32.const -123)
        (i32.const -128)
        (i32.const 127)
        (i32.const 0))
      (call $define_param
        (local.get 0)
        (i32.const 1)
        (i32.const 8227)
        (i32.const -93)
        (i32.const -128)
        (i32.const 127)
        (i32.const 0)))
    (block  ;; label = @1
      (br_if 0 (;@1;)
        (i32.eq
          (local.tee 0
            (i32.and
              (call $set_instrument_at_column
                (i32.const 8242)
                (i32.const 1)
                (i32.const 13)
                (i32.const 16)
                (i32.const 17)
                (i32.const 18))
              (i32.const 255)))
          (i32.const 255)))
      (call $define_param
        (local.get 0)
        (i32.const 0)
        (i32.const 8308)
        (i32.const 4)
        (i32.const -128)
        (i32.const 127)
        (i32.const 0)))
    (block  ;; label = @1
      (br_if 0 (;@1;)
        (i32.eq
          (i32.and
            (local.tee 0
              (call $set_instrument_at_column
                (i32.const 8254)
                (i32.const 1)
                (i32.const 0)
                (i32.const 19)
                (i32.const 0)
                (i32.const 0)))
            (i32.const 255))
          (i32.const 255)))
      (call $define_param
        (i32.and
          (local.get 0)
          (i32.const 255))
        (i32.const 0)
        (i32.const 8192)
        (i32.const 2)
        (i32.const 0)
        (i32.const 3)
        (i32.const 0)))
    (block  ;; label = @1
      (br_if 0 (;@1;)
        (i32.eq
          (local.tee 0
            (i32.and
              (call $set_instrument_at_column
                (i32.const 8263)
                (i32.const 1)
                (i32.const 24)
                (i32.const 20)
                (i32.const 21)
                (i32.const 22))
              (i32.const 255)))
          (i32.const 255)))
      (call $define_param
        (local.get 0)
        (i32.const 0)
        (i32.const 8349)
        (i32.const 4)
        (i32.const -128)
        (i32.const 127)
        (i32.const 0))
      (call $define_param
        (local.get 0)
        (i32.const 1)
        (i32.const 8327)
        (i32.const 7)
        (i32.const -128)
        (i32.const 127)
        (i32.const 0)))
    (block  ;; label = @1
      (br_if 0 (;@1;)
        (i32.eq
          (i32.and
            (local.tee 0
              (call $set_instrument_at_column
                (i32.const 8230)
                (i32.const 1)
                (i32.const 0)
                (i32.const 23)
                (i32.const 24)
                (i32.const 25)))
            (i32.const 255))
          (i32.const 255)))
      (call $define_param
        (local.tee 0
          (i32.and
            (local.get 0)
            (i32.const 255)))
        (i32.const 0)
        (i32.const 8428)
        (i32.const 4)
        (i32.const -128)
        (i32.const 127)
        (i32.const 0))
      (call $define_param
        (local.get 0)
        (i32.const 1)
        (i32.const 8406)
        (i32.const 5)
        (i32.const -128)
        (i32.const 127)
        (i32.const 0)))
    (block  ;; label = @1
      (br_if 0 (;@1;)
        (i32.eq
          (local.tee 0
            (i32.and
              (call $set_instrument_at_column
                (i32.const 8284)
                (i32.const 1)
                (i32.const 15)
                (i32.const 26)
                (i32.const 21)
                (i32.const 27))
              (i32.const 255)))
          (i32.const 255)))
      (call $define_param
        (local.get 0)
        (i32.const 0)
        (i32.const 8192)
        (i32.const 2)
        (i32.const 0)
        (i32.const 3)
        (i32.const 28))
      (call $define_param
        (local.get 0)
        (i32.const 1)
        (i32.const 8227)
        (i32.const -93)
        (i32.const -128)
        (i32.const 127)
        (i32.const 0)))
    (block  ;; label = @1
      (br_if 0 (;@1;)
        (i32.eq
          (i32.and
            (local.tee 0
              (call $set_instrument_at_column
                (i32.const 8278)
                (i32.const 1)
                (i32.const 15)
                (i32.const 29)
                (i32.const 21)
                (i32.const 30)))
            (i32.const 255))
          (i32.const 255)))
      (call $define_param
        (local.tee 0
          (i32.and
            (local.get 0)
            (i32.const 255)))
        (i32.const 0)
        (i32.const 8248)
        (i32.const -123)
        (i32.const -128)
        (i32.const 127)
        (i32.const 0))
      (call $define_param
        (local.get 0)
        (i32.const 1)
        (i32.const 8227)
        (i32.const -93)
        (i32.const -128)
        (i32.const 127)
        (i32.const 0)))
    (block  ;; label = @1
      (br_if 0 (;@1;)
        (i32.eq
          (local.tee 0
            (i32.and
              (call $set_instrument_at_column
                (i32.const 8272)
                (i32.const 1)
                (i32.const 15)
                (i32.const 29)
                (i32.const 21)
                (i32.const 31))
              (i32.const 255)))
          (i32.const 255)))
      (call $define_param
        (local.get 0)
        (i32.const 0)
        (i32.const 8248)
        (i32.const -123)
        (i32.const -128)
        (i32.const 127)
        (i32.const 0))
      (call $define_param
        (local.get 0)
        (i32.const 1)
        (i32.const 8227)
        (i32.const -93)
        (i32.const -128)
        (i32.const 127)
        (i32.const 0)))
    (block  ;; label = @1
      (br_if 0 (;@1;)
        (i32.eq
          (i32.and
            (local.tee 0
              (call $set_instrument_at_column
                (i32.const 8266)
                (i32.const 1)
                (i32.const 15)
                (i32.const 29)
                (i32.const 21)
                (i32.const 32)))
            (i32.const 255))
          (i32.const 255)))
      (call $define_param
        (local.tee 0
          (i32.and
            (local.get 0)
            (i32.const 255)))
        (i32.const 0)
        (i32.const 8248)
        (i32.const -123)
        (i32.const -128)
        (i32.const 127)
        (i32.const 0))
      (call $define_param
        (local.get 0)
        (i32.const 1)
        (i32.const 8227)
        (i32.const -93)
        (i32.const -128)
        (i32.const 127)
        (i32.const 0)))
    (block  ;; label = @1
      (br_if 0 (;@1;)
        (i32.eq
          (local.tee 0
            (i32.and
              (call $set_instrument_at_column
                (i32.const 8221)
                (i32.const 2)
                (i32.const 15)
                (i32.const 33)
                (i32.const 34)
                (i32.const 35))
              (i32.const 255)))
          (i32.const 255)))
      (call $define_param
        (local.get 0)
        (i32.const 0)
        (i32.const 8248)
        (i32.const -123)
        (i32.const -128)
        (i32.const 127)
        (i32.const 0))
      (call $define_param
        (local.get 0)
        (i32.const 1)
        (i32.const 8227)
        (i32.const -93)
        (i32.const -128)
        (i32.const 127)
        (i32.const 0)))
    (block  ;; label = @1
      (br_if 0 (;@1;)
        (i32.eq
          (i32.and
            (local.tee 0
              (call $set_instrument_at_column
                (i32.const 8251)
                (i32.const 2)
                (i32.const 15)
                (i32.const 36)
                (i32.const 34)
                (i32.const 35)))
            (i32.const 255))
          (i32.const 255)))
      (call $define_param
        (local.tee 0
          (i32.and
            (local.get 0)
            (i32.const 255)))
        (i32.const 0)
        (i32.const 8248)
        (i32.const -123)
        (i32.const -128)
        (i32.const 127)
        (i32.const 0))
      (call $define_param
        (local.get 0)
        (i32.const 1)
        (i32.const 8227)
        (i32.const -93)
        (i32.const -128)
        (i32.const 127)
        (i32.const 0)))
    (block  ;; label = @1
      (br_if 0 (;@1;)
        (i32.eq
          (local.tee 0
            (i32.and
              (call $set_instrument_at_column
                (i32.const 8260)
                (i32.const 2)
                (i32.const 4)
                (i32.const 37)
                (i32.const 34)
                (i32.const 38))
              (i32.const 255)))
          (i32.const 255)))
      (call $define_param
        (local.get 0)
        (i32.const 0)
        (i32.const 8349)
        (i32.const 4)
        (i32.const -128)
        (i32.const 127)
        (i32.const 0))
      (call $define_param
        (local.get 0)
        (i32.const 1)
        (i32.const 8327)
        (i32.const 7)
        (i32.const -128)
        (i32.const 127)
        (i32.const 0)))
    (block  ;; label = @1
      (br_if 0 (;@1;)
        (i32.eq
          (i32.and
            (local.tee 0
              (call $set_instrument_at_column
                (i32.const 8239)
                (i32.const 2)
                (i32.const 15)
                (i32.const 39)
                (i32.const 34)
                (i32.const 35)))
            (i32.const 255))
          (i32.const 255)))
      (call $define_param
        (local.tee 0
          (i32.and
            (local.get 0)
            (i32.const 255)))
        (i32.const 0)
        (i32.const 8248)
        (i32.const -123)
        (i32.const -128)
        (i32.const 127)
        (i32.const 0))
      (call $define_param
        (local.get 0)
        (i32.const 1)
        (i32.const 8227)
        (i32.const -93)
        (i32.const -128)
        (i32.const 127)
        (i32.const 0)))
    (block  ;; label = @1
      (br_if 0 (;@1;)
        (i32.eq
          (local.tee 0
            (i32.and
              (call $set_instrument_at_column
                (i32.const 8224)
                (i32.const 2)
                (i32.const 16)
                (i32.const 40)
                (i32.const 34)
                (i32.const 41))
              (i32.const 255)))
          (i32.const 255)))
      (call $define_param
        (local.get 0)
        (i32.const 0)
        (i32.const 8290)
        (i32.const 4)
        (i32.const 0)
        (i32.const 12)
        (i32.const 0))
      (call $define_param
        (local.get 0)
        (i32.const 1)
        (i32.const 8227)
        (i32.const -93)
        (i32.const -128)
        (i32.const 127)
        (i32.const 0)))
    (drop
      (call $set_instrument_at_column
        (i32.const 8233)
        (i32.const 3)
        (i32.const 15)
        (i32.const 42)
        (i32.const 0)
        (i32.const 43)))
    (block  ;; label = @1
      (br_if 0 (;@1;)
        (i32.eq
          (i32.and
            (local.tee 0
              (call $set_instrument_at_column
                (i32.const 8236)
                (i32.const 3)
                (i32.const 0)
                (i32.const 44)
                (i32.const 0)
                (i32.const 0)))
            (i32.const 255))
          (i32.const 255)))
      (call $define_param
        (local.tee 0
          (i32.and
            (local.get 0)
            (i32.const 255)))
        (i32.const 0)
        (i32.const 8371)
        (i32.const 4)
        (i32.const 0)
        (i32.const 15)
        (i32.const 0))
      (call $define_param
        (local.get 0)
        (i32.const 1)
        (i32.const 8389)
        (i32.const 15)
        (i32.const 0)
        (i32.const 31)
        (i32.const 0))))
  (func $default-instruments.square1_vibrato.press (type 4) (param i32 i32 i32 i32)
    (i32.store offset=9116 align=1
      (i32.const 0)
      (i32.load offset=8453 align=1
        (i32.const 0)))
    (i32.store16 offset=9120 align=1
      (i32.const 0)
      (i32.load16_u offset=8457 align=1
        (i32.const 0)))
    (i32.store16 offset=9122
      (i32.const 0)
      (i32.or
        (i32.and
          (i32.load16_u offset=9122
            (i32.const 0))
          (i32.const 65343))
        (i32.and
          (i32.shl
            (local.get 2)
            (i32.const 6))
          (i32.const 192))))
    (i32.store16 offset=9124
      (i32.const 0)
      (i32.extend8_s
        (select
          (local.tee 2
            (i32.and
              (local.get 3)
              (i32.const 255)))
          (i32.const 1)
          (i32.gt_u
            (local.get 2)
            (i32.const 1)))))
    (call $gba_set_sound_reg
      (i32.const 67108960)
      (i32.const 8)))
  (func $default-instruments.square1_1_4.release (type 5) (param i32 i32 i32)
    (block  ;; label = @1
      (br_if 0 (;@1;)
        (i32.gt_u
          (i32.and
            (i32.load8_u offset=9117
              (i32.const 0))
            (i32.const 7))
          (i32.const 2)))
      (i32.store8 offset=9117
        (i32.const 0)
        (i32.const 3))
      (i32.store8 offset=9116
        (i32.const 0)
        (i32.and
          (i32.load8_u offset=9120
            (i32.const 0))
          (i32.const 15)))))
  (func $default-instruments.square1_vibrato.frame (type 5) (param i32 i32 i32)
    (local i32 i32 i32 i32)
    (local.set 3
      (i32.const 0))
    (local.set 4
      (i32.load16_u offset=9122
        (i32.const 0)))
    (block  ;; label = @1
      (block  ;; label = @2
        (block  ;; label = @3
          (block  ;; label = @4
            (block  ;; label = @5
              (br_table 2 (;@3;) 0 (;@5;) 4 (;@1;) 1 (;@4;) 4 (;@1;) 2 (;@3;)
                (i32.and
                  (i32.load8_u offset=9117
                    (i32.const 0))
                  (i32.const 7))))
            (i32.store8 offset=9116
              (i32.const 0)
              (local.tee 3
                (i32.sub
                  (i32.load8_u offset=9116
                    (i32.const 0))
                  (i32.and
                    (i32.load8_u offset=9119
                      (i32.const 0))
                    (i32.const 15)))))
            (local.set 5
              (i32.const 2))
            (br_if 3 (;@1;)
              (i32.gt_s
                (i32.extend8_s
                  (local.get 3))
                (local.tee 3
                  (i32.and
                    (i32.load8_u offset=9120
                      (i32.const 0))
                    (i32.const 15)))))
            (br 2 (;@2;)))
          (i32.store8 offset=9116
            (i32.const 0)
            (local.tee 5
              (i32.sub
                (i32.load8_u offset=9116
                  (i32.const 0))
                (i32.and
                  (i32.load8_u offset=9121
                    (i32.const 0))
                  (i32.const 15)))))
          (br_if 2 (;@1;)
            (i32.ge_s
              (i32.extend8_s
                (local.get 5))
              (i32.const 0)))
          (local.set 5
            (i32.const 4))
          (br 1 (;@2;)))
        (local.set 3
          (i32.const 15))
        (i32.store8 offset=9116
          (i32.const 0)
          (local.tee 6
            (i32.add
              (i32.and
                (i32.load8_u offset=9118
                  (i32.const 0))
                (i32.const 15))
              (i32.load8_u offset=9116
                (i32.const 0)))))
        (local.set 5
          (i32.const 1))
        (br_if 1 (;@1;)
          (i32.le_s
            (i32.extend8_s
              (local.get 6))
            (i32.const 14))))
      (i32.store8 offset=9116
        (i32.const 0)
        (local.get 3))
      (i32.store8 offset=9117
        (i32.const 0)
        (local.get 5)))
    (call $gba_set_sound_reg
      (i32.const 67108962)
      (i32.and
        (i32.or
          (i32.shl
            (i32.load8_u offset=9116
              (i32.const 0))
            (i32.const 12))
          (i32.and
            (local.get 4)
            (i32.const 4095)))
        (i32.const 65535)))
    (block  ;; label = @1
      (br_if 0 (;@1;)
        (i32.le_u
          (local.get 2)
          (i32.const 21)))
      (local.set 0
        (i32.add
          (i32.add
            (i32.sub
              (local.get 0)
              (local.tee 4
                (i32.div_u
                  (local.get 0)
                  (i32.const 36))))
            (i32.mul
              (i32.sub
                (i32.xor
                  (local.tee 2
                    (i32.add
                      (i32.sub
                        (local.tee 3
                          (i32.rem_s
                            (local.tee 2
                              (i32.add
                                (i32.add
                                  (local.tee 3
                                    (i32.rem_s
                                      (local.tee 2
                                        (i32.add
                                          (i32.sub
                                            (local.get 2)
                                            (i32.shr_u
                                              (local.tee 0
                                                (i32.load16_u offset=9124
                                                  (i32.const 0)))
                                              (i32.const 2)))
                                          (i32.const -21)))
                                      (local.get 0)))
                                  (local.get 0))
                                (select
                                  (i32.and
                                    (i32.shr_s
                                      (local.get 2)
                                      (i32.const 31))
                                    (local.get 0))
                                  (i32.const 0)
                                  (local.get 3))))
                            (local.get 0)))
                        (i32.shr_u
                          (local.get 0)
                          (i32.const 1)))
                      (select
                        (i32.and
                          (i32.shr_s
                            (local.get 2)
                            (i32.const 31))
                          (local.get 0))
                        (i32.const 0)
                        (local.get 3))))
                  (local.tee 2
                    (i32.shr_s
                      (local.get 2)
                      (i32.const 31))))
                (local.get 2))
              (i32.div_u
                (i32.shl
                  (local.get 4)
                  (i32.const 2))
                (local.get 0))))
          (i32.const 1))))
    (call $gba_set_sound_reg
      (i32.const 67108964)
      (i32.or
        (i32.and
          (i32.sub
            (i32.const 0)
            (i32.div_u
              (i32.const 33554432)
              (local.get 0)))
          (i32.const 2047))
        (i32.const 32768))))
  (func $default-instruments.square1_vibrato.set_duty (type 6) (param i32)
    (i32.store16 offset=9122
      (i32.const 0)
      (i32.or
        (i32.and
          (i32.load16_u offset=9122
            (i32.const 0))
          (i32.const 65343))
        (i32.and
          (i32.shl
            (local.get 0)
            (i32.const 6))
          (i32.const 192)))))
  (func $default-instruments.square1_vibrato.set_p (type 6) (param i32)
    (i32.store16 offset=9124
      (i32.const 0)
      (i32.extend8_s
        (select
          (local.tee 0
            (i32.and
              (local.get 0)
              (i32.const 255)))
          (i32.const 1)
          (i32.gt_u
            (local.get 0)
            (i32.const 1))))))
  (func $default-instruments.square1_bleep.press (type 4) (param i32 i32 i32 i32)
    (call $gba_set_sound_reg
      (i32.const 67108960)
      (i32.const 8))
    (call $gba_set_sound_reg
      (i32.const 67108962)
      (i32.or
        (i32.and
          (i32.shl
            (local.get 2)
            (i32.const 6))
          (i32.const 192))
        (i32.const 41008)))
    (call $gba_set_sound_reg
      (i32.const 67108964)
      (i32.or
        (i32.and
          (i32.sub
            (i32.const 0)
            (i32.div_u
              (i32.const 33554432)
              (local.get 0)))
          (i32.const 2047))
        (i32.const 49152))))
  (func $default-instruments.square1_1_4.press (type 4) (param i32 i32 i32 i32)
    (local i32 i32 i32 i32 i32)
    (global.set $__stack_pointer
      (local.tee 4
        (i32.sub
          (global.get $__stack_pointer)
          (i32.const 16))))
    (local.set 5
      (i32.shr_u
        (i32.and
          (local.get 3)
          (i32.const 240))
        (i32.const 4)))
    (local.set 6
      (i32.const 8984))
    (block  ;; label = @1
      (br_if 0 (;@1;)
        (i32.and
          (i32.or
            (local.tee 7
              (i32.shr_u
                (i32.and
                  (local.get 2)
                  (i32.const 240))
                (i32.const 4)))
            (local.get 2))
          (i32.const 15)))
      (i32.store8 offset=10
        (local.get 4)
        (i32.const 2))
      (i32.store8 offset=9
        (local.get 4)
        (local.get 5))
      (i32.store8 offset=8
        (local.get 4)
        (local.get 5))
      (local.set 6
        (i32.add
          (local.get 4)
          (i32.const 9))))
    (i32.store8 offset=14
      (local.get 4)
      (local.get 7))
    (i32.store8 offset=12
      (local.get 4)
      (local.get 5))
    (i32.store8 offset=13
      (local.get 4)
      (local.tee 2
        (i32.and
          (local.get 2)
          (i32.const 15))))
    (i32.store8 offset=11
      (local.get 4)
      (local.tee 3
        (i32.and
          (local.get 3)
          (i32.const 15))))
    (i32.store8 offset=15
      (local.get 4)
      (local.tee 8
        (i32.and
          (i32.load8_u offset=1
            (local.get 6))
          (i32.const 7))))
    (local.set 6
      (i32.load8_u
        (local.get 6)))
    (i32.store8 offset=9121
      (i32.const 0)
      (local.get 3))
    (i32.store8 offset=9120
      (i32.const 0)
      (local.get 5))
    (i32.store8 offset=9119
      (i32.const 0)
      (local.get 2))
    (i32.store8 offset=9118
      (i32.const 0)
      (local.get 7))
    (i32.store8 offset=9117
      (i32.const 0)
      (local.get 8))
    (i32.store8 offset=9116
      (i32.const 0)
      (i32.and
        (local.get 6)
        (i32.const 15)))
    (call $gba_set_sound_reg
      (i32.const 67108960)
      (i32.const 8))
    (global.set $__stack_pointer
      (i32.add
        (local.get 4)
        (i32.const 16))))
  (func $default-instruments.square1_duty.frame (type 5) (param i32 i32 i32)
    (local i32 i32 i32)
    (local.set 3
      (i32.const 0))
    (local.set 4
      (i32.const 1))
    (local.set 2
      (i32.load8_u
        (i32.add
          (i32.and
            (i32.shr_u
              (local.get 2)
              (i32.const 1))
            (i32.const 3))
          (i32.const 8449))))
    (block  ;; label = @1
      (block  ;; label = @2
        (block  ;; label = @3
          (block  ;; label = @4
            (block  ;; label = @5
              (br_table 2 (;@3;) 0 (;@5;) 4 (;@1;) 1 (;@4;) 4 (;@1;) 2 (;@3;)
                (i32.and
                  (i32.load8_u offset=9117
                    (i32.const 0))
                  (i32.const 7))))
            (i32.store8 offset=9116
              (i32.const 0)
              (local.tee 3
                (i32.sub
                  (i32.load8_u offset=9116
                    (i32.const 0))
                  (i32.and
                    (i32.load8_u offset=9119
                      (i32.const 0))
                    (i32.const 15)))))
            (local.set 4
              (i32.const 2))
            (br_if 3 (;@1;)
              (i32.gt_s
                (i32.extend8_s
                  (local.get 3))
                (local.tee 3
                  (i32.and
                    (i32.load8_u offset=9120
                      (i32.const 0))
                    (i32.const 15)))))
            (br 2 (;@2;)))
          (i32.store8 offset=9116
            (i32.const 0)
            (local.tee 4
              (i32.sub
                (i32.load8_u offset=9116
                  (i32.const 0))
                (i32.and
                  (i32.load8_u offset=9121
                    (i32.const 0))
                  (i32.const 15)))))
          (br_if 2 (;@1;)
            (i32.ge_s
              (i32.extend8_s
                (local.get 4))
              (i32.const 0)))
          (local.set 4
            (i32.const 4))
          (br 1 (;@2;)))
        (local.set 3
          (i32.const 15))
        (i32.store8 offset=9116
          (i32.const 0)
          (local.tee 5
            (i32.add
              (i32.and
                (i32.load8_u offset=9118
                  (i32.const 0))
                (i32.const 15))
              (i32.load8_u offset=9116
                (i32.const 0)))))
        (br_if 1 (;@1;)
          (i32.le_s
            (i32.extend8_s
              (local.get 5))
            (i32.const 14))))
      (i32.store8 offset=9116
        (i32.const 0)
        (local.get 3))
      (i32.store8 offset=9117
        (i32.const 0)
        (local.get 4)))
    (call $gba_set_sound_reg
      (i32.const 67108962)
      (i32.or
        (i32.shl
          (i32.and
            (i32.load8_u offset=9116
              (i32.const 0))
            (i32.const 15))
          (i32.const 12))
        (i32.and
          (i32.shl
            (local.get 2)
            (i32.const 6))
          (i32.const 192))))
    (call $gba_set_sound_reg
      (i32.const 67108964)
      (i32.or
        (i32.and
          (i32.sub
            (i32.const 0)
            (i32.div_u
              (i32.const 33554432)
              (local.get 0)))
          (i32.const 2047))
        (i32.const 32768))))
  (func $default-instruments.square1_sweep.press (type 4) (param i32 i32 i32 i32)
    (call $gba_set_sound_reg
      (i32.const 67108960)
      (i32.const 42))
    (call $gba_set_sound_reg
      (i32.const 67108962)
      (i32.or
        (i32.and
          (i32.shl
            (local.get 2)
            (i32.const 6))
          (i32.const 192))
        (i32.const 53760)))
    (call $gba_set_sound_reg
      (i32.const 67108964)
      (i32.or
        (i32.and
          (i32.sub
            (i32.const 0)
            (i32.div_u
              (i32.const 33554432)
              (local.get 0)))
          (i32.const 2047))
        (i32.const 32768))))
  (func $default-instruments.square1_base.press (type 4) (param i32 i32 i32 i32)
    (local i32)
    (global.set $__stack_pointer
      (local.tee 4
        (i32.sub
          (global.get $__stack_pointer)
          (i32.const 16))))
    (i32.store16 offset=9126
      (i32.const 0)
      (i32.or
        (i32.and
          (i32.load16_u offset=9126
            (i32.const 0))
          (i32.const 65343))
        (i32.and
          (i32.shl
            (local.get 2)
            (i32.const 6))
          (i32.const 192))))
    (i32.store8 offset=15
      (local.get 4)
      (i32.const 0))
    (i32.store8 offset=14
      (local.get 4)
      (i32.const 8))
    (i32.store8 offset=13
      (local.get 4)
      (i32.const 5))
    (i32.store8 offset=12
      (local.get 4)
      (local.tee 2
        (i32.and
          (i32.shr_u
            (local.get 3)
            (i32.const 4))
          (i32.const 15))))
    (i32.store8 offset=11
      (local.get 4)
      (local.tee 3
        (i32.and
          (local.get 3)
          (i32.const 15))))
    (i32.store8 offset=9121
      (i32.const 0)
      (local.get 3))
    (i32.store8 offset=9120
      (i32.const 0)
      (local.get 2))
    (i32.store8 offset=9119
      (i32.const 0)
      (i32.const 5))
    (i32.store8 offset=9118
      (i32.const 0)
      (i32.const 8))
    (i32.store8 offset=9117
      (i32.const 0)
      (i32.const 0))
    (i32.store8 offset=9116
      (i32.const 0)
      (i32.const 0))
    (call $gba_set_sound_reg
      (i32.const 67108960)
      (i32.const 8))
    (global.set $__stack_pointer
      (i32.add
        (local.get 4)
        (i32.const 16))))
  (func $default-instruments.square1_base.frame (type 5) (param i32 i32 i32)
    (local i32 i32 i32 i32)
    (local.set 3
      (i32.const 0))
    (local.set 4
      (i32.load16_u offset=9126
        (i32.const 0)))
    (block  ;; label = @1
      (block  ;; label = @2
        (block  ;; label = @3
          (block  ;; label = @4
            (block  ;; label = @5
              (br_table 2 (;@3;) 0 (;@5;) 4 (;@1;) 1 (;@4;) 4 (;@1;) 2 (;@3;)
                (i32.and
                  (i32.load8_u offset=9117
                    (i32.const 0))
                  (i32.const 7))))
            (i32.store8 offset=9116
              (i32.const 0)
              (local.tee 3
                (i32.sub
                  (i32.load8_u offset=9116
                    (i32.const 0))
                  (i32.and
                    (i32.load8_u offset=9119
                      (i32.const 0))
                    (i32.const 15)))))
            (local.set 5
              (i32.const 2))
            (br_if 3 (;@1;)
              (i32.gt_s
                (i32.extend8_s
                  (local.get 3))
                (local.tee 3
                  (i32.and
                    (i32.load8_u offset=9120
                      (i32.const 0))
                    (i32.const 15)))))
            (br 2 (;@2;)))
          (i32.store8 offset=9116
            (i32.const 0)
            (local.tee 5
              (i32.sub
                (i32.load8_u offset=9116
                  (i32.const 0))
                (i32.and
                  (i32.load8_u offset=9121
                    (i32.const 0))
                  (i32.const 15)))))
          (br_if 2 (;@1;)
            (i32.ge_s
              (i32.extend8_s
                (local.get 5))
              (i32.const 0)))
          (local.set 5
            (i32.const 4))
          (br 1 (;@2;)))
        (local.set 3
          (i32.const 15))
        (i32.store8 offset=9116
          (i32.const 0)
          (local.tee 6
            (i32.add
              (i32.and
                (i32.load8_u offset=9118
                  (i32.const 0))
                (i32.const 15))
              (i32.load8_u offset=9116
                (i32.const 0)))))
        (local.set 5
          (i32.const 1))
        (br_if 1 (;@1;)
          (i32.le_s
            (i32.extend8_s
              (local.get 6))
            (i32.const 14))))
      (i32.store8 offset=9116
        (i32.const 0)
        (local.get 3))
      (i32.store8 offset=9117
        (i32.const 0)
        (local.get 5)))
    (call $gba_set_sound_reg
      (i32.const 67108962)
      (i32.and
        (i32.or
          (i32.shl
            (i32.load8_u offset=9116
              (i32.const 0))
            (i32.const 12))
          (i32.and
            (local.get 4)
            (i32.const 4095)))
        (i32.const 65535)))
    (call $gba_set_sound_reg
      (i32.const 67108964)
      (i32.or
        (i32.and
          (i32.sub
            (i32.const 0)
            (i32.div_u
              (i32.const 33554432)
              (local.get 0)))
          (i32.const 2047))
        (i32.const 32768))))
  (func $default-instruments.square1_base.set_duty (type 6) (param i32)
    (i32.store16 offset=9126
      (i32.const 0)
      (i32.or
        (i32.and
          (i32.load16_u offset=9126
            (i32.const 0))
          (i32.const 65343))
        (i32.and
          (i32.shl
            (local.get 0)
            (i32.const 6))
          (i32.const 192)))))
  (func $default-instruments.square1_2_4.frame (type 5) (param i32 i32 i32)
    (local i32 i32 i32)
    (local.set 3
      (i32.const 0))
    (block  ;; label = @1
      (block  ;; label = @2
        (block  ;; label = @3
          (block  ;; label = @4
            (block  ;; label = @5
              (br_table 2 (;@3;) 0 (;@5;) 4 (;@1;) 1 (;@4;) 4 (;@1;) 2 (;@3;)
                (i32.and
                  (i32.load8_u offset=9117
                    (i32.const 0))
                  (i32.const 7))))
            (i32.store8 offset=9116
              (i32.const 0)
              (local.tee 3
                (i32.sub
                  (i32.load8_u offset=9116
                    (i32.const 0))
                  (i32.and
                    (i32.load8_u offset=9119
                      (i32.const 0))
                    (i32.const 15)))))
            (local.set 4
              (i32.const 2))
            (br_if 3 (;@1;)
              (i32.gt_s
                (i32.extend8_s
                  (local.get 3))
                (local.tee 3
                  (i32.and
                    (i32.load8_u offset=9120
                      (i32.const 0))
                    (i32.const 15)))))
            (br 2 (;@2;)))
          (i32.store8 offset=9116
            (i32.const 0)
            (local.tee 4
              (i32.sub
                (i32.load8_u offset=9116
                  (i32.const 0))
                (i32.and
                  (i32.load8_u offset=9121
                    (i32.const 0))
                  (i32.const 15)))))
          (br_if 2 (;@1;)
            (i32.ge_s
              (i32.extend8_s
                (local.get 4))
              (i32.const 0)))
          (local.set 4
            (i32.const 4))
          (br 1 (;@2;)))
        (local.set 3
          (i32.const 15))
        (i32.store8 offset=9116
          (i32.const 0)
          (local.tee 5
            (i32.add
              (i32.and
                (i32.load8_u offset=9118
                  (i32.const 0))
                (i32.const 15))
              (i32.load8_u offset=9116
                (i32.const 0)))))
        (local.set 4
          (i32.const 1))
        (br_if 1 (;@1;)
          (i32.le_s
            (i32.extend8_s
              (local.get 5))
            (i32.const 14))))
      (i32.store8 offset=9116
        (i32.const 0)
        (local.get 3))
      (i32.store8 offset=9117
        (i32.const 0)
        (local.get 4)))
    (call $gba_set_sound_reg
      (i32.const 67108962)
      (i32.or
        (i32.shl
          (i32.and
            (i32.load8_u offset=9116
              (i32.const 0))
            (i32.const 15))
          (i32.const 12))
        (i32.const 128)))
    (call $gba_set_sound_reg
      (i32.const 67108964)
      (i32.or
        (i32.and
          (i32.sub
            (i32.const 0)
            (i32.div_u
              (i32.const 33554432)
              (local.get 0)))
          (i32.const 2047))
        (i32.const 32768))))
  (func $default-instruments.square1_1_4.frame (type 5) (param i32 i32 i32)
    (local i32 i32 i32)
    (local.set 3
      (i32.const 0))
    (block  ;; label = @1
      (block  ;; label = @2
        (block  ;; label = @3
          (block  ;; label = @4
            (block  ;; label = @5
              (br_table 2 (;@3;) 0 (;@5;) 4 (;@1;) 1 (;@4;) 4 (;@1;) 2 (;@3;)
                (i32.and
                  (i32.load8_u offset=9117
                    (i32.const 0))
                  (i32.const 7))))
            (i32.store8 offset=9116
              (i32.const 0)
              (local.tee 3
                (i32.sub
                  (i32.load8_u offset=9116
                    (i32.const 0))
                  (i32.and
                    (i32.load8_u offset=9119
                      (i32.const 0))
                    (i32.const 15)))))
            (local.set 4
              (i32.const 2))
            (br_if 3 (;@1;)
              (i32.gt_s
                (i32.extend8_s
                  (local.get 3))
                (local.tee 3
                  (i32.and
                    (i32.load8_u offset=9120
                      (i32.const 0))
                    (i32.const 15)))))
            (br 2 (;@2;)))
          (i32.store8 offset=9116
            (i32.const 0)
            (local.tee 4
              (i32.sub
                (i32.load8_u offset=9116
                  (i32.const 0))
                (i32.and
                  (i32.load8_u offset=9121
                    (i32.const 0))
                  (i32.const 15)))))
          (br_if 2 (;@1;)
            (i32.ge_s
              (i32.extend8_s
                (local.get 4))
              (i32.const 0)))
          (local.set 4
            (i32.const 4))
          (br 1 (;@2;)))
        (local.set 3
          (i32.const 15))
        (i32.store8 offset=9116
          (i32.const 0)
          (local.tee 5
            (i32.add
              (i32.and
                (i32.load8_u offset=9118
                  (i32.const 0))
                (i32.const 15))
              (i32.load8_u offset=9116
                (i32.const 0)))))
        (local.set 4
          (i32.const 1))
        (br_if 1 (;@1;)
          (i32.le_s
            (i32.extend8_s
              (local.get 5))
            (i32.const 14))))
      (i32.store8 offset=9116
        (i32.const 0)
        (local.get 3))
      (i32.store8 offset=9117
        (i32.const 0)
        (local.get 4)))
    (call $gba_set_sound_reg
      (i32.const 67108962)
      (i32.or
        (i32.shl
          (i32.and
            (i32.load8_u offset=9116
              (i32.const 0))
            (i32.const 15))
          (i32.const 12))
        (i32.const 64)))
    (call $gba_set_sound_reg
      (i32.const 67108964)
      (i32.or
        (i32.and
          (i32.sub
            (i32.const 0)
            (i32.div_u
              (i32.const 33554432)
              (local.get 0)))
          (i32.const 2047))
        (i32.const 32768))))
  (func $default-instruments.square1_1_8.frame (type 5) (param i32 i32 i32)
    (local i32 i32 i32)
    (local.set 3
      (i32.const 0))
    (block  ;; label = @1
      (block  ;; label = @2
        (block  ;; label = @3
          (block  ;; label = @4
            (block  ;; label = @5
              (br_table 2 (;@3;) 0 (;@5;) 4 (;@1;) 1 (;@4;) 4 (;@1;) 2 (;@3;)
                (i32.and
                  (i32.load8_u offset=9117
                    (i32.const 0))
                  (i32.const 7))))
            (i32.store8 offset=9116
              (i32.const 0)
              (local.tee 3
                (i32.sub
                  (i32.load8_u offset=9116
                    (i32.const 0))
                  (i32.and
                    (i32.load8_u offset=9119
                      (i32.const 0))
                    (i32.const 15)))))
            (local.set 4
              (i32.const 2))
            (br_if 3 (;@1;)
              (i32.gt_s
                (i32.extend8_s
                  (local.get 3))
                (local.tee 3
                  (i32.and
                    (i32.load8_u offset=9120
                      (i32.const 0))
                    (i32.const 15)))))
            (br 2 (;@2;)))
          (i32.store8 offset=9116
            (i32.const 0)
            (local.tee 4
              (i32.sub
                (i32.load8_u offset=9116
                  (i32.const 0))
                (i32.and
                  (i32.load8_u offset=9121
                    (i32.const 0))
                  (i32.const 15)))))
          (br_if 2 (;@1;)
            (i32.ge_s
              (i32.extend8_s
                (local.get 4))
              (i32.const 0)))
          (local.set 4
            (i32.const 4))
          (br 1 (;@2;)))
        (local.set 3
          (i32.const 15))
        (i32.store8 offset=9116
          (i32.const 0)
          (local.tee 5
            (i32.add
              (i32.and
                (i32.load8_u offset=9118
                  (i32.const 0))
                (i32.const 15))
              (i32.load8_u offset=9116
                (i32.const 0)))))
        (local.set 4
          (i32.const 1))
        (br_if 1 (;@1;)
          (i32.le_s
            (i32.extend8_s
              (local.get 5))
            (i32.const 14))))
      (i32.store8 offset=9116
        (i32.const 0)
        (local.get 3))
      (i32.store8 offset=9117
        (i32.const 0)
        (local.get 4)))
    (call $gba_set_sound_reg
      (i32.const 67108962)
      (i32.shl
        (i32.and
          (i32.load8_u offset=9116
            (i32.const 0))
          (i32.const 15))
        (i32.const 12)))
    (call $gba_set_sound_reg
      (i32.const 67108964)
      (i32.or
        (i32.and
          (i32.sub
            (i32.const 0)
            (i32.div_u
              (i32.const 33554432)
              (local.get 0)))
          (i32.const 2047))
        (i32.const 32768))))
  (func $default-instruments.square2_dyad.press (type 4) (param i32 i32 i32 i32)
    (local i32 i32 i32)
    (i32.store8 offset=9164
      (i32.const 0)
      (local.get 2))
    (call $gba_set_sound_reg
      (i32.const 67108960)
      (i32.const 8))
    (call $gba_set_sound_reg
      (i32.const 67108962)
      (i32.const 41152))
    (call $gba_set_sound_reg
      (i32.const 67108968)
      (i32.const 53376))
    (call $gba_set_sound_reg
      (i32.const 67108964)
      (i32.or
        (i32.and
          (i32.sub
            (i32.const 0)
            (i32.div_u
              (i32.const 33554432)
              (local.get 0)))
          (i32.const 2047))
        (i32.const 32768)))
    (call $gba_set_sound_reg
      (i32.const 67108972)
      (i32.or
        (i32.and
          (i32.sub
            (i32.const 0)
            (i32.div_u
              (i32.const 33554432)
              (i32.div_u
                (i32.mul
                  (i32.and
                    (select
                      (local.tee 6
                        (i32.shr_u
                          (local.tee 5
                            (i32.load align=2
                              (i32.add
                                (i32.shl
                                  (i32.and
                                    (i32.sub
                                      (local.tee 4
                                        (i32.sub
                                          (i32.xor
                                            (local.tee 2
                                              (i32.load8_s offset=9164
                                                (i32.const 0)))
                                            (local.tee 4
                                              (i32.shr_s
                                                (i32.extend8_s
                                                  (local.get 2))
                                                (i32.const 7))))
                                          (local.get 4)))
                                      (i32.mul
                                        (local.tee 4
                                          (i32.div_u
                                            (i32.and
                                              (local.get 4)
                                              (i32.const 255))
                                            (i32.const 12)))
                                        (i32.const 12)))
                                    (i32.const 255))
                                  (i32.const 2))
                                (i32.const 8932))))
                          (i32.const 16)))
                      (local.tee 4
                        (i32.shl
                          (local.get 5)
                          (local.get 4)))
                      (local.tee 2
                        (i32.lt_s
                          (local.get 2)
                          (i32.const 0))))
                    (i32.const 65535))
                  (local.get 0))
                (i32.and
                  (select
                    (local.get 4)
                    (local.get 6)
                    (local.get 2))
                  (i32.const 65535)))))
          (i32.const 2047))
        (i32.const 32768))))
  (func $default-instruments.square2_dyad.release (type 5) (param i32 i32 i32)
    (local i32 i32 i32 i32)
    (call $gba_set_sound_reg
      (i32.const 67108962)
      (i32.const 41408))
    (call $gba_set_sound_reg
      (i32.const 67108968)
      (i32.const 53632))
    (call $gba_set_sound_reg
      (i32.const 67108964)
      (i32.or
        (i32.and
          (i32.sub
            (i32.const 0)
            (i32.div_u
              (i32.const 33554432)
              (local.get 0)))
          (i32.const 2047))
        (i32.const 32768)))
    (call $gba_set_sound_reg
      (i32.const 67108972)
      (i32.or
        (i32.and
          (i32.sub
            (i32.const 0)
            (i32.div_u
              (i32.const 33554432)
              (i32.div_u
                (i32.mul
                  (i32.and
                    (select
                      (local.tee 6
                        (i32.shr_u
                          (local.tee 5
                            (i32.load align=2
                              (i32.add
                                (i32.shl
                                  (i32.and
                                    (i32.sub
                                      (local.tee 4
                                        (i32.sub
                                          (i32.xor
                                            (local.tee 3
                                              (i32.load8_s offset=9164
                                                (i32.const 0)))
                                            (local.tee 4
                                              (i32.shr_s
                                                (i32.extend8_s
                                                  (local.get 3))
                                                (i32.const 7))))
                                          (local.get 4)))
                                      (i32.mul
                                        (local.tee 4
                                          (i32.div_u
                                            (i32.and
                                              (local.get 4)
                                              (i32.const 255))
                                            (i32.const 12)))
                                        (i32.const 12)))
                                    (i32.const 255))
                                  (i32.const 2))
                                (i32.const 8932))))
                          (i32.const 16)))
                      (local.tee 4
                        (i32.shl
                          (local.get 5)
                          (local.get 4)))
                      (local.tee 3
                        (i32.lt_s
                          (local.get 3)
                          (i32.const 0))))
                    (i32.const 65535))
                  (local.get 0))
                (i32.and
                  (select
                    (local.get 4)
                    (local.get 6)
                    (local.get 3))
                  (i32.const 65535)))))
          (i32.const 2047))
        (i32.const 32768))))
  (func $default-instruments.square2_dyad.frame (type 5) (param i32 i32 i32)
    (local i32 i32 i32 i32)
    (block  ;; label = @1
      (br_if 0 (;@1;)
        (i32.le_u
          (local.get 2)
          (i32.const 14)))
      (call $gba_set_sound_reg
        (i32.const 67108964)
        (i32.and
          (i32.sub
            (i32.const 0)
            (i32.div_u
              (i32.const 33554432)
              (i32.add
                (i32.add
                  (i32.sub
                    (local.get 0)
                    (i32.div_u
                      (local.get 0)
                      (i32.const 36)))
                  (i32.mul
                    (i32.sub
                      (i32.xor
                        (local.tee 3
                          (i32.add
                            (i32.rem_u
                              (i32.and
                                (i32.add
                                  (i32.add
                                    (local.tee 4
                                      (i32.rem_s
                                        (local.tee 3
                                          (i32.add
                                            (local.get 2)
                                            (i32.const -17)))
                                        (i32.const 12)))
                                    (select
                                      (i32.and
                                        (i32.shr_s
                                          (local.get 3)
                                          (i32.const 31))
                                        (i32.const 12))
                                      (i32.const 0)
                                      (local.get 4)))
                                  (i32.const 12))
                                (i32.const 255))
                              (i32.const 12))
                            (i32.const -6)))
                        (local.tee 3
                          (i32.shr_s
                            (local.get 3)
                            (i32.const 31))))
                      (local.get 3))
                    (i32.div_u
                      (local.get 0)
                      (i32.const 108))))
                (i32.const 1))))
          (i32.const 2047)))
      (call $gba_set_sound_reg
        (i32.const 67108972)
        (i32.and
          (i32.sub
            (i32.const 0)
            (i32.div_u
              (i32.const 33554432)
              (i32.add
                (i32.add
                  (i32.sub
                    (local.tee 0
                      (i32.div_u
                        (i32.mul
                          (i32.and
                            (select
                              (local.tee 6
                                (i32.shr_u
                                  (local.tee 5
                                    (i32.load align=2
                                      (i32.add
                                        (i32.shl
                                          (i32.and
                                            (i32.sub
                                              (local.tee 4
                                                (i32.sub
                                                  (i32.xor
                                                    (local.tee 3
                                                      (i32.load8_s offset=9164
                                                        (i32.const 0)))
                                                    (local.tee 4
                                                      (i32.shr_s
                                                        (i32.extend8_s
                                                          (local.get 3))
                                                        (i32.const 7))))
                                                  (local.get 4)))
                                              (i32.mul
                                                (local.tee 4
                                                  (i32.div_u
                                                    (i32.and
                                                      (local.get 4)
                                                      (i32.const 255))
                                                    (i32.const 12)))
                                                (i32.const 12)))
                                            (i32.const 255))
                                          (i32.const 2))
                                        (i32.const 8932))))
                                  (i32.const 16)))
                              (local.tee 4
                                (i32.shl
                                  (local.get 5)
                                  (local.get 4)))
                              (local.tee 3
                                (i32.lt_s
                                  (local.get 3)
                                  (i32.const 0))))
                            (i32.const 65535))
                          (local.get 0))
                        (i32.and
                          (select
                            (local.get 4)
                            (local.get 6)
                            (local.get 3))
                          (i32.const 65535))))
                    (i32.div_u
                      (local.get 0)
                      (i32.const 36)))
                  (i32.mul
                    (i32.div_u
                      (local.get 0)
                      (i32.const 108))
                    (i32.sub
                      (i32.xor
                        (local.tee 2
                          (i32.add
                            (i32.rem_u
                              (i32.and
                                (i32.add
                                  (i32.add
                                    (local.tee 0
                                      (i32.rem_s
                                        (local.tee 2
                                          (i32.add
                                            (local.get 2)
                                            (i32.const -23)))
                                        (i32.const 12)))
                                    (select
                                      (i32.and
                                        (i32.shr_s
                                          (local.get 2)
                                          (i32.const 31))
                                        (i32.const 12))
                                      (i32.const 0)
                                      (local.get 0)))
                                  (i32.const 12))
                                (i32.const 255))
                              (i32.const 12))
                            (i32.const -6)))
                        (local.tee 2
                          (i32.shr_s
                            (local.get 2)
                            (i32.const 31))))
                      (local.get 2))))
                (i32.const 1))))
          (i32.const 2047)))))
  (func $default-instruments.square2_bleep.press (type 4) (param i32 i32 i32 i32)
    (call $gba_set_sound_reg
      (i32.const 67108968)
      (i32.or
        (i32.and
          (i32.shl
            (local.get 2)
            (i32.const 6))
          (i32.const 192))
        (i32.const 41216)))
    (call $gba_set_sound_reg
      (i32.const 67108972)
      (i32.or
        (i32.and
          (i32.sub
            (i32.const 0)
            (i32.div_u
              (i32.const 33554432)
              (local.get 0)))
          (i32.const 2047))
        (i32.const 32768))))
  (func $default-instruments.square2_arp.press (type 4) (param i32 i32 i32 i32)
    (i32.store8 offset=9136
      (i32.const 0)
      (local.get 3))
    (i32.store8 offset=9135
      (i32.const 0)
      (local.get 2))
    (i32.store16 offset=9132 align=1
      (i32.const 0)
      (i32.load16_u offset=8457 align=1
        (i32.const 0)))
    (i32.store offset=9128 align=1
      (i32.const 0)
      (i32.load offset=8453 align=1
        (i32.const 0))))
  (func $default-instruments.square2_1_4.release (type 5) (param i32 i32 i32)
    (block  ;; label = @1
      (br_if 0 (;@1;)
        (i32.gt_u
          (i32.and
            (i32.load8_u offset=9129
              (i32.const 0))
            (i32.const 7))
          (i32.const 2)))
      (i32.store8 offset=9129
        (i32.const 0)
        (i32.const 3))
      (i32.store8 offset=9128
        (i32.const 0)
        (i32.and
          (i32.load8_u offset=9132
            (i32.const 0))
          (i32.const 15)))))
  (func $default-instruments.square2_arp.frame (type 5) (param i32 i32 i32)
    (local i32 i32 i32)
    (local.set 3
      (i32.const 0))
    (block  ;; label = @1
      (block  ;; label = @2
        (block  ;; label = @3
          (block  ;; label = @4
            (block  ;; label = @5
              (br_table 2 (;@3;) 0 (;@5;) 4 (;@1;) 1 (;@4;) 4 (;@1;) 2 (;@3;)
                (i32.and
                  (i32.load8_u offset=9129
                    (i32.const 0))
                  (i32.const 7))))
            (i32.store8 offset=9128
              (i32.const 0)
              (local.tee 3
                (i32.sub
                  (i32.load8_u offset=9128
                    (i32.const 0))
                  (i32.and
                    (i32.load8_u offset=9131
                      (i32.const 0))
                    (i32.const 15)))))
            (local.set 4
              (i32.const 2))
            (br_if 3 (;@1;)
              (i32.gt_s
                (i32.extend8_s
                  (local.get 3))
                (local.tee 3
                  (i32.and
                    (i32.load8_u offset=9132
                      (i32.const 0))
                    (i32.const 15)))))
            (br 2 (;@2;)))
          (i32.store8 offset=9128
            (i32.const 0)
            (local.tee 4
              (i32.sub
                (i32.load8_u offset=9128
                  (i32.const 0))
                (i32.and
                  (i32.load8_u offset=9133
                    (i32.const 0))
                  (i32.const 15)))))
          (br_if 2 (;@1;)
            (i32.ge_s
              (i32.extend8_s
                (local.get 4))
              (i32.const 0)))
          (local.set 4
            (i32.const 4))
          (br 1 (;@2;)))
        (local.set 3
          (i32.const 15))
        (i32.store8 offset=9128
          (i32.const 0)
          (local.tee 5
            (i32.add
              (i32.and
                (i32.load8_u offset=9130
                  (i32.const 0))
                (i32.const 15))
              (i32.load8_u offset=9128
                (i32.const 0)))))
        (local.set 4
          (i32.const 1))
        (br_if 1 (;@1;)
          (i32.le_s
            (i32.extend8_s
              (local.get 5))
            (i32.const 14))))
      (i32.store8 offset=9128
        (i32.const 0)
        (local.get 3))
      (i32.store8 offset=9129
        (i32.const 0)
        (local.get 4)))
    (call $gba_set_sound_reg
      (i32.const 67108968)
      (i32.or
        (i32.shl
          (i32.and
            (i32.load8_u offset=9128
              (i32.const 0))
            (i32.const 15))
          (i32.const 12))
        (i32.const 128)))
    (call $gba_set_sound_reg
      (i32.const 67108972)
      (i32.or
        (i32.and
          (i32.sub
            (i32.const 0)
            (i32.div_u
              (i32.const 33554432)
              (i32.div_u
                (i32.mul
                  (i32.and
                    (select
                      (local.tee 5
                        (i32.shr_u
                          (local.tee 4
                            (i32.load align=2
                              (i32.add
                                (i32.shl
                                  (i32.and
                                    (i32.sub
                                      (local.tee 3
                                        (i32.sub
                                          (i32.xor
                                            (local.tee 2
                                              (i32.load8_s
                                                (i32.add
                                                  (i32.and
                                                    (local.get 2)
                                                    (i32.const 3))
                                                  (i32.const 9134))))
                                            (local.tee 3
                                              (i32.shr_s
                                                (i32.extend8_s
                                                  (local.get 2))
                                                (i32.const 7))))
                                          (local.get 3)))
                                      (i32.mul
                                        (local.tee 3
                                          (i32.div_u
                                            (i32.and
                                              (local.get 3)
                                              (i32.const 255))
                                            (i32.const 12)))
                                        (i32.const 12)))
                                    (i32.const 255))
                                  (i32.const 2))
                                (i32.const 8932))))
                          (i32.const 16)))
                      (local.tee 3
                        (i32.shl
                          (local.get 4)
                          (local.get 3)))
                      (local.tee 2
                        (i32.lt_s
                          (local.get 2)
                          (i32.const 0))))
                    (i32.const 65535))
                  (local.get 0))
                (i32.and
                  (select
                    (local.get 3)
                    (local.get 5)
                    (local.get 2))
                  (i32.const 65535)))))
          (i32.const 2047))
        (i32.const 32768))))
  (func $default-instruments.square2_pan.press (type 4) (param i32 i32 i32 i32)
    (i32.store8 offset=9166
      (i32.const 0)
      (i32.and
        (local.get 3)
        (i32.const 127)))
    (i32.store8 offset=9165
      (i32.const 0)
      (i32.and
        (local.get 2)
        (i32.const 127)))
    (call $gba_set_sound_reg
      (i32.const 67108968)
      (i32.const 53376))
    (call $gba_set_sound_reg
      (i32.const 67108972)
      (i32.or
        (i32.and
          (i32.sub
            (i32.const 0)
            (i32.div_u
              (i32.const 33554432)
              (local.get 0)))
          (i32.const 2047))
        (i32.const 32768))))
  (func $default-instruments.square2_pan.release (type 5) (param i32 i32 i32)
    (call $gba_set_sound_reg
      (i32.const 67108968)
      (i32.const 53632))
    (call $gba_set_sound_reg
      (i32.const 67108972)
      (i32.or
        (i32.and
          (i32.sub
            (i32.const 0)
            (i32.div_u
              (i32.const 33554432)
              (local.get 0)))
          (i32.const 2047))
        (i32.const 32768)))
    (i32.store16 offset=9138
      (i32.const 0)
      (local.tee 0
        (i32.or
          (i32.load16_u offset=9138
            (i32.const 0))
          (i32.const 8704))))
    (call $gba_set_sound_reg
      (i32.const 67108992)
      (local.get 0)))
  (func $default-instruments.square2_pan.frame (type 5) (param i32 i32 i32)
    (local i32)
    (local.set 3
      (i32.load16_u offset=9138
        (i32.const 0)))
    (block  ;; label = @1
      (br_if 0 (;@1;)
        (i32.rem_u
          (local.tee 2
            (i32.and
              (local.get 2)
              (i32.const 127)))
          (i32.and
            (i32.load8_u offset=9165
              (i32.const 0))
            (i32.const 127))))
      (i32.store16 offset=9138
        (i32.const 0)
        (local.tee 3
          (i32.xor
            (local.get 3)
            (i32.const 8192)))))
    (block  ;; label = @1
      (br_if 0 (;@1;)
        (i32.rem_u
          (local.get 2)
          (i32.and
            (i32.load8_u offset=9166
              (i32.const 0))
            (i32.const 127))))
      (i32.store16 offset=9138
        (i32.const 0)
        (local.tee 3
          (i32.xor
            (local.get 3)
            (i32.const 512)))))
    (call $gba_set_sound_reg
      (i32.const 67108992)
      (i32.and
        (local.get 3)
        (i32.const 65535))))
  (func $default-instruments.square2_base.press (type 4) (param i32 i32 i32 i32)
    (local i32)
    (global.set $__stack_pointer
      (local.tee 4
        (i32.sub
          (global.get $__stack_pointer)
          (i32.const 16))))
    (i32.store16 offset=9140
      (i32.const 0)
      (i32.or
        (i32.and
          (i32.load16_u offset=9140
            (i32.const 0))
          (i32.const 65343))
        (i32.and
          (i32.shl
            (local.get 2)
            (i32.const 6))
          (i32.const 192))))
    (i32.store8 offset=15
      (local.get 4)
      (i32.const 0))
    (i32.store8 offset=14
      (local.get 4)
      (i32.const 8))
    (i32.store8 offset=13
      (local.get 4)
      (i32.const 5))
    (i32.store8 offset=12
      (local.get 4)
      (local.tee 2
        (i32.and
          (i32.shr_u
            (local.get 3)
            (i32.const 4))
          (i32.const 15))))
    (i32.store8 offset=11
      (local.get 4)
      (local.tee 3
        (i32.and
          (local.get 3)
          (i32.const 15))))
    (i32.store8 offset=9133
      (i32.const 0)
      (local.get 3))
    (i32.store8 offset=9132
      (i32.const 0)
      (local.get 2))
    (i32.store8 offset=9131
      (i32.const 0)
      (i32.const 5))
    (i32.store8 offset=9130
      (i32.const 0)
      (i32.const 8))
    (i32.store8 offset=9129
      (i32.const 0)
      (i32.const 0))
    (i32.store8 offset=9128
      (i32.const 0)
      (i32.const 0))
    (call $gba_set_sound_reg
      (i32.const 67108960)
      (i32.const 8))
    (global.set $__stack_pointer
      (i32.add
        (local.get 4)
        (i32.const 16))))
  (func $default-instruments.square2_base.frame (type 5) (param i32 i32 i32)
    (local i32 i32 i32 i32)
    (local.set 3
      (i32.const 0))
    (local.set 4
      (i32.load16_u offset=9140
        (i32.const 0)))
    (block  ;; label = @1
      (block  ;; label = @2
        (block  ;; label = @3
          (block  ;; label = @4
            (block  ;; label = @5
              (br_table 2 (;@3;) 0 (;@5;) 4 (;@1;) 1 (;@4;) 4 (;@1;) 2 (;@3;)
                (i32.and
                  (i32.load8_u offset=9129
                    (i32.const 0))
                  (i32.const 7))))
            (i32.store8 offset=9128
              (i32.const 0)
              (local.tee 3
                (i32.sub
                  (i32.load8_u offset=9128
                    (i32.const 0))
                  (i32.and
                    (i32.load8_u offset=9131
                      (i32.const 0))
                    (i32.const 15)))))
            (local.set 5
              (i32.const 2))
            (br_if 3 (;@1;)
              (i32.gt_s
                (i32.extend8_s
                  (local.get 3))
                (local.tee 3
                  (i32.and
                    (i32.load8_u offset=9132
                      (i32.const 0))
                    (i32.const 15)))))
            (br 2 (;@2;)))
          (i32.store8 offset=9128
            (i32.const 0)
            (local.tee 5
              (i32.sub
                (i32.load8_u offset=9128
                  (i32.const 0))
                (i32.and
                  (i32.load8_u offset=9133
                    (i32.const 0))
                  (i32.const 15)))))
          (br_if 2 (;@1;)
            (i32.ge_s
              (i32.extend8_s
                (local.get 5))
              (i32.const 0)))
          (local.set 5
            (i32.const 4))
          (br 1 (;@2;)))
        (local.set 3
          (i32.const 15))
        (i32.store8 offset=9128
          (i32.const 0)
          (local.tee 6
            (i32.add
              (i32.and
                (i32.load8_u offset=9130
                  (i32.const 0))
                (i32.const 15))
              (i32.load8_u offset=9128
                (i32.const 0)))))
        (local.set 5
          (i32.const 1))
        (br_if 1 (;@1;)
          (i32.le_s
            (i32.extend8_s
              (local.get 6))
            (i32.const 14))))
      (i32.store8 offset=9128
        (i32.const 0)
        (local.get 3))
      (i32.store8 offset=9129
        (i32.const 0)
        (local.get 5)))
    (call $gba_set_sound_reg
      (i32.const 67108968)
      (i32.and
        (i32.or
          (i32.shl
            (i32.load8_u offset=9128
              (i32.const 0))
            (i32.const 12))
          (i32.and
            (local.get 4)
            (i32.const 4095)))
        (i32.const 65535)))
    (call $gba_set_sound_reg
      (i32.const 67108972)
      (i32.or
        (i32.and
          (i32.sub
            (i32.const 0)
            (i32.div_u
              (i32.const 33554432)
              (local.get 0)))
          (i32.const 2047))
        (i32.const 32768))))
  (func $default-instruments.square2_base.set_duty (type 6) (param i32)
    (i32.store16 offset=9140
      (i32.const 0)
      (i32.or
        (i32.and
          (i32.load16_u offset=9140
            (i32.const 0))
          (i32.const 65343))
        (i32.and
          (i32.shl
            (local.get 0)
            (i32.const 6))
          (i32.const 192)))))
  (func $default-instruments.square2_1_4.press (type 4) (param i32 i32 i32 i32)
    (local i32 i32 i32 i32 i32)
    (global.set $__stack_pointer
      (local.tee 4
        (i32.sub
          (global.get $__stack_pointer)
          (i32.const 16))))
    (local.set 5
      (i32.shr_u
        (i32.and
          (local.get 3)
          (i32.const 240))
        (i32.const 4)))
    (local.set 6
      (i32.const 8984))
    (block  ;; label = @1
      (br_if 0 (;@1;)
        (i32.and
          (i32.or
            (local.tee 7
              (i32.shr_u
                (i32.and
                  (local.get 2)
                  (i32.const 240))
                (i32.const 4)))
            (local.get 2))
          (i32.const 15)))
      (i32.store8 offset=10
        (local.get 4)
        (i32.const 2))
      (i32.store8 offset=9
        (local.get 4)
        (local.get 5))
      (i32.store8 offset=8
        (local.get 4)
        (local.get 5))
      (local.set 6
        (i32.add
          (local.get 4)
          (i32.const 9))))
    (i32.store8 offset=14
      (local.get 4)
      (local.get 7))
    (i32.store8 offset=12
      (local.get 4)
      (local.get 5))
    (i32.store8 offset=13
      (local.get 4)
      (local.tee 2
        (i32.and
          (local.get 2)
          (i32.const 15))))
    (i32.store8 offset=11
      (local.get 4)
      (local.tee 3
        (i32.and
          (local.get 3)
          (i32.const 15))))
    (i32.store8 offset=15
      (local.get 4)
      (local.tee 8
        (i32.and
          (i32.load8_u offset=1
            (local.get 6))
          (i32.const 7))))
    (local.set 6
      (i32.load8_u
        (local.get 6)))
    (i32.store8 offset=9133
      (i32.const 0)
      (local.get 3))
    (i32.store8 offset=9132
      (i32.const 0)
      (local.get 5))
    (i32.store8 offset=9131
      (i32.const 0)
      (local.get 2))
    (i32.store8 offset=9130
      (i32.const 0)
      (local.get 7))
    (i32.store8 offset=9129
      (i32.const 0)
      (local.get 8))
    (i32.store8 offset=9128
      (i32.const 0)
      (i32.and
        (local.get 6)
        (i32.const 15)))
    (call $gba_set_sound_reg
      (i32.const 67108960)
      (i32.const 8))
    (global.set $__stack_pointer
      (i32.add
        (local.get 4)
        (i32.const 16))))
  (func $default-instruments.square2_2_4.frame (type 5) (param i32 i32 i32)
    (local i32 i32 i32)
    (local.set 3
      (i32.const 0))
    (block  ;; label = @1
      (block  ;; label = @2
        (block  ;; label = @3
          (block  ;; label = @4
            (block  ;; label = @5
              (br_table 2 (;@3;) 0 (;@5;) 4 (;@1;) 1 (;@4;) 4 (;@1;) 2 (;@3;)
                (i32.and
                  (i32.load8_u offset=9129
                    (i32.const 0))
                  (i32.const 7))))
            (i32.store8 offset=9128
              (i32.const 0)
              (local.tee 3
                (i32.sub
                  (i32.load8_u offset=9128
                    (i32.const 0))
                  (i32.and
                    (i32.load8_u offset=9131
                      (i32.const 0))
                    (i32.const 15)))))
            (local.set 4
              (i32.const 2))
            (br_if 3 (;@1;)
              (i32.gt_s
                (i32.extend8_s
                  (local.get 3))
                (local.tee 3
                  (i32.and
                    (i32.load8_u offset=9132
                      (i32.const 0))
                    (i32.const 15)))))
            (br 2 (;@2;)))
          (i32.store8 offset=9128
            (i32.const 0)
            (local.tee 4
              (i32.sub
                (i32.load8_u offset=9128
                  (i32.const 0))
                (i32.and
                  (i32.load8_u offset=9133
                    (i32.const 0))
                  (i32.const 15)))))
          (br_if 2 (;@1;)
            (i32.ge_s
              (i32.extend8_s
                (local.get 4))
              (i32.const 0)))
          (local.set 4
            (i32.const 4))
          (br 1 (;@2;)))
        (local.set 3
          (i32.const 15))
        (i32.store8 offset=9128
          (i32.const 0)
          (local.tee 5
            (i32.add
              (i32.and
                (i32.load8_u offset=9130
                  (i32.const 0))
                (i32.const 15))
              (i32.load8_u offset=9128
                (i32.const 0)))))
        (local.set 4
          (i32.const 1))
        (br_if 1 (;@1;)
          (i32.le_s
            (i32.extend8_s
              (local.get 5))
            (i32.const 14))))
      (i32.store8 offset=9128
        (i32.const 0)
        (local.get 3))
      (i32.store8 offset=9129
        (i32.const 0)
        (local.get 4)))
    (call $gba_set_sound_reg
      (i32.const 67108968)
      (i32.or
        (i32.shl
          (i32.and
            (i32.load8_u offset=9128
              (i32.const 0))
            (i32.const 15))
          (i32.const 12))
        (i32.const 128)))
    (call $gba_set_sound_reg
      (i32.const 67108972)
      (i32.or
        (i32.and
          (i32.sub
            (i32.const 0)
            (i32.div_u
              (i32.const 33554432)
              (local.get 0)))
          (i32.const 2047))
        (i32.const 32768))))
  (func $default-instruments.square2_1_4.frame (type 5) (param i32 i32 i32)
    (local i32 i32 i32)
    (local.set 3
      (i32.const 0))
    (block  ;; label = @1
      (block  ;; label = @2
        (block  ;; label = @3
          (block  ;; label = @4
            (block  ;; label = @5
              (br_table 2 (;@3;) 0 (;@5;) 4 (;@1;) 1 (;@4;) 4 (;@1;) 2 (;@3;)
                (i32.and
                  (i32.load8_u offset=9129
                    (i32.const 0))
                  (i32.const 7))))
            (i32.store8 offset=9128
              (i32.const 0)
              (local.tee 3
                (i32.sub
                  (i32.load8_u offset=9128
                    (i32.const 0))
                  (i32.and
                    (i32.load8_u offset=9131
                      (i32.const 0))
                    (i32.const 15)))))
            (local.set 4
              (i32.const 2))
            (br_if 3 (;@1;)
              (i32.gt_s
                (i32.extend8_s
                  (local.get 3))
                (local.tee 3
                  (i32.and
                    (i32.load8_u offset=9132
                      (i32.const 0))
                    (i32.const 15)))))
            (br 2 (;@2;)))
          (i32.store8 offset=9128
            (i32.const 0)
            (local.tee 4
              (i32.sub
                (i32.load8_u offset=9128
                  (i32.const 0))
                (i32.and
                  (i32.load8_u offset=9133
                    (i32.const 0))
                  (i32.const 15)))))
          (br_if 2 (;@1;)
            (i32.ge_s
              (i32.extend8_s
                (local.get 4))
              (i32.const 0)))
          (local.set 4
            (i32.const 4))
          (br 1 (;@2;)))
        (local.set 3
          (i32.const 15))
        (i32.store8 offset=9128
          (i32.const 0)
          (local.tee 5
            (i32.add
              (i32.and
                (i32.load8_u offset=9130
                  (i32.const 0))
                (i32.const 15))
              (i32.load8_u offset=9128
                (i32.const 0)))))
        (local.set 4
          (i32.const 1))
        (br_if 1 (;@1;)
          (i32.le_s
            (i32.extend8_s
              (local.get 5))
            (i32.const 14))))
      (i32.store8 offset=9128
        (i32.const 0)
        (local.get 3))
      (i32.store8 offset=9129
        (i32.const 0)
        (local.get 4)))
    (call $gba_set_sound_reg
      (i32.const 67108968)
      (i32.or
        (i32.shl
          (i32.and
            (i32.load8_u offset=9128
              (i32.const 0))
            (i32.const 15))
          (i32.const 12))
        (i32.const 64)))
    (call $gba_set_sound_reg
      (i32.const 67108972)
      (i32.or
        (i32.and
          (i32.sub
            (i32.const 0)
            (i32.div_u
              (i32.const 33554432)
              (local.get 0)))
          (i32.const 2047))
        (i32.const 32768))))
  (func $default-instruments.square2_1_8.frame (type 5) (param i32 i32 i32)
    (local i32 i32 i32)
    (local.set 3
      (i32.const 0))
    (block  ;; label = @1
      (block  ;; label = @2
        (block  ;; label = @3
          (block  ;; label = @4
            (block  ;; label = @5
              (br_table 2 (;@3;) 0 (;@5;) 4 (;@1;) 1 (;@4;) 4 (;@1;) 2 (;@3;)
                (i32.and
                  (i32.load8_u offset=9129
                    (i32.const 0))
                  (i32.const 7))))
            (i32.store8 offset=9128
              (i32.const 0)
              (local.tee 3
                (i32.sub
                  (i32.load8_u offset=9128
                    (i32.const 0))
                  (i32.and
                    (i32.load8_u offset=9131
                      (i32.const 0))
                    (i32.const 15)))))
            (local.set 4
              (i32.const 2))
            (br_if 3 (;@1;)
              (i32.gt_s
                (i32.extend8_s
                  (local.get 3))
                (local.tee 3
                  (i32.and
                    (i32.load8_u offset=9132
                      (i32.const 0))
                    (i32.const 15)))))
            (br 2 (;@2;)))
          (i32.store8 offset=9128
            (i32.const 0)
            (local.tee 4
              (i32.sub
                (i32.load8_u offset=9128
                  (i32.const 0))
                (i32.and
                  (i32.load8_u offset=9133
                    (i32.const 0))
                  (i32.const 15)))))
          (br_if 2 (;@1;)
            (i32.ge_s
              (i32.extend8_s
                (local.get 4))
              (i32.const 0)))
          (local.set 4
            (i32.const 4))
          (br 1 (;@2;)))
        (local.set 3
          (i32.const 15))
        (i32.store8 offset=9128
          (i32.const 0)
          (local.tee 5
            (i32.add
              (i32.and
                (i32.load8_u offset=9130
                  (i32.const 0))
                (i32.const 15))
              (i32.load8_u offset=9128
                (i32.const 0)))))
        (local.set 4
          (i32.const 1))
        (br_if 1 (;@1;)
          (i32.le_s
            (i32.extend8_s
              (local.get 5))
            (i32.const 14))))
      (i32.store8 offset=9128
        (i32.const 0)
        (local.get 3))
      (i32.store8 offset=9129
        (i32.const 0)
        (local.get 4)))
    (call $gba_set_sound_reg
      (i32.const 67108968)
      (i32.shl
        (i32.and
          (i32.load8_u offset=9128
            (i32.const 0))
          (i32.const 15))
        (i32.const 12)))
    (call $gba_set_sound_reg
      (i32.const 67108972)
      (i32.or
        (i32.and
          (i32.sub
            (i32.const 0)
            (i32.div_u
              (i32.const 33554432)
              (local.get 0)))
          (i32.const 2047))
        (i32.const 32768))))
  (func $default-instruments.wave_triangle.press (type 4) (param i32 i32 i32 i32)
    (call $default-instruments.wave_p
      (local.get 0)
      (local.get 2)
      (local.get 3)
      (i32.const 8459)))
  (func $default-instruments.wave_env_r (type 5) (param i32 i32 i32)
    (block  ;; label = @1
      (br_if 0 (;@1;)
        (i32.gt_u
          (i32.and
            (i32.load8_u offset=9143
              (i32.const 0))
            (i32.const 7))
          (i32.const 2)))
      (i32.store8 offset=9143
        (i32.const 0)
        (i32.const 3))
      (i32.store8 offset=9142
        (i32.const 0)
        (i32.and
          (i32.load8_u offset=9146
            (i32.const 0))
          (i32.const 15)))))
  (func $default-instruments.wave_env_f (type 5) (param i32 i32 i32)
    (local i32 i32 i32)
    (local.set 3
      (i32.const 0))
    (block  ;; label = @1
      (block  ;; label = @2
        (block  ;; label = @3
          (block  ;; label = @4
            (block  ;; label = @5
              (br_table 2 (;@3;) 0 (;@5;) 4 (;@1;) 1 (;@4;) 4 (;@1;) 2 (;@3;)
                (i32.and
                  (i32.load8_u offset=9143
                    (i32.const 0))
                  (i32.const 7))))
            (i32.store8 offset=9142
              (i32.const 0)
              (local.tee 3
                (i32.sub
                  (i32.load8_u offset=9142
                    (i32.const 0))
                  (i32.and
                    (i32.load8_u offset=9145
                      (i32.const 0))
                    (i32.const 15)))))
            (local.set 4
              (i32.const 2))
            (br_if 3 (;@1;)
              (i32.gt_s
                (i32.extend8_s
                  (local.get 3))
                (local.tee 3
                  (i32.and
                    (i32.load8_u offset=9146
                      (i32.const 0))
                    (i32.const 15)))))
            (br 2 (;@2;)))
          (i32.store8 offset=9142
            (i32.const 0)
            (local.tee 4
              (i32.sub
                (i32.load8_u offset=9142
                  (i32.const 0))
                (i32.and
                  (i32.load8_u offset=9147
                    (i32.const 0))
                  (i32.const 15)))))
          (br_if 2 (;@1;)
            (i32.ge_s
              (i32.extend8_s
                (local.get 4))
              (i32.const 0)))
          (local.set 4
            (i32.const 4))
          (br 1 (;@2;)))
        (local.set 3
          (i32.const 15))
        (i32.store8 offset=9142
          (i32.const 0)
          (local.tee 5
            (i32.add
              (i32.and
                (i32.load8_u offset=9144
                  (i32.const 0))
                (i32.const 15))
              (i32.load8_u offset=9142
                (i32.const 0)))))
        (local.set 4
          (i32.const 1))
        (br_if 1 (;@1;)
          (i32.le_s
            (i32.extend8_s
              (local.get 5))
            (i32.const 14))))
      (i32.store8 offset=9142
        (i32.const 0)
        (local.get 3))
      (i32.store8 offset=9143
        (i32.const 0)
        (local.get 4)))
    (call $gba_set_sound_reg
      (i32.const 67108978)
      (i32.shl
        (i32.and
          (i32.load8_u
            (i32.add
              (i32.and
                (i32.load8_u offset=9142
                  (i32.const 0))
                (i32.const 15))
              (i32.const 8539)))
          (i32.const 7))
        (i32.const 13))))
  (func $default-instruments.wave_bass.press (type 4) (param i32 i32 i32 i32)
    (call $default-instruments.wave_p
      (local.get 0)
      (local.get 2)
      (local.get 3)
      (i32.const 8475)))
  (func $default-instruments.wave_arp.press (type 4) (param i32 i32 i32 i32)
    (local i32)
    (global.set $__stack_pointer
      (local.tee 4
        (i32.sub
          (global.get $__stack_pointer)
          (i32.const 16))))
    (i32.store8 offset=15
      (local.get 4)
      (i32.const 0))
    (i32.store8 offset=14
      (local.get 4)
      (i32.const 8))
    (i32.store8 offset=13
      (local.get 4)
      (i32.const 5))
    (i32.store8 offset=12
      (local.get 4)
      (i32.const 10))
    (i32.store8 offset=11
      (local.get 4)
      (i32.const 3))
    (i32.store8 offset=9147
      (i32.const 0)
      (i32.const 3))
    (i32.store8 offset=9146
      (i32.const 0)
      (i32.const 10))
    (i32.store8 offset=9145
      (i32.const 0)
      (i32.const 5))
    (i32.store8 offset=9144
      (i32.const 0)
      (i32.const 8))
    (i32.store8 offset=9143
      (i32.const 0)
      (i32.const 0))
    (i32.store8 offset=9142
      (i32.const 0)
      (i32.const 0))
    (call $gba_set_sound_reg
      (i32.const 67108976)
      (i32.const 0))
    (call $gba_set_wave_table
      (i32.const 8491)
      (i32.const 16))
    (call $gba_set_sound_reg
      (i32.const 67108976)
      (i32.const 192))
    (call $gba_set_sound_reg
      (i32.const 67108978)
      (i32.const 8192))
    (call $gba_set_sound_reg
      (i32.const 67108980)
      (i32.or
        (i32.and
          (i32.sub
            (i32.const 0)
            (i32.div_u
              (i32.const 16777216)
              (local.get 0)))
          (i32.const 2047))
        (i32.const 32768)))
    (i32.store8 offset=9150
      (i32.const 0)
      (local.get 3))
    (i32.store8 offset=9149
      (i32.const 0)
      (local.get 2))
    (global.set $__stack_pointer
      (i32.add
        (local.get 4)
        (i32.const 16))))
  (func $default-instruments.wave_arp.frame (type 5) (param i32 i32 i32)
    (local i32 i32 i32 i32 i32)
    (local.set 3
      (i32.const 2))
    (local.set 4
      (i32.const 0))
    (call $gba_set_sound_reg
      (i32.const 67108980)
      (i32.and
        (i32.sub
          (i32.const 0)
          (i32.div_u
            (i32.const 16777216)
            (i32.div_u
              (i32.mul
                (i32.and
                  (select
                    (local.tee 7
                      (i32.shr_u
                        (local.tee 6
                          (i32.load align=2
                            (i32.add
                              (i32.shl
                                (i32.and
                                  (i32.sub
                                    (local.tee 5
                                      (i32.sub
                                        (i32.xor
                                          (local.tee 2
                                            (i32.load8_s
                                              (i32.add
                                                (i32.and
                                                  (local.get 2)
                                                  (i32.const 3))
                                                (i32.const 9148))))
                                          (local.tee 5
                                            (i32.shr_s
                                              (i32.extend8_s
                                                (local.get 2))
                                              (i32.const 7))))
                                        (local.get 5)))
                                    (i32.mul
                                      (local.tee 5
                                        (i32.div_u
                                          (i32.and
                                            (local.get 5)
                                            (i32.const 255))
                                          (i32.const 12)))
                                      (i32.const 12)))
                                  (i32.const 255))
                                (i32.const 2))
                              (i32.const 8932))))
                        (i32.const 16)))
                    (local.tee 5
                      (i32.shl
                        (local.get 6)
                        (local.get 5)))
                    (local.tee 2
                      (i32.lt_s
                        (local.get 2)
                        (i32.const 0))))
                  (i32.const 65535))
                (local.get 0))
              (i32.and
                (select
                  (local.get 5)
                  (local.get 7)
                  (local.get 2))
                (i32.const 65535)))))
        (i32.const 2047)))
    (block  ;; label = @1
      (block  ;; label = @2
        (block  ;; label = @3
          (block  ;; label = @4
            (block  ;; label = @5
              (br_table 2 (;@3;) 0 (;@5;) 4 (;@1;) 1 (;@4;) 4 (;@1;) 2 (;@3;)
                (i32.and
                  (i32.load8_u offset=9143
                    (i32.const 0))
                  (i32.const 7))))
            (i32.store8 offset=9142
              (i32.const 0)
              (local.tee 2
                (i32.sub
                  (i32.load8_u offset=9142
                    (i32.const 0))
                  (i32.and
                    (i32.load8_u offset=9145
                      (i32.const 0))
                    (i32.const 15)))))
            (br_if 3 (;@1;)
              (i32.gt_s
                (i32.extend8_s
                  (local.get 2))
                (local.tee 4
                  (i32.and
                    (i32.load8_u offset=9146
                      (i32.const 0))
                    (i32.const 15)))))
            (br 2 (;@2;)))
          (i32.store8 offset=9142
            (i32.const 0)
            (local.tee 2
              (i32.sub
                (i32.load8_u offset=9142
                  (i32.const 0))
                (i32.and
                  (i32.load8_u offset=9147
                    (i32.const 0))
                  (i32.const 15)))))
          (br_if 2 (;@1;)
            (i32.ge_s
              (i32.extend8_s
                (local.get 2))
              (i32.const 0)))
          (local.set 3
            (i32.const 4))
          (br 1 (;@2;)))
        (local.set 4
          (i32.const 15))
        (i32.store8 offset=9142
          (i32.const 0)
          (local.tee 2
            (i32.add
              (i32.and
                (i32.load8_u offset=9144
                  (i32.const 0))
                (i32.const 15))
              (i32.load8_u offset=9142
                (i32.const 0)))))
        (local.set 3
          (i32.const 1))
        (br_if 1 (;@1;)
          (i32.le_s
            (i32.extend8_s
              (local.get 2))
            (i32.const 14))))
      (i32.store8 offset=9142
        (i32.const 0)
        (local.get 4))
      (i32.store8 offset=9143
        (i32.const 0)
        (local.get 3)))
    (call $gba_set_sound_reg
      (i32.const 67108978)
      (i32.shl
        (i32.and
          (i32.load8_u
            (i32.add
              (i32.and
                (i32.load8_u offset=9142
                  (i32.const 0))
                (i32.const 15))
              (i32.const 8539)))
          (i32.const 7))
        (i32.const 13))))
  (func $default-instruments.wave_duty.press (type 4) (param i32 i32 i32 i32)
    (call $default-instruments.wave_p
      (local.get 0)
      (local.get 2)
      (local.get 3)
      (i32.const 8507)))
  (func $default-instruments.wave_sweep.press (type 4) (param i32 i32 i32 i32)
    (local i32 i32)
    (global.set $__stack_pointer
      (local.tee 4
        (i32.sub
          (global.get $__stack_pointer)
          (i32.const 16))))
    (i32.store8 offset=15
      (local.get 4)
      (i32.const 0))
    (i32.store8 offset=14
      (local.get 4)
      (i32.const 8))
    (i32.store8 offset=13
      (local.get 4)
      (i32.const 5))
    (i32.store8 offset=12
      (local.get 4)
      (local.tee 5
        (i32.and
          (i32.shr_u
            (local.get 3)
            (i32.const 4))
          (i32.const 15))))
    (i32.store8 offset=11
      (local.get 4)
      (local.tee 3
        (i32.and
          (local.get 3)
          (i32.const 15))))
    (i32.store8 offset=9147
      (i32.const 0)
      (local.get 3))
    (i32.store8 offset=9146
      (i32.const 0)
      (local.get 5))
    (i32.store8 offset=9145
      (i32.const 0)
      (i32.const 5))
    (i32.store8 offset=9144
      (i32.const 0)
      (i32.const 8))
    (i32.store8 offset=9143
      (i32.const 0)
      (i32.const 0))
    (i32.store8 offset=9142
      (i32.const 0)
      (i32.const 0))
    (call $gba_set_sound_reg
      (i32.const 67108976)
      (i32.const 0))
    (call $gba_set_wave_table
      (i32.const 8523)
      (i32.const 16))
    (call $gba_set_sound_reg
      (i32.const 67108976)
      (i32.const 192))
    (call $gba_set_sound_reg
      (i32.const 67108978)
      (i32.const 8192))
    (call $gba_set_sound_reg
      (i32.const 67108980)
      (i32.or
        (i32.and
          (i32.sub
            (i32.const 0)
            (i32.div_u
              (i32.const 16777216)
              (local.get 0)))
          (i32.const 2047))
        (i32.const 32768)))
    (i32.store offset=9168
      (i32.const 0)
      (local.get 0))
    (i32.store offset=9152
      (i32.const 0)
      (local.get 2))
    (global.set $__stack_pointer
      (i32.add
        (local.get 4)
        (i32.const 16))))
  (func $default-instruments.wave_sweep.frame (type 5) (param i32 i32 i32)
    (local i32 i32 i32)
    (block  ;; label = @1
      (block  ;; label = @2
        (br_if 0 (;@2;)
          (i32.ge_u
            (local.tee 3
              (i32.load offset=9168
                (i32.const 0)))
            (i32.const 16777216)))
        (i32.store offset=9168
          (i32.const 0)
          (i32.div_u
            (i32.mul
              (local.get 3)
              (i32.load16_u
                (i32.add
                  (local.tee 4
                    (i32.shl
                      (i32.load offset=9152
                        (i32.const 0))
                      (i32.const 2)))
                  (i32.const 8932))))
            (i32.load16_u
              (i32.add
                (local.get 4)
                (i32.const 8934)))))
        (call $gba_set_sound_reg
          (i32.const 67108980)
          (i32.and
            (i32.sub
              (i32.const 0)
              (i32.div_u
                (i32.const 16777216)
                (local.get 3)))
            (i32.const 2047)))
        (br 1 (;@1;)))
      (br_if 0 (;@1;)
        (i32.gt_u
          (i32.and
            (i32.load8_u offset=9143
              (i32.const 0))
            (i32.const 7))
          (i32.const 2)))
      (i32.store8 offset=9143
        (i32.const 0)
        (i32.const 3))
      (i32.store8 offset=9142
        (i32.const 0)
        (i32.and
          (i32.load8_u offset=9146
            (i32.const 0))
          (i32.const 15))))
    (local.set 3
      (i32.const 0))
    (block  ;; label = @1
      (block  ;; label = @2
        (block  ;; label = @3
          (block  ;; label = @4
            (block  ;; label = @5
              (br_table 0 (;@5;) 2 (;@3;) 4 (;@1;) 1 (;@4;) 4 (;@1;) 0 (;@5;)
                (i32.and
                  (i32.load8_u offset=9143
                    (i32.const 0))
                  (i32.const 7))))
            (local.set 3
              (i32.const 15))
            (i32.store8 offset=9142
              (i32.const 0)
              (local.tee 5
                (i32.add
                  (i32.and
                    (i32.load8_u offset=9144
                      (i32.const 0))
                    (i32.const 15))
                  (i32.load8_u offset=9142
                    (i32.const 0)))))
            (local.set 4
              (i32.const 1))
            (br_if 2 (;@2;)
              (i32.gt_s
                (i32.extend8_s
                  (local.get 5))
                (i32.const 14)))
            (br 3 (;@1;)))
          (i32.store8 offset=9142
            (i32.const 0)
            (local.tee 4
              (i32.sub
                (i32.load8_u offset=9142
                  (i32.const 0))
                (i32.and
                  (i32.load8_u offset=9147
                    (i32.const 0))
                  (i32.const 15)))))
          (br_if 2 (;@1;)
            (i32.ge_s
              (i32.extend8_s
                (local.get 4))
              (i32.const 0)))
          (local.set 4
            (i32.const 4))
          (br 1 (;@2;)))
        (i32.store8 offset=9142
          (i32.const 0)
          (local.tee 3
            (i32.sub
              (i32.load8_u offset=9142
                (i32.const 0))
              (i32.and
                (i32.load8_u offset=9145
                  (i32.const 0))
                (i32.const 15)))))
        (local.set 4
          (i32.const 2))
        (br_if 1 (;@1;)
          (i32.gt_s
            (i32.extend8_s
              (local.get 3))
            (local.tee 3
              (i32.and
                (i32.load8_u offset=9146
                  (i32.const 0))
                (i32.const 15))))))
      (i32.store8 offset=9142
        (i32.const 0)
        (local.get 3))
      (i32.store8 offset=9143
        (i32.const 0)
        (local.get 4)))
    (call $gba_set_sound_reg
      (i32.const 67108978)
      (i32.shl
        (i32.and
          (i32.load8_u
            (i32.add
              (i32.and
                (i32.load8_u offset=9142
                  (i32.const 0))
                (i32.const 15))
              (i32.const 8539)))
          (i32.const 7))
        (i32.const 13))))
  (func $default-instruments.noise_predef.press (type 4) (param i32 i32 i32 i32)
    (local i32 i32 i32 i32)
    (local.set 4
      (i32.const -1431655766))
    (local.set 5
      (i32.const 0))
    (local.set 6
      (i32.const -1431655766))
    (local.set 7
      (i32.const 0))
    (block  ;; label = @1
      (br_if 0 (;@1;)
        (i32.gt_u
          (local.tee 1
            (i32.rem_u
              (local.get 1)
              (i32.const 12)))
          (i32.const 7)))
      (local.set 7
        (i32.load
          (i32.add
            (local.tee 4
              (i32.shl
                (local.get 1)
                (i32.const 2)))
            (i32.const 9084))))
      (local.set 6
        (i32.load
          (i32.add
            (local.get 4)
            (i32.const 9052))))
      (local.set 5
        (i32.load
          (i32.add
            (local.get 4)
            (i32.const 9020))))
      (local.set 4
        (i32.load
          (i32.add
            (local.get 4)
            (i32.const 8988)))))
    (i32.store offset=9172
      (i32.const 0)
      (local.get 5))
    (i32.store offset=9156
      (i32.const 0)
      (local.get 4))
    (i32.store offset=9160
      (i32.const 0)
      (local.get 6))
    (i32.store offset=9176
      (i32.const 0)
      (local.get 7)))
  (func $default-instruments.noise_predef.frame (type 5) (param i32 i32 i32)
    (local i32)
    (block  ;; label = @1
      (br_if 0 (;@1;)
        (i32.le_u
          (i32.load offset=9172
            (i32.const 0))
          (local.get 2)))
      (br_if 0 (;@1;)
        (i32.eqz
          (i32.load8_u offset=2
            (local.tee 3
              (i32.add
                (i32.load offset=9156
                  (i32.const 0))
                (i32.shl
                  (local.get 2)
                  (i32.const 2)))))))
      (call $gba_set_sound_reg
        (i32.const 67108984)
        (i32.load16_u
          (local.get 3))))
    (block  ;; label = @1
      (br_if 0 (;@1;)
        (i32.le_u
          (i32.load offset=9176
            (i32.const 0))
          (local.get 2)))
      (br_if 0 (;@1;)
        (i32.eqz
          (i32.load8_u offset=2
            (local.tee 2
              (i32.add
                (i32.load offset=9160
                  (i32.const 0))
                (i32.shl
                  (local.get 2)
                  (i32.const 2)))))))
      (call $gba_set_sound_reg
        (i32.const 67108988)
        (i32.load16_u
          (local.get 2)))
      (return)))
  (func $default-instruments.noise_manual.press (type 4) (param i32 i32 i32 i32)
    (call $gba_set_sound_reg
      (i32.const 67108984)
      (i32.or
        (i32.shl
          (i32.and
            (local.get 3)
            (i32.const 15))
          (i32.const 12))
        (i32.const 256)))
    (call $gba_set_sound_reg
      (i32.const 67108988)
      (i32.or
        (i32.and
          (i32.or
            (i32.or
              (i32.and
                (i32.shr_u
                  (local.get 3)
                  (i32.const 1))
                (i32.const 8))
              (i32.shl
                (local.get 2)
                (i32.const 4)))
            (i32.and
              (i32.rem_u
                (local.get 1)
                (i32.const 12))
              (i32.const 7)))
          (i32.const 255))
        (i32.const 32768))))
  (func $default-instruments.wave_p (type 4) (param i32 i32 i32 i32)
    (local i32 i32 i32 i32 i32)
    (global.set $__stack_pointer
      (local.tee 4
        (i32.sub
          (global.get $__stack_pointer)
          (i32.const 16))))
    (local.set 5
      (i32.shr_u
        (i32.and
          (local.get 2)
          (i32.const 240))
        (i32.const 4)))
    (local.set 6
      (i32.const 8984))
    (block  ;; label = @1
      (br_if 0 (;@1;)
        (i32.and
          (i32.or
            (local.tee 7
              (i32.shr_u
                (i32.and
                  (local.get 1)
                  (i32.const 240))
                (i32.const 4)))
            (local.get 1))
          (i32.const 15)))
      (i32.store8 offset=10
        (local.get 4)
        (i32.const 2))
      (i32.store8 offset=9
        (local.get 4)
        (local.get 5))
      (i32.store8 offset=8
        (local.get 4)
        (local.get 5))
      (local.set 6
        (i32.add
          (local.get 4)
          (i32.const 9))))
    (i32.store8 offset=14
      (local.get 4)
      (local.get 7))
    (i32.store8 offset=12
      (local.get 4)
      (local.get 5))
    (i32.store8 offset=13
      (local.get 4)
      (local.tee 1
        (i32.and
          (local.get 1)
          (i32.const 15))))
    (i32.store8 offset=11
      (local.get 4)
      (local.tee 2
        (i32.and
          (local.get 2)
          (i32.const 15))))
    (i32.store8 offset=15
      (local.get 4)
      (local.tee 8
        (i32.and
          (i32.load8_u offset=1
            (local.get 6))
          (i32.const 7))))
    (local.set 6
      (i32.load8_u
        (local.get 6)))
    (i32.store8 offset=9147
      (i32.const 0)
      (local.get 2))
    (i32.store8 offset=9146
      (i32.const 0)
      (local.get 5))
    (i32.store8 offset=9145
      (i32.const 0)
      (local.get 1))
    (i32.store8 offset=9144
      (i32.const 0)
      (local.get 7))
    (i32.store8 offset=9143
      (i32.const 0)
      (local.get 8))
    (i32.store8 offset=9142
      (i32.const 0)
      (i32.and
        (local.get 6)
        (i32.const 15)))
    (call $gba_set_sound_reg
      (i32.const 67108976)
      (i32.const 0))
    (call $gba_set_wave_table
      (local.get 3)
      (i32.const 16))
    (call $gba_set_sound_reg
      (i32.const 67108976)
      (i32.const 192))
    (call $gba_set_sound_reg
      (i32.const 67108978)
      (i32.const 8192))
    (call $gba_set_sound_reg
      (i32.const 67108980)
      (i32.or
        (i32.and
          (i32.sub
            (i32.const 0)
            (i32.div_u
              (i32.const 16777216)
              (local.get 0)))
          (i32.const 2047))
        (i32.const 32768)))
    (global.set $__stack_pointer
      (i32.add
        (local.get 4)
        (i32.const 16))))
  (table (;0;) 45 45 funcref)
  (memory (;0;) 1 1)
  (global $__stack_pointer (mut i32) (i32.const 8192))
  (export "memory" (memory 0))
  (export "_start" (func $_start))
  (export "__indirect_function_table" (table 0))
  (elem (;0;) (i32.const 1) func $default-instruments.square1_vibrato.press $default-instruments.square1_1_4.release $default-instruments.square1_vibrato.frame $default-instruments.square1_vibrato.set_duty $default-instruments.square1_vibrato.set_p $default-instruments.square1_bleep.press $default-instruments.square1_1_4.press $default-instruments.square1_duty.frame $default-instruments.square1_sweep.press $default-instruments.square1_base.press $default-instruments.square1_base.frame $default-instruments.square1_base.set_duty $default-instruments.square1_2_4.frame $default-instruments.square1_1_4.frame $default-instruments.square1_1_8.frame $default-instruments.square2_dyad.press $default-instruments.square2_dyad.release $default-instruments.square2_dyad.frame $default-instruments.square2_bleep.press $default-instruments.square2_arp.press $default-instruments.square2_1_4.release $default-instruments.square2_arp.frame $default-instruments.square2_pan.press $default-instruments.square2_pan.release $default-instruments.square2_pan.frame $default-instruments.square2_base.press $default-instruments.square2_base.frame $default-instruments.square2_base.set_duty $default-instruments.square2_1_4.press $default-instruments.square2_2_4.frame $default-instruments.square2_1_4.frame $default-instruments.square2_1_8.frame $default-instruments.wave_triangle.press $default-instruments.wave_env_r $default-instruments.wave_env_f $default-instruments.wave_bass.press $default-instruments.wave_arp.press $default-instruments.wave_arp.frame $default-instruments.wave_duty.press $default-instruments.wave_sweep.press $default-instruments.wave_sweep.frame $default-instruments.noise_predef.press $default-instruments.noise_predef.frame $default-instruments.noise_manual.press)
  (data $.rodata (i32.const 8192) "Duty\00VP Vibrato Period\00SW\00SV\00WT\00WS\00SR\00TP\00NP\00NM\00WD\00TD\00SD\00AD\00WB\00TB\00SB\00WA\00TA\00T8\00S8\00T4\00S4\00T2\00S2\00T0\00S0\00Sweep (semitones)\00Detune (semitones)\00A2 Arp 2. (semitones)\00A1 Arp 1. (semitones)\00FD (Freq divider)\00WV (Width / Vol)\00RP (right pan period)\00LP (left pan period)\00\01\02\03\02\00\00\08\05\0a\03\01#Eg\89\ab\cd\ef\fe\dc\ba\98vT2\10\11#Vx\99\98vg\9a\df\fe\c9\85B\111\de\dc\ba\98vT2\10\00\00\00\00\11\11\11\11\f0\f0\f0\f0\f0\f0\f0\f0\ff\00\ff\00\ff\00\ff\00\024g\9a\cd\ff\ff\ee\ee\ff\ff\dc\a9vC\10\00\03\03\03\03\03\02\02\02\02\02\05\05\05\05\01\00\00q\01\00\10\80\01\00\00\a1\01\00y\80\01\00i\00\01\00Y\00\01\00Q\00\01\00\00r\01\00\11\80\01\00\12\00\01\00\13\00\01\00\14\00\01\00\15\00\01\00Y\80\01\00[\00\01\00[\00\01\00]\00\01\00_\00\01\00`\00\01\00\00\a2\01\00Y\80\01\00y\00\01\00i\00\01\00P\00\01\00P\00\01\00Q\00\01\00A\00\01\00Q\00\01\00\00\90\01\00\00\80\01\00\000\01\00\004\01\00\00\00\00\00\00\00\00\00\00\00\00\00\00`\01\00\00@\01\00\00 \01\00\00\03\01\00\04\80\01\00\02\80\01\00\06\80\01\00\03\80\01\00\00\00\00\00\00\00\00\00\00\00\00\00\04\80\01\00\02\80\01\00\01\80\01\00\01\80\01\00\00\d0\01\00\00\d0\01\00\00\b0\01\00\00p\01\00\00P\01\00\000\01\00\00!\01\00\00`\01\00\00@\01\00\00 \01\00\00\03\01\00\02\80\01\00Q\80\01\00a\80\01\00q\80\01\00\91\80\01\00q\80\01\00`\80\01\00\04\80\01\00\02\80\01\00\01\80\01\00\01\80\01\00\00\d0\01\00\00\d0\01\00\00\d0\01\00\00\80\01\00\00\10\01\00\00 \01\00\003\01\00\00`\01\00\00@\01\00\00 \01\00\00\03\01\00`\80\01\00R\80\01\00B\80\01\00A\80\01\00\22\80\01\00\11\80\01\00\04\80\01\00\04\80\01\00\02\80\01\00\01\80\01\00\01\80\01\00\01\00\01\00k\00e\007\001\00,\00%\00\a0\00\7f\00\e3\00\aa\00\ef\00\a9\00\fd\00\a9\00\e3\00\8f\00%\00\16\00b\007\00\b9\00b\00\02\00\01\00\00\00\00\00l!\00\00t!\00\00\88!\00\00t!\00\00\b8!\00\00\dc!\00\004\22\00\00\8c\22\00\00\01\00\00\00\01\00\00\00\01\00\00\00\01\00\00\00\01\00\00\00\0b\00\00\00\0b\00\00\00\0b\00\00\00p!\00\00x!\00\00\8c!\00\00\a0!\00\00\bc!\00\00\08\22\00\00`\22\00\00\b8\22\00\00\01\00\00\00\04\00\00\00\05\00\00\00\06\00\00\00\08\00\00\00\0b\00\00\00\0b\00\00\00\0b\00\00\00")
  (data $.data (i32.const 9116) "\00\02\00\00\00\00@\00\08\00\80\00\00\02\00\00\00\00\00\04\07\0c\ff\ff\80\00\00\02\00\00\00\00\00\04\07\0c\04\00\00\00\aa\aa\aa\aa\aa\aa\aa\aa"))
