(module
  (type (;0;) (func (param i32 i32 i32 i32 i32 i32 i32)))
  (type (;1;) (func (param i32 i32)))
  (type (;2;) (func))
  (type (;3;) (func (param i32 i32 i32 i32)))
  (type (;4;) (func (param i32 i32 i32)))
  (import "env" "set_instrument_at_column" (func $set_instrument_at_column (type 0)))
  (import "env" "gba_set_sound_reg" (func $gba_set_sound_reg (type 1)))
  (import "env" "gba_set_wave_table" (func $gba_set_wave_table (type 1)))
  (func $_start (type 2)
    (call $set_instrument_at_column
      (i32.const 8234)
      (i32.const 0)
      (i32.const 0)
      (i32.const 1)
      (i32.const 2)
      (i32.const 0)
      (i32.const 0))
    (call $set_instrument_at_column
      (i32.const 8222)
      (i32.const 0)
      (i32.const 4)
      (i32.const 3)
      (i32.const 4)
      (i32.const 5)
      (i32.const 6))
    (call $set_instrument_at_column
      (i32.const 8213)
      (i32.const 0)
      (i32.const 0)
      (i32.const 7)
      (i32.const 0)
      (i32.const 0)
      (i32.const 0))
    (call $set_instrument_at_column
      (i32.const 8204)
      (i32.const 0)
      (i32.const 4)
      (i32.const 8)
      (i32.const 4)
      (i32.const 9)
      (i32.const 0))
    (call $set_instrument_at_column
      (i32.const 8195)
      (i32.const 0)
      (i32.const 0)
      (i32.const 10)
      (i32.const 0)
      (i32.const 0)
      (i32.const 0))
    (call $set_instrument_at_column
      (i32.const 8231)
      (i32.const 1)
      (i32.const 13)
      (i32.const 11)
      (i32.const 12)
      (i32.const 13)
      (i32.const 0))
    (call $set_instrument_at_column
      (i32.const 8219)
      (i32.const 1)
      (i32.const 24)
      (i32.const 14)
      (i32.const 15)
      (i32.const 16)
      (i32.const 0))
    (call $set_instrument_at_column
      (i32.const 8210)
      (i32.const 1)
      (i32.const 0)
      (i32.const 17)
      (i32.const 18)
      (i32.const 19)
      (i32.const 0))
    (call $set_instrument_at_column
      (i32.const 8201)
      (i32.const 1)
      (i32.const 4)
      (i32.const 20)
      (i32.const 15)
      (i32.const 21)
      (i32.const 0))
    (call $set_instrument_at_column
      (i32.const 8228)
      (i32.const 2)
      (i32.const 4)
      (i32.const 22)
      (i32.const 23)
      (i32.const 24)
      (i32.const 0))
    (call $set_instrument_at_column
      (i32.const 8216)
      (i32.const 2)
      (i32.const 4)
      (i32.const 25)
      (i32.const 23)
      (i32.const 24)
      (i32.const 0))
    (call $set_instrument_at_column
      (i32.const 8207)
      (i32.const 2)
      (i32.const 4)
      (i32.const 26)
      (i32.const 23)
      (i32.const 27)
      (i32.const 0))
    (call $set_instrument_at_column
      (i32.const 8198)
      (i32.const 2)
      (i32.const 4)
      (i32.const 28)
      (i32.const 23)
      (i32.const 24)
      (i32.const 0))
    (call $set_instrument_at_column
      (i32.const 8192)
      (i32.const 2)
      (i32.const 16)
      (i32.const 29)
      (i32.const 0)
      (i32.const 30)
      (i32.const 0))
    (call $set_instrument_at_column
      (i32.const 8237)
      (i32.const 3)
      (i32.const 15)
      (i32.const 31)
      (i32.const 0)
      (i32.const 32)
      (i32.const 0))
    (call $set_instrument_at_column
      (i32.const 8225)
      (i32.const 3)
      (i32.const 0)
      (i32.const 33)
      (i32.const 0)
      (i32.const 0)
      (i32.const 0)))
  (func $default-instruments.square1_1.press (type 3) (param i32 i32 i32 i32)
    (call $gba_set_sound_reg
      (i32.const 67108960)
      (i32.const 8))
    (call $gba_set_sound_reg
      (i32.const 67108962)
      (i32.const 41088))
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
  (func $default-instruments.square1_1.release (type 4) (param i32 i32 i32)
    (call $gba_set_sound_reg
      (i32.const 67108962)
      (i32.const 41344))
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
  (func $default-instruments.square1_2.press (type 3) (param i32 i32 i32 i32)
    (i32.store16 offset=8898
      (i32.const 0)
      (select
        (local.get 3)
        (i32.const 12)
        (local.get 3)))
    (i32.store offset=8936 align=1
      (i32.const 0)
      (i32.load offset=8244 align=1
        (i32.const 0)))
    (i32.store16 offset=8940 align=1
      (i32.const 0)
      (i32.load16_u offset=8248 align=1
        (i32.const 0)))
    (i32.store16 offset=8896
      (i32.const 0)
      (i32.or
        (i32.and
          (i32.load16_u offset=8896
            (i32.const 0))
          (i32.const 65343))
        (i32.and
          (i32.shl
            (local.get 2)
            (i32.const 6))
          (i32.const 192))))
    (call $gba_set_sound_reg
      (i32.const 67108960)
      (i32.const 8)))
  (func $default-instruments.square1_2.release (type 4) (param i32 i32 i32)
    (i32.store8 offset=8937
      (i32.const 0)
      (i32.const 3))
    (i32.store8 offset=8936
      (i32.const 0)
      (i32.load8_u offset=8940
        (i32.const 0))))
  (func $default-instruments.square1_2.frame (type 4) (param i32 i32 i32)
    (local i32 i32 i32)
    (local.set 3
      (i32.load16_u offset=8896
        (i32.const 0)))
    (block  ;; label = @1
      (block  ;; label = @2
        (block  ;; label = @3
          (block  ;; label = @4
            (block  ;; label = @5
              (br_table 1 (;@4;) 2 (;@3;) 0 (;@5;) 3 (;@2;) 1 (;@4;)
                (i32.load8_u offset=8937
                  (i32.const 0))))
            (local.set 4
              (i32.load8_u offset=8936
                (i32.const 0)))
            (br 3 (;@1;)))
          (i32.store8 offset=8936
            (i32.const 0)
            (local.tee 4
              (i32.add
                (i32.load8_u offset=8938
                  (i32.const 0))
                (i32.load8_u offset=8936
                  (i32.const 0)))))
          (br_if 2 (;@1;)
            (i32.lt_s
              (i32.extend8_s
                (local.get 4))
              (i32.const 15)))
          (i32.store8 offset=8936
            (i32.const 0)
            (i32.const 15))
          (i32.store8 offset=8937
            (i32.const 0)
            (i32.const 1))
          (local.set 4
            (i32.const 15))
          (br 2 (;@1;)))
        (i32.store8 offset=8936
          (i32.const 0)
          (local.tee 4
            (i32.sub
              (i32.load8_u offset=8936
                (i32.const 0))
              (i32.load8_u offset=8939
                (i32.const 0)))))
        (br_if 1 (;@1;)
          (i32.gt_s
            (i32.extend8_s
              (local.get 4))
            (local.tee 5
              (i32.load8_s offset=8940
                (i32.const 0)))))
        (i32.store8 offset=8936
          (i32.const 0)
          (local.get 5))
        (i32.store8 offset=8937
          (i32.const 0)
          (i32.const 2))
        (local.set 4
          (local.get 5))
        (br 1 (;@1;)))
      (i32.store8 offset=8936
        (i32.const 0)
        (local.tee 4
          (i32.sub
            (i32.load8_u offset=8936
              (i32.const 0))
            (i32.load8_u offset=8941
              (i32.const 0)))))
      (br_if 0 (;@1;)
        (i32.gt_s
          (i32.extend8_s
            (local.get 4))
          (i32.const -1)))
      (local.set 4
        (i32.const 0))
      (i32.store8 offset=8937
        (i32.const 0)
        (i32.const 2))
      (i32.store8 offset=8936
        (i32.const 0)
        (i32.const 0)))
    (call $gba_set_sound_reg
      (i32.const 67108962)
      (i32.and
        (i32.or
          (i32.shl
            (local.get 4)
            (i32.const 12))
          (i32.and
            (local.get 3)
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
                                                (i32.load16_u offset=8898
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
  (func $default-instruments.square1_2.set_param (type 1) (param i32 i32)
    (block  ;; label = @1
      (br_if 0 (;@1;)
        (local.get 0))
      (i32.store16 offset=8896
        (i32.const 0)
        (i32.or
          (i32.and
            (i32.load16_u offset=8896
              (i32.const 0))
            (i32.const 65343))
          (i32.and
            (i32.shl
              (local.get 1)
              (i32.const 6))
            (i32.const 192))))
      (return))
    (i32.store16 offset=8898
      (i32.const 0)
      (select
        (local.get 1)
        (i32.const 12)
        (local.get 1))))
  (func $default-instruments.square1_3.press (type 3) (param i32 i32 i32 i32)
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
        (i32.const 41264)))
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
  (func $default-instruments.square1_4.press (type 3) (param i32 i32 i32 i32)
    (i32.store16 offset=8940 align=1
      (i32.const 0)
      (i32.load16_u offset=8248 align=1
        (i32.const 0)))
    (i32.store offset=8936 align=1
      (i32.const 0)
      (i32.load offset=8244 align=1
        (i32.const 0)))
    (call $gba_set_sound_reg
      (i32.const 67108960)
      (i32.const 8)))
  (func $default-instruments.square1_4.frame (type 4) (param i32 i32 i32)
    (local i32 i32)
    (local.set 3
      (i32.load8_u
        (i32.add
          (i32.and
            (i32.shr_u
              (local.get 2)
              (i32.const 1))
            (i32.const 3))
          (i32.const 8240))))
    (block  ;; label = @1
      (block  ;; label = @2
        (block  ;; label = @3
          (block  ;; label = @4
            (block  ;; label = @5
              (br_table 1 (;@4;) 2 (;@3;) 0 (;@5;) 3 (;@2;) 1 (;@4;)
                (i32.load8_u offset=8937
                  (i32.const 0))))
            (local.set 2
              (i32.load8_u offset=8936
                (i32.const 0)))
            (br 3 (;@1;)))
          (i32.store8 offset=8936
            (i32.const 0)
            (local.tee 2
              (i32.add
                (i32.load8_u offset=8938
                  (i32.const 0))
                (i32.load8_u offset=8936
                  (i32.const 0)))))
          (br_if 2 (;@1;)
            (i32.lt_s
              (local.tee 2
                (i32.extend8_s
                  (local.get 2)))
              (i32.const 15)))
          (i32.store8 offset=8936
            (i32.const 0)
            (i32.const 15))
          (i32.store8 offset=8937
            (i32.const 0)
            (i32.const 1))
          (local.set 2
            (i32.const 15))
          (br 2 (;@1;)))
        (i32.store8 offset=8936
          (i32.const 0)
          (local.tee 2
            (i32.sub
              (i32.load8_u offset=8936
                (i32.const 0))
              (i32.load8_u offset=8939
                (i32.const 0)))))
        (br_if 1 (;@1;)
          (i32.gt_s
            (local.tee 2
              (i32.extend8_s
                (local.get 2)))
            (local.tee 4
              (i32.load8_s offset=8940
                (i32.const 0)))))
        (i32.store8 offset=8936
          (i32.const 0)
          (local.get 4))
        (i32.store8 offset=8937
          (i32.const 0)
          (i32.const 2))
        (local.set 2
          (local.get 4))
        (br 1 (;@1;)))
      (i32.store8 offset=8936
        (i32.const 0)
        (local.tee 2
          (i32.sub
            (i32.load8_u offset=8936
              (i32.const 0))
            (i32.load8_u offset=8941
              (i32.const 0)))))
      (br_if 0 (;@1;)
        (i32.gt_s
          (local.tee 2
            (i32.extend8_s
              (local.get 2)))
          (i32.const -1)))
      (local.set 2
        (i32.const 0))
      (i32.store8 offset=8937
        (i32.const 0)
        (i32.const 2))
      (i32.store8 offset=8936
        (i32.const 0)
        (i32.const 0)))
    (call $gba_set_sound_reg
      (i32.const 67108962)
      (i32.or
        (i32.shl
          (i32.and
            (local.get 2)
            (i32.const 15))
          (i32.const 12))
        (i32.shl
          (i32.and
            (local.get 3)
            (i32.const 3))
          (i32.const 6))))
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
  (func $default-instruments.square1_5.press (type 3) (param i32 i32 i32 i32)
    (call $gba_set_sound_reg
      (i32.const 67108960)
      (i32.const 42))
    (call $gba_set_sound_reg
      (i32.const 67108962)
      (i32.const 53888))
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
  (func $default-instruments.square2_1.press (type 3) (param i32 i32 i32 i32)
    (local i32 i32 i32)
    (i32.store8 offset=8942
      (i32.const 0)
      (select
        (local.get 2)
        (i32.const 4)
        (local.get 2)))
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
                                              (i32.load8_s offset=8942
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
                                (i32.const 8714))))
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
                (select
                  (i32.and
                    (local.get 4)
                    (i32.const 65535))
                  (local.get 6)
                  (local.get 2)))))
          (i32.const 2047))
        (i32.const 32768))))
  (func $default-instruments.square2_1.release (type 4) (param i32 i32 i32)
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
                                              (i32.load8_s offset=8942
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
                                (i32.const 8714))))
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
                (select
                  (i32.and
                    (local.get 4)
                    (i32.const 65535))
                  (local.get 6)
                  (local.get 3)))))
          (i32.const 2047))
        (i32.const 32768))))
  (func $default-instruments.square2_1.frame (type 4) (param i32 i32 i32)
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
                                                      (i32.load8_s offset=8942
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
                                        (i32.const 8714))))
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
                        (select
                          (i32.and
                            (local.get 4)
                            (i32.const 65535))
                          (local.get 6)
                          (local.get 3))))
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
  (func $default-instruments.square2_2.press (type 3) (param i32 i32 i32 i32)
    (i32.store16 offset=8947 align=1
      (i32.const 0)
      (i32.load16_u offset=8248 align=1
        (i32.const 0)))
    (i32.store offset=8943 align=1
      (i32.const 0)
      (i32.load offset=8244 align=1
        (i32.const 0)))
    (i32.store8 offset=8902
      (i32.const 0)
      (select
        (local.get 3)
        (i32.const 7)
        (local.get 3)))
    (i32.store8 offset=8901
      (i32.const 0)
      (select
        (local.get 2)
        (i32.const 4)
        (local.get 2))))
  (func $default-instruments.square2_2.release (type 4) (param i32 i32 i32)
    (i32.store8 offset=8944
      (i32.const 0)
      (i32.const 3))
    (i32.store8 offset=8943
      (i32.const 0)
      (i32.load8_u offset=8947
        (i32.const 0))))
  (func $default-instruments.square2_2.frame (type 4) (param i32 i32 i32)
    (local i32 i32 i32)
    (block  ;; label = @1
      (block  ;; label = @2
        (block  ;; label = @3
          (block  ;; label = @4
            (block  ;; label = @5
              (br_table 1 (;@4;) 2 (;@3;) 0 (;@5;) 3 (;@2;) 1 (;@4;)
                (i32.load8_u offset=8944
                  (i32.const 0))))
            (local.set 3
              (i32.load8_u offset=8943
                (i32.const 0)))
            (br 3 (;@1;)))
          (i32.store8 offset=8943
            (i32.const 0)
            (local.tee 3
              (i32.add
                (i32.load8_u offset=8945
                  (i32.const 0))
                (i32.load8_u offset=8943
                  (i32.const 0)))))
          (br_if 2 (;@1;)
            (i32.lt_s
              (local.tee 3
                (i32.extend8_s
                  (local.get 3)))
              (i32.const 15)))
          (i32.store8 offset=8943
            (i32.const 0)
            (i32.const 15))
          (i32.store8 offset=8944
            (i32.const 0)
            (i32.const 1))
          (local.set 3
            (i32.const 15))
          (br 2 (;@1;)))
        (i32.store8 offset=8943
          (i32.const 0)
          (local.tee 3
            (i32.sub
              (i32.load8_u offset=8943
                (i32.const 0))
              (i32.load8_u offset=8946
                (i32.const 0)))))
        (br_if 1 (;@1;)
          (i32.gt_s
            (local.tee 3
              (i32.extend8_s
                (local.get 3)))
            (local.tee 4
              (i32.load8_s offset=8947
                (i32.const 0)))))
        (i32.store8 offset=8943
          (i32.const 0)
          (local.get 4))
        (i32.store8 offset=8944
          (i32.const 0)
          (i32.const 2))
        (local.set 3
          (local.get 4))
        (br 1 (;@1;)))
      (i32.store8 offset=8943
        (i32.const 0)
        (local.tee 3
          (i32.sub
            (i32.load8_u offset=8943
              (i32.const 0))
            (i32.load8_u offset=8948
              (i32.const 0)))))
      (br_if 0 (;@1;)
        (i32.gt_s
          (local.tee 3
            (i32.extend8_s
              (local.get 3)))
          (i32.const -1)))
      (local.set 3
        (i32.const 0))
      (i32.store8 offset=8944
        (i32.const 0)
        (i32.const 2))
      (i32.store8 offset=8943
        (i32.const 0)
        (i32.const 0)))
    (call $gba_set_sound_reg
      (i32.const 67108968)
      (i32.or
        (i32.shl
          (i32.and
            (local.get 3)
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
                                      (local.tee 2
                                        (i32.sub
                                          (i32.xor
                                            (local.tee 3
                                              (i32.load8_s
                                                (i32.add
                                                  (i32.and
                                                    (local.get 2)
                                                    (i32.const 3))
                                                  (i32.const 8900))))
                                            (local.tee 2
                                              (i32.shr_s
                                                (i32.extend8_s
                                                  (local.get 3))
                                                (i32.const 7))))
                                          (local.get 2)))
                                      (i32.mul
                                        (local.tee 2
                                          (i32.div_u
                                            (i32.and
                                              (local.get 2)
                                              (i32.const 255))
                                            (i32.const 12)))
                                        (i32.const 12)))
                                    (i32.const 255))
                                  (i32.const 2))
                                (i32.const 8714))))
                          (i32.const 16)))
                      (local.tee 2
                        (i32.shl
                          (local.get 4)
                          (local.get 2)))
                      (local.tee 3
                        (i32.lt_s
                          (local.get 3)
                          (i32.const 0))))
                    (i32.const 65535))
                  (local.get 0))
                (select
                  (i32.and
                    (local.get 2)
                    (i32.const 65535))
                  (local.get 5)
                  (local.get 3)))))
          (i32.const 2047))
        (i32.const 32768))))
  (func $default-instruments.square2_3.press (type 3) (param i32 i32 i32 i32)
    (i32.store8 offset=8950
      (i32.const 0)
      (i32.and
        (select
          (local.get 3)
          (i32.const 5)
          (local.get 3))
        (i32.const 127)))
    (i32.store8 offset=8949
      (i32.const 0)
      (i32.and
        (select
          (local.get 2)
          (i32.const 4)
          (local.get 2))
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
  (func $default-instruments.square2_3.release (type 4) (param i32 i32 i32)
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
    (i32.store16 offset=8904
      (i32.const 0)
      (local.tee 0
        (i32.or
          (i32.load16_u offset=8904
            (i32.const 0))
          (i32.const 8704))))
    (call $gba_set_sound_reg
      (i32.const 67108992)
      (local.get 0)))
  (func $default-instruments.square2_3.frame (type 4) (param i32 i32 i32)
    (local i32)
    (block  ;; label = @1
      (br_if 0 (;@1;)
        (i32.rem_u
          (local.tee 3
            (i32.and
              (local.get 2)
              (i32.const 127)))
          (i32.load8_u offset=8949
            (i32.const 0))))
      (i32.store16 offset=8904
        (i32.const 0)
        (i32.xor
          (i32.load16_u offset=8904
            (i32.const 0))
          (i32.const 8192))))
    (local.set 2
      (i32.load16_u offset=8904
        (i32.const 0)))
    (block  ;; label = @1
      (br_if 0 (;@1;)
        (i32.rem_u
          (local.get 3)
          (i32.load8_u offset=8950
            (i32.const 0))))
      (i32.store16 offset=8904
        (i32.const 0)
        (local.tee 2
          (i32.xor
            (local.get 2)
            (i32.const 512)))))
    (call $gba_set_sound_reg
      (i32.const 67108992)
      (i32.and
        (local.get 2)
        (i32.const 65535))))
  (func $default-instruments.square2_4.press (type 3) (param i32 i32 i32 i32)
    (i32.store16 offset=8947 align=1
      (i32.const 0)
      (i32.load16_u offset=8248 align=1
        (i32.const 0)))
    (i32.store offset=8943 align=1
      (i32.const 0)
      (i32.load offset=8244 align=1
        (i32.const 0))))
  (func $default-instruments.square2_4.frame (type 4) (param i32 i32 i32)
    (local i32 i32)
    (block  ;; label = @1
      (block  ;; label = @2
        (block  ;; label = @3
          (block  ;; label = @4
            (block  ;; label = @5
              (br_table 1 (;@4;) 2 (;@3;) 0 (;@5;) 3 (;@2;) 1 (;@4;)
                (i32.load8_u offset=8944
                  (i32.const 0))))
            (local.set 3
              (i32.load8_u offset=8943
                (i32.const 0)))
            (br 3 (;@1;)))
          (i32.store8 offset=8943
            (i32.const 0)
            (local.tee 3
              (i32.add
                (i32.load8_u offset=8945
                  (i32.const 0))
                (i32.load8_u offset=8943
                  (i32.const 0)))))
          (br_if 2 (;@1;)
            (i32.lt_s
              (local.tee 3
                (i32.extend8_s
                  (local.get 3)))
              (i32.const 15)))
          (i32.store8 offset=8943
            (i32.const 0)
            (i32.const 15))
          (i32.store8 offset=8944
            (i32.const 0)
            (i32.const 1))
          (local.set 3
            (i32.const 15))
          (br 2 (;@1;)))
        (i32.store8 offset=8943
          (i32.const 0)
          (local.tee 3
            (i32.sub
              (i32.load8_u offset=8943
                (i32.const 0))
              (i32.load8_u offset=8946
                (i32.const 0)))))
        (br_if 1 (;@1;)
          (i32.gt_s
            (local.tee 3
              (i32.extend8_s
                (local.get 3)))
            (local.tee 4
              (i32.load8_s offset=8947
                (i32.const 0)))))
        (i32.store8 offset=8943
          (i32.const 0)
          (local.get 4))
        (i32.store8 offset=8944
          (i32.const 0)
          (i32.const 2))
        (local.set 3
          (local.get 4))
        (br 1 (;@1;)))
      (i32.store8 offset=8943
        (i32.const 0)
        (local.tee 3
          (i32.sub
            (i32.load8_u offset=8943
              (i32.const 0))
            (i32.load8_u offset=8948
              (i32.const 0)))))
      (br_if 0 (;@1;)
        (i32.gt_s
          (local.tee 3
            (i32.extend8_s
              (local.get 3)))
          (i32.const -1)))
      (local.set 3
        (i32.const 0))
      (i32.store8 offset=8944
        (i32.const 0)
        (i32.const 2))
      (i32.store8 offset=8943
        (i32.const 0)
        (i32.const 0)))
    (call $gba_set_sound_reg
      (i32.const 67108968)
      (i32.or
        (i32.shl
          (i32.and
            (local.get 3)
            (i32.const 15))
          (i32.const 12))
        (i32.const 192)))
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
  (func $default-instruments.wave_1.press (type 3) (param i32 i32 i32 i32)
    (local i32)
    (call $gba_set_wave_table
      (i32.const 8250)
      (i32.const 16))
    (i32.store8 offset=8980
      (i32.const 0)
      (i32.xor
        (local.tee 4
          (i32.load8_u offset=8980
            (i32.const 0)))
        (i32.const 1)))
    (call $gba_set_sound_reg
      (i32.const 67108976)
      (select
        (i32.const 128)
        (i32.const 192)
        (local.get 4)))
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
    (i64.store offset=8952
      (i32.const 0)
      (i64.const 0)))
  (func $default-instruments.wave_env_r (type 4) (param i32 i32 i32)
    (i32.store8 offset=8956
      (i32.const 0)
      (i32.const 1))
    (i32.store offset=8952
      (i32.const 0)
      (local.get 2)))
  (func $default-instruments.wave_env_f (type 4) (param i32 i32 i32)
    (block  ;; label = @1
      (br_if 0 (;@1;)
        (i32.eqz
          (i32.load8_u offset=8956
            (i32.const 0))))
      (br_if 0 (;@1;)
        (i32.gt_u
          (local.tee 2
            (i32.sub
              (local.get 2)
              (i32.load offset=8952
                (i32.const 0))))
          (i32.const 3)))
      (call $gba_set_sound_reg
        (i32.const 67108978)
        (i32.load16_u
          (i32.add
            (i32.shl
              (local.get 2)
              (i32.const 1))
            (i32.const 8266))))))
  (func $default-instruments.wave_2.press (type 3) (param i32 i32 i32 i32)
    (local i32)
    (call $gba_set_wave_table
      (i32.const 8274)
      (i32.const 16))
    (i32.store8 offset=8980
      (i32.const 0)
      (i32.xor
        (local.tee 4
          (i32.load8_u offset=8980
            (i32.const 0)))
        (i32.const 1)))
    (call $gba_set_sound_reg
      (i32.const 67108976)
      (select
        (i32.const 128)
        (i32.const 192)
        (local.get 4)))
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
    (i64.store offset=8952
      (i32.const 0)
      (i64.const 0)))
  (func $default-instruments.wave_3.press (type 3) (param i32 i32 i32 i32)
    (local i32)
    (call $gba_set_wave_table
      (i32.const 8290)
      (i32.const 16))
    (i32.store8 offset=8980
      (i32.const 0)
      (i32.xor
        (local.tee 4
          (i32.load8_u offset=8980
            (i32.const 0)))
        (i32.const 1)))
    (call $gba_set_sound_reg
      (i32.const 67108976)
      (select
        (i32.const 128)
        (i32.const 192)
        (local.get 4)))
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
    (i32.store8 offset=8908
      (i32.const 0)
      (select
        (local.get 3)
        (i32.const 7)
        (local.get 3)))
    (i32.store8 offset=8907
      (i32.const 0)
      (select
        (local.get 2)
        (i32.const 4)
        (local.get 2)))
    (i64.store offset=8952
      (i32.const 0)
      (i64.const 0)))
  (func $default-instruments.wave_3.frame (type 4) (param i32 i32 i32)
    (local i32 i32 i32 i32)
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
                                            (i32.load8_s
                                              (i32.add
                                                (i32.and
                                                  (local.get 2)
                                                  (i32.const 3))
                                                (i32.const 8906))))
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
                              (i32.const 8714))))
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
              (select
                (i32.and
                  (local.get 4)
                  (i32.const 65535))
                (local.get 6)
                (local.get 3)))))
        (i32.const 2047)))
    (block  ;; label = @1
      (br_if 0 (;@1;)
        (i32.eqz
          (i32.load8_u offset=8956
            (i32.const 0))))
      (br_if 0 (;@1;)
        (i32.gt_u
          (local.tee 3
            (i32.sub
              (local.get 2)
              (i32.load offset=8952
                (i32.const 0))))
          (i32.const 3)))
      (call $gba_set_sound_reg
        (i32.const 67108978)
        (i32.load16_u
          (i32.add
            (i32.shl
              (local.get 3)
              (i32.const 1))
            (i32.const 8266))))))
  (func $default-instruments.wave_4.press (type 3) (param i32 i32 i32 i32)
    (local i32)
    (call $gba_set_wave_table
      (i32.const 8306)
      (i32.const 16))
    (i32.store8 offset=8980
      (i32.const 0)
      (i32.xor
        (local.tee 4
          (i32.load8_u offset=8980
            (i32.const 0)))
        (i32.const 1)))
    (call $gba_set_sound_reg
      (i32.const 67108976)
      (select
        (i32.const 128)
        (i32.const 192)
        (local.get 4)))
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
    (i64.store offset=8952
      (i32.const 0)
      (i64.const 0)))
  (func $default-instruments.wave_5.press (type 3) (param i32 i32 i32 i32)
    (local i32)
    (call $gba_set_wave_table
      (i32.const 8322)
      (i32.const 16))
    (i32.store8 offset=8980
      (i32.const 0)
      (i32.xor
        (local.tee 4
          (i32.load8_u offset=8980
            (i32.const 0)))
        (i32.const 1)))
    (call $gba_set_sound_reg
      (i32.const 67108976)
      (select
        (i32.const 128)
        (i32.const 192)
        (local.get 4)))
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
    (i64.store offset=8952
      (i32.const 0)
      (i64.const 4294967308))
    (block  ;; label = @1
      (block  ;; label = @2
        (br_if 0 (;@2;)
          (local.get 2))
        (local.set 2
          (i32.const 4))
        (br 1 (;@1;)))
      (local.set 2
        (i32.rem_u
          (local.get 2)
          (i32.const 12))))
    (i32.store offset=8960
      (i32.const 0)
      (local.get 0))
    (i32.store offset=8912
      (i32.const 0)
      (local.get 2)))
  (func $default-instruments.wave_5.frame (type 4) (param i32 i32 i32)
    (local i32 i32)
    (i32.store offset=8960
      (i32.const 0)
      (i32.div_u
        (i32.mul
          (local.tee 3
            (i32.load offset=8960
              (i32.const 0)))
          (i32.load16_u
            (i32.add
              (local.tee 4
                (i32.shl
                  (i32.load offset=8912
                    (i32.const 0))
                  (i32.const 2)))
              (i32.const 8714))))
        (i32.load16_u
          (i32.add
            (local.get 4)
            (i32.const 8716)))))
    (call $gba_set_sound_reg
      (i32.const 67108980)
      (i32.and
        (i32.sub
          (i32.const 0)
          (i32.div_u
            (i32.const 16777216)
            (local.get 3)))
        (i32.const 2047)))
    (block  ;; label = @1
      (br_if 0 (;@1;)
        (i32.eqz
          (i32.load8_u offset=8956
            (i32.const 0))))
      (br_if 0 (;@1;)
        (i32.gt_u
          (local.tee 3
            (i32.sub
              (local.get 2)
              (i32.load offset=8952
                (i32.const 0))))
          (i32.const 3)))
      (call $gba_set_sound_reg
        (i32.const 67108978)
        (i32.load16_u
          (i32.add
            (i32.shl
              (local.get 3)
              (i32.const 1))
            (i32.const 8266))))))
  (func $default-instruments.noise_1.press (type 3) (param i32 i32 i32 i32)
    (local i32 i32 i32 i32)
    (local.set 4
      (i32.const 2))
    (local.set 5
      (i32.const 0))
    (local.set 6
      (i32.const 2))
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
            (i32.const 8864))))
      (local.set 6
        (i32.load
          (i32.add
            (local.get 4)
            (i32.const 8832))))
      (local.set 5
        (i32.load
          (i32.add
            (local.get 4)
            (i32.const 8800))))
      (local.set 4
        (i32.load
          (i32.add
            (local.get 4)
            (i32.const 8768)))))
    (i32.store offset=8968
      (i32.const 0)
      (local.get 5))
    (i32.store offset=8920
      (i32.const 0)
      (local.get 4))
    (i32.store offset=8928
      (i32.const 0)
      (local.get 6))
    (i32.store offset=8976
      (i32.const 0)
      (local.get 7)))
  (func $default-instruments.noise_1.frame (type 4) (param i32 i32 i32)
    (local i32)
    (block  ;; label = @1
      (br_if 0 (;@1;)
        (i32.le_u
          (i32.load offset=8968
            (i32.const 0))
          (local.get 2)))
      (br_if 0 (;@1;)
        (i32.eqz
          (i32.load8_u offset=2
            (local.tee 3
              (i32.add
                (i32.load offset=8920
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
          (i32.load offset=8976
            (i32.const 0))
          (local.get 2)))
      (br_if 0 (;@1;)
        (i32.eqz
          (i32.load8_u offset=2
            (local.tee 2
              (i32.add
                (i32.load offset=8928
                  (i32.const 0))
                (i32.shl
                  (local.get 2)
                  (i32.const 2)))))))
      (call $gba_set_sound_reg
        (i32.const 67108988)
        (i32.load16_u
          (local.get 2)))
      (return)))
  (func $default-instruments.noise_2.press (type 3) (param i32 i32 i32 i32)
    (call $gba_set_sound_reg
      (i32.const 67108984)
      (i32.const 61696))
    (call $gba_set_sound_reg
      (i32.const 67108988)
      (i32.or
        (i32.and
          (i32.sub
            (i32.const 0)
            (i32.div_u
              (i32.const 33554432)
              (local.get 0)))
          (i32.const 247))
        (i32.const 32768))))
  (table (;0;) 34 34 funcref)
  (memory (;0;) 1 1)
  (global $__stack_pointer (mut i32) (i32.const 8192))
  (export "memory" (memory 0))
  (export "_start" (func $_start))
  (export "__indirect_function_table" (table 0))
  (elem (;0;) (i32.const 1) func $default-instruments.square1_1.press $default-instruments.square1_1.release $default-instruments.square1_2.press $default-instruments.square1_2.release $default-instruments.square1_2.frame $default-instruments.square1_2.set_param $default-instruments.square1_3.press $default-instruments.square1_4.press $default-instruments.square1_4.frame $default-instruments.square1_5.press $default-instruments.square2_1.press $default-instruments.square2_1.release $default-instruments.square2_1.frame $default-instruments.square2_2.press $default-instruments.square2_2.release $default-instruments.square2_2.frame $default-instruments.square2_3.press $default-instruments.square2_3.release $default-instruments.square2_3.frame $default-instruments.square2_4.press $default-instruments.square2_4.frame $default-instruments.wave_1.press $default-instruments.wave_env_r $default-instruments.wave_env_f $default-instruments.wave_2.press $default-instruments.wave_3.press $default-instruments.wave_3.frame $default-instruments.wave_4.press $default-instruments.wave_5.press $default-instruments.wave_5.frame $default-instruments.noise_1.press $default-instruments.noise_1.frame $default-instruments.noise_2.press)
  (data $.rodata (i32.const 8192) "W5\00S5\00W4\00T4\00S4\00W3\00T3\00S3\00W2\00T2\00S2\00N2\00W1\00T1\00S1\00N1\00\01\02\03\02\00\00\08\05\0a\03\01#Eg\89\ab\cd\ef\fe\dc\ba\98vT2\10\00\a0\00@\00`\00\00\11#Vx\99\98vg\9a\df\fe\c9\85B\111\de\dc\ba\98vT2\10\00\00\00\00\11\11\11\11\f0\f0\f0\f0\f0\f0\f0\f0\ff\00\ff\00\ff\00\ff\00\024g\9a\cd\ff\ff\ee\ee\ff\ff\dc\a9vC\10\00q\01\00\10\80\01\00y\80\01\00i\00\01\00Y\00\01\00Q\00\01\00\00r\01\00\11\80\01\00\12\00\01\00\13\00\01\00\14\00\01\00\15\00\01\00\00\a1\01\00Y\80\01\00[\00\01\00[\00\01\00]\00\01\00_\00\01\00`\00\01\00\00\a2\01\00Y\80\01\00y\00\01\00i\00\01\00P\00\01\00P\00\01\00Q\00\01\00A\00\01\00Q\00\01\00\00\90\01\00\00\80\01\00\000\01\00\004\01\00\00\00\00\00\00\00\00\00\00\00\00\00\00`\01\00\00@\01\00\00 \01\00\00\03\01\00\04\80\01\00\02\80\01\00\06\80\01\00\03\80\01\00\00\00\00\00\00\00\00\00\00\00\00\00\04\80\01\00\02\80\01\00\01\80\01\00\01\80\01\00\00\d0\01\00\00\d0\01\00\00\b0\01\00\00p\01\00\00P\01\00\000\01\00\00!\01\00\00`\01\00\00@\01\00\00 \01\00\00\03\01\00\02\80\01\00Q\80\01\00a\80\01\00q\80\01\00\91\80\01\00q\80\01\00`\80\01\00\04\80\01\00\02\80\01\00\01\80\01\00\01\80\01\00\00\d0\01\00\00\d0\01\00\00\d0\01\00\00\80\01\00\00\10\01\00\00 \01\00\003\01\00\00`\01\00\00@\01\00\00 \01\00\00\03\01\00`\80\01\00R\80\01\00B\80\01\00A\80\01\00\22\80\01\00\11\80\01\00\04\80\01\00\04\80\01\00\02\80\01\00\01\80\01\00\01\80\01\00\01\00\01\00k\00e\007\001\00,\00%\00\a0\00\7f\00\e3\00\aa\00\ef\00\a9\00\fd\00\a9\00\e3\00\8f\00%\00\16\00b\007\00\b9\00b\00\02\00\01\00\00\00\92 \00\00\c2 \00\00\aa \00\00\c2 \00\00\de \00\00\02!\00\00Z!\00\00\b2!\00\00\01\00\00\00\01\00\00\00\01\00\00\00\01\00\00\00\01\00\00\00\0b\00\00\00\0b\00\00\00\0b\00\00\00\96 \00\00\9a \00\00\ae \00\00\c6 \00\00\e2 \00\00.!\00\00\86!\00\00\de!\00\00\01\00\00\00\04\00\00\00\05\00\00\00\06\00\00\00\08\00\00\00\0b\00\00\00\0b\00\00\00\0b\00\00\00")
  (data $.data (i32.const 8896) "@\00\08\00\00\04\07\0c\ff\ff\00\04\07\0c\00\00\04\00\00\00\00\00\00\00\02\00\00\00\00\00\00\00\02\00\00\00"))
