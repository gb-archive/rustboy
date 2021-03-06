use super::*;
use super::super::mmu;

impl Cpu {
    // TODO: implement
    pub fn stop(&mut self) {
        warn!("STOP instruction issued at {:04X}", self.regs.pc);
        self.is_running = false;
    }
}

//  ======================================
//  |          CPU INSTRUCTIONS          |
//  ======================================

pub fn exec(inst: u8, r: &mut Registers, m: &mut mmu::Memory) -> u32 {
    macro_rules! ld (
        ($reg1:ident, $reg2:ident) => ({ r.$reg1 = r.$reg2;
        1 }) );

    macro_rules! ld_n (
        ($reg1:ident) => ({ r.$reg1 = m.rb(r.bump());
        2 }) );

    macro_rules! ld_nn (
        ($reg1:ident, $reg2:ident) => ({
            r.$reg2 = m.rb(r.bump());
            r.$reg1 = m.rb(r.bump());
        3 }) );

    macro_rules! call (
        () => ({
            r.sp = r.sp.wrapping_sub(2);
            m.ww(r.sp, r.pc + 2);
            let target = m.rw(r.pc);
            debug!("CALL to {:04X}", target);
            r.pc = target;
        6 }) );

    macro_rules! call_if (
        ($should_call:expr) => (if $should_call {call!()} else {r.pc += 2;
        3 }) );

    macro_rules! ret_if (
        ($should_ret:expr) => (if $should_ret {r.ret(m); 5} else {
        2 }) );

    macro_rules! jp (
        () => ({
            let j_addr = m.rw(r.pc);
            r.pc = j_addr;
        4 }) );

    macro_rules! jp_n (
        ($should_jp:expr) => (if $should_jp {jp!()} else {r.pc += 2;
        3 }) );

    macro_rules! jr (
        () => ({
            let target = add_signed(r.pc, m.rb(r.bump())) + 1;
            //debug!("JUMP(REL) to {:04X}", target);
            r.pc = target;
        3 }) );

    macro_rules! jr_n {
        ($cond:expr) => (if $cond {jr!()} else {r.pc = r.pc.wrapping_add(1); 2})
    }

    macro_rules! inc (
        ($reg:ident) => ({
            r.$reg = r.$reg.wrapping_add(1);
            r.f.n.unset();
            r.f.z.set_if(r.$reg == 0);
            r.f.h.set_if(r.$reg & 0xF == 0);
        1 }) );

    macro_rules! inc_16(
        ($reg1:ident, $reg2: ident) => ({
            r.$reg2 = r.$reg2.wrapping_add(1);
            if r.$reg2 == 0 { r.$reg1 = r.$reg1.wrapping_add(1); }
        2 }) );

    macro_rules! dec (
        ($reg:ident) => ({
            r.$reg = r.$reg.wrapping_sub(1);
            r.f.n.set();
            r.f.z.set_if(r.$reg == 0);
            r.f.h.set_if(r.$reg & 0xF == 0xF);
        1 }) );

    macro_rules! dec_16(
        ($reg1:ident, $reg2: ident) => ({
            r.$reg2 = r.$reg2.wrapping_sub(1);
            if r.$reg2 == 0xFF { r.$reg1 = r.$reg1.wrapping_sub(1); }
        2 }) );

    macro_rules! rst (
    ($e:expr) => ({
        r.sp = r.sp.wrapping_sub(2);
        m.ww(r.sp, r.pc);
        r.pc = $e;
    4 }) );

    macro_rules! xor_a (
    ($val:expr) => ({
        r.a ^= $val;
        r.f.reset();
        r.f.z.set_if(r.a == 0);
    4 }) );

    macro_rules! or_a (
    ($val:expr) => ({
        r.a |= $val;
        r.f.reset();
        r.f.z.set_if(r.a == 0);
    4 }) );

    macro_rules! and_a (
    ($val:expr) => ({
        r.a &= $val;
        r.f.n.unset();
        r.f.h.set();
        r.f.c.unset();
        r.f.z.set_if(r.a == 0);
    4 }) );

    macro_rules! cp_a (
    ($val:expr) => ({
        let v = $val;
        r.f.n.set();
        if r.a == v {r.f.z.set()} else {r.f.z.unset()};
        if r.a < v {r.f.c.set()} else {r.f.c.unset()};
        r.f.h.set_if((r.a & 0xF) < (v & 0xF));
        //debug!("{:02X} & 0xF < ({:2X} & 0xF)    c:{:?} h:{:?} ", r.a, v, r.f.c.get(),r.f.h.get());
    4 }) );

    macro_rules! rl( ($reg:expr, $cy:expr) => ({
        let ci = if r.f.c.get() {1} else {0};
        let co = $reg & 0x80;
        $reg = ($reg << 1) | ci;
        r.f.reset();
        if co != 0 { r.f.c.set() };
        $cy
    }) );

    macro_rules! rr( ($reg:expr, $cy:expr) => ({
        let ci = if r.f.c.get() {0x80} else {0};
        let co = $reg & 1;
        $reg = ($reg >> 1) | ci;
        r.f.reset();
        r.f.c.set_if(co == 1);
        $cy
    }) );

    macro_rules! rlc (
    ($reg:ident, $n:expr) => ({
        r.f.reset();
        r.$reg = r.$reg.rotate_left($n);
        r.f.c.set_if(r.$reg & 0x1 == 1);
    4 }) );

    macro_rules! rrc (
    ($reg:ident, $n:expr) => ({
        r.f.reset();
        r.$reg = r.$reg.rotate_right($n);
        r.f.c.set_if(r.$reg & 0x80 != 0);
    4 }) );

    macro_rules! add_hl(
    ($reg:expr) => ({
        let a = r.hl() as u32;
        let b = $reg as u32;
        let hl = a + b;
        r.f.n.unset();
        r.f.c.set_if(hl > 0xffff);
        r.f.h.set_if((a as u32 & 0xfff) > (hl & 0xfff));
        r.l = hl as u8;
        r.h = (hl >> 8) as u8;
    2 }) );

    macro_rules! push (
    ($reg:ident) => ({
        r.sp = r.sp.wrapping_sub(2);
        m.ww(r.sp, r.$reg());
    4 }) );

    macro_rules! add_a (
    ($reg:expr) => ({
        let i = r.a;
        let j = $reg;
        r.f.n.unset();
        r.f.h.set_if((i & 0xF) + (j & 0xF) > 0xF);
        r.f.c.set_if((i as u16 + j as u16) > 0xFF);
        r.a = i.wrapping_add(j);
        r.f.z.set_if(r.a == 0);
    1 }) );

    macro_rules! sub_a (
    ($reg:expr) => ({
        let a = r.a;
        let b = $reg;
        r.f.n.set();
        r.f.c.set_if(a < b);
        r.f.h.set_if((a & 0xF) < (b & 0xF));
        r.a = a.wrapping_sub(b);
        r.f.z.set_if(r.a == 0);
    1 }) );

    macro_rules! adc_a (
    ($reg:expr) => ({
        let a = r.a as u16;
        let b = $reg as u16;
        let c = if r.f.c.get() {1} else {0};
        let v = a.wrapping_add(b).wrapping_add(c);

        r.f.n.unset();
        r.f.h.set_if(((a & 0x0F) + (b & 0x0F) + c) > 0x0F);
        r.f.c.set_if(v > 0xFF);
        r.f.z.set_if((v & 0xFF) == 0);
        r.a = v as u8;
    1 }) );

    macro_rules! sbc_a (
    ($reg:expr) => ({
        let a = r.a as i16;
        let b = $reg as i16;
        let c = if r.f.c.get() {1} else {0};
        let v = a.wrapping_sub(b).wrapping_sub(c);

        r.f.n.set();
        r.f.c.set_if(v < 0);
        r.f.h.set_if(((((a as i16) & 0x0F) - ((b as i16) & 0x0F) - (c as i16)) < 0));
        r.f.z.set_if((v & 0xFF) == 0);
        r.a = v as u8;
    1 }) );

    macro_rules! ld_hlspn (
    () => ({
        r.f.reset();
        let b = m.rb(r.bump()) as i8 as i16 as u16;
        let res = b.wrapping_add(r.sp);
        r.h = (res >> 8) as u8;
        r.l = res as u8;
        let tmp = b ^ r.sp ^ r.hl();
        r.f.c.set_if(tmp & 0x100 != 0);
        r.f.h.set_if(tmp & 0x010 != 0);
    3 }) );

    macro_rules! ld_aIOn (
    () => ({
        let b = m.rb(r.bump()) as u16;
        r.a = m.rb(0xff00 | b);
    3 }) );

    // TODO: use set_or_else for everything

    macro_rules! daa (
    ($r:ident) => ({
        let mut a = r.a as u16;

        let c = r.f.c.get();
        let h = r.f.h.get();
        let n = r.f.n.get();

        let mut correction = if c {
            0x60u16
        } else {
            0x00u16
        };

        if h || (!n && a & 0x0F > 9) {
            correction |= 0x06;
        }

        if c || (!n && a > 0x99) {
            correction |= 0x60;
        }

        if n {
            a = a.wrapping_sub(correction);
        } else {
            a = a.wrapping_add(correction);
        }

        if (correction << 2 & 0x100) != 0 {
            r.f.c.set();
        }

        // Half-carry is always unset (unlike a Z-80)
        r.f.h.unset();
        r.f.z.set_if(a & 0xFF == 0);

        r.a = (a & 0xFF) as u8;
    }) );

    // if inst != 0 {
    //  info!("Decoding {:02X}", inst);
    // }

    // Table is partially from
    // https://github.com/alexcrichton/jba/blob/rust/src/cpu/z80/imp.rs#L279-L549
    // Instruction macros implemented by me
    match inst {
        0x00 => 1,                                                  // nop
        0x01 => ld_nn!(b, c),                                       // ld_bcnn

        0x02 => { m.wb(r.bc(), r.a); 2 }                            // ld_bca
        0x03 => inc_16!(b, c),                                      // inc_bc
        0x04 => inc!(b),                                            // inc_b
        0x05 => dec!(b),                                            // dec_b
        0x06 => ld_n!(b),                                           // ld_bn
        0x07 => rlc!(a, 1),                                         // rlca
        0x08 => { let a = m.rw(r.pc); m.ww(a, r.sp); r.pc += 2; 5 } // ld_nnsp
        0x09 => add_hl!(r.bc()),                                    // add_hlbc
        0x0a => { r.a = m.rb(r.bc()); 2 }                           // ld_abc
        0x0b => dec_16!(b, c),                                      // dec_bc
        0x0c => inc!(c),                                            // inc_c
        0x0d => dec!(c),                                            // dec_c
        0x0e => ld_n!(c),                                           // ld_cn
        0x0f => rrc!(a, 1),                                         // rrca

        0x10 => { r.stop = true; 1}                                 // stop
        0x11 => ld_nn!(d, e),                                       // ld_denn
        0x12 => { m.wb(r.de(), r.a); 2 }                            // ld_dea
        0x13 => inc_16!(d, e),                                      // inc_de
        0x14 => inc!(d),                                            // inc_d
        0x15 => dec!(d),                                            // dec_d
        0x16 => ld_n!(d),                                           // ld_dn
        0x17 => rl!(r.a, 1),                                        // rla
        0x18 => jr!(),                                              // jr_n
        0x19 => add_hl!(r.de()),                                    // add_hlde
        0x1a => { r.a = m.rb(r.de()); 2 }                           // ld_ade
        0x1b => dec_16!(d, e),                                      // dec_de
        0x1c => inc!(e),                                            // inc_e
        0x1d => dec!(e),                                            // dec_e
        0x1e => ld_n!(e),                                           // ld_en
        0x1f => rr!(r.a, 1),                                        // rr_a

        0x20 => jr_n!(!r.f.z.get()),                                // jr_nz_n
        0x21 => ld_nn!(h, l),                                       // ld_hlnn
        0x22 => { m.wb(r.hl(), r.a); r.inc_hl(); 2 },               // ld_hlma
        0x23 => inc_16!(h, l),                                      // inc_hl
        0x24 => inc!(h),                                            // inc_h
        0x25 => dec!(h),                                            // dec_h
        0x26 => ld_n!(h),                                           // ld_hn
        0x27 => { daa!(r); 1 },                                     // daa
        0x28 => jr_n!(r.f.z.get()),                                 // jr_z_n
        0x29 => add_hl!(r.hl()),                                    // add_hlhl
        0x2a => { r.a = m.rb(r.hl()); r.inc_hl(); 2 },              // ldi_ahlm
        0x2b => dec_16!(h, l),                                      // dec_hl
        0x2c => inc!(l),                                            // inc_l
        0x2d => dec!(l),                                            // dec_l
        0x2e => ld_n!(l),                                           // ld_ln
        0x2f => { r.a ^= 0xff; r.f.n.set(); r.f.h.set(); 1 }        // cpl

        0x30 => jr_n!(!r.f.c.get()),                                // jr_nc_n
        0x31 => { r.sp = m.rw(r.pc); r.pc += 2; 3 }                 // ld_spnn
        0x32 => { m.wb(r.hl(), r.a); r.dec_hl(); 2 }                // ldd_hlma
        0x33 => { r.sp = r.sp.wrapping_add(1); 2 }                  // inc_sp
        0x34 => { r.inc_hlm(m); 3 }                                 // inc_hlm
        0x35 => { r.dec_hlm(m); 3 }                                 // dec_hlm
        0x36 => { let v = m.rb(r.bump()); m.wb(r.hl(), v); 3 }      // ld_hlmn
        0x37 => { r.f.n.unset(); r.f.h.unset(); r.f.c.set(); 1 }    // scf
        0x38 => jr_n!(r.f.c.get()),                                 // jr_c_n
        0x39 => { r.add_hlsp(); 2 }                                 // add_hlsp
        0x3a => { r.a = m.rb(r.hl()); r.dec_hl(); 2 }               // ldd_ahlm
        0x3b => { r.sp = r.sp.wrapping_sub(1); 2 }                  // dec_sp
        //0x3c => {inc!(a); info!("inc a: {}",r.a); 1 },                                            // inc_a
        0x3c => inc!(a),                                            // inc_a
        0x3d => dec!(a),                                            // dec_a
        0x3e => ld_n!(a),                                           // ld_an
        0x3f => { r.f.h.unset(); r.f.n.unset(); r.f.c.toggle(); 1 } // ccf

        0x40 => ld!(b, b),                                          // ld_bb
        0x41 => ld!(b, c),                                          // ld_bc
        0x42 => ld!(b, d),                                          // ld_bd
        0x43 => ld!(b, e),                                          // ld_be
        0x44 => ld!(b, h),                                          // ld_bh
        0x45 => ld!(b, l),                                          // ld_bl
        0x46 => { r.b = m.rb(r.hl()); 2 }                           // ld_bhlm
        0x47 => ld!(b, a),                                          // ld_ba
        0x48 => ld!(c, b),                                          // ld_cb
        0x49 => ld!(c, c),                                          // ld_cc
        0x4a => ld!(c, d),                                          // ld_cd
        0x4b => ld!(c, e),                                          // ld_ce
        0x4c => ld!(c, h),                                          // ld_ch
        0x4d => ld!(c, l),                                          // ld_cl
        0x4e => { r.c = m.rb(r.hl()); 2 }                           // ld_chlm
        0x4f => ld!(c, a),                                          // ld_ca

        0x50 => ld!(d, b),                                          // ld_db
        0x51 => ld!(d, c),                                          // ld_dc
        0x52 => ld!(d, d),                                          // ld_dd
        0x53 => ld!(d, e),                                          // ld_de
        0x54 => ld!(d, h),                                          // ld_dh
        0x55 => ld!(d, l),                                          // ld_dl
        0x56 => { r.d = m.rb(r.hl()); 2 }                           // ld_dhlm
        0x57 => ld!(d, a),                                          // ld_da
        0x58 => ld!(e, b),                                          // ld_eb
        0x59 => ld!(e, c),                                          // ld_ec
        0x5a => ld!(e, d),                                          // ld_ed
        0x5b => ld!(e, e),                                          // ld_ee
        0x5c => ld!(e, h),                                          // ld_eh
        0x5d => ld!(e, l),                                          // ld_el
        0x5e => { r.e = m.rb(r.hl()); 2 }                           // ld_ehlm
        0x5f => ld!(e, a),                                          // ld_ea

        0x60 => ld!(h, b),                                          // ld_hb
        0x61 => ld!(h, c),                                          // ld_hc
        0x62 => ld!(h, d),                                          // ld_hd
        0x63 => ld!(h, e),                                          // ld_he
        0x64 => ld!(h, h),                                          // ld_hh
        0x65 => ld!(h, l),                                          // ld_hl
        0x66 => { r.h = m.rb(r.hl()); 2 }                           // ld_hhlm
        0x67 => ld!(h, a),                                          // ld_ha
        0x68 => ld!(l, b),                                          // ld_lb
        0x69 => ld!(l, c),                                          // ld_lc
        0x6a => ld!(l, d),                                          // ld_ld
        0x6b => ld!(l, e),                                          // ld_le
        0x6c => ld!(l, h),                                          // ld_lh
        0x6d => ld!(l, l),                                          // ld_ll
        0x6e => { r.l = m.rb(r.hl()); 2 }                           // ld_lhlm
        0x6f => ld!(l, a),                                          // ld_la

        0x70 => { m.wb(r.hl(), r.b); 2 }                            // ld_hlmb
        0x71 => { m.wb(r.hl(), r.c); 2 }                            // ld_hlmc
        0x72 => { m.wb(r.hl(), r.d); 2 }                            // ld_hlmd
        0x73 => { m.wb(r.hl(), r.e); 2 }                            // ld_hlme
        0x74 => { m.wb(r.hl(), r.h); 2 }                            // ld_hlmh
        0x75 => { m.wb(r.hl(), r.l); 2 }                            // ld_hlml
        0x76 => { r.halt = true; 1 }                                // halt
        0x77 => { m.wb(r.hl(), r.a); 2 }                            // ld_hlma
        0x78 => ld!(a, b),                                          // ld_ab
        0x79 => ld!(a, c),                                          // ld_ac
        0x7a => ld!(a, d),                                          // ld_ad
        0x7b => ld!(a, e),                                          // ld_ae
        0x7c => ld!(a, h),                                          // ld_ah
        0x7d => ld!(a, l),                                          // ld_al
        0x7e => { r.a = m.rb(r.hl()); 2 }                           // ld_ahlm
        0x7f => ld!(a, a),                                          // ld_aa

        0x80 => add_a!(r.b),                                        // add_ab
        0x81 => add_a!(r.c),                                        // add_ac
        0x82 => add_a!(r.d),                                        // add_ad
        0x83 => add_a!(r.e),                                        // add_ae
        0x84 => add_a!(r.h),                                        // add_ah
        0x85 => add_a!(r.l),                                        // add_al
        0x86 => { add_a!(m.rb(r.hl())); 2 }                         // add_ahlm
        0x87 => add_a!(r.a),                                        // add_aa
        0x88 => adc_a!(r.b),                                        // adc_ab
        0x89 => adc_a!(r.c),                                        // adc_ac
        0x8a => adc_a!(r.d),                                        // adc_ad
        0x8b => adc_a!(r.e),                                        // adc_ae
        0x8c => adc_a!(r.h),                                        // adc_ah
        0x8d => adc_a!(r.l),                                        // adc_al
        0x8e => { adc_a!(m.rb(r.hl())); 2 }                         // adc_ahlm
        0x8f => adc_a!(r.a),                                        // adc_aa

        0x90 => sub_a!(r.b),                                        // sub_ab
        0x91 => sub_a!(r.c),                                        // sub_ac
        0x92 => sub_a!(r.d),                                        // sub_ad
        0x93 => sub_a!(r.e),                                        // sub_ae
        0x94 => sub_a!(r.h),                                        // sub_ah
        0x95 => sub_a!(r.l),                                        // sub_al
        0x96 => { sub_a!(m.rb(r.hl())); 2 }                         // sub_ahlm
        0x97 => sub_a!(r.a),                                        // sub_aa
        0x98 => sbc_a!(r.b),                                        // sbc_ab
        0x99 => sbc_a!(r.c),                                        // sbc_ac
        0x9a => sbc_a!(r.d),                                        // sbc_ad
        0x9b => sbc_a!(r.e),                                        // sbc_ae
        0x9c => sbc_a!(r.h),                                        // sbc_ah
        0x9d => sbc_a!(r.l),                                        // sbc_al
        0x9e => { sbc_a!(m.rb(r.hl())); 2 }                         // sbc_ahlm
        0x9f => sbc_a!(r.a),                                        // sbc_aa

        0xa0 => and_a!(r.b),                                        // and_ab
        0xa1 => and_a!(r.c),                                        // and_ac
        0xa2 => and_a!(r.d),                                        // and_ad
        0xa3 => and_a!(r.e),                                        // and_ae
        0xa4 => and_a!(r.h),                                        // and_ah
        0xa5 => and_a!(r.l),                                        // and_al
        0xa6 => { and_a!(m.rb(r.hl())); 2 }                         // and_ahlm
        0xa7 => and_a!(r.a),                                        // and_aa
        0xa8 => xor_a!(r.b),                                        // xor_ab
        0xa9 => xor_a!(r.c),                                        // xor_ac
        0xaa => xor_a!(r.d),                                        // xor_ad
        0xab => xor_a!(r.e),                                        // xor_ae
        0xac => xor_a!(r.h),                                        // xor_ah
        0xad => xor_a!(r.l),                                        // xor_al
        0xae => { xor_a!(m.rb(r.hl())); 2 }                         // xor_ahlm
        0xaf => xor_a!(r.a),                                        // xor_aa

        0xb0 => or_a!(r.b),                                         // or_ab
        0xb1 => or_a!(r.c),                                         // or_ac
        0xb2 => or_a!(r.d),                                         // or_ad
        0xb3 => or_a!(r.e),                                         // or_ae
        0xb4 => or_a!(r.h),                                         // or_ah
        0xb5 => or_a!(r.l),                                         // or_al
        0xb6 => { or_a!(m.rb(r.hl())); 2 }                          // or_ahlm
        0xb7 => or_a!(r.a),                                         // or_aa
        0xb8 => cp_a!(r.b),                                         // cp_ab
        0xb9 => cp_a!(r.c),                                         // cp_ac
        0xba => cp_a!(r.d),                                         // cp_ad
        0xbb => cp_a!(r.e),                                         // cp_ae
        0xbc => cp_a!(r.h),                                         // cp_ah
        0xbd => cp_a!(r.l),                                         // cp_al
        0xbe => { cp_a!(m.rb(r.hl())); 2 }                          // cp_ahlm
        0xbf => cp_a!(r.a),                                         // cp_aa

        0xc0 => ret_if!(!r.f.z.get()),                              // ret_nz
        0xc1 => {let sp=r.sp; r.bc_set(m.rw(sp)); r.sp += 2; 3},    // pop_bc
        //0xc2 => { warn!("jp at {:04X}",r.pc);jp_n!(!r.f.z.get())},                                // jp_nz_nn
        0xc2 => jp_n!(!r.f.z.get()),                                // jp_nz_nn
        0xc3 => jp!(),                                              // jp_nn
        0xc4 => call_if!(!r.f.z.get()),                             // call_nz_n
        0xc5 => push!(bc),                                          // push_bc
        0xc6 => { add_a!(m.rb(r.bump())); 2 }                       // add_an
        0xc7 => rst!(0x00),                                         // rst_00
        0xc8 => ret_if!(r.f.z.get()),                               // ret_z
        0xc9 => { r.ret(m); 4 }                                     // ret
        0xca => jp_n!(r.f.z.get()),                                 // jp_z_nn
        0xcb => { exec_cb(m.rb(r.bump()), r, m) }                   // map_cb
        0xcc => call_if!(r.f.z.get()),                              // call_z_n
        0xcd => call!(),                                            // call
        0xce => { adc_a!(m.rb(r.bump())); 2 }                       // adc_an
        0xcf => rst!(0x08),                                         // rst_08

        0xd0 => ret_if!(!r.f.c.get()),                              // ret_nc
        0xd1 => {let sp=r.sp; r.de_set(m.rw(sp)); r.sp += 2; 3},    // pop_de
        0xd2 => jp_n!(!r.f.c.get()),                                // jp_nc_nn
        0xd3 => xx(),                                               // xx
        0xd4 => call_if!(!r.f.c.get()),                             // call_nc_n
        0xd5 => push!(de),                                          // push_de
        0xd6 => { sub_a!(m.rb(r.bump())); 2 }                       // sub_an
        0xd7 => rst!(0x10),                                         // rst_10
        0xd8 => ret_if!(r.f.c.get()),                               // ret_c
        0xd9 => { r.ei(m); r.ret(m); 4 }                            // reti
        0xda => jp_n!(r.f.c.get()),                                 // jp_c_nn
        0xdb => xx(),                                               // xx
        0xdc => call_if!(r.f.c.get()),                              // call_c_n
        0xdd => xx(),                                               // xx
        0xde => { sbc_a!(m.rb(r.bump())); 2 }                       // sbc_an
        0xdf => rst!(0x18),                                         // rst_18

        0xe0 => {let n=m.rb(r.bump());
            m.wb(0xFF00 | n as u16, r.a); 3 }                       // ld_IOan
        0xe1 => {let sp=r.sp; r.hl_set(m.rw(sp)); r.sp += 2; 3},    // pop_hl
        0xe2 => { m.wb(0xFF00 | (r.c as u16), r.a); 2 }             // ld_IOca
        0xe3 => xx(),                                               // xx
        0xe4 => xx(),                                               // xx
        0xe5 => push!(hl),                                          // push_hl
        0xe6 => and_a!(m.rb(r.bump())),                             // and_an
        //0xe6 => {and_a!(m.rb(r.bump())); warn!("and a:{:02X}",r.a); 2 }                       // and_an
        0xe7 => rst!(0x20),                                         // rst_20
        0xe8 => { add_spn(r, m); 4 }                                // add_spn
        0xe9 => { r.pc = r.hl(); 1 }                                // jp_hl
        0xea => { let n = m.rw(r.pc); m.wb(n, r.a); r.pc += 2; 4 }  // ld_nna
        0xeb => xx(),                                               // xx
        0xec => xx(),                                               // xx
        0xed => xx(),                                               // xx
        0xee => { xor_a!(m.rb(r.bump())); 2 }                       // xor_an
        0xef => rst!(0x28),                                         // rst_28

        0xf0 => ld_aIOn!(),                                         // ld_aIOn
        0xf1 => { let sp=r.sp; r.af_set(m.rw(sp)); r.sp += 2; 3 },  // pop_af
        0xf2 => { r.a = m.rb(0xff00 | (r.c as u16)); 2 }            // ld_aIOc
        0xf3 => { r.di(); 1 }                                       // di
        0xf4 => xx(),                                               // xx
        0xf5 => push!(af),                                          // push_af
        0xf6 => { or_a!(m.rb(r.bump())); 2 }                        // or_an
        0xf7 => rst!(0x30),                                         // rst_30
        0xf8 => { ld_hlspn!() }                                     // ld_hlspn
        0xf9 => { r.sp = r.hl(); 2 }                                // ld_sphl
        0xfa => { let b = m.rw(r.pc); r.a = m.rb(b); r.pc += 2; 4 } // ld_ann
        0xfb => { r.ei(m); 1 }                                      // ei
        0xfc => xx(),                                               // xx
        0xfd => xx(),                                               // xx
        0xfe => { cp_a!(m.rb(r.bump())); 2 }                        // cp_an
        0xff => rst!(0x38),                                         // rst_38

        _ => {
            panic!("Unknown instruction opcode: {:02X}", inst);
        },
    }
}

fn xx() -> u32 { panic!("Invalid instruction opcode"); }

fn add_signed(a: u16, b: u8) -> u16 {
    (a as i16 + (b as i8 as i16)) as u16
}

fn add_spn(r: &mut Registers, m: &mut mmu::Memory) {
    let b = m.rb(r.bump()) as i8 as i16 as u16;
    let res = r.sp.wrapping_add(b);
    let tmp = b ^ res ^ r.sp;
    r.f.c.set_if(tmp & 0x100 != 0);
    r.f.h.set_if(tmp & 0x010 != 0);
    r.f.n.unset();
    r.f.z.unset();
    r.sp = res;
}

//  ======================================
//  |           CB INSTRUCTIONS          |
//  ======================================


// From https://github.com/alexcrichton/jba/blob/rust/src/cpu/z80/imp.rs#L555-L896
#[allow(unused_parens)]
pub fn exec_cb(inst: u8, r: &mut Registers, m: &mut mmu::Memory) -> u32 {
    macro_rules! rl( ($reg:expr, $cy:expr) => ({
        let ci = if r.f.c.get() {1} else {0};
        let co = $reg & 0x80;
        r.f.h.unset(); r.f.n.unset();
        $reg = ($reg << 1) | ci;
        if $reg == 0 {r.f.z.set()} else {r.f.z.unset()};
        if co != 0 {r.f.c.set()} else {r.f.c.unset()};
        $cy as u32
    }) );

    macro_rules! rlc( ($reg:expr, $cy:expr) => ({
        let ci = if ($reg & 0x80) != 0 {1} else {0};
        r.f.h.unset(); r.f.n.unset();
        $reg = ($reg << 1) | ci;
        if $reg == 0 {r.f.z.set()} else {r.f.z.unset()};
        if ci != 0 {r.f.c.set()} else {r.f.c.unset()};
        $cy as u32
    }) );

    macro_rules! rr( ($reg:expr, $cy:expr) => ({
        let ci = if r.f.c.get() {0x80} else {0};
        let co = if ($reg & 0x01) != 0 {true} else {false};
        r.f.h.unset(); r.f.n.unset();
        $reg = ($reg >> 1) | ci;
        if $reg == 0 {r.f.z.set()} else {r.f.z.unset()};
        if co {r.f.c.set()} else {r.f.c.unset()};
        $cy as u32
    }) );

    macro_rules! rrc( ($reg:expr, $cy:expr) => ({
        let ci = $reg & 0x01;
        r.f.h.unset(); r.f.n.unset();
        $reg = ($reg >> 1) | (ci << 7);
        if $reg == 0 {r.f.z.set()} else {r.f.z.unset()};
        if ci != 0 {r.f.c.set()} else {r.f.c.unset()};
        $cy as u32
    }) );
    macro_rules! hlm( ($i:ident, $s:stmt) => ({
        let mut $i = m.rb(r.hl());
        r.f.h.unset(); r.f.n.unset();
        $s;
        m.wb(r.hl(), $i);
    }) );
    macro_rules! hlfrob( ($e:expr) => ({
        let hl = m.rb(r.hl());
        //r.f.h.unset(); r.f.n.unset();
        m.wb(r.hl(), hl & $e);
    }) );
    macro_rules! hlfrob_or( ($e:expr) => ({
        let hl = m.rb(r.hl());
        //r.f.h.unset(); r.f.n.unset();
        m.wb(r.hl(), hl | $e);
    }) );
    macro_rules! sra( ($e:expr, $cy:expr) => ({
        let co = $e & 1;
        r.f.h.unset(); r.f.n.unset();
        $e = (($e as i8) >> 1) as u8;
        if $e == 0 {r.f.z.set()} else {r.f.z.unset()};
        if co != 0 {r.f.c.set()} else {r.f.c.unset()};
        $cy as u32
    }) );
    macro_rules! srl( ($e:expr, $cy:expr) => ({
        let co = $e & 1;
        r.f.h.unset(); r.f.n.unset();
        $e = $e >> 1;
        if $e == 0 {r.f.z.set()} else {r.f.z.unset()};
        if co != 0 {r.f.c.set()} else {r.f.c.unset()};
        $cy as u32
    }) );
    macro_rules! sla( ($e:expr, $cy:expr) => ({
        let co = ($e >> 7) & 1;
        r.f.h.unset(); r.f.n.unset();
        $e = $e << 1;
        if $e == 0 {r.f.z.set()} else {r.f.z.unset()};
        if co != 0 {r.f.c.set()} else {r.f.c.unset()};
        $cy as u32
    }) );
    macro_rules! swap( ($e:expr) => ({
        r.f.h.unset(); r.f.n.unset(); r.f.c.unset();
        $e = ($e << 4) | (($e & 0xf0) >> 4);
        if $e == 0 {r.f.z.set()} else {r.f.z.unset()};
        2 as u32
    }) );
    macro_rules! bit( ($e:expr, $bit:expr) => ({
        r.f.h.set(); r.f.n.unset();
        if $e & (1 << $bit) == 0 {r.f.z.set()} else {r.f.z.unset()};
        2 as u32
    }) );

    trace!("CB {:02X} executing", inst);


    match inst {
        0x00 => rlc!(r.b, 2),                                       // rlc_b
        0x01 => rlc!(r.c, 2),                                       // rlc_c
        0x02 => rlc!(r.d, 2),                                       // rlc_d
        0x03 => rlc!(r.e, 2),                                       // rlc_e
        0x04 => rlc!(r.h, 2),                                       // rlc_h
        0x05 => rlc!(r.l, 2),                                       // rlc_l
        0x06 => { hlm!(hl, rlc!(hl, 1)); 4 }                                 // rlc_hlm
        0x07 => rlc!(r.a, 2),                                       // rlc_a
        0x08 => rrc!(r.b, 2),                                       // rrc_b
        0x09 => rrc!(r.c, 2),                                       // rrc_c
        0x0a => rrc!(r.d, 2),                                       // rrc_d
        0x0b => rrc!(r.e, 2),                                       // rrc_e
        0x0c => rrc!(r.h, 2),                                       // rrc_h
        0x0d => rrc!(r.l, 2),                                       // rrc_l
        0x0e => { hlm!(hl, rrc!(hl, 1)); 4 }                                 // rrc_hlm
        0x0f => rrc!(r.a, 2),                                       // rrc_a

        0x10 => rl!(r.b, 2),                                        // rl_b
        0x11 => rl!(r.c, 2),                                        // rl_c
        0x12 => rl!(r.d, 2),                                        // rl_d
        0x13 => rl!(r.e, 2),                                        // rl_e
        0x14 => rl!(r.h, 2),                                        // rl_h
        0x15 => rl!(r.l, 2),                                        // rl_l
        0x16 => { hlm!(hl, rl!(hl, 1)); 4 }                                  // rl_hlm
        0x17 => rl!(r.a, 2),                                        // rl_a
        0x18 => rr!(r.b, 2),                                        // rr_b
        0x19 => rr!(r.c, 2),                                        // rr_c
        0x1a => rr!(r.d, 2),                                        // rr_d
        0x1b => rr!(r.e, 2),                                        // rr_e
        0x1c => rr!(r.h, 2),                                        // rr_h
        0x1d => rr!(r.l, 2),                                        // rr_l
        0x1e => { hlm!(hl, rr!(hl, 1)); 4 }                                  // rr_hlm
        0x1f => rr!(r.a, 2),                                        // rr_a

        0x20 => sla!(r.b, 2),                                       // sla_b
        0x21 => sla!(r.c, 2),                                       // sla_c
        0x22 => sla!(r.d, 2),                                       // sla_d
        0x23 => sla!(r.e, 2),                                       // sla_e
        0x24 => sla!(r.h, 2),                                       // sla_h
        0x25 => sla!(r.l, 2),                                       // sla_l
        0x26 => { hlm!(hl, sla!(hl, 1)); 4 }                                 // sla_hlm
        0x27 => sla!(r.a, 2),                                       // sla_a
        0x28 => sra!(r.b, 2),                                       // sra_b
        0x29 => sra!(r.c, 2),                                       // sra_c
        0x2a => sra!(r.d, 2),                                       // sra_d
        0x2b => sra!(r.e, 2),                                       // sra_e
        0x2c => sra!(r.h, 2),                                       // sra_h
        0x2d => sra!(r.l, 2),                                       // sra_l
        0x2e => { hlm!(hl, sra!(hl, 1)); 4 }                                 // sra_hlm
        0x2f => sra!(r.a, 2),                                       // sra_a

        0x30 => swap!(r.b),                                         // swap_b
        0x31 => swap!(r.c),                                         // swap_c
        0x32 => swap!(r.d),                                         // swap_d
        0x33 => swap!(r.e),                                         // swap_e
        0x34 => swap!(r.h),                                         // swap_h
        0x35 => swap!(r.l),                                         // swap_l
        0x36 => { hlm!(hl, swap!(hl)); 4 }                                   // swap_hlm
        0x37 => swap!(r.a),                                         // swap_a
        0x38 => srl!(r.b, 2),                                       // srl_b
        0x39 => srl!(r.c, 2),                                       // srl_c
        0x3a => srl!(r.d, 2),                                       // srl_d
        0x3b => srl!(r.e, 2),                                       // srl_e
        0x3c => srl!(r.h, 2),                                       // srl_h
        0x3d => srl!(r.l, 2),                                       // srl_l
        0x3e => { hlm!(hl, srl!(hl, 1)); 4 }                                 // srl_hlm
        0x3f => srl!(r.a, 2),                                       // srl_a

        0x40 => bit!(r.b, 0),                                       // bit_0b
        0x41 => bit!(r.c, 0),                                       // bit_0c
        0x42 => bit!(r.d, 0),                                       // bit_0d
        0x43 => bit!(r.e, 0),                                       // bit_0e
        0x44 => bit!(r.h, 0),                                       // bit_0h
        0x45 => bit!(r.l, 0),                                       // bit_0l
        0x46 => { bit!(m.rb(r.hl()), 0); 3 }                        // bit_0hlm
        0x47 => bit!(r.a, 0),                                       // bit_0a
        0x48 => bit!(r.b, 1),                                       // bit_1b
        0x49 => bit!(r.c, 1),                                       // bit_1c
        0x4a => bit!(r.d, 1),                                       // bit_1d
        0x4b => bit!(r.e, 1),                                       // bit_1e
        0x4c => bit!(r.h, 1),                                       // bit_1h
        0x4d => bit!(r.l, 1),                                       // bit_1l
        0x4e => { bit!(m.rb(r.hl()), 1); 3 }                        // bit_1hlm
        0x4f => bit!(r.a, 1),                                       // bit_1a

        0x50 => bit!(r.b, 2),                                       // bit_2b
        0x51 => bit!(r.c, 2),                                       // bit_2c
        0x52 => bit!(r.d, 2),                                       // bit_2d
        0x53 => bit!(r.e, 2),                                       // bit_2e
        0x54 => bit!(r.h, 2),                                       // bit_2h
        0x55 => bit!(r.l, 2),                                       // bit_2l
        0x56 => { bit!(m.rb(r.hl()), 2); 3 }                        // bit_2hlm
        0x57 => bit!(r.a, 2),                                       // bit_2a
        0x58 => bit!(r.b, 3),                                       // bit_3b
        0x59 => bit!(r.c, 3),                                       // bit_3c
        0x5a => bit!(r.d, 3),                                       // bit_3d
        0x5b => bit!(r.e, 3),                                       // bit_3e
        0x5c => bit!(r.h, 3),                                       // bit_3h
        0x5d => bit!(r.l, 3),                                       // bit_3l
        0x5e => { bit!(m.rb(r.hl()), 3); 3 }                        // bit_3hlm
        0x5f => bit!(r.a, 3),                                       // bit_3a

        0x60 => bit!(r.b, 4),                                       // bit_4b
        0x61 => bit!(r.c, 4),                                       // bit_4c
        0x62 => bit!(r.d, 4),                                       // bit_4d
        0x63 => bit!(r.e, 4),                                       // bit_4e
        0x64 => bit!(r.h, 4),                                       // bit_4h
        0x65 => bit!(r.l, 4),                                       // bit_4l
        0x66 => { bit!(m.rb(r.hl()), 4); 3 }                        // bit_4hlm
        0x67 => bit!(r.a, 4),                                       // bit_4a
        0x68 => bit!(r.b, 5),                                       // bit_5b
        0x69 => bit!(r.c, 5),                                       // bit_5c
        0x6a => bit!(r.d, 5),                                       // bit_5d
        0x6b => bit!(r.e, 5),                                       // bit_5e
        0x6c => bit!(r.h, 5),                                       // bit_5h
        0x6d => bit!(r.l, 5),                                       // bit_5l
        0x6e => { bit!(m.rb(r.hl()), 5); 3 }                        // bit_5hlm
        0x6f => bit!(r.a, 5),                                       // bit_5a

        0x70 => bit!(r.b, 6),                                       // bit_6b
        0x71 => bit!(r.c, 6),                                       // bit_6c
        0x72 => bit!(r.d, 6),                                       // bit_6d
        0x73 => bit!(r.e, 6),                                       // bit_6e
        0x74 => bit!(r.h, 6),                                       // bit_6h
        0x75 => bit!(r.l, 6),                                       // bit_6l
        0x76 => { bit!(m.rb(r.hl()), 6); 3 }                        // bit_6hlm
        0x77 => bit!(r.a, 6),                                       // bit_6a
        0x78 => bit!(r.b, 7),                                       // bit_7b
        0x79 => bit!(r.c, 7),                                       // bit_7c
        0x7a => bit!(r.d, 7),                                       // bit_7d
        0x7b => bit!(r.e, 7),                                       // bit_7e
        0x7c => bit!(r.h, 7),                                       // bit_7h
        0x7d => bit!(r.l, 7),                                       // bit_7l
        0x7e => { bit!(m.rb(r.hl()), 7); 3 }                        // bit_7hlm
        0x7f => bit!(r.a, 7),                                       // bit_7a

        0x80 => { r.b &= !(1 << 0); 2 }                             // res_0b
        0x81 => { r.c &= !(1 << 0); 2 }                             // res_0c
        0x82 => { r.d &= !(1 << 0); 2 }                             // res_0d
        0x83 => { r.e &= !(1 << 0); 2 }                             // res_0e
        0x84 => { r.h &= !(1 << 0); 2 }                             // res_0h
        0x85 => { r.l &= !(1 << 0); 2 }                             // res_0l
        0x86 => { hlfrob!(!(1 << 0)); 4 }                           // set_0hlm
        0x87 => { r.a &= !(1 << 0); 2 }                             // res_0a
        0x88 => { r.b &= !(1 << 1); 2 }                             // res_1b
        0x89 => { r.c &= !(1 << 1); 2 }                             // res_1c
        0x8a => { r.d &= !(1 << 1); 2 }                             // res_1d
        0x8b => { r.e &= !(1 << 1); 2 }                             // res_1e
        0x8c => { r.h &= !(1 << 1); 2 }                             // res_1h
        0x8d => { r.l &= !(1 << 1); 2 }                             // res_1l
        0x8e => { hlfrob!(!(1 << 1)); 4 }                           // set_1hlm
        0x8f => { r.a &= !(1 << 1); 2 }                             // res_1a

        0x90 => { r.b &= !(1 << 2); 2 }                             // res_2b
        0x91 => { r.c &= !(1 << 2); 2 }                             // res_2c
        0x92 => { r.d &= !(1 << 2); 2 }                             // res_2d
        0x93 => { r.e &= !(1 << 2); 2 }                             // res_2e
        0x94 => { r.h &= !(1 << 2); 2 }                             // res_2h
        0x95 => { r.l &= !(1 << 2); 2 }                             // res_2l
        0x96 => { hlfrob!(!(1 << 2)); 4 }                           // set_2hlm
        0x97 => { r.a &= !(1 << 2); 2 }                             // res_2a
        0x98 => { r.b &= !(1 << 3); 2 }                             // res_3b
        0x99 => { r.c &= !(1 << 3); 2 }                             // res_3c
        0x9a => { r.d &= !(1 << 3); 2 }                             // res_3d
        0x9b => { r.e &= !(1 << 3); 2 }                             // res_3e
        0x9c => { r.h &= !(1 << 3); 2 }                             // res_3h
        0x9d => { r.l &= !(1 << 3); 2 }                             // res_3l
        0x9e => { hlfrob!(!(1 << 3)); 4 }                           // set_3hlm
        0x9f => { r.a &= !(1 << 3); 2 }                             // res_3a

        0xa0 => { r.b &= !(1 << 4); 2 }                             // res_4b
        0xa1 => { r.c &= !(1 << 4); 2 }                             // res_4c
        0xa2 => { r.d &= !(1 << 4); 2 }                             // res_4d
        0xa3 => { r.e &= !(1 << 4); 2 }                             // res_4e
        0xa4 => { r.h &= !(1 << 4); 2 }                             // res_4h
        0xa5 => { r.l &= !(1 << 4); 2 }                             // res_4l
        0xa6 => { hlfrob!(!(1 << 4)); 4 }                           // set_4hlm
        0xa7 => { r.a &= !(1 << 4); 2 }                             // res_4a
        0xa8 => { r.b &= !(1 << 5); 2 }                             // res_5b
        0xa9 => { r.c &= !(1 << 5); 2 }                             // res_5c
        0xaa => { r.d &= !(1 << 5); 2 }                             // res_5d
        0xab => { r.e &= !(1 << 5); 2 }                             // res_5e
        0xac => { r.h &= !(1 << 5); 2 }                             // res_5h
        0xad => { r.l &= !(1 << 5); 2 }                             // res_5l
        0xae => { hlfrob!(!(1 << 5)); 4 }                           // set_5hlm
        0xaf => { r.a &= !(1 << 5); 2 }                             // res_5a

        0xb0 => { r.b &= !(1 << 6); 2 }                             // res_6b
        0xb1 => { r.c &= !(1 << 6); 2 }                             // res_6c
        0xb2 => { r.d &= !(1 << 6); 2 }                             // res_6d
        0xb3 => { r.e &= !(1 << 6); 2 }                             // res_6e
        0xb4 => { r.h &= !(1 << 6); 2 }                             // res_6h
        0xb5 => { r.l &= !(1 << 6); 2 }                             // res_6l
        0xb6 => { hlfrob!(!(1 << 6)); 4 }                           // set_6hlm
        0xb7 => { r.a &= !(1 << 6); 2 }                             // res_6a
        0xb8 => { r.b &= !(1 << 7); 2 }                             // res_7b
        0xb9 => { r.c &= !(1 << 7); 2 }                             // res_7c
        0xba => { r.d &= !(1 << 7); 2 }                             // res_7d
        0xbb => { r.e &= !(1 << 7); 2 }                             // res_7e
        0xbc => { r.h &= !(1 << 7); 2 }                             // res_7h
        0xbd => { r.l &= !(1 << 7); 2 }                             // res_7l
        0xbe => { hlfrob!(!(1 << 7)); 4 }                           // set_7hlm
        0xbf => { r.a &= !(1 << 7); 2 }                             // res_7a

        0xc0 => { r.b |= (1 << 0); 2 }                              // set_0b
        0xc1 => { r.c |= (1 << 0); 2 }                              // set_0c
        0xc2 => { r.d |= (1 << 0); 2 }                              // set_0d
        0xc3 => { r.e |= (1 << 0); 2 }                              // set_0e
        0xc4 => { r.h |= (1 << 0); 2 }                              // set_0h
        0xc5 => { r.l |= (1 << 0); 2 }                              // set_0l
        0xc6 => { hlfrob_or!((1 << 0)); 4 }                         // set_0hlm
        0xc7 => { r.a |= (1 << 0); 2 }                              // set_0a
        0xc8 => { r.b |= (1 << 1); 2 }                              // set_1b
        0xc9 => { r.c |= (1 << 1); 2 }                              // set_1c
        0xca => { r.d |= (1 << 1); 2 }                              // set_1d
        0xcb => { r.e |= (1 << 1); 2 }                              // set_1e
        0xcc => { r.h |= (1 << 1); 2 }                              // set_1h
        0xcd => { r.l |= (1 << 1); 2 }                              // set_1l
        0xce => { hlfrob_or!((1 << 1)); 4 }                         // set_1hlm
        0xcf => { r.a |= (1 << 1); 2 }                              // set_1a

        0xd0 => { r.b |= (1 << 2); 2 }                              // set_2b
        0xd1 => { r.c |= (1 << 2); 2 }                              // set_2c
        0xd2 => { r.d |= (1 << 2); 2 }                              // set_2d
        0xd3 => { r.e |= (1 << 2); 2 }                              // set_2e
        0xd4 => { r.h |= (1 << 2); 2 }                              // set_2h
        0xd5 => { r.l |= (1 << 2); 2 }                              // set_2l
        0xd6 => { hlfrob_or!((1 << 2)); 4 }                         // set_2hlm
        0xd7 => { r.a |= (1 << 2); 2 }                              // set_2a
        0xd8 => { r.b |= (1 << 3); 2 }                              // set_3b
        0xd9 => { r.c |= (1 << 3); 2 }                              // set_3c
        0xda => { r.d |= (1 << 3); 2 }                              // set_3d
        0xdb => { r.e |= (1 << 3); 2 }                              // set_3e
        0xdc => { r.h |= (1 << 3); 2 }                              // set_3h
        0xdd => { r.l |= (1 << 3); 2 }                              // set_3l
        0xde => { hlfrob_or!((1 << 3)); 4 }                         // set_3hlm
        0xdf => { r.a |= (1 << 3); 2 }                              // set_3a

        0xe0 => { r.b |= (1 << 4); 2 }                              // set_4b
        0xe1 => { r.c |= (1 << 4); 2 }                              // set_4c
        0xe2 => { r.d |= (1 << 4); 2 }                              // set_4d
        0xe3 => { r.e |= (1 << 4); 2 }                              // set_4e
        0xe4 => { r.h |= (1 << 4); 2 }                              // set_4h
        0xe5 => { r.l |= (1 << 4); 2 }                              // set_4l
        0xe6 => { hlfrob_or!((1 << 4)); 4 }                         // set_4hlm
        0xe7 => { r.a |= (1 << 4); 2 }                              // set_4a
        0xe8 => { r.b |= (1 << 5); 2 }                              // set_5b
        0xe9 => { r.c |= (1 << 5); 2 }                              // set_5c
        0xea => { r.d |= (1 << 5); 2 }                              // set_5d
        0xeb => { r.e |= (1 << 5); 2 }                              // set_5e
        0xec => { r.h |= (1 << 5); 2 }                              // set_5h
        0xed => { r.l |= (1 << 5); 2 }                              // set_5l
        0xee => { hlfrob_or!((1 << 5)); 4 }                         // set_5hlm
        0xef => { r.a |= (1 << 5); 2 }                              // set_5a

        0xf0 => { r.b |= (1 << 6); 2 }                              // set_6b
        0xf1 => { r.c |= (1 << 6); 2 }                              // set_6c
        0xf2 => { r.d |= (1 << 6); 2 }                              // set_6d
        0xf3 => { r.e |= (1 << 6); 2 }                              // set_6e
        0xf4 => { r.h |= (1 << 6); 2 }                              // set_6h
        0xf5 => { r.l |= (1 << 6); 2 }                              // set_6l
        0xf6 => { hlfrob_or!((1 << 6)); 4 }                         // set_6hlm
        0xf7 => { r.a |= (1 << 6); 2 }                              // set_6a
        0xf8 => { r.b |= (1 << 7); 2 }                              // set_7b
        0xf9 => { r.c |= (1 << 7); 2 }                              // set_7c
        0xfa => { r.d |= (1 << 7); 2 }                              // set_7d
        0xfb => { r.e |= (1 << 7); 2 }                              // set_7e
        0xfc => { r.h |= (1 << 7); 2 }                              // set_7h
        0xfd => { r.l |= (1 << 7); 2 }                              // set_7l
        0xfe => { hlfrob_or!((1 << 7)); 4 }                         // set_7hlm
        0xff => { r.a |= (1 << 7); 2 }                              // set_7a

        _ => 0
    }
}
