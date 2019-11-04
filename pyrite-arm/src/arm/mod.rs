mod instr_dataproc;
mod instr_mul;
mod instr_single_data_transfer;
mod instr_hws_data_transfer;
mod instr_block_data_transfer;
mod instr_coprocessor;

use self::instr_coprocessor::*;
use self::instr_dataproc::*;
use self::instr_mul::*;
use self::instr_single_data_transfer::*;
use self::instr_hws_data_transfer::*;
use self::instr_block_data_transfer::*;

use super::{ ArmCpu, ArmMemory };
use super::clock;

/// Branch and Exchange
///
/// BX Rd
fn arm_bx(cpu: &mut ArmCpu, memory: &mut dyn ArmMemory, instr: u32) {
	let mut dest = cpu.registers.read(instr & 0xF);
        cpu.cycles += clock::cycles_prefetch(memory, false, cpu.registers.read(15));
	if (dest & 1) == 0 {
            dest &= 0xFFFFFFFC;
            cpu.arm_branch_to(dest, memory);
            cpu.cycles += clock::cycles_branch_refill(memory, false, dest);
	} else {
            dest &= 0xFFFFFFFE;
            cpu.registers.setf_t();
            cpu.thumb_branch_to(dest, memory);
            cpu.cycles += clock::cycles_branch_refill(memory, true, dest);
	}
}

/// Branch
///
/// B <offset>
fn arm_b(cpu: &mut ArmCpu, memory: &mut dyn ArmMemory, instr: u32) {
	let offset = sign_extend_32!(instr & 0xFFFFFF, 24).wrapping_shl(2);
	let pc = cpu.registers.read(15);
        let dest = pc.wrapping_add(offset);
	cpu.arm_branch_to(dest, memory);
        cpu.cycles += clock::cycles_branch(memory, false, pc, dest);
}

/// Branch and Link
///
/// BL <offset>
fn arm_bl(cpu: &mut ArmCpu, memory: &mut dyn ArmMemory, instr: u32) {
	let offset = sign_extend_32!(instr & 0xFFFFFF, 24).wrapping_shl(2);
	let pc = cpu.registers.read(15);
        let dest = pc.wrapping_add(offset);
	cpu.registers.write(14, (pc.wrapping_sub(4)) & 0xFFFFFFFC);
	cpu.arm_branch_to(dest, memory);
        cpu.cycles += clock::cycles_branch(memory, false, pc, dest);
}


/// Move status word to register, Register, CPSR
///
/// MRS Rd, CPSR
pub fn arm_mrs_rc(cpu: &mut ArmCpu, memory: &mut dyn ArmMemory, instr: u32) {
    // #NOTE: The PC is not allowed to be a destination register here but I don't check it.
    let rd = (instr >> 12) & 0xf;
    let cpsr = cpu.registers.read_cpsr();
    cpu.registers.write(rd, cpsr);
    cpu.cycles += clock::cycles_prefetch(memory, false, cpu.registers.read(15));
}

/// Move status word to register, Register, SPSR
///
/// MRS Rd, SPSR
pub fn arm_mrs_rs(cpu: &mut ArmCpu, memory: &mut dyn ArmMemory, instr: u32) {
    // #NOTE: The PC is not allowed to be a destination register here but I don't check it.
    let rd = (instr >> 12) & 0xf;
    let spsr = cpu.registers.read_spsr();
    cpu.registers.write(rd, spsr);
    cpu.cycles += clock::cycles_prefetch(memory, false, cpu.registers.read(15));
}

/// Move value to status word, Immediate, CPSR
///
/// MSR CPSR_flg, <#expression>
pub fn arm_msr_ic(cpu: &mut ArmCpu, memory: &mut dyn ArmMemory, instr: u32) {
    if likely!((instr & 0x00010000) == 0) { // CPSR_flg
        let src = super::alu::bs::imm_nc(instr);
        let cpsr = cpu.registers.read_cpsr();
        cpu.registers.write_cpsr((cpsr & !0xF0000000) | (src & 0xF0000000));
        cpu.cycles += clock::cycles_prefetch(memory, false, cpu.registers.read(15));
    } else { // CPSR_all (not supported using immediate value)
        arm_undefined(cpu, memory, instr);
    }
}


/// Move value to status word, Immediate, SPSR
///
/// MSR SPSR_flg, <#expression>
pub fn arm_msr_is(cpu: &mut ArmCpu, memory: &mut dyn ArmMemory, instr: u32) {
    if likely!((instr & 0x00010000) == 0) { // SPSR_flg
        let src = super::alu::bs::imm_nc(instr);
        let spsr = cpu.registers.read_spsr();
        cpu.registers.write_spsr((spsr & !0xF0000000) | (src & 0xF0000000));
        cpu.cycles += clock::cycles_prefetch(memory, false, cpu.registers.read(15));
    } else { // SPSR_all (not supported using immediate value)
        arm_undefined(cpu, memory, instr);
    }
}

/// Move value to status word, Register, CPSR
///
/// MSR CPSR/CPSR_flg, Rm
pub fn arm_msr_rc(cpu: &mut ArmCpu, memory: &mut dyn ArmMemory, instr: u32) {
    // #NOTE: The PC is not allowed to be an operand register here but I don't check it.
    let src = cpu.registers.read(instr & 0xf);
    if likely!((instr & 0x00010000) == 0) { // CPSR_flg
        let cpsr = cpu.registers.read_cpsr();
        cpu.registers.write_cpsr((cpsr & !0xF0000000) | (src & 0xF0000000));
    } else { // CPSR_all
        cpu.registers.write_cpsr(src);
    }
    cpu.cycles += clock::cycles_prefetch(memory, false, cpu.registers.read(15));
}

/// Move value to status word, Register, SPSR
///
/// MSR SPSR/SPSR_flg, Rm
pub fn arm_msr_rs(cpu: &mut ArmCpu, memory: &mut dyn ArmMemory, instr: u32) {
    // #NOTE: The PC is not allowed to be an operand register here, but I don't check it.
    let src = cpu.registers.read(instr & 0xf);
    if likely!((instr & 0x00010000) == 0) { // SPSR_flg
        let spsr = cpu.registers.read_spsr();
        cpu.registers.write_spsr((spsr & !0xF0000000) | (src & 0xF0000000));
    } else { // CPSR_all
        cpu.registers.write_spsr(src);
    }
    cpu.cycles += clock::cycles_prefetch(memory, false, cpu.registers.read(15));
}

/// Swap registers with memory word
pub fn arm_swp(cpu: &mut ArmCpu, memory: &mut dyn ArmMemory, instr: u32) {
    let rm = bits!(instr, 0, 3);
    let rd = bits!(instr, 12, 15);
    let rn = bits!(instr, 16, 19);

    let addr = cpu.registers.read(rn);
    let temp = memory.load32(addr); // we use temp because Rd and Rm might be the same.
    memory.store32(addr, cpu.registers.read(rm));
    cpu.registers.write(rd, temp);

    cpu.cycles += clock::cycles_prefetch(memory, false, cpu.registers.read(15));
    cpu.cycles += memory.data_access_nonseq32(addr);
}

/// Swap registers with memory byte
pub fn arm_swpb(cpu: &mut ArmCpu, memory: &mut dyn ArmMemory, instr: u32) {
    let rm = bits!(instr, 0, 3);
    let rd = bits!(instr, 12, 15);
    let rn = bits!(instr, 16, 19);

    let addr = cpu.registers.read(rn);
    let temp = memory.load8(addr); // we use temp because Rd and Rm might be the same.
    memory.store8(addr, cpu.registers.read(rm) as u8);
    cpu.registers.write(rd, temp as u32);

    cpu.cycles += clock::cycles_prefetch(memory, false, cpu.registers.read(15));
    cpu.cycles += memory.data_access_nonseq8(addr);
}

/// SWI INSTR
fn arm_swi(cpu: &mut ArmCpu, memory: &mut dyn ArmMemory, _instr: u32) {
    cpu.cycles += clock::cycles_prefetch(memory, false, cpu.registers.read(15));
    cpu.handle_exception(super::cpu::CpuException::SWI, memory, cpu.registers.read(15).wrapping_sub(4));
}

/// UNDEFINED INSTR
fn arm_undefined(cpu: &mut ArmCpu, memory: &mut dyn ArmMemory, _instr: u32) {
    cpu.cycles += clock::cycles_prefetch(memory, false, cpu.registers.read(15));
    cpu.handle_exception(super::cpu::CpuException::Undefined, memory, cpu.registers.read(15).wrapping_sub(4));
}

#[allow(dead_code)]
pub const ARM_OPCODE_TABLE: [fn(&mut ArmCpu, memory: &mut dyn ArmMemory, u32); 4096] = [
	arm_and_lli,arm_and_llr,arm_and_lri,arm_and_lrr,arm_and_ari,arm_and_arr,arm_and_rri,arm_and_rrr,arm_and_lli,arm_mul,arm_and_lri,arm_strh_ptrm,arm_and_ari,arm_undefined,arm_and_rri,arm_undefined,
	arm_ands_lli,arm_ands_llr,arm_ands_lri,arm_ands_lrr,arm_ands_ari,arm_ands_arr,arm_ands_rri,arm_ands_rrr,arm_ands_lli,arm_muls,arm_ands_lri,arm_ldrh_ptrm,arm_ands_ari,arm_ldrsb_ptrm,arm_ands_rri,arm_ldrsh_ptrm,
	arm_eor_lli,arm_eor_llr,arm_eor_lri,arm_eor_lrr,arm_eor_ari,arm_eor_arr,arm_eor_rri,arm_eor_rrr,arm_eor_lli,arm_mla,arm_eor_lri,arm_strh_ptrm,arm_eor_ari,arm_undefined,arm_eor_rri,arm_undefined,
	arm_eors_lli,arm_eors_llr,arm_eors_lri,arm_eors_lrr,arm_eors_ari,arm_eors_arr,arm_eors_rri,arm_eors_rrr,arm_eors_lli,arm_mlas,arm_eors_lri,arm_ldrh_ptrm,arm_eors_ari,arm_ldrsb_ptrm,arm_eors_rri,arm_ldrsh_ptrm,
	arm_sub_lli,arm_sub_llr,arm_sub_lri,arm_sub_lrr,arm_sub_ari,arm_sub_arr,arm_sub_rri,arm_sub_rrr,arm_sub_lli,arm_undefined,arm_sub_lri,arm_strh_ptim,arm_sub_ari,arm_undefined,arm_sub_rri,arm_undefined,
	arm_subs_lli,arm_subs_llr,arm_subs_lri,arm_subs_lrr,arm_subs_ari,arm_subs_arr,arm_subs_rri,arm_subs_rrr,arm_subs_lli,arm_undefined,arm_subs_lri,arm_ldrh_ptim,arm_subs_ari,arm_ldrsb_ptim,arm_subs_rri,arm_ldrsh_ptim,
	arm_rsb_lli,arm_rsb_llr,arm_rsb_lri,arm_rsb_lrr,arm_rsb_ari,arm_rsb_arr,arm_rsb_rri,arm_rsb_rrr,arm_rsb_lli,arm_undefined,arm_rsb_lri,arm_strh_ptim,arm_rsb_ari,arm_undefined,arm_rsb_rri,arm_undefined,
	arm_rsbs_lli,arm_rsbs_llr,arm_rsbs_lri,arm_rsbs_lrr,arm_rsbs_ari,arm_rsbs_arr,arm_rsbs_rri,arm_rsbs_rrr,arm_rsbs_lli,arm_undefined,arm_rsbs_lri,arm_ldrh_ptim,arm_rsbs_ari,arm_ldrsb_ptim,arm_rsbs_rri,arm_ldrsh_ptim,
	arm_add_lli,arm_add_llr,arm_add_lri,arm_add_lrr,arm_add_ari,arm_add_arr,arm_add_rri,arm_add_rrr,arm_add_lli,arm_umull,arm_add_lri,arm_strh_ptrp,arm_add_ari,arm_undefined,arm_add_rri,arm_undefined,
	arm_adds_lli,arm_adds_llr,arm_adds_lri,arm_adds_lrr,arm_adds_ari,arm_adds_arr,arm_adds_rri,arm_adds_rrr,arm_adds_lli,arm_umulls,arm_adds_lri,arm_ldrh_ptrp,arm_adds_ari,arm_ldrsb_ptrp,arm_adds_rri,arm_ldrsh_ptrp,
	arm_adc_lli,arm_adc_llr,arm_adc_lri,arm_adc_lrr,arm_adc_ari,arm_adc_arr,arm_adc_rri,arm_adc_rrr,arm_adc_lli,arm_umlal,arm_adc_lri,arm_strh_ptrp,arm_adc_ari,arm_undefined,arm_adc_rri,arm_undefined,
	arm_adcs_lli,arm_adcs_llr,arm_adcs_lri,arm_adcs_lrr,arm_adcs_ari,arm_adcs_arr,arm_adcs_rri,arm_adcs_rrr,arm_adcs_lli,arm_umlals,arm_adcs_lri,arm_ldrh_ptrp,arm_adcs_ari,arm_ldrsb_ptrp,arm_adcs_rri,arm_ldrsh_ptrp,
	arm_sbc_lli,arm_sbc_llr,arm_sbc_lri,arm_sbc_lrr,arm_sbc_ari,arm_sbc_arr,arm_sbc_rri,arm_sbc_rrr,arm_sbc_lli,arm_smull,arm_sbc_lri,arm_strh_ptip,arm_sbc_ari,arm_undefined,arm_sbc_rri,arm_undefined,
	arm_sbcs_lli,arm_sbcs_llr,arm_sbcs_lri,arm_sbcs_lrr,arm_sbcs_ari,arm_sbcs_arr,arm_sbcs_rri,arm_sbcs_rrr,arm_sbcs_lli,arm_smulls,arm_sbcs_lri,arm_ldrh_ptip,arm_sbcs_ari,arm_ldrsb_ptip,arm_sbcs_rri,arm_ldrsh_ptip,
	arm_rsc_lli,arm_rsc_llr,arm_rsc_lri,arm_rsc_lrr,arm_rsc_ari,arm_rsc_arr,arm_rsc_rri,arm_rsc_rrr,arm_rsc_lli,arm_smlal,arm_rsc_lri,arm_strh_ptip,arm_rsc_ari,arm_undefined,arm_rsc_rri,arm_undefined,
	arm_rscs_lli,arm_rscs_llr,arm_rscs_lri,arm_rscs_lrr,arm_rscs_ari,arm_rscs_arr,arm_rscs_rri,arm_rscs_rrr,arm_rscs_lli,arm_smlals,arm_rscs_lri,arm_ldrh_ptip,arm_rscs_ari,arm_ldrsb_ptip,arm_rscs_rri,arm_ldrsh_ptip,
	arm_mrs_rc,arm_undefined,arm_undefined,arm_undefined,arm_undefined,arm_undefined,arm_undefined,arm_undefined,arm_undefined,arm_swp,arm_undefined,arm_strh_ofrm,arm_undefined,arm_undefined,arm_undefined,arm_undefined,
	arm_tsts_lli,arm_tsts_llr,arm_tsts_lri,arm_tsts_lrr,arm_tsts_ari,arm_tsts_arr,arm_tsts_rri,arm_tsts_rrr,arm_tsts_lli,arm_undefined,arm_tsts_lri,arm_ldrh_ofrm,arm_tsts_ari,arm_ldrsb_ofrm,arm_tsts_rri,arm_ldrsh_ofrm,
	arm_msr_rc,arm_bx,arm_undefined,arm_undefined,arm_undefined,arm_undefined,arm_undefined,arm_undefined,arm_undefined,arm_undefined,arm_undefined,arm_strh_prrm,arm_undefined,arm_undefined,arm_undefined,arm_undefined,
	arm_teqs_lli,arm_teqs_llr,arm_teqs_lri,arm_teqs_lrr,arm_teqs_ari,arm_teqs_arr,arm_teqs_rri,arm_teqs_rrr,arm_teqs_lli,arm_undefined,arm_teqs_lri,arm_ldrh_prrm,arm_teqs_ari,arm_ldrsb_prrm,arm_teqs_rri,arm_ldrsh_prrm,
	arm_mrs_rs,arm_undefined,arm_undefined,arm_undefined,arm_undefined,arm_undefined,arm_undefined,arm_undefined,arm_undefined,arm_swpb,arm_undefined,arm_strh_ofim,arm_undefined,arm_undefined,arm_undefined,arm_undefined,
	arm_cmps_lli,arm_cmps_llr,arm_cmps_lri,arm_cmps_lrr,arm_cmps_ari,arm_cmps_arr,arm_cmps_rri,arm_cmps_rrr,arm_cmps_lli,arm_undefined,arm_cmps_lri,arm_ldrh_ofim,arm_cmps_ari,arm_ldrsb_ofim,arm_cmps_rri,arm_ldrsh_ofim,
	arm_msr_rs,arm_undefined,arm_undefined,arm_undefined,arm_undefined,arm_undefined,arm_undefined,arm_undefined,arm_undefined,arm_undefined,arm_undefined,arm_strh_prim,arm_undefined,arm_undefined,arm_undefined,arm_undefined,
	arm_cmns_lli,arm_cmns_llr,arm_cmns_lri,arm_cmns_lrr,arm_cmns_ari,arm_cmns_arr,arm_cmns_rri,arm_cmns_rrr,arm_cmns_lli,arm_undefined,arm_cmns_lri,arm_ldrh_prim,arm_cmns_ari,arm_ldrsb_prim,arm_cmns_rri,arm_ldrsh_prim,
	arm_orr_lli,arm_orr_llr,arm_orr_lri,arm_orr_lrr,arm_orr_ari,arm_orr_arr,arm_orr_rri,arm_orr_rrr,arm_orr_lli,arm_undefined,arm_orr_lri,arm_strh_ofrp,arm_orr_ari,arm_undefined,arm_orr_rri,arm_undefined,
	arm_orrs_lli,arm_orrs_llr,arm_orrs_lri,arm_orrs_lrr,arm_orrs_ari,arm_orrs_arr,arm_orrs_rri,arm_orrs_rrr,arm_orrs_lli,arm_undefined,arm_orrs_lri,arm_ldrh_ofrp,arm_orrs_ari,arm_ldrsb_ofrp,arm_orrs_rri,arm_ldrsh_ofrp,
	arm_mov_lli,arm_mov_llr,arm_mov_lri,arm_mov_lrr,arm_mov_ari,arm_mov_arr,arm_mov_rri,arm_mov_rrr,arm_mov_lli,arm_undefined,arm_mov_lri,arm_strh_prrp,arm_mov_ari,arm_undefined,arm_mov_rri,arm_undefined,
	arm_movs_lli,arm_movs_llr,arm_movs_lri,arm_movs_lrr,arm_movs_ari,arm_movs_arr,arm_movs_rri,arm_movs_rrr,arm_movs_lli,arm_undefined,arm_movs_lri,arm_ldrh_prrp,arm_movs_ari,arm_ldrsb_prrp,arm_movs_rri,arm_ldrsh_prrp,
	arm_bic_lli,arm_bic_llr,arm_bic_lri,arm_bic_lrr,arm_bic_ari,arm_bic_arr,arm_bic_rri,arm_bic_rrr,arm_bic_lli,arm_undefined,arm_bic_lri,arm_strh_ofip,arm_bic_ari,arm_undefined,arm_bic_rri,arm_undefined,
	arm_bics_lli,arm_bics_llr,arm_bics_lri,arm_bics_lrr,arm_bics_ari,arm_bics_arr,arm_bics_rri,arm_bics_rrr,arm_bics_lli,arm_undefined,arm_bics_lri,arm_ldrh_ofip,arm_bics_ari,arm_ldrsb_ofip,arm_bics_rri,arm_ldrsh_ofip,
	arm_mvn_lli,arm_mvn_llr,arm_mvn_lri,arm_mvn_lrr,arm_mvn_ari,arm_mvn_arr,arm_mvn_rri,arm_mvn_rrr,arm_mvn_lli,arm_undefined,arm_mvn_lri,arm_strh_prip,arm_mvn_ari,arm_undefined,arm_mvn_rri,arm_undefined,
	arm_mvns_lli,arm_mvns_llr,arm_mvns_lri,arm_mvns_lrr,arm_mvns_ari,arm_mvns_arr,arm_mvns_rri,arm_mvns_rrr,arm_mvns_lli,arm_undefined,arm_mvns_lri,arm_ldrh_prip,arm_mvns_ari,arm_ldrsb_prip,arm_mvns_rri,arm_ldrsh_prip,
	arm_and_imm,arm_and_imm,arm_and_imm,arm_and_imm,arm_and_imm,arm_and_imm,arm_and_imm,arm_and_imm,arm_and_imm,arm_and_imm,arm_and_imm,arm_and_imm,arm_and_imm,arm_and_imm,arm_and_imm,arm_and_imm,
	arm_ands_imm,arm_ands_imm,arm_ands_imm,arm_ands_imm,arm_ands_imm,arm_ands_imm,arm_ands_imm,arm_ands_imm,arm_ands_imm,arm_ands_imm,arm_ands_imm,arm_ands_imm,arm_ands_imm,arm_ands_imm,arm_ands_imm,arm_ands_imm,
	arm_eor_imm,arm_eor_imm,arm_eor_imm,arm_eor_imm,arm_eor_imm,arm_eor_imm,arm_eor_imm,arm_eor_imm,arm_eor_imm,arm_eor_imm,arm_eor_imm,arm_eor_imm,arm_eor_imm,arm_eor_imm,arm_eor_imm,arm_eor_imm,
	arm_eors_imm,arm_eors_imm,arm_eors_imm,arm_eors_imm,arm_eors_imm,arm_eors_imm,arm_eors_imm,arm_eors_imm,arm_eors_imm,arm_eors_imm,arm_eors_imm,arm_eors_imm,arm_eors_imm,arm_eors_imm,arm_eors_imm,arm_eors_imm,
	arm_sub_imm,arm_sub_imm,arm_sub_imm,arm_sub_imm,arm_sub_imm,arm_sub_imm,arm_sub_imm,arm_sub_imm,arm_sub_imm,arm_sub_imm,arm_sub_imm,arm_sub_imm,arm_sub_imm,arm_sub_imm,arm_sub_imm,arm_sub_imm,
	arm_subs_imm,arm_subs_imm,arm_subs_imm,arm_subs_imm,arm_subs_imm,arm_subs_imm,arm_subs_imm,arm_subs_imm,arm_subs_imm,arm_subs_imm,arm_subs_imm,arm_subs_imm,arm_subs_imm,arm_subs_imm,arm_subs_imm,arm_subs_imm,
	arm_rsb_imm,arm_rsb_imm,arm_rsb_imm,arm_rsb_imm,arm_rsb_imm,arm_rsb_imm,arm_rsb_imm,arm_rsb_imm,arm_rsb_imm,arm_rsb_imm,arm_rsb_imm,arm_rsb_imm,arm_rsb_imm,arm_rsb_imm,arm_rsb_imm,arm_rsb_imm,
	arm_rsbs_imm,arm_rsbs_imm,arm_rsbs_imm,arm_rsbs_imm,arm_rsbs_imm,arm_rsbs_imm,arm_rsbs_imm,arm_rsbs_imm,arm_rsbs_imm,arm_rsbs_imm,arm_rsbs_imm,arm_rsbs_imm,arm_rsbs_imm,arm_rsbs_imm,arm_rsbs_imm,arm_rsbs_imm,
	arm_add_imm,arm_add_imm,arm_add_imm,arm_add_imm,arm_add_imm,arm_add_imm,arm_add_imm,arm_add_imm,arm_add_imm,arm_add_imm,arm_add_imm,arm_add_imm,arm_add_imm,arm_add_imm,arm_add_imm,arm_add_imm,
	arm_adds_imm,arm_adds_imm,arm_adds_imm,arm_adds_imm,arm_adds_imm,arm_adds_imm,arm_adds_imm,arm_adds_imm,arm_adds_imm,arm_adds_imm,arm_adds_imm,arm_adds_imm,arm_adds_imm,arm_adds_imm,arm_adds_imm,arm_adds_imm,
	arm_adc_imm,arm_adc_imm,arm_adc_imm,arm_adc_imm,arm_adc_imm,arm_adc_imm,arm_adc_imm,arm_adc_imm,arm_adc_imm,arm_adc_imm,arm_adc_imm,arm_adc_imm,arm_adc_imm,arm_adc_imm,arm_adc_imm,arm_adc_imm,
	arm_adcs_imm,arm_adcs_imm,arm_adcs_imm,arm_adcs_imm,arm_adcs_imm,arm_adcs_imm,arm_adcs_imm,arm_adcs_imm,arm_adcs_imm,arm_adcs_imm,arm_adcs_imm,arm_adcs_imm,arm_adcs_imm,arm_adcs_imm,arm_adcs_imm,arm_adcs_imm,
	arm_sbc_imm,arm_sbc_imm,arm_sbc_imm,arm_sbc_imm,arm_sbc_imm,arm_sbc_imm,arm_sbc_imm,arm_sbc_imm,arm_sbc_imm,arm_sbc_imm,arm_sbc_imm,arm_sbc_imm,arm_sbc_imm,arm_sbc_imm,arm_sbc_imm,arm_sbc_imm,
	arm_sbcs_imm,arm_sbcs_imm,arm_sbcs_imm,arm_sbcs_imm,arm_sbcs_imm,arm_sbcs_imm,arm_sbcs_imm,arm_sbcs_imm,arm_sbcs_imm,arm_sbcs_imm,arm_sbcs_imm,arm_sbcs_imm,arm_sbcs_imm,arm_sbcs_imm,arm_sbcs_imm,arm_sbcs_imm,
	arm_rsc_imm,arm_rsc_imm,arm_rsc_imm,arm_rsc_imm,arm_rsc_imm,arm_rsc_imm,arm_rsc_imm,arm_rsc_imm,arm_rsc_imm,arm_rsc_imm,arm_rsc_imm,arm_rsc_imm,arm_rsc_imm,arm_rsc_imm,arm_rsc_imm,arm_rsc_imm,
	arm_rscs_imm,arm_rscs_imm,arm_rscs_imm,arm_rscs_imm,arm_rscs_imm,arm_rscs_imm,arm_rscs_imm,arm_rscs_imm,arm_rscs_imm,arm_rscs_imm,arm_rscs_imm,arm_rscs_imm,arm_rscs_imm,arm_rscs_imm,arm_rscs_imm,arm_rscs_imm,
	arm_undefined,arm_undefined,arm_undefined,arm_undefined,arm_undefined,arm_undefined,arm_undefined,arm_undefined,arm_undefined,arm_undefined,arm_undefined,arm_undefined,arm_undefined,arm_undefined,arm_undefined,arm_undefined,
	arm_tsts_imm,arm_tsts_imm,arm_tsts_imm,arm_tsts_imm,arm_tsts_imm,arm_tsts_imm,arm_tsts_imm,arm_tsts_imm,arm_tsts_imm,arm_tsts_imm,arm_tsts_imm,arm_tsts_imm,arm_tsts_imm,arm_tsts_imm,arm_tsts_imm,arm_tsts_imm,
	arm_msr_ic,arm_msr_ic,arm_msr_ic,arm_msr_ic,arm_msr_ic,arm_msr_ic,arm_msr_ic,arm_msr_ic,arm_msr_ic,arm_msr_ic,arm_msr_ic,arm_msr_ic,arm_msr_ic,arm_msr_ic,arm_msr_ic,arm_msr_ic,
	arm_teqs_imm,arm_teqs_imm,arm_teqs_imm,arm_teqs_imm,arm_teqs_imm,arm_teqs_imm,arm_teqs_imm,arm_teqs_imm,arm_teqs_imm,arm_teqs_imm,arm_teqs_imm,arm_teqs_imm,arm_teqs_imm,arm_teqs_imm,arm_teqs_imm,arm_teqs_imm,
	arm_undefined,arm_undefined,arm_undefined,arm_undefined,arm_undefined,arm_undefined,arm_undefined,arm_undefined,arm_undefined,arm_undefined,arm_undefined,arm_undefined,arm_undefined,arm_undefined,arm_undefined,arm_undefined,
	arm_cmps_imm,arm_cmps_imm,arm_cmps_imm,arm_cmps_imm,arm_cmps_imm,arm_cmps_imm,arm_cmps_imm,arm_cmps_imm,arm_cmps_imm,arm_cmps_imm,arm_cmps_imm,arm_cmps_imm,arm_cmps_imm,arm_cmps_imm,arm_cmps_imm,arm_cmps_imm,
	arm_msr_is,arm_msr_is,arm_msr_is,arm_msr_is,arm_msr_is,arm_msr_is,arm_msr_is,arm_msr_is,arm_msr_is,arm_msr_is,arm_msr_is,arm_msr_is,arm_msr_is,arm_msr_is,arm_msr_is,arm_msr_is,
	arm_cmns_imm,arm_cmns_imm,arm_cmns_imm,arm_cmns_imm,arm_cmns_imm,arm_cmns_imm,arm_cmns_imm,arm_cmns_imm,arm_cmns_imm,arm_cmns_imm,arm_cmns_imm,arm_cmns_imm,arm_cmns_imm,arm_cmns_imm,arm_cmns_imm,arm_cmns_imm,
	arm_orr_imm,arm_orr_imm,arm_orr_imm,arm_orr_imm,arm_orr_imm,arm_orr_imm,arm_orr_imm,arm_orr_imm,arm_orr_imm,arm_orr_imm,arm_orr_imm,arm_orr_imm,arm_orr_imm,arm_orr_imm,arm_orr_imm,arm_orr_imm,
	arm_orrs_imm,arm_orrs_imm,arm_orrs_imm,arm_orrs_imm,arm_orrs_imm,arm_orrs_imm,arm_orrs_imm,arm_orrs_imm,arm_orrs_imm,arm_orrs_imm,arm_orrs_imm,arm_orrs_imm,arm_orrs_imm,arm_orrs_imm,arm_orrs_imm,arm_orrs_imm,
	arm_mov_imm,arm_mov_imm,arm_mov_imm,arm_mov_imm,arm_mov_imm,arm_mov_imm,arm_mov_imm,arm_mov_imm,arm_mov_imm,arm_mov_imm,arm_mov_imm,arm_mov_imm,arm_mov_imm,arm_mov_imm,arm_mov_imm,arm_mov_imm,
	arm_movs_imm,arm_movs_imm,arm_movs_imm,arm_movs_imm,arm_movs_imm,arm_movs_imm,arm_movs_imm,arm_movs_imm,arm_movs_imm,arm_movs_imm,arm_movs_imm,arm_movs_imm,arm_movs_imm,arm_movs_imm,arm_movs_imm,arm_movs_imm,
	arm_bic_imm,arm_bic_imm,arm_bic_imm,arm_bic_imm,arm_bic_imm,arm_bic_imm,arm_bic_imm,arm_bic_imm,arm_bic_imm,arm_bic_imm,arm_bic_imm,arm_bic_imm,arm_bic_imm,arm_bic_imm,arm_bic_imm,arm_bic_imm,
	arm_bics_imm,arm_bics_imm,arm_bics_imm,arm_bics_imm,arm_bics_imm,arm_bics_imm,arm_bics_imm,arm_bics_imm,arm_bics_imm,arm_bics_imm,arm_bics_imm,arm_bics_imm,arm_bics_imm,arm_bics_imm,arm_bics_imm,arm_bics_imm,
	arm_mvn_imm,arm_mvn_imm,arm_mvn_imm,arm_mvn_imm,arm_mvn_imm,arm_mvn_imm,arm_mvn_imm,arm_mvn_imm,arm_mvn_imm,arm_mvn_imm,arm_mvn_imm,arm_mvn_imm,arm_mvn_imm,arm_mvn_imm,arm_mvn_imm,arm_mvn_imm,
	arm_mvns_imm,arm_mvns_imm,arm_mvns_imm,arm_mvns_imm,arm_mvns_imm,arm_mvns_imm,arm_mvns_imm,arm_mvns_imm,arm_mvns_imm,arm_mvns_imm,arm_mvns_imm,arm_mvns_imm,arm_mvns_imm,arm_mvns_imm,arm_mvns_imm,arm_mvns_imm,
	arm_str_ptim,arm_str_ptim,arm_str_ptim,arm_str_ptim,arm_str_ptim,arm_str_ptim,arm_str_ptim,arm_str_ptim,arm_str_ptim,arm_str_ptim,arm_str_ptim,arm_str_ptim,arm_str_ptim,arm_str_ptim,arm_str_ptim,arm_str_ptim,
	arm_ldr_ptim,arm_ldr_ptim,arm_ldr_ptim,arm_ldr_ptim,arm_ldr_ptim,arm_ldr_ptim,arm_ldr_ptim,arm_ldr_ptim,arm_ldr_ptim,arm_ldr_ptim,arm_ldr_ptim,arm_ldr_ptim,arm_ldr_ptim,arm_ldr_ptim,arm_ldr_ptim,arm_ldr_ptim,
	arm_strt_ptim,arm_strt_ptim,arm_strt_ptim,arm_strt_ptim,arm_strt_ptim,arm_strt_ptim,arm_strt_ptim,arm_strt_ptim,arm_strt_ptim,arm_strt_ptim,arm_strt_ptim,arm_strt_ptim,arm_strt_ptim,arm_strt_ptim,arm_strt_ptim,arm_strt_ptim,
	arm_ldrt_ptim,arm_ldrt_ptim,arm_ldrt_ptim,arm_ldrt_ptim,arm_ldrt_ptim,arm_ldrt_ptim,arm_ldrt_ptim,arm_ldrt_ptim,arm_ldrt_ptim,arm_ldrt_ptim,arm_ldrt_ptim,arm_ldrt_ptim,arm_ldrt_ptim,arm_ldrt_ptim,arm_ldrt_ptim,arm_ldrt_ptim,
	arm_strb_ptim,arm_strb_ptim,arm_strb_ptim,arm_strb_ptim,arm_strb_ptim,arm_strb_ptim,arm_strb_ptim,arm_strb_ptim,arm_strb_ptim,arm_strb_ptim,arm_strb_ptim,arm_strb_ptim,arm_strb_ptim,arm_strb_ptim,arm_strb_ptim,arm_strb_ptim,
	arm_ldrb_ptim,arm_ldrb_ptim,arm_ldrb_ptim,arm_ldrb_ptim,arm_ldrb_ptim,arm_ldrb_ptim,arm_ldrb_ptim,arm_ldrb_ptim,arm_ldrb_ptim,arm_ldrb_ptim,arm_ldrb_ptim,arm_ldrb_ptim,arm_ldrb_ptim,arm_ldrb_ptim,arm_ldrb_ptim,arm_ldrb_ptim,
	arm_strbt_ptim,arm_strbt_ptim,arm_strbt_ptim,arm_strbt_ptim,arm_strbt_ptim,arm_strbt_ptim,arm_strbt_ptim,arm_strbt_ptim,arm_strbt_ptim,arm_strbt_ptim,arm_strbt_ptim,arm_strbt_ptim,arm_strbt_ptim,arm_strbt_ptim,arm_strbt_ptim,arm_strbt_ptim,
	arm_ldrbt_ptim,arm_ldrbt_ptim,arm_ldrbt_ptim,arm_ldrbt_ptim,arm_ldrbt_ptim,arm_ldrbt_ptim,arm_ldrbt_ptim,arm_ldrbt_ptim,arm_ldrbt_ptim,arm_ldrbt_ptim,arm_ldrbt_ptim,arm_ldrbt_ptim,arm_ldrbt_ptim,arm_ldrbt_ptim,arm_ldrbt_ptim,arm_ldrbt_ptim,
	arm_str_ptip,arm_str_ptip,arm_str_ptip,arm_str_ptip,arm_str_ptip,arm_str_ptip,arm_str_ptip,arm_str_ptip,arm_str_ptip,arm_str_ptip,arm_str_ptip,arm_str_ptip,arm_str_ptip,arm_str_ptip,arm_str_ptip,arm_str_ptip,
	arm_ldr_ptip,arm_ldr_ptip,arm_ldr_ptip,arm_ldr_ptip,arm_ldr_ptip,arm_ldr_ptip,arm_ldr_ptip,arm_ldr_ptip,arm_ldr_ptip,arm_ldr_ptip,arm_ldr_ptip,arm_ldr_ptip,arm_ldr_ptip,arm_ldr_ptip,arm_ldr_ptip,arm_ldr_ptip,
	arm_strt_ptip,arm_strt_ptip,arm_strt_ptip,arm_strt_ptip,arm_strt_ptip,arm_strt_ptip,arm_strt_ptip,arm_strt_ptip,arm_strt_ptip,arm_strt_ptip,arm_strt_ptip,arm_strt_ptip,arm_strt_ptip,arm_strt_ptip,arm_strt_ptip,arm_strt_ptip,
	arm_ldrt_ptip,arm_ldrt_ptip,arm_ldrt_ptip,arm_ldrt_ptip,arm_ldrt_ptip,arm_ldrt_ptip,arm_ldrt_ptip,arm_ldrt_ptip,arm_ldrt_ptip,arm_ldrt_ptip,arm_ldrt_ptip,arm_ldrt_ptip,arm_ldrt_ptip,arm_ldrt_ptip,arm_ldrt_ptip,arm_ldrt_ptip,
	arm_strb_ptip,arm_strb_ptip,arm_strb_ptip,arm_strb_ptip,arm_strb_ptip,arm_strb_ptip,arm_strb_ptip,arm_strb_ptip,arm_strb_ptip,arm_strb_ptip,arm_strb_ptip,arm_strb_ptip,arm_strb_ptip,arm_strb_ptip,arm_strb_ptip,arm_strb_ptip,
	arm_ldrb_ptip,arm_ldrb_ptip,arm_ldrb_ptip,arm_ldrb_ptip,arm_ldrb_ptip,arm_ldrb_ptip,arm_ldrb_ptip,arm_ldrb_ptip,arm_ldrb_ptip,arm_ldrb_ptip,arm_ldrb_ptip,arm_ldrb_ptip,arm_ldrb_ptip,arm_ldrb_ptip,arm_ldrb_ptip,arm_ldrb_ptip,
	arm_strbt_ptip,arm_strbt_ptip,arm_strbt_ptip,arm_strbt_ptip,arm_strbt_ptip,arm_strbt_ptip,arm_strbt_ptip,arm_strbt_ptip,arm_strbt_ptip,arm_strbt_ptip,arm_strbt_ptip,arm_strbt_ptip,arm_strbt_ptip,arm_strbt_ptip,arm_strbt_ptip,arm_strbt_ptip,
	arm_ldrbt_ptip,arm_ldrbt_ptip,arm_ldrbt_ptip,arm_ldrbt_ptip,arm_ldrbt_ptip,arm_ldrbt_ptip,arm_ldrbt_ptip,arm_ldrbt_ptip,arm_ldrbt_ptip,arm_ldrbt_ptip,arm_ldrbt_ptip,arm_ldrbt_ptip,arm_ldrbt_ptip,arm_ldrbt_ptip,arm_ldrbt_ptip,arm_ldrbt_ptip,
	arm_str_ofim,arm_str_ofim,arm_str_ofim,arm_str_ofim,arm_str_ofim,arm_str_ofim,arm_str_ofim,arm_str_ofim,arm_str_ofim,arm_str_ofim,arm_str_ofim,arm_str_ofim,arm_str_ofim,arm_str_ofim,arm_str_ofim,arm_str_ofim,
	arm_ldr_ofim,arm_ldr_ofim,arm_ldr_ofim,arm_ldr_ofim,arm_ldr_ofim,arm_ldr_ofim,arm_ldr_ofim,arm_ldr_ofim,arm_ldr_ofim,arm_ldr_ofim,arm_ldr_ofim,arm_ldr_ofim,arm_ldr_ofim,arm_ldr_ofim,arm_ldr_ofim,arm_ldr_ofim,
	arm_str_prim,arm_str_prim,arm_str_prim,arm_str_prim,arm_str_prim,arm_str_prim,arm_str_prim,arm_str_prim,arm_str_prim,arm_str_prim,arm_str_prim,arm_str_prim,arm_str_prim,arm_str_prim,arm_str_prim,arm_str_prim,
	arm_ldr_prim,arm_ldr_prim,arm_ldr_prim,arm_ldr_prim,arm_ldr_prim,arm_ldr_prim,arm_ldr_prim,arm_ldr_prim,arm_ldr_prim,arm_ldr_prim,arm_ldr_prim,arm_ldr_prim,arm_ldr_prim,arm_ldr_prim,arm_ldr_prim,arm_ldr_prim,
	arm_strb_ofim,arm_strb_ofim,arm_strb_ofim,arm_strb_ofim,arm_strb_ofim,arm_strb_ofim,arm_strb_ofim,arm_strb_ofim,arm_strb_ofim,arm_strb_ofim,arm_strb_ofim,arm_strb_ofim,arm_strb_ofim,arm_strb_ofim,arm_strb_ofim,arm_strb_ofim,
	arm_ldrb_ofim,arm_ldrb_ofim,arm_ldrb_ofim,arm_ldrb_ofim,arm_ldrb_ofim,arm_ldrb_ofim,arm_ldrb_ofim,arm_ldrb_ofim,arm_ldrb_ofim,arm_ldrb_ofim,arm_ldrb_ofim,arm_ldrb_ofim,arm_ldrb_ofim,arm_ldrb_ofim,arm_ldrb_ofim,arm_ldrb_ofim,
	arm_strb_prim,arm_strb_prim,arm_strb_prim,arm_strb_prim,arm_strb_prim,arm_strb_prim,arm_strb_prim,arm_strb_prim,arm_strb_prim,arm_strb_prim,arm_strb_prim,arm_strb_prim,arm_strb_prim,arm_strb_prim,arm_strb_prim,arm_strb_prim,
	arm_ldrb_prim,arm_ldrb_prim,arm_ldrb_prim,arm_ldrb_prim,arm_ldrb_prim,arm_ldrb_prim,arm_ldrb_prim,arm_ldrb_prim,arm_ldrb_prim,arm_ldrb_prim,arm_ldrb_prim,arm_ldrb_prim,arm_ldrb_prim,arm_ldrb_prim,arm_ldrb_prim,arm_ldrb_prim,
	arm_str_ofip,arm_str_ofip,arm_str_ofip,arm_str_ofip,arm_str_ofip,arm_str_ofip,arm_str_ofip,arm_str_ofip,arm_str_ofip,arm_str_ofip,arm_str_ofip,arm_str_ofip,arm_str_ofip,arm_str_ofip,arm_str_ofip,arm_str_ofip,
	arm_ldr_ofip,arm_ldr_ofip,arm_ldr_ofip,arm_ldr_ofip,arm_ldr_ofip,arm_ldr_ofip,arm_ldr_ofip,arm_ldr_ofip,arm_ldr_ofip,arm_ldr_ofip,arm_ldr_ofip,arm_ldr_ofip,arm_ldr_ofip,arm_ldr_ofip,arm_ldr_ofip,arm_ldr_ofip,
	arm_str_prip,arm_str_prip,arm_str_prip,arm_str_prip,arm_str_prip,arm_str_prip,arm_str_prip,arm_str_prip,arm_str_prip,arm_str_prip,arm_str_prip,arm_str_prip,arm_str_prip,arm_str_prip,arm_str_prip,arm_str_prip,
	arm_ldr_prip,arm_ldr_prip,arm_ldr_prip,arm_ldr_prip,arm_ldr_prip,arm_ldr_prip,arm_ldr_prip,arm_ldr_prip,arm_ldr_prip,arm_ldr_prip,arm_ldr_prip,arm_ldr_prip,arm_ldr_prip,arm_ldr_prip,arm_ldr_prip,arm_ldr_prip,
	arm_strb_ofip,arm_strb_ofip,arm_strb_ofip,arm_strb_ofip,arm_strb_ofip,arm_strb_ofip,arm_strb_ofip,arm_strb_ofip,arm_strb_ofip,arm_strb_ofip,arm_strb_ofip,arm_strb_ofip,arm_strb_ofip,arm_strb_ofip,arm_strb_ofip,arm_strb_ofip,
	arm_ldrb_ofip,arm_ldrb_ofip,arm_ldrb_ofip,arm_ldrb_ofip,arm_ldrb_ofip,arm_ldrb_ofip,arm_ldrb_ofip,arm_ldrb_ofip,arm_ldrb_ofip,arm_ldrb_ofip,arm_ldrb_ofip,arm_ldrb_ofip,arm_ldrb_ofip,arm_ldrb_ofip,arm_ldrb_ofip,arm_ldrb_ofip,
	arm_strb_prip,arm_strb_prip,arm_strb_prip,arm_strb_prip,arm_strb_prip,arm_strb_prip,arm_strb_prip,arm_strb_prip,arm_strb_prip,arm_strb_prip,arm_strb_prip,arm_strb_prip,arm_strb_prip,arm_strb_prip,arm_strb_prip,arm_strb_prip,
	arm_ldrb_prip,arm_ldrb_prip,arm_ldrb_prip,arm_ldrb_prip,arm_ldrb_prip,arm_ldrb_prip,arm_ldrb_prip,arm_ldrb_prip,arm_ldrb_prip,arm_ldrb_prip,arm_ldrb_prip,arm_ldrb_prip,arm_ldrb_prip,arm_ldrb_prip,arm_ldrb_prip,arm_ldrb_prip,
	arm_str_ptrmll,arm_undefined,arm_str_ptrmlr,arm_undefined,arm_str_ptrmar,arm_undefined,arm_str_ptrmrr,arm_undefined,arm_str_ptrmll,arm_undefined,arm_str_ptrmlr,arm_undefined,arm_str_ptrmar,arm_undefined,arm_str_ptrmrr,arm_undefined,
	arm_ldr_ptrmll,arm_undefined,arm_ldr_ptrmlr,arm_undefined,arm_ldr_ptrmar,arm_undefined,arm_ldr_ptrmrr,arm_undefined,arm_ldr_ptrmll,arm_undefined,arm_ldr_ptrmlr,arm_undefined,arm_ldr_ptrmar,arm_undefined,arm_ldr_ptrmrr,arm_undefined,
	arm_strt_ptrmll,arm_undefined,arm_strt_ptrmlr,arm_undefined,arm_strt_ptrmar,arm_undefined,arm_strt_ptrmrr,arm_undefined,arm_strt_ptrmll,arm_undefined,arm_strt_ptrmlr,arm_undefined,arm_strt_ptrmar,arm_undefined,arm_strt_ptrmrr,arm_undefined,
	arm_ldrt_ptrmll,arm_undefined,arm_ldrt_ptrmlr,arm_undefined,arm_ldrt_ptrmar,arm_undefined,arm_ldrt_ptrmrr,arm_undefined,arm_ldrt_ptrmll,arm_undefined,arm_ldrt_ptrmlr,arm_undefined,arm_ldrt_ptrmar,arm_undefined,arm_ldrt_ptrmrr,arm_undefined,
	arm_strb_ptrmll,arm_undefined,arm_strb_ptrmlr,arm_undefined,arm_strb_ptrmar,arm_undefined,arm_strb_ptrmrr,arm_undefined,arm_strb_ptrmll,arm_undefined,arm_strb_ptrmlr,arm_undefined,arm_strb_ptrmar,arm_undefined,arm_strb_ptrmrr,arm_undefined,
	arm_ldrb_ptrmll,arm_undefined,arm_ldrb_ptrmlr,arm_undefined,arm_ldrb_ptrmar,arm_undefined,arm_ldrb_ptrmrr,arm_undefined,arm_ldrb_ptrmll,arm_undefined,arm_ldrb_ptrmlr,arm_undefined,arm_ldrb_ptrmar,arm_undefined,arm_ldrb_ptrmrr,arm_undefined,
	arm_strbt_ptrmll,arm_undefined,arm_strbt_ptrmlr,arm_undefined,arm_strbt_ptrmar,arm_undefined,arm_strbt_ptrmrr,arm_undefined,arm_strbt_ptrmll,arm_undefined,arm_strbt_ptrmlr,arm_undefined,arm_strbt_ptrmar,arm_undefined,arm_strbt_ptrmrr,arm_undefined,
	arm_ldrbt_ptrmll,arm_undefined,arm_ldrbt_ptrmlr,arm_undefined,arm_ldrbt_ptrmar,arm_undefined,arm_ldrbt_ptrmrr,arm_undefined,arm_ldrbt_ptrmll,arm_undefined,arm_ldrbt_ptrmlr,arm_undefined,arm_ldrbt_ptrmar,arm_undefined,arm_ldrbt_ptrmrr,arm_undefined,
	arm_str_ptrpll,arm_undefined,arm_str_ptrplr,arm_undefined,arm_str_ptrpar,arm_undefined,arm_str_ptrprr,arm_undefined,arm_str_ptrpll,arm_undefined,arm_str_ptrplr,arm_undefined,arm_str_ptrpar,arm_undefined,arm_str_ptrprr,arm_undefined,
	arm_ldr_ptrpll,arm_undefined,arm_ldr_ptrplr,arm_undefined,arm_ldr_ptrpar,arm_undefined,arm_ldr_ptrprr,arm_undefined,arm_ldr_ptrpll,arm_undefined,arm_ldr_ptrplr,arm_undefined,arm_ldr_ptrpar,arm_undefined,arm_ldr_ptrprr,arm_undefined,
	arm_strt_ptrpll,arm_undefined,arm_strt_ptrplr,arm_undefined,arm_strt_ptrpar,arm_undefined,arm_strt_ptrprr,arm_undefined,arm_strt_ptrpll,arm_undefined,arm_strt_ptrplr,arm_undefined,arm_strt_ptrpar,arm_undefined,arm_strt_ptrprr,arm_undefined,
	arm_ldrt_ptrpll,arm_undefined,arm_ldrt_ptrplr,arm_undefined,arm_ldrt_ptrpar,arm_undefined,arm_ldrt_ptrprr,arm_undefined,arm_ldrt_ptrpll,arm_undefined,arm_ldrt_ptrplr,arm_undefined,arm_ldrt_ptrpar,arm_undefined,arm_ldrt_ptrprr,arm_undefined,
	arm_strb_ptrpll,arm_undefined,arm_strb_ptrplr,arm_undefined,arm_strb_ptrpar,arm_undefined,arm_strb_ptrprr,arm_undefined,arm_strb_ptrpll,arm_undefined,arm_strb_ptrplr,arm_undefined,arm_strb_ptrpar,arm_undefined,arm_strb_ptrprr,arm_undefined,
	arm_ldrb_ptrpll,arm_undefined,arm_ldrb_ptrplr,arm_undefined,arm_ldrb_ptrpar,arm_undefined,arm_ldrb_ptrprr,arm_undefined,arm_ldrb_ptrpll,arm_undefined,arm_ldrb_ptrplr,arm_undefined,arm_ldrb_ptrpar,arm_undefined,arm_ldrb_ptrprr,arm_undefined,
	arm_strbt_ptrpll,arm_undefined,arm_strbt_ptrplr,arm_undefined,arm_strbt_ptrpar,arm_undefined,arm_strbt_ptrprr,arm_undefined,arm_strbt_ptrpll,arm_undefined,arm_strbt_ptrplr,arm_undefined,arm_strbt_ptrpar,arm_undefined,arm_strbt_ptrprr,arm_undefined,
	arm_ldrbt_ptrpll,arm_undefined,arm_ldrbt_ptrplr,arm_undefined,arm_ldrbt_ptrpar,arm_undefined,arm_ldrbt_ptrprr,arm_undefined,arm_ldrbt_ptrpll,arm_undefined,arm_ldrbt_ptrplr,arm_undefined,arm_ldrbt_ptrpar,arm_undefined,arm_ldrbt_ptrprr,arm_undefined,
	arm_str_ofrmll,arm_undefined,arm_str_ofrmlr,arm_undefined,arm_str_ofrmar,arm_undefined,arm_str_ofrmrr,arm_undefined,arm_str_ofrmll,arm_undefined,arm_str_ofrmlr,arm_undefined,arm_str_ofrmar,arm_undefined,arm_str_ofrmrr,arm_undefined,
	arm_ldr_ofrmll,arm_undefined,arm_ldr_ofrmlr,arm_undefined,arm_ldr_ofrmar,arm_undefined,arm_ldr_ofrmrr,arm_undefined,arm_ldr_ofrmll,arm_undefined,arm_ldr_ofrmlr,arm_undefined,arm_ldr_ofrmar,arm_undefined,arm_ldr_ofrmrr,arm_undefined,
	arm_str_prrmll,arm_undefined,arm_str_prrmlr,arm_undefined,arm_str_prrmar,arm_undefined,arm_str_prrmrr,arm_undefined,arm_str_prrmll,arm_undefined,arm_str_prrmlr,arm_undefined,arm_str_prrmar,arm_undefined,arm_str_prrmrr,arm_undefined,
	arm_ldr_prrmll,arm_undefined,arm_ldr_prrmlr,arm_undefined,arm_ldr_prrmar,arm_undefined,arm_ldr_prrmrr,arm_undefined,arm_ldr_prrmll,arm_undefined,arm_ldr_prrmlr,arm_undefined,arm_ldr_prrmar,arm_undefined,arm_ldr_prrmrr,arm_undefined,
	arm_strb_ofrmll,arm_undefined,arm_strb_ofrmlr,arm_undefined,arm_strb_ofrmar,arm_undefined,arm_strb_ofrmrr,arm_undefined,arm_strb_ofrmll,arm_undefined,arm_strb_ofrmlr,arm_undefined,arm_strb_ofrmar,arm_undefined,arm_strb_ofrmrr,arm_undefined,
	arm_ldrb_ofrmll,arm_undefined,arm_ldrb_ofrmlr,arm_undefined,arm_ldrb_ofrmar,arm_undefined,arm_ldrb_ofrmrr,arm_undefined,arm_ldrb_ofrmll,arm_undefined,arm_ldrb_ofrmlr,arm_undefined,arm_ldrb_ofrmar,arm_undefined,arm_ldrb_ofrmrr,arm_undefined,
	arm_strb_prrmll,arm_undefined,arm_strb_prrmlr,arm_undefined,arm_strb_prrmar,arm_undefined,arm_strb_prrmrr,arm_undefined,arm_strb_prrmll,arm_undefined,arm_strb_prrmlr,arm_undefined,arm_strb_prrmar,arm_undefined,arm_strb_prrmrr,arm_undefined,
	arm_ldrb_prrmll,arm_undefined,arm_ldrb_prrmlr,arm_undefined,arm_ldrb_prrmar,arm_undefined,arm_ldrb_prrmrr,arm_undefined,arm_ldrb_prrmll,arm_undefined,arm_ldrb_prrmlr,arm_undefined,arm_ldrb_prrmar,arm_undefined,arm_ldrb_prrmrr,arm_undefined,
	arm_str_ofrpll,arm_undefined,arm_str_ofrplr,arm_undefined,arm_str_ofrpar,arm_undefined,arm_str_ofrprr,arm_undefined,arm_str_ofrpll,arm_undefined,arm_str_ofrplr,arm_undefined,arm_str_ofrpar,arm_undefined,arm_str_ofrprr,arm_undefined,
	arm_ldr_ofrpll,arm_undefined,arm_ldr_ofrplr,arm_undefined,arm_ldr_ofrpar,arm_undefined,arm_ldr_ofrprr,arm_undefined,arm_ldr_ofrpll,arm_undefined,arm_ldr_ofrplr,arm_undefined,arm_ldr_ofrpar,arm_undefined,arm_ldr_ofrprr,arm_undefined,
	arm_str_prrpll,arm_undefined,arm_str_prrplr,arm_undefined,arm_str_prrpar,arm_undefined,arm_str_prrprr,arm_undefined,arm_str_prrpll,arm_undefined,arm_str_prrplr,arm_undefined,arm_str_prrpar,arm_undefined,arm_str_prrprr,arm_undefined,
	arm_ldr_prrpll,arm_undefined,arm_ldr_prrplr,arm_undefined,arm_ldr_prrpar,arm_undefined,arm_ldr_prrprr,arm_undefined,arm_ldr_prrpll,arm_undefined,arm_ldr_prrplr,arm_undefined,arm_ldr_prrpar,arm_undefined,arm_ldr_prrprr,arm_undefined,
	arm_strb_ofrpll,arm_undefined,arm_strb_ofrplr,arm_undefined,arm_strb_ofrpar,arm_undefined,arm_strb_ofrprr,arm_undefined,arm_strb_ofrpll,arm_undefined,arm_strb_ofrplr,arm_undefined,arm_strb_ofrpar,arm_undefined,arm_strb_ofrprr,arm_undefined,
	arm_ldrb_ofrpll,arm_undefined,arm_ldrb_ofrplr,arm_undefined,arm_ldrb_ofrpar,arm_undefined,arm_ldrb_ofrprr,arm_undefined,arm_ldrb_ofrpll,arm_undefined,arm_ldrb_ofrplr,arm_undefined,arm_ldrb_ofrpar,arm_undefined,arm_ldrb_ofrprr,arm_undefined,
	arm_strb_prrpll,arm_undefined,arm_strb_prrplr,arm_undefined,arm_strb_prrpar,arm_undefined,arm_strb_prrprr,arm_undefined,arm_strb_prrpll,arm_undefined,arm_strb_prrplr,arm_undefined,arm_strb_prrpar,arm_undefined,arm_strb_prrprr,arm_undefined,
	arm_ldrb_prrpll,arm_undefined,arm_ldrb_prrplr,arm_undefined,arm_ldrb_prrpar,arm_undefined,arm_ldrb_prrprr,arm_undefined,arm_ldrb_prrpll,arm_undefined,arm_ldrb_prrplr,arm_undefined,arm_ldrb_prrpar,arm_undefined,arm_ldrb_prrprr,arm_undefined,
	arm_stmda,arm_stmda,arm_stmda,arm_stmda,arm_stmda,arm_stmda,arm_stmda,arm_stmda,arm_stmda,arm_stmda,arm_stmda,arm_stmda,arm_stmda,arm_stmda,arm_stmda,arm_stmda,
	arm_ldmda,arm_ldmda,arm_ldmda,arm_ldmda,arm_ldmda,arm_ldmda,arm_ldmda,arm_ldmda,arm_ldmda,arm_ldmda,arm_ldmda,arm_ldmda,arm_ldmda,arm_ldmda,arm_ldmda,arm_ldmda,
	arm_stmda_w,arm_stmda_w,arm_stmda_w,arm_stmda_w,arm_stmda_w,arm_stmda_w,arm_stmda_w,arm_stmda_w,arm_stmda_w,arm_stmda_w,arm_stmda_w,arm_stmda_w,arm_stmda_w,arm_stmda_w,arm_stmda_w,arm_stmda_w,
	arm_ldmda_w,arm_ldmda_w,arm_ldmda_w,arm_ldmda_w,arm_ldmda_w,arm_ldmda_w,arm_ldmda_w,arm_ldmda_w,arm_ldmda_w,arm_ldmda_w,arm_ldmda_w,arm_ldmda_w,arm_ldmda_w,arm_ldmda_w,arm_ldmda_w,arm_ldmda_w,
	arm_stmda_u,arm_stmda_u,arm_stmda_u,arm_stmda_u,arm_stmda_u,arm_stmda_u,arm_stmda_u,arm_stmda_u,arm_stmda_u,arm_stmda_u,arm_stmda_u,arm_stmda_u,arm_stmda_u,arm_stmda_u,arm_stmda_u,arm_stmda_u,
	arm_ldmda_u,arm_ldmda_u,arm_ldmda_u,arm_ldmda_u,arm_ldmda_u,arm_ldmda_u,arm_ldmda_u,arm_ldmda_u,arm_ldmda_u,arm_ldmda_u,arm_ldmda_u,arm_ldmda_u,arm_ldmda_u,arm_ldmda_u,arm_ldmda_u,arm_ldmda_u,
	arm_stmda_uw,arm_stmda_uw,arm_stmda_uw,arm_stmda_uw,arm_stmda_uw,arm_stmda_uw,arm_stmda_uw,arm_stmda_uw,arm_stmda_uw,arm_stmda_uw,arm_stmda_uw,arm_stmda_uw,arm_stmda_uw,arm_stmda_uw,arm_stmda_uw,arm_stmda_uw,
	arm_ldmda_uw,arm_ldmda_uw,arm_ldmda_uw,arm_ldmda_uw,arm_ldmda_uw,arm_ldmda_uw,arm_ldmda_uw,arm_ldmda_uw,arm_ldmda_uw,arm_ldmda_uw,arm_ldmda_uw,arm_ldmda_uw,arm_ldmda_uw,arm_ldmda_uw,arm_ldmda_uw,arm_ldmda_uw,
	arm_stmia,arm_stmia,arm_stmia,arm_stmia,arm_stmia,arm_stmia,arm_stmia,arm_stmia,arm_stmia,arm_stmia,arm_stmia,arm_stmia,arm_stmia,arm_stmia,arm_stmia,arm_stmia,
	arm_ldmia,arm_ldmia,arm_ldmia,arm_ldmia,arm_ldmia,arm_ldmia,arm_ldmia,arm_ldmia,arm_ldmia,arm_ldmia,arm_ldmia,arm_ldmia,arm_ldmia,arm_ldmia,arm_ldmia,arm_ldmia,
	arm_stmia_w,arm_stmia_w,arm_stmia_w,arm_stmia_w,arm_stmia_w,arm_stmia_w,arm_stmia_w,arm_stmia_w,arm_stmia_w,arm_stmia_w,arm_stmia_w,arm_stmia_w,arm_stmia_w,arm_stmia_w,arm_stmia_w,arm_stmia_w,
	arm_ldmia_w,arm_ldmia_w,arm_ldmia_w,arm_ldmia_w,arm_ldmia_w,arm_ldmia_w,arm_ldmia_w,arm_ldmia_w,arm_ldmia_w,arm_ldmia_w,arm_ldmia_w,arm_ldmia_w,arm_ldmia_w,arm_ldmia_w,arm_ldmia_w,arm_ldmia_w,
	arm_stmia_u,arm_stmia_u,arm_stmia_u,arm_stmia_u,arm_stmia_u,arm_stmia_u,arm_stmia_u,arm_stmia_u,arm_stmia_u,arm_stmia_u,arm_stmia_u,arm_stmia_u,arm_stmia_u,arm_stmia_u,arm_stmia_u,arm_stmia_u,
	arm_ldmia_u,arm_ldmia_u,arm_ldmia_u,arm_ldmia_u,arm_ldmia_u,arm_ldmia_u,arm_ldmia_u,arm_ldmia_u,arm_ldmia_u,arm_ldmia_u,arm_ldmia_u,arm_ldmia_u,arm_ldmia_u,arm_ldmia_u,arm_ldmia_u,arm_ldmia_u,
	arm_stmia_uw,arm_stmia_uw,arm_stmia_uw,arm_stmia_uw,arm_stmia_uw,arm_stmia_uw,arm_stmia_uw,arm_stmia_uw,arm_stmia_uw,arm_stmia_uw,arm_stmia_uw,arm_stmia_uw,arm_stmia_uw,arm_stmia_uw,arm_stmia_uw,arm_stmia_uw,
	arm_ldmia_uw,arm_ldmia_uw,arm_ldmia_uw,arm_ldmia_uw,arm_ldmia_uw,arm_ldmia_uw,arm_ldmia_uw,arm_ldmia_uw,arm_ldmia_uw,arm_ldmia_uw,arm_ldmia_uw,arm_ldmia_uw,arm_ldmia_uw,arm_ldmia_uw,arm_ldmia_uw,arm_ldmia_uw,
	arm_stmdb,arm_stmdb,arm_stmdb,arm_stmdb,arm_stmdb,arm_stmdb,arm_stmdb,arm_stmdb,arm_stmdb,arm_stmdb,arm_stmdb,arm_stmdb,arm_stmdb,arm_stmdb,arm_stmdb,arm_stmdb,
	arm_ldmdb,arm_ldmdb,arm_ldmdb,arm_ldmdb,arm_ldmdb,arm_ldmdb,arm_ldmdb,arm_ldmdb,arm_ldmdb,arm_ldmdb,arm_ldmdb,arm_ldmdb,arm_ldmdb,arm_ldmdb,arm_ldmdb,arm_ldmdb,
	arm_stmdb_w,arm_stmdb_w,arm_stmdb_w,arm_stmdb_w,arm_stmdb_w,arm_stmdb_w,arm_stmdb_w,arm_stmdb_w,arm_stmdb_w,arm_stmdb_w,arm_stmdb_w,arm_stmdb_w,arm_stmdb_w,arm_stmdb_w,arm_stmdb_w,arm_stmdb_w,
	arm_ldmdb_w,arm_ldmdb_w,arm_ldmdb_w,arm_ldmdb_w,arm_ldmdb_w,arm_ldmdb_w,arm_ldmdb_w,arm_ldmdb_w,arm_ldmdb_w,arm_ldmdb_w,arm_ldmdb_w,arm_ldmdb_w,arm_ldmdb_w,arm_ldmdb_w,arm_ldmdb_w,arm_ldmdb_w,
	arm_stmdb_u,arm_stmdb_u,arm_stmdb_u,arm_stmdb_u,arm_stmdb_u,arm_stmdb_u,arm_stmdb_u,arm_stmdb_u,arm_stmdb_u,arm_stmdb_u,arm_stmdb_u,arm_stmdb_u,arm_stmdb_u,arm_stmdb_u,arm_stmdb_u,arm_stmdb_u,
	arm_ldmdb_u,arm_ldmdb_u,arm_ldmdb_u,arm_ldmdb_u,arm_ldmdb_u,arm_ldmdb_u,arm_ldmdb_u,arm_ldmdb_u,arm_ldmdb_u,arm_ldmdb_u,arm_ldmdb_u,arm_ldmdb_u,arm_ldmdb_u,arm_ldmdb_u,arm_ldmdb_u,arm_ldmdb_u,
	arm_stmdb_uw,arm_stmdb_uw,arm_stmdb_uw,arm_stmdb_uw,arm_stmdb_uw,arm_stmdb_uw,arm_stmdb_uw,arm_stmdb_uw,arm_stmdb_uw,arm_stmdb_uw,arm_stmdb_uw,arm_stmdb_uw,arm_stmdb_uw,arm_stmdb_uw,arm_stmdb_uw,arm_stmdb_uw,
	arm_ldmdb_uw,arm_ldmdb_uw,arm_ldmdb_uw,arm_ldmdb_uw,arm_ldmdb_uw,arm_ldmdb_uw,arm_ldmdb_uw,arm_ldmdb_uw,arm_ldmdb_uw,arm_ldmdb_uw,arm_ldmdb_uw,arm_ldmdb_uw,arm_ldmdb_uw,arm_ldmdb_uw,arm_ldmdb_uw,arm_ldmdb_uw,
	arm_stmib,arm_stmib,arm_stmib,arm_stmib,arm_stmib,arm_stmib,arm_stmib,arm_stmib,arm_stmib,arm_stmib,arm_stmib,arm_stmib,arm_stmib,arm_stmib,arm_stmib,arm_stmib,
	arm_ldmib,arm_ldmib,arm_ldmib,arm_ldmib,arm_ldmib,arm_ldmib,arm_ldmib,arm_ldmib,arm_ldmib,arm_ldmib,arm_ldmib,arm_ldmib,arm_ldmib,arm_ldmib,arm_ldmib,arm_ldmib,
	arm_stmib_w,arm_stmib_w,arm_stmib_w,arm_stmib_w,arm_stmib_w,arm_stmib_w,arm_stmib_w,arm_stmib_w,arm_stmib_w,arm_stmib_w,arm_stmib_w,arm_stmib_w,arm_stmib_w,arm_stmib_w,arm_stmib_w,arm_stmib_w,
	arm_ldmib_w,arm_ldmib_w,arm_ldmib_w,arm_ldmib_w,arm_ldmib_w,arm_ldmib_w,arm_ldmib_w,arm_ldmib_w,arm_ldmib_w,arm_ldmib_w,arm_ldmib_w,arm_ldmib_w,arm_ldmib_w,arm_ldmib_w,arm_ldmib_w,arm_ldmib_w,
	arm_stmib_u,arm_stmib_u,arm_stmib_u,arm_stmib_u,arm_stmib_u,arm_stmib_u,arm_stmib_u,arm_stmib_u,arm_stmib_u,arm_stmib_u,arm_stmib_u,arm_stmib_u,arm_stmib_u,arm_stmib_u,arm_stmib_u,arm_stmib_u,
	arm_ldmib_u,arm_ldmib_u,arm_ldmib_u,arm_ldmib_u,arm_ldmib_u,arm_ldmib_u,arm_ldmib_u,arm_ldmib_u,arm_ldmib_u,arm_ldmib_u,arm_ldmib_u,arm_ldmib_u,arm_ldmib_u,arm_ldmib_u,arm_ldmib_u,arm_ldmib_u,
	arm_stmib_uw,arm_stmib_uw,arm_stmib_uw,arm_stmib_uw,arm_stmib_uw,arm_stmib_uw,arm_stmib_uw,arm_stmib_uw,arm_stmib_uw,arm_stmib_uw,arm_stmib_uw,arm_stmib_uw,arm_stmib_uw,arm_stmib_uw,arm_stmib_uw,arm_stmib_uw,
	arm_ldmib_uw,arm_ldmib_uw,arm_ldmib_uw,arm_ldmib_uw,arm_ldmib_uw,arm_ldmib_uw,arm_ldmib_uw,arm_ldmib_uw,arm_ldmib_uw,arm_ldmib_uw,arm_ldmib_uw,arm_ldmib_uw,arm_ldmib_uw,arm_ldmib_uw,arm_ldmib_uw,arm_ldmib_uw,
	arm_b,arm_b,arm_b,arm_b,arm_b,arm_b,arm_b,arm_b,arm_b,arm_b,arm_b,arm_b,arm_b,arm_b,arm_b,arm_b,
	arm_b,arm_b,arm_b,arm_b,arm_b,arm_b,arm_b,arm_b,arm_b,arm_b,arm_b,arm_b,arm_b,arm_b,arm_b,arm_b,
	arm_b,arm_b,arm_b,arm_b,arm_b,arm_b,arm_b,arm_b,arm_b,arm_b,arm_b,arm_b,arm_b,arm_b,arm_b,arm_b,
	arm_b,arm_b,arm_b,arm_b,arm_b,arm_b,arm_b,arm_b,arm_b,arm_b,arm_b,arm_b,arm_b,arm_b,arm_b,arm_b,
	arm_b,arm_b,arm_b,arm_b,arm_b,arm_b,arm_b,arm_b,arm_b,arm_b,arm_b,arm_b,arm_b,arm_b,arm_b,arm_b,
	arm_b,arm_b,arm_b,arm_b,arm_b,arm_b,arm_b,arm_b,arm_b,arm_b,arm_b,arm_b,arm_b,arm_b,arm_b,arm_b,
	arm_b,arm_b,arm_b,arm_b,arm_b,arm_b,arm_b,arm_b,arm_b,arm_b,arm_b,arm_b,arm_b,arm_b,arm_b,arm_b,
	arm_b,arm_b,arm_b,arm_b,arm_b,arm_b,arm_b,arm_b,arm_b,arm_b,arm_b,arm_b,arm_b,arm_b,arm_b,arm_b,
	arm_b,arm_b,arm_b,arm_b,arm_b,arm_b,arm_b,arm_b,arm_b,arm_b,arm_b,arm_b,arm_b,arm_b,arm_b,arm_b,
	arm_b,arm_b,arm_b,arm_b,arm_b,arm_b,arm_b,arm_b,arm_b,arm_b,arm_b,arm_b,arm_b,arm_b,arm_b,arm_b,
	arm_b,arm_b,arm_b,arm_b,arm_b,arm_b,arm_b,arm_b,arm_b,arm_b,arm_b,arm_b,arm_b,arm_b,arm_b,arm_b,
	arm_b,arm_b,arm_b,arm_b,arm_b,arm_b,arm_b,arm_b,arm_b,arm_b,arm_b,arm_b,arm_b,arm_b,arm_b,arm_b,
	arm_b,arm_b,arm_b,arm_b,arm_b,arm_b,arm_b,arm_b,arm_b,arm_b,arm_b,arm_b,arm_b,arm_b,arm_b,arm_b,
	arm_b,arm_b,arm_b,arm_b,arm_b,arm_b,arm_b,arm_b,arm_b,arm_b,arm_b,arm_b,arm_b,arm_b,arm_b,arm_b,
	arm_b,arm_b,arm_b,arm_b,arm_b,arm_b,arm_b,arm_b,arm_b,arm_b,arm_b,arm_b,arm_b,arm_b,arm_b,arm_b,
	arm_b,arm_b,arm_b,arm_b,arm_b,arm_b,arm_b,arm_b,arm_b,arm_b,arm_b,arm_b,arm_b,arm_b,arm_b,arm_b,
	arm_bl,arm_bl,arm_bl,arm_bl,arm_bl,arm_bl,arm_bl,arm_bl,arm_bl,arm_bl,arm_bl,arm_bl,arm_bl,arm_bl,arm_bl,arm_bl,
	arm_bl,arm_bl,arm_bl,arm_bl,arm_bl,arm_bl,arm_bl,arm_bl,arm_bl,arm_bl,arm_bl,arm_bl,arm_bl,arm_bl,arm_bl,arm_bl,
	arm_bl,arm_bl,arm_bl,arm_bl,arm_bl,arm_bl,arm_bl,arm_bl,arm_bl,arm_bl,arm_bl,arm_bl,arm_bl,arm_bl,arm_bl,arm_bl,
	arm_bl,arm_bl,arm_bl,arm_bl,arm_bl,arm_bl,arm_bl,arm_bl,arm_bl,arm_bl,arm_bl,arm_bl,arm_bl,arm_bl,arm_bl,arm_bl,
	arm_bl,arm_bl,arm_bl,arm_bl,arm_bl,arm_bl,arm_bl,arm_bl,arm_bl,arm_bl,arm_bl,arm_bl,arm_bl,arm_bl,arm_bl,arm_bl,
	arm_bl,arm_bl,arm_bl,arm_bl,arm_bl,arm_bl,arm_bl,arm_bl,arm_bl,arm_bl,arm_bl,arm_bl,arm_bl,arm_bl,arm_bl,arm_bl,
	arm_bl,arm_bl,arm_bl,arm_bl,arm_bl,arm_bl,arm_bl,arm_bl,arm_bl,arm_bl,arm_bl,arm_bl,arm_bl,arm_bl,arm_bl,arm_bl,
	arm_bl,arm_bl,arm_bl,arm_bl,arm_bl,arm_bl,arm_bl,arm_bl,arm_bl,arm_bl,arm_bl,arm_bl,arm_bl,arm_bl,arm_bl,arm_bl,
	arm_bl,arm_bl,arm_bl,arm_bl,arm_bl,arm_bl,arm_bl,arm_bl,arm_bl,arm_bl,arm_bl,arm_bl,arm_bl,arm_bl,arm_bl,arm_bl,
	arm_bl,arm_bl,arm_bl,arm_bl,arm_bl,arm_bl,arm_bl,arm_bl,arm_bl,arm_bl,arm_bl,arm_bl,arm_bl,arm_bl,arm_bl,arm_bl,
	arm_bl,arm_bl,arm_bl,arm_bl,arm_bl,arm_bl,arm_bl,arm_bl,arm_bl,arm_bl,arm_bl,arm_bl,arm_bl,arm_bl,arm_bl,arm_bl,
	arm_bl,arm_bl,arm_bl,arm_bl,arm_bl,arm_bl,arm_bl,arm_bl,arm_bl,arm_bl,arm_bl,arm_bl,arm_bl,arm_bl,arm_bl,arm_bl,
	arm_bl,arm_bl,arm_bl,arm_bl,arm_bl,arm_bl,arm_bl,arm_bl,arm_bl,arm_bl,arm_bl,arm_bl,arm_bl,arm_bl,arm_bl,arm_bl,
	arm_bl,arm_bl,arm_bl,arm_bl,arm_bl,arm_bl,arm_bl,arm_bl,arm_bl,arm_bl,arm_bl,arm_bl,arm_bl,arm_bl,arm_bl,arm_bl,
	arm_bl,arm_bl,arm_bl,arm_bl,arm_bl,arm_bl,arm_bl,arm_bl,arm_bl,arm_bl,arm_bl,arm_bl,arm_bl,arm_bl,arm_bl,arm_bl,
	arm_bl,arm_bl,arm_bl,arm_bl,arm_bl,arm_bl,arm_bl,arm_bl,arm_bl,arm_bl,arm_bl,arm_bl,arm_bl,arm_bl,arm_bl,arm_bl,
	arm_stc_ofm,arm_stc_ofm,arm_stc_ofm,arm_stc_ofm,arm_stc_ofm,arm_stc_ofm,arm_stc_ofm,arm_stc_ofm,arm_stc_ofm,arm_stc_ofm,arm_stc_ofm,arm_stc_ofm,arm_stc_ofm,arm_stc_ofm,arm_stc_ofm,arm_stc_ofm,
	arm_ldc_ofm,arm_ldc_ofm,arm_ldc_ofm,arm_ldc_ofm,arm_ldc_ofm,arm_ldc_ofm,arm_ldc_ofm,arm_ldc_ofm,arm_ldc_ofm,arm_ldc_ofm,arm_ldc_ofm,arm_ldc_ofm,arm_ldc_ofm,arm_ldc_ofm,arm_ldc_ofm,arm_ldc_ofm,
	arm_stc_prm,arm_stc_prm,arm_stc_prm,arm_stc_prm,arm_stc_prm,arm_stc_prm,arm_stc_prm,arm_stc_prm,arm_stc_prm,arm_stc_prm,arm_stc_prm,arm_stc_prm,arm_stc_prm,arm_stc_prm,arm_stc_prm,arm_stc_prm,
	arm_ldc_prm,arm_ldc_prm,arm_ldc_prm,arm_ldc_prm,arm_ldc_prm,arm_ldc_prm,arm_ldc_prm,arm_ldc_prm,arm_ldc_prm,arm_ldc_prm,arm_ldc_prm,arm_ldc_prm,arm_ldc_prm,arm_ldc_prm,arm_ldc_prm,arm_ldc_prm,
	arm_stc_ofm,arm_stc_ofm,arm_stc_ofm,arm_stc_ofm,arm_stc_ofm,arm_stc_ofm,arm_stc_ofm,arm_stc_ofm,arm_stc_ofm,arm_stc_ofm,arm_stc_ofm,arm_stc_ofm,arm_stc_ofm,arm_stc_ofm,arm_stc_ofm,arm_stc_ofm,
	arm_ldc_ofm,arm_ldc_ofm,arm_ldc_ofm,arm_ldc_ofm,arm_ldc_ofm,arm_ldc_ofm,arm_ldc_ofm,arm_ldc_ofm,arm_ldc_ofm,arm_ldc_ofm,arm_ldc_ofm,arm_ldc_ofm,arm_ldc_ofm,arm_ldc_ofm,arm_ldc_ofm,arm_ldc_ofm,
	arm_stc_prm,arm_stc_prm,arm_stc_prm,arm_stc_prm,arm_stc_prm,arm_stc_prm,arm_stc_prm,arm_stc_prm,arm_stc_prm,arm_stc_prm,arm_stc_prm,arm_stc_prm,arm_stc_prm,arm_stc_prm,arm_stc_prm,arm_stc_prm,
	arm_ldc_prm,arm_ldc_prm,arm_ldc_prm,arm_ldc_prm,arm_ldc_prm,arm_ldc_prm,arm_ldc_prm,arm_ldc_prm,arm_ldc_prm,arm_ldc_prm,arm_ldc_prm,arm_ldc_prm,arm_ldc_prm,arm_ldc_prm,arm_ldc_prm,arm_ldc_prm,
	arm_stc_ofp,arm_stc_ofp,arm_stc_ofp,arm_stc_ofp,arm_stc_ofp,arm_stc_ofp,arm_stc_ofp,arm_stc_ofp,arm_stc_ofp,arm_stc_ofp,arm_stc_ofp,arm_stc_ofp,arm_stc_ofp,arm_stc_ofp,arm_stc_ofp,arm_stc_ofp,
	arm_ldc_ofp,arm_ldc_ofp,arm_ldc_ofp,arm_ldc_ofp,arm_ldc_ofp,arm_ldc_ofp,arm_ldc_ofp,arm_ldc_ofp,arm_ldc_ofp,arm_ldc_ofp,arm_ldc_ofp,arm_ldc_ofp,arm_ldc_ofp,arm_ldc_ofp,arm_ldc_ofp,arm_ldc_ofp,
	arm_stc_prp,arm_stc_prp,arm_stc_prp,arm_stc_prp,arm_stc_prp,arm_stc_prp,arm_stc_prp,arm_stc_prp,arm_stc_prp,arm_stc_prp,arm_stc_prp,arm_stc_prp,arm_stc_prp,arm_stc_prp,arm_stc_prp,arm_stc_prp,
	arm_ldc_prp,arm_ldc_prp,arm_ldc_prp,arm_ldc_prp,arm_ldc_prp,arm_ldc_prp,arm_ldc_prp,arm_ldc_prp,arm_ldc_prp,arm_ldc_prp,arm_ldc_prp,arm_ldc_prp,arm_ldc_prp,arm_ldc_prp,arm_ldc_prp,arm_ldc_prp,
	arm_stc_ofp,arm_stc_ofp,arm_stc_ofp,arm_stc_ofp,arm_stc_ofp,arm_stc_ofp,arm_stc_ofp,arm_stc_ofp,arm_stc_ofp,arm_stc_ofp,arm_stc_ofp,arm_stc_ofp,arm_stc_ofp,arm_stc_ofp,arm_stc_ofp,arm_stc_ofp,
	arm_ldc_ofp,arm_ldc_ofp,arm_ldc_ofp,arm_ldc_ofp,arm_ldc_ofp,arm_ldc_ofp,arm_ldc_ofp,arm_ldc_ofp,arm_ldc_ofp,arm_ldc_ofp,arm_ldc_ofp,arm_ldc_ofp,arm_ldc_ofp,arm_ldc_ofp,arm_ldc_ofp,arm_ldc_ofp,
	arm_stc_prp,arm_stc_prp,arm_stc_prp,arm_stc_prp,arm_stc_prp,arm_stc_prp,arm_stc_prp,arm_stc_prp,arm_stc_prp,arm_stc_prp,arm_stc_prp,arm_stc_prp,arm_stc_prp,arm_stc_prp,arm_stc_prp,arm_stc_prp,
	arm_ldc_prp,arm_ldc_prp,arm_ldc_prp,arm_ldc_prp,arm_ldc_prp,arm_ldc_prp,arm_ldc_prp,arm_ldc_prp,arm_ldc_prp,arm_ldc_prp,arm_ldc_prp,arm_ldc_prp,arm_ldc_prp,arm_ldc_prp,arm_ldc_prp,arm_ldc_prp,
	arm_stc_unm,arm_stc_unm,arm_stc_unm,arm_stc_unm,arm_stc_unm,arm_stc_unm,arm_stc_unm,arm_stc_unm,arm_stc_unm,arm_stc_unm,arm_stc_unm,arm_stc_unm,arm_stc_unm,arm_stc_unm,arm_stc_unm,arm_stc_unm,
	arm_ldc_unm,arm_ldc_unm,arm_ldc_unm,arm_ldc_unm,arm_ldc_unm,arm_ldc_unm,arm_ldc_unm,arm_ldc_unm,arm_ldc_unm,arm_ldc_unm,arm_ldc_unm,arm_ldc_unm,arm_ldc_unm,arm_ldc_unm,arm_ldc_unm,arm_ldc_unm,
	arm_stc_ptm,arm_stc_ptm,arm_stc_ptm,arm_stc_ptm,arm_stc_ptm,arm_stc_ptm,arm_stc_ptm,arm_stc_ptm,arm_stc_ptm,arm_stc_ptm,arm_stc_ptm,arm_stc_ptm,arm_stc_ptm,arm_stc_ptm,arm_stc_ptm,arm_stc_ptm,
	arm_ldc_ptm,arm_ldc_ptm,arm_ldc_ptm,arm_ldc_ptm,arm_ldc_ptm,arm_ldc_ptm,arm_ldc_ptm,arm_ldc_ptm,arm_ldc_ptm,arm_ldc_ptm,arm_ldc_ptm,arm_ldc_ptm,arm_ldc_ptm,arm_ldc_ptm,arm_ldc_ptm,arm_ldc_ptm,
	arm_stc_unm,arm_stc_unm,arm_stc_unm,arm_stc_unm,arm_stc_unm,arm_stc_unm,arm_stc_unm,arm_stc_unm,arm_stc_unm,arm_stc_unm,arm_stc_unm,arm_stc_unm,arm_stc_unm,arm_stc_unm,arm_stc_unm,arm_stc_unm,
	arm_ldc_unm,arm_ldc_unm,arm_ldc_unm,arm_ldc_unm,arm_ldc_unm,arm_ldc_unm,arm_ldc_unm,arm_ldc_unm,arm_ldc_unm,arm_ldc_unm,arm_ldc_unm,arm_ldc_unm,arm_ldc_unm,arm_ldc_unm,arm_ldc_unm,arm_ldc_unm,
	arm_stc_ptm,arm_stc_ptm,arm_stc_ptm,arm_stc_ptm,arm_stc_ptm,arm_stc_ptm,arm_stc_ptm,arm_stc_ptm,arm_stc_ptm,arm_stc_ptm,arm_stc_ptm,arm_stc_ptm,arm_stc_ptm,arm_stc_ptm,arm_stc_ptm,arm_stc_ptm,
	arm_ldc_ptm,arm_ldc_ptm,arm_ldc_ptm,arm_ldc_ptm,arm_ldc_ptm,arm_ldc_ptm,arm_ldc_ptm,arm_ldc_ptm,arm_ldc_ptm,arm_ldc_ptm,arm_ldc_ptm,arm_ldc_ptm,arm_ldc_ptm,arm_ldc_ptm,arm_ldc_ptm,arm_ldc_ptm,
	arm_stc_unp,arm_stc_unp,arm_stc_unp,arm_stc_unp,arm_stc_unp,arm_stc_unp,arm_stc_unp,arm_stc_unp,arm_stc_unp,arm_stc_unp,arm_stc_unp,arm_stc_unp,arm_stc_unp,arm_stc_unp,arm_stc_unp,arm_stc_unp,
	arm_ldc_unp,arm_ldc_unp,arm_ldc_unp,arm_ldc_unp,arm_ldc_unp,arm_ldc_unp,arm_ldc_unp,arm_ldc_unp,arm_ldc_unp,arm_ldc_unp,arm_ldc_unp,arm_ldc_unp,arm_ldc_unp,arm_ldc_unp,arm_ldc_unp,arm_ldc_unp,
	arm_stc_ptp,arm_stc_ptp,arm_stc_ptp,arm_stc_ptp,arm_stc_ptp,arm_stc_ptp,arm_stc_ptp,arm_stc_ptp,arm_stc_ptp,arm_stc_ptp,arm_stc_ptp,arm_stc_ptp,arm_stc_ptp,arm_stc_ptp,arm_stc_ptp,arm_stc_ptp,
	arm_ldc_ptp,arm_ldc_ptp,arm_ldc_ptp,arm_ldc_ptp,arm_ldc_ptp,arm_ldc_ptp,arm_ldc_ptp,arm_ldc_ptp,arm_ldc_ptp,arm_ldc_ptp,arm_ldc_ptp,arm_ldc_ptp,arm_ldc_ptp,arm_ldc_ptp,arm_ldc_ptp,arm_ldc_ptp,
	arm_stc_unp,arm_stc_unp,arm_stc_unp,arm_stc_unp,arm_stc_unp,arm_stc_unp,arm_stc_unp,arm_stc_unp,arm_stc_unp,arm_stc_unp,arm_stc_unp,arm_stc_unp,arm_stc_unp,arm_stc_unp,arm_stc_unp,arm_stc_unp,
	arm_ldc_unp,arm_ldc_unp,arm_ldc_unp,arm_ldc_unp,arm_ldc_unp,arm_ldc_unp,arm_ldc_unp,arm_ldc_unp,arm_ldc_unp,arm_ldc_unp,arm_ldc_unp,arm_ldc_unp,arm_ldc_unp,arm_ldc_unp,arm_ldc_unp,arm_ldc_unp,
	arm_stc_ptp,arm_stc_ptp,arm_stc_ptp,arm_stc_ptp,arm_stc_ptp,arm_stc_ptp,arm_stc_ptp,arm_stc_ptp,arm_stc_ptp,arm_stc_ptp,arm_stc_ptp,arm_stc_ptp,arm_stc_ptp,arm_stc_ptp,arm_stc_ptp,arm_stc_ptp,
	arm_ldc_ptp,arm_ldc_ptp,arm_ldc_ptp,arm_ldc_ptp,arm_ldc_ptp,arm_ldc_ptp,arm_ldc_ptp,arm_ldc_ptp,arm_ldc_ptp,arm_ldc_ptp,arm_ldc_ptp,arm_ldc_ptp,arm_ldc_ptp,arm_ldc_ptp,arm_ldc_ptp,arm_ldc_ptp,
	arm_cdp,arm_mcr,arm_cdp,arm_mcr,arm_cdp,arm_mcr,arm_cdp,arm_mcr,arm_cdp,arm_mcr,arm_cdp,arm_mcr,arm_cdp,arm_mcr,arm_cdp,arm_mcr,
	arm_cdp,arm_mrc,arm_cdp,arm_mrc,arm_cdp,arm_mrc,arm_cdp,arm_mrc,arm_cdp,arm_mrc,arm_cdp,arm_mrc,arm_cdp,arm_mrc,arm_cdp,arm_mrc,
	arm_cdp,arm_mcr,arm_cdp,arm_mcr,arm_cdp,arm_mcr,arm_cdp,arm_mcr,arm_cdp,arm_mcr,arm_cdp,arm_mcr,arm_cdp,arm_mcr,arm_cdp,arm_mcr,
	arm_cdp,arm_mrc,arm_cdp,arm_mrc,arm_cdp,arm_mrc,arm_cdp,arm_mrc,arm_cdp,arm_mrc,arm_cdp,arm_mrc,arm_cdp,arm_mrc,arm_cdp,arm_mrc,
	arm_cdp,arm_mcr,arm_cdp,arm_mcr,arm_cdp,arm_mcr,arm_cdp,arm_mcr,arm_cdp,arm_mcr,arm_cdp,arm_mcr,arm_cdp,arm_mcr,arm_cdp,arm_mcr,
	arm_cdp,arm_mrc,arm_cdp,arm_mrc,arm_cdp,arm_mrc,arm_cdp,arm_mrc,arm_cdp,arm_mrc,arm_cdp,arm_mrc,arm_cdp,arm_mrc,arm_cdp,arm_mrc,
	arm_cdp,arm_mcr,arm_cdp,arm_mcr,arm_cdp,arm_mcr,arm_cdp,arm_mcr,arm_cdp,arm_mcr,arm_cdp,arm_mcr,arm_cdp,arm_mcr,arm_cdp,arm_mcr,
	arm_cdp,arm_mrc,arm_cdp,arm_mrc,arm_cdp,arm_mrc,arm_cdp,arm_mrc,arm_cdp,arm_mrc,arm_cdp,arm_mrc,arm_cdp,arm_mrc,arm_cdp,arm_mrc,
	arm_cdp,arm_mcr,arm_cdp,arm_mcr,arm_cdp,arm_mcr,arm_cdp,arm_mcr,arm_cdp,arm_mcr,arm_cdp,arm_mcr,arm_cdp,arm_mcr,arm_cdp,arm_mcr,
	arm_cdp,arm_mrc,arm_cdp,arm_mrc,arm_cdp,arm_mrc,arm_cdp,arm_mrc,arm_cdp,arm_mrc,arm_cdp,arm_mrc,arm_cdp,arm_mrc,arm_cdp,arm_mrc,
	arm_cdp,arm_mcr,arm_cdp,arm_mcr,arm_cdp,arm_mcr,arm_cdp,arm_mcr,arm_cdp,arm_mcr,arm_cdp,arm_mcr,arm_cdp,arm_mcr,arm_cdp,arm_mcr,
	arm_cdp,arm_mrc,arm_cdp,arm_mrc,arm_cdp,arm_mrc,arm_cdp,arm_mrc,arm_cdp,arm_mrc,arm_cdp,arm_mrc,arm_cdp,arm_mrc,arm_cdp,arm_mrc,
	arm_cdp,arm_mcr,arm_cdp,arm_mcr,arm_cdp,arm_mcr,arm_cdp,arm_mcr,arm_cdp,arm_mcr,arm_cdp,arm_mcr,arm_cdp,arm_mcr,arm_cdp,arm_mcr,
	arm_cdp,arm_mrc,arm_cdp,arm_mrc,arm_cdp,arm_mrc,arm_cdp,arm_mrc,arm_cdp,arm_mrc,arm_cdp,arm_mrc,arm_cdp,arm_mrc,arm_cdp,arm_mrc,
	arm_cdp,arm_mcr,arm_cdp,arm_mcr,arm_cdp,arm_mcr,arm_cdp,arm_mcr,arm_cdp,arm_mcr,arm_cdp,arm_mcr,arm_cdp,arm_mcr,arm_cdp,arm_mcr,
	arm_cdp,arm_mrc,arm_cdp,arm_mrc,arm_cdp,arm_mrc,arm_cdp,arm_mrc,arm_cdp,arm_mrc,arm_cdp,arm_mrc,arm_cdp,arm_mrc,arm_cdp,arm_mrc,
	arm_swi,arm_swi,arm_swi,arm_swi,arm_swi,arm_swi,arm_swi,arm_swi,arm_swi,arm_swi,arm_swi,arm_swi,arm_swi,arm_swi,arm_swi,arm_swi,
	arm_swi,arm_swi,arm_swi,arm_swi,arm_swi,arm_swi,arm_swi,arm_swi,arm_swi,arm_swi,arm_swi,arm_swi,arm_swi,arm_swi,arm_swi,arm_swi,
	arm_swi,arm_swi,arm_swi,arm_swi,arm_swi,arm_swi,arm_swi,arm_swi,arm_swi,arm_swi,arm_swi,arm_swi,arm_swi,arm_swi,arm_swi,arm_swi,
	arm_swi,arm_swi,arm_swi,arm_swi,arm_swi,arm_swi,arm_swi,arm_swi,arm_swi,arm_swi,arm_swi,arm_swi,arm_swi,arm_swi,arm_swi,arm_swi,
	arm_swi,arm_swi,arm_swi,arm_swi,arm_swi,arm_swi,arm_swi,arm_swi,arm_swi,arm_swi,arm_swi,arm_swi,arm_swi,arm_swi,arm_swi,arm_swi,
	arm_swi,arm_swi,arm_swi,arm_swi,arm_swi,arm_swi,arm_swi,arm_swi,arm_swi,arm_swi,arm_swi,arm_swi,arm_swi,arm_swi,arm_swi,arm_swi,
	arm_swi,arm_swi,arm_swi,arm_swi,arm_swi,arm_swi,arm_swi,arm_swi,arm_swi,arm_swi,arm_swi,arm_swi,arm_swi,arm_swi,arm_swi,arm_swi,
	arm_swi,arm_swi,arm_swi,arm_swi,arm_swi,arm_swi,arm_swi,arm_swi,arm_swi,arm_swi,arm_swi,arm_swi,arm_swi,arm_swi,arm_swi,arm_swi,
	arm_swi,arm_swi,arm_swi,arm_swi,arm_swi,arm_swi,arm_swi,arm_swi,arm_swi,arm_swi,arm_swi,arm_swi,arm_swi,arm_swi,arm_swi,arm_swi,
	arm_swi,arm_swi,arm_swi,arm_swi,arm_swi,arm_swi,arm_swi,arm_swi,arm_swi,arm_swi,arm_swi,arm_swi,arm_swi,arm_swi,arm_swi,arm_swi,
	arm_swi,arm_swi,arm_swi,arm_swi,arm_swi,arm_swi,arm_swi,arm_swi,arm_swi,arm_swi,arm_swi,arm_swi,arm_swi,arm_swi,arm_swi,arm_swi,
	arm_swi,arm_swi,arm_swi,arm_swi,arm_swi,arm_swi,arm_swi,arm_swi,arm_swi,arm_swi,arm_swi,arm_swi,arm_swi,arm_swi,arm_swi,arm_swi,
	arm_swi,arm_swi,arm_swi,arm_swi,arm_swi,arm_swi,arm_swi,arm_swi,arm_swi,arm_swi,arm_swi,arm_swi,arm_swi,arm_swi,arm_swi,arm_swi,
	arm_swi,arm_swi,arm_swi,arm_swi,arm_swi,arm_swi,arm_swi,arm_swi,arm_swi,arm_swi,arm_swi,arm_swi,arm_swi,arm_swi,arm_swi,arm_swi,
	arm_swi,arm_swi,arm_swi,arm_swi,arm_swi,arm_swi,arm_swi,arm_swi,arm_swi,arm_swi,arm_swi,arm_swi,arm_swi,arm_swi,arm_swi,arm_swi,
	arm_swi,arm_swi,arm_swi,arm_swi,arm_swi,arm_swi,arm_swi,arm_swi,arm_swi,arm_swi,arm_swi,arm_swi,arm_swi,arm_swi,arm_swi,arm_swi,
];
