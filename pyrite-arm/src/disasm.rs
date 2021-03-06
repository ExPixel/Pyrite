use super::ArmMemory;
use std::fmt::Write;
// ARM
const ARM_OPCODE_TABLE: [(u32, u32, ARMInstrType); 15] = [
    (0x0ffffff0, 0x012fff10, ARMInstrType::BranchAndExchange), // Branch and Exchange
    (0x0fb00ff0, 0x01000090, ARMInstrType::SingleDataSwap),    // Single Data Swap
    (0x0fc000f0, 0x00000090, ARMInstrType::Multiply),          // Multiply
    (0x0e400f90, 0x00000090, ARMInstrType::HalfwordDataTransfer), // Halfword Data Transfer (register offset)
    (0x0f8000f0, 0x00800090, ARMInstrType::MultiplyLong),         // Multiply Long
    (0x0e400090, 0x00400090, ARMInstrType::HalfwordDataTransfer), // Halfword Data Transfer (immediate offset)
    (
        0x0f000010,
        0x0e000000,
        ARMInstrType::CoprocessorDataOperation,
    ), // Coprocessor Data Operation
    (
        0x0f000010,
        0x0e000010,
        ARMInstrType::CoprocessorRegisterTransfer,
    ), // Coprocessor Register Transfer
    (0x0e000010, 0x06000010, ARMInstrType::Undefined),            // Undefined
    (0x0f000000, 0x0f000000, ARMInstrType::SoftwareInterrupt),    // Software Interrupt
    (0x0e000000, 0x08000000, ARMInstrType::BlockDataTransfer),    // Block Data Transfer
    (0x0e000000, 0x0a000000, ARMInstrType::Branch),               // Branch
    (
        0x0e000000,
        0x0c000000,
        ARMInstrType::CoprocessorDataTransfer,
    ), // Coprocessor Data Transfer
    (0x0c000000, 0x00000000, ARMInstrType::DataProcessing),       // Data Processing / PSR Transfer
    (0x0c000000, 0x04000000, ARMInstrType::SingleDataTransfer),   // Single Data Transfer
];

#[derive(Debug, PartialEq, Eq)]
pub enum ARMInstrType {
    BranchAndExchange,
    SingleDataSwap,
    Multiply,
    HalfwordDataTransfer,
    MultiplyLong,
    CoprocessorDataOperation,
    CoprocessorRegisterTransfer,
    Undefined,
    SoftwareInterrupt,
    BlockDataTransfer,
    Branch,
    CoprocessorDataTransfer,
    DataProcessing,
    SingleDataTransfer,
}

// THUMB
const THUMB_OPCODE_TABLE: [(u32, u32, THUMBInstrType); 19] = [
    (0xff00, 0xb000, THUMBInstrType::AddOffsetToStackPointer), // Add Offset to Stack Pointer
    (0xff00, 0xdf00, THUMBInstrType::SoftwareInterrupt),       // Software Interrupt
    (0xfc00, 0x4000, THUMBInstrType::ALUOperations),           // ALU Operations
    (0xfc00, 0x4400, THUMBInstrType::HiRegisterOperations), // Hi Register Operations / Branch Exchange
    (0xf600, 0xb400, THUMBInstrType::PushPopRegisters),     // Push/Pop Registers
    (0xf800, 0x1800, THUMBInstrType::AddSubtract),          // Add / Subtract
    (0xf800, 0x4800, THUMBInstrType::PCRelativeLoad),       // PC-relative Load
    (0xf200, 0x5000, THUMBInstrType::LoadStoreWithRegisterOffset), // Load/Store with register offset
    (0xf200, 0x5200, THUMBInstrType::LoadStoreSignHalfwordByte), // Load/Store Sign-Extended Byte/Halfword
    (0xf800, 0xe000, THUMBInstrType::UnconditionalBranch),       // Unconditional Branch
    (0xf000, 0x8000, THUMBInstrType::LoadStoreHalfword),         // Load/Store Halfword
    (0xf000, 0x9000, THUMBInstrType::SPRelativeLoadStore),       // SP-relative Load/Store
    (0xf000, 0xa000, THUMBInstrType::LoadAddress),               // Load Address
    (0xf000, 0xc000, THUMBInstrType::MultipleLoadStore),         // Multiple Load/Store
    (0xf000, 0xd000, THUMBInstrType::ConditionalBranch),         // Conditional Branch
    (0xf000, 0xf000, THUMBInstrType::LongBranchWithLink),        // Long Branch with Link
    (0xe000, 0x0000, THUMBInstrType::MoveShiftedRegister),       // Move Shifted Register
    (0xe000, 0x2000, THUMBInstrType::MoveCompareAddSubtractImm), // Move/ Compare/ Add/ Subtract Immediate
    (0xe000, 0x6000, THUMBInstrType::LoadStoreWithImmOffset),    // Load/Store with Immediate Offset
];

#[derive(Debug, PartialEq, Eq)]
pub enum THUMBInstrType {
    AddOffsetToStackPointer,
    SoftwareInterrupt,
    ALUOperations,
    HiRegisterOperations,
    PushPopRegisters,
    AddSubtract,
    PCRelativeLoad,
    LoadStoreWithRegisterOffset,
    LoadStoreSignHalfwordByte,
    UnconditionalBranch,
    LoadStoreHalfword,
    SPRelativeLoadStore,
    LoadAddress,
    MultipleLoadStore,
    ConditionalBranch,
    LongBranchWithLink,
    MoveShiftedRegister,
    MoveCompareAddSubtractImm,
    LoadStoreWithImmOffset,
}

pub fn disassemble_arm(dest: &mut String, address: u32, memory: &dyn ArmMemory) {
    let opcode = memory.view_word(address);
    for (select_bits, diff, instr_type) in ARM_OPCODE_TABLE.iter() {
        if ((opcode & select_bits) ^ diff) == 0 {
            write!(
                dest,
                "undefined @ = {:08X} ({:?})",
                memory.view_word(address),
                instr_type
            )
            .unwrap();
            return;
        }
    }
    dest.push_str("undefined");
}

pub fn disassemble_thumb(dest: &mut String, address: u32, memory: &dyn ArmMemory) {
    let opcode = memory.view_halfword(address) as u32;
    for (select_bits, diff, instr_type) in THUMB_OPCODE_TABLE.iter() {
        if ((opcode & *select_bits) ^ *diff) == 0 {
            match instr_type {
                THUMBInstrType::MoveShiftedRegister => {
                    thumb_disasm_move_shifted_register(opcode, dest)
                }
                THUMBInstrType::AddSubtract => thumb_disasm_add_sub(opcode, dest),
                THUMBInstrType::MoveCompareAddSubtractImm => {
                    thumb_disasm_mov_cmp_add_sub_imm(opcode, dest)
                }
                THUMBInstrType::ALUOperations => thumb_disasm_alu(opcode, dest),
                THUMBInstrType::HiRegisterOperations => thumb_disasm_hi_register_ops(opcode, dest),
                THUMBInstrType::PCRelativeLoad => {
                    thumb_disasm_pc_relative_load(opcode, dest, address, memory)
                }
                THUMBInstrType::LoadStoreWithRegisterOffset => {
                    thumb_disasm_load_store_reg(opcode, dest)
                }
                THUMBInstrType::LoadStoreSignHalfwordByte => {
                    thumb_disasm_load_store_se_byte_halfword(opcode, dest)
                }
                THUMBInstrType::LoadStoreWithImmOffset => thumb_disasm_load_store_imm(opcode, dest),
                THUMBInstrType::LoadStoreHalfword => thumb_disasm_load_store_halfword(opcode, dest),
                THUMBInstrType::SPRelativeLoadStore => {
                    thumb_disasm_load_store_sp_relative(opcode, dest)
                }
                THUMBInstrType::LoadAddress => thumb_disasm_add_offset_to_pc(opcode, dest, address),
                THUMBInstrType::AddOffsetToStackPointer => {
                    thumb_disasm_add_offset_to_sp(opcode, dest)
                }
                THUMBInstrType::PushPopRegisters => thumb_disasm_push_pop_reg(opcode, dest),
                THUMBInstrType::MultipleLoadStore => thumb_disasm_multiple_load_store(opcode, dest),
                THUMBInstrType::ConditionalBranch => {
                    thumb_disasm_conditional_branch(opcode, dest, address)
                }
                THUMBInstrType::SoftwareInterrupt => thumb_disasm_swi(opcode, dest),
                THUMBInstrType::UnconditionalBranch => {
                    thumb_disasm_unconditional_branch(opcode, dest, address)
                }
                THUMBInstrType::LongBranchWithLink => {
                    thumb_disasm_branch_with_link(opcode, dest, address, memory)
                } // _ => write!(dest, "undefined @ = {:04X} ({:?})", memory.view_halfword(address), instr_type).unwrap(),
            }
            return;
        }
    }
    dest.push_str("undefined");
}

fn thumb_disasm_conditional_branch(opcode: u32, buffer: &mut String, address: u32) {
    let pc = address.wrapping_add(4); // PC is 4 ahead in THUMB mode.
    let condition = condition_code_str(bits!(opcode, 8, 11));
    let offset = sign_extend_32!((opcode & 0xFF) << 1, 9);
    let dest = pc.wrapping_add(offset) & 0xFFFFFFFE;

    write!(buffer, "b{} 0x{:08X}", condition, dest).unwrap();
}

fn thumb_disasm_add_offset_to_sp(opcode: u32, buffer: &mut String) {
    let offset = sign_extend_32!((opcode & 0xFF) << 2, 10) as i32;
    write!(buffer, "add {}, #{}", reg_str(13), offset).unwrap();
}

fn thumb_disasm_add_offset_to_pc(opcode: u32, buffer: &mut String, address: u32) {
    let pc = address.wrapping_add(4); // PC is 4 ahead in THUMB mode.
    let offset = (opcode & 0xFF) << 2;
    let loaded_addr = (pc & 0xFFFFFFFD).wrapping_add(offset); // bit 1 of PC is forced to 1 for this.
    write!(
        buffer,
        "add {}, #{} ; = 0x{:08X}",
        reg_str(15),
        offset,
        loaded_addr
    )
    .unwrap();
}

fn thumb_disasm_move_shifted_register(opcode: u32, buffer: &mut String) {
    let rd = bits!(opcode, 0, 2);
    let rs = bits!(opcode, 3, 5);
    let offset = bits!(opcode, 6, 10);
    let op = match bits!(opcode, 11, 12) {
        0 => "lsl",
        1 => "lsr",
        2 => "asr",
        3 => "UND",
        _ => unreachable!(),
    };
    write!(
        buffer,
        "{} {}, {}, #{}",
        op,
        reg_str(rd),
        reg_str(rs),
        offset
    )
    .unwrap();
}

fn thumb_disasm_add_sub(opcode: u32, buffer: &mut String) {
    let rd = bits!(opcode, 0, 2);
    let rs = bits!(opcode, 3, 5);
    let op = if bits_b!(opcode, 9) { "sub" } else { "add" };

    if bits_b!(opcode, 10) {
        // immediate operand
        let imm = bits!(opcode, 6, 8);
        write!(buffer, "{} {}, {}, #{}", op, reg_str(rd), reg_str(rs), imm).unwrap();
    } else {
        // register operand
        let rn = bits!(opcode, 6, 8);
        write!(
            buffer,
            "{} {}, {}, {}",
            op,
            reg_str(rd),
            reg_str(rs),
            reg_str(rn)
        )
        .unwrap();
    }
}

fn thumb_disasm_mov_cmp_add_sub_imm(opcode: u32, buffer: &mut String) {
    let offset = bits!(opcode, 0, 7);
    let rd = bits!(opcode, 8, 10);
    let op = match bits!(opcode, 11, 12) {
        0 => "mov",
        1 => "cmp",
        2 => "add",
        3 => "sub",
        _ => unreachable!(),
    };
    write!(buffer, "{} {}, #{}", op, reg_str(rd), offset).unwrap();
}

fn thumb_disasm_alu(opcode: u32, buffer: &mut String) {
    let rd = bits!(opcode, 0, 2);
    let rs = bits!(opcode, 3, 5);
    let op = match bits!(opcode, 6, 9) {
        0x0 => "and",
        0x1 => "eor",
        0x2 => "lsl",
        0x3 => "lsr",
        0x4 => "asr",
        0x5 => "adc",
        0x6 => "sbc",
        0x7 => "ror",
        0x8 => "tst",
        0x9 => "neg",
        0xA => "cmp",
        0xB => "cmn",
        0xC => "orr",
        0xD => "mul",
        0xE => "bic",
        0xF => "mvn",
        _ => unreachable!(),
    };
    write!(buffer, "{} {}, {}", op, reg_str(rd), reg_str(rs)).unwrap();
}

fn thumb_disasm_hi_register_ops(opcode: u32, buffer: &mut String) {
    let rs_hi = bits_b!(opcode, 6);
    let rd_hi = bits_b!(opcode, 7);
    let rd = bits!(opcode, 0, 2) + (if rd_hi { 8 } else { 0 });
    let rs = bits!(opcode, 3, 5) + (if rs_hi { 8 } else { 0 });
    let op_imm = bits!(opcode, 8, 9);

    if op_imm == 3 {
        write!(buffer, "bx {}, {}", reg_str(rd), reg_str(rs)).unwrap();
    } else {
        let op = match op_imm {
            0 => "add",
            1 => "cmp",
            2 => "mov",
            _ => "???", // using unreachable!() here causes linking to fail on my machine :\
        };
        write!(buffer, "{} {}, {}", op, reg_str(rd), reg_str(rs)).unwrap();
    }
}

fn thumb_disasm_pc_relative_load(
    opcode: u32,
    buffer: &mut String,
    address: u32,
    memory: &dyn ArmMemory,
) {
    let pc = address.wrapping_add(4); // PC is 4 ahead in THUMB mode.
    let rd = bits!(opcode, 8, 10);
    let offset = (opcode & 0xFF) << 2;
    let addr = pc.wrapping_add(offset);
    let data = memory.view_word(addr);
    write!(
        buffer,
        "ldr {} [{}, #{}] ; [0x{:08X}] = 0x{:08X}",
        reg_str(rd),
        reg_str(15),
        offset,
        addr,
        data
    )
    .unwrap();
}

fn thumb_disasm_load_store_reg(opcode: u32, buffer: &mut String) {
    let rd = bits!(opcode, 0, 2);
    let rb = bits!(opcode, 3, 5);
    let ro = bits!(opcode, 6, 8);
    let b = bits_b!(opcode, 10);
    let l = bits_b!(opcode, 11);

    let op = match (l, b) {
        (true, true) => "ldrb",
        (true, false) => "ldr",
        (false, true) => "strb",
        (false, false) => "strb",
    };

    write!(
        buffer,
        "{} {}, [{}, {}]",
        op,
        reg_str(rd),
        reg_str(rb),
        reg_str(ro)
    )
    .unwrap();
}

fn thumb_disasm_load_store_se_byte_halfword(opcode: u32, buffer: &mut String) {
    let rd = bits!(opcode, 0, 2);
    let rb = bits!(opcode, 3, 5);
    let ro = bits!(opcode, 6, 8);
    let s = bits_b!(opcode, 10);
    let h = bits_b!(opcode, 11);

    let op = match (s, h) {
        (false, false) => "strh",
        (false, true) => "ldrh",
        (true, false) => "ldsb",
        (true, true) => "ldsh",
    };

    write!(
        buffer,
        "{} {}, [{}, {}]",
        op,
        reg_str(rd),
        reg_str(rb),
        reg_str(ro)
    )
    .unwrap();
}

fn thumb_disasm_load_store_imm(opcode: u32, buffer: &mut String) {
    let rd = bits!(opcode, 0, 2);
    let rb = bits!(opcode, 3, 5);
    let b = bits_b!(opcode, 10);
    let l = bits_b!(opcode, 11);

    let offset = if b {
        bits!(opcode, 6, 10)
    } else {
        bits!(opcode, 6, 10) << 2
    };

    let op = match (l, b) {
        (true, true) => "ldrb",
        (true, false) => "ldr",
        (false, true) => "strb",
        (false, false) => "strb",
    };

    write!(
        buffer,
        "{} {}, [{}, #{}]",
        op,
        reg_str(rd),
        reg_str(rb),
        offset
    )
    .unwrap();
}

fn thumb_disasm_load_store_halfword(opcode: u32, buffer: &mut String) {
    let rd = bits!(opcode, 0, 2);
    let rb = bits!(opcode, 3, 5);
    let l = bits_b!(opcode, 11);
    let op = if l { "ldrh" } else { "strh" };
    let offset = bits!(opcode, 6, 10) << 1;
    write!(
        buffer,
        "{} {}, [{}, #{}]",
        op,
        reg_str(rd),
        reg_str(rb),
        offset
    )
    .unwrap();
}

fn thumb_disasm_load_store_sp_relative(opcode: u32, buffer: &mut String) {
    let rd = bits!(opcode, 8, 10);
    let offset = bits!(opcode, 0, 7) << 2;
    let l = bits_b!(opcode, 11);
    let op = if l { "ldr" } else { "str" };
    write!(
        buffer,
        "{} {}, [{}, #{}]",
        op,
        reg_str(rd),
        reg_str(13),
        offset
    )
    .unwrap();
}

fn thumb_disasm_push_pop_reg(opcode: u32, buffer: &mut String) {
    let rlist = opcode & 0xFF;
    let r = bits_b!(opcode, 8);
    let l = bits_b!(opcode, 11);
    let op = if l { "pop" } else { "push" };
    let dangle = if l { reg_str(15) } else { reg_str(14) };

    buffer.push_str(op);
    buffer.push_str(" { ");
    let comma = write_register_list(rlist, 8, buffer);
    if r {
        if comma {
            buffer.push_str(", ");
        }
        buffer.push_str(dangle);
    }
    buffer.push_str(" }");
}

fn thumb_disasm_multiple_load_store(opcode: u32, buffer: &mut String) {
    let rlist = opcode & 0xFF;
    let l = bits_b!(opcode, 11);
    let op = if l { "ldmia" } else { "stmia" };
    buffer.push_str(op);
    buffer.push_str(" { ");
    write_register_list(rlist, 8, buffer);
    buffer.push_str(" }");
}

fn thumb_disasm_swi(opcode: u32, buffer: &mut String) {
    let comment = opcode & 0xFF;
    write!(buffer, "swi {}", comment).unwrap();
}

fn thumb_disasm_unconditional_branch(opcode: u32, buffer: &mut String, address: u32) {
    let pc = address.wrapping_add(4); // PC is 4 ahead in THUMB mode.
    let offset = sign_extend_32!((opcode & 0x7FF) << 1, 12);
    let dest = pc.wrapping_add(offset) & 0xFFFFFFFE;
    write!(buffer, "b 0x{:08X}", dest).unwrap();
}

fn thumb_disasm_branch_with_link(
    opcode: u32,
    buffer: &mut String,
    address: u32,
    memory: &dyn ArmMemory,
) {
    if bits_b!(opcode, 11) {
        thumb_disasm_branch_with_link_offset(opcode, buffer, address, memory);
    } else {
        thumb_disasm_branch_with_link_setup(opcode, buffer, address);
    }
}

fn thumb_disasm_branch_with_link_setup(opcode: u32, buffer: &mut String, address: u32) {
    let pc = address.wrapping_add(4); // PC is 4 ahead in THUMB mode.
    let off = sign_extend_32!((opcode & 0x7FF) << 12, 23);
    let setup = pc.wrapping_add(off);
    write!(buffer, "bl.setup 0x{:08X}", setup).unwrap();
}

fn thumb_disasm_branch_with_link_offset(
    opcode: u32,
    buffer: &mut String,
    address: u32,
    memory: &dyn ArmMemory,
) {
    // let pc = address.wrapping_add(4); // PC is 4 ahead in THUMB mode.

    let previous_address = address.wrapping_sub(2);
    let previous_opcode = memory.view_halfword(previous_address);
    let previous_pc = previous_address.wrapping_add(4);
    let setup = previous_pc.wrapping_add(sign_extend_32!((previous_opcode & 0x7FF) << 12, 23));

    let off = (opcode & 0x7FF) << 1;
    let dest = setup.wrapping_add(off) & 0xFFFFFFFE;

    write!(buffer, "bl 0x{:08X}", dest).unwrap();
}

fn write_register_list(rlist: u32, reg_count: u32, buffer: &mut String) -> bool {
    assert!(reg_count < 16);

    let mut start = 0u32;
    let mut count = 0u32;
    let mut first = true;

    for r in 0..=reg_count {
        if r != reg_count && (rlist & (1 << r)) != 0 {
            if count == 0 {
                start = r;
                count += 1;
            } else {
                count += 1;
            }
        } else if count != 0 {
            if first {
                first = false;
            } else {
                buffer.push_str(", ");
            }

            if count == 1 {
                write!(buffer, "{}", reg_str(start)).unwrap();
            } else {
                // here we don't use reg str because it's probably better to have something
                // like r0-r14 instead of r0-lr
                write!(buffer, "r{}-r{}", start, start + count - 1).unwrap();
            }
            count = 0;
        }
    }

    return !first;
}

const REGISTERS: [&str; 16] = [
    "r0", "r1", "r2", "r3", "r4", "r5", "r6", "r7", "r8", "r9", "r10", "r11", "r12", "sp", "lr",
    "pc",
];

fn reg_str(reg: u32) -> &'static str {
    if reg > 15 {
        return "r??";
    }
    REGISTERS[reg as usize]
}

const CONDITION_CODES: [&str; 16] = [
    "eq", "ne", "cs", "cc", "mi", "pl", "vs", "vc", "hi", "ls", "ge", "lt", "gt", "le", "al", "nv",
];

fn condition_code_str(code: u32) -> &'static str {
    if code > 15 {
        return "??";
    }
    CONDITION_CODES[code as usize]
}

// AddOffsetToStackPointer,
// SoftwareInterrupt,
// ALUOperations,
// HiRegisterOperations,
// PushPopRegisters,
// AddSubtract,
// PCRelativeLoad,
// LoadStoreWithRegisterOffset,
// LoadStoreSignHalfwordByte,
// LoadStoreHalfword,
// SPRelativeLoadStore,
// LoadAddress,
// MultipleLoadStore,
// ConditionalBranch,
// MoveShiftedRegister,
// MoveCompareAddSubtractImm,
// LoadStoreWithImmOffset,
