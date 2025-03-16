use std::{env, fs::File, io::Read};

fn main() {
    let filename = env::args().nth(1).expect("No filename provided!");

    let mut f = match File::open(&filename) {
        Ok(file) => file,
        Err(e) => {
            panic!("Failed to open {} - {e}", &filename)
        }
    };

    let mut done = false;
    println!("bits 16");
    while !done {
        let mut bytes: [u8; 1] = [0; 1];

        // Read 1st and 2nd byte
        match f.read(&mut bytes[0..1]) {
            Ok(1) => {}
            Err(e) => panic!("IO Error occured - {e}"),
            _ => {
                done = true;
                continue;
            }
        }
        println!(
            "{}",
            base_8086_instr_read(bytes[0], &mut f)
                .or_else(|| jump_read(bytes[0], &mut f))
                .expect("Not implemented!")
        );
    }
}

fn base_8086_instr_read(byte: u8, f: &mut File) -> Option<String> {
    match byte >> 5 {
        0b100 => match (byte >> 3) & 0b11 {
            0b01 => {
                if (byte >> 2) & 1 == 1 {
                    None
                } else {
                    Some(mov_rm_to_rm(byte, f))
                }
            }
            0b00 => {
                if (byte >> 2) & 1 == 1 {
                    let mut nextb = [0_u8; 1];
                    assert!(
                        matches!(f.read(&mut nextb), Ok(1)),
                        "Could not read next bytes!"
                    );
                    match (nextb[0] >> 3) & 0b111 {
                        0b000 => Some(add_imm_to_rm(byte, nextb[0], f)),
                        0b101 => Some(sub_imm_to_rm(byte, nextb[0], f)),
                        0b111 => Some(cmp_imm_to_rm(byte, nextb[0], f)),
                        _ => None,
                    }
                } else {
                    None
                }
            }
            _ => None,
        },
        0b101 => {
            if (byte >> 4) & 1 == 1 {
                Some(mov_immediate_to_register(byte, f))
            } else {
                match (byte >> 1) & 0b111 {
                    0b000 => Some(memory_to_accumulator(byte, f)),
                    0b001 => Some(accumulator_to_memory(byte, f)),
                    _ => panic!("Not supported!"),
                }
            }
        }
        0b110 => match (byte >> 3) & 0b11 {
            0b00 => match (byte >> 1) & 0b11 {
                0b11 => Some(mov_imm_to_rm(byte, f)),
                _ => None,
            },
            _ => None,
        },
        0b000 => match (byte >> 2) & 0b111 {
            0b000 => Some(add_rm_to_rm(byte, f)),
            0b001 => {
                if byte >> 1 == 1 {
                    None
                } else {
                    Some(add_imm_to_acc(byte, f))
                }
            }
            _ => None,
        },
        0b001 => match (byte >> 2) & 0b111 {
            0b010 => Some(sub_rm_to_rm(byte, f)),
            0b110 => Some(cmp_rm_to_rm(byte, f)),
            0b011 => {
                if byte >> 1 == 1 {
                    None
                } else {
                    Some(sub_imm_to_acc(byte, f))
                }
            }
            0b111 => {
                if byte >> 1 == 1 {
                    None
                } else {
                    Some(cmp_imm_to_acc(byte, f))
                }
            }
            _ => None,
        },
        _ => None,
    }
}

fn jump_read(byte: u8, f: &mut File) -> Option<String> {
    match byte {
        0b01110100 => Some(cond_jmp("je", f)),
        0b01111100 => Some(cond_jmp("jl", f)),
        0b01111110 => Some(cond_jmp("jle", f)),
        0b01110010 => Some(cond_jmp("jb", f)),
        0b01110110 => Some(cond_jmp("jbe", f)),
        0b01111010 => Some(cond_jmp("jp", f)),
        0b01110000 => Some(cond_jmp("jo", f)),
        0b01111000 => Some(cond_jmp("js", f)),
        0b01110101 => Some(cond_jmp("jne", f)),
        0b01111101 => Some(cond_jmp("jnl", f)),
        0b01111111 => Some(cond_jmp("jnle", f)),
        0b01110011 => Some(cond_jmp("jnb", f)),
        0b01110111 => Some(cond_jmp("jnbe", f)),
        0b01111011 => Some(cond_jmp("jnp", f)),
        0b01110001 => Some(cond_jmp("jno", f)),
        _ => None
    }
}

//////////////////
// Instructions //
//////////////////


fn cond_jmp(instr: &str, f: &mut File) -> String {
    let mut nextb = [0_u8; 1];
    assert!(
        matches!(f.read(&mut nextb), Ok(1)),
        "Could not read next bytes!"
    );
    format!("{instr} {}", nextb[0] as i8)
}

fn mov_rm_to_rm(byte: u8, f: &mut File) -> String {
    let (dst, src) = register_mem_to_register_mem(byte, f);
    format!("mov {dst}, {src}")
}

fn mov_immediate_to_register(byte: u8, f: &mut File) -> String {
    let (imm, reg) = immediate_to_register(byte, f);
    format!("mov {reg}, {imm}")
}

fn mov_imm_to_rm(byte: u8, f: &mut File) -> String {
    let w = byte & 1;
    let mut nextb = [0_u8; 1];
    assert!(
        matches!(f.read(&mut nextb), Ok(1)),
        "Could not read next bytes!"
    );
    let r#mod = nextb[0] >> 6;
    format!(
        "mov {}, {}",
        get_rm_code(nextb[0] & 0b111, w == 1, r#mod, f),
        get_immediate(w, r#mod != 0b11, f)
    )
}

fn memory_to_accumulator(byte: u8, f: &mut File) -> String {
    let w = byte & 1 == 1;
    format!(
        "mov {}, {}",
        if w { "ax" } else { "al" },
        get_direct_memory(f)
    )
}

fn accumulator_to_memory(byte: u8, f: &mut File) -> String {
    let w = byte & 1 == 1;
    format!(
        "mov {}, {}",
        get_direct_memory(f),
        if w { "ax" } else { "al" }
    )
}

fn add_rm_to_rm(byte: u8, f: &mut File) -> String {
    let (dst, src) = register_mem_to_register_mem(byte, f);
    format!("add {dst}, {src}")
}

fn add_imm_to_rm(byte0: u8, byte1: u8, f: &mut File) -> String {
    let (imm, rm) = immediate_to_rm(byte0, byte1, f);
    format!("add {rm}, {imm}")
}

fn add_imm_to_acc(byte: u8, f: &mut File) -> String {
    let (imm, acc) = immediate_to_acc(byte, f);
    format!("add {acc}, {imm}")
}

fn sub_rm_to_rm(byte: u8, f: &mut File) -> String {
    let (dst, src) = register_mem_to_register_mem(byte, f);
    format!("sub {dst}, {src}")
}

fn sub_imm_to_rm(byte0: u8, byte1: u8, f: &mut File) -> String {
    let (imm, rm) = immediate_to_rm(byte0, byte1, f);
    format!("sub {rm}, {imm}")
}

fn sub_imm_to_acc(byte: u8, f: &mut File) -> String {
    let (imm, acc) = immediate_to_acc(byte, f);
    format!("sub {acc}, {imm}")
}

fn cmp_rm_to_rm(byte: u8, f: &mut File) -> String {
    let (dst, src) = register_mem_to_register_mem(byte, f);
    format!("cmp {dst}, {src}")
}

fn cmp_imm_to_rm(byte0: u8, byte1: u8, f: &mut File) -> String {
    let (imm, rm) = immediate_to_rm(byte0, byte1, f);
    format!("cmp {rm}, {imm}")
}

fn cmp_imm_to_acc(byte: u8, f: &mut File) -> String {
    let (imm, acc) = immediate_to_acc(byte, f);
    format!("cmp {acc}, {imm}")
}

////////////////////////////////////////
// Common operations for instructions //
////////////////////////////////////////

fn get_immediate(sw: u8, explicit: bool, f: &mut File) -> String {
    let immediate = if sw == 1 {
        let mut nextb = [0_u8; 2];
        assert!(
            matches!(f.read(&mut nextb), Ok(2)),
            "Could not read next bytes!"
        );
        (u16::from(nextb[1]) << 8) + u16::from(nextb[0])
    } else {
        let mut nextb = [0_u8; 1];
        assert!(
            matches!(f.read(&mut nextb), Ok(1)),
            "Could not read next bytes!"
        );
        u16::from(nextb[0])
    };
    if explicit {
        format!("{} {immediate}", if sw & 1 == 1 { "word" } else { "byte" })
    } else {
        format!("{}", immediate)
    }
}

fn get_rm_code(rm: u8, wide: bool, r#mod: u8, f: &mut File) -> String {
    match r#mod {
        0b11 => get_reg_code(rm, wide).to_owned(),
        0b01 => {
            let mut offset: [u8; 1] = [0; 1];
            assert!(
                matches!(f.read(&mut offset), Ok(1)),
                "Could not read next byte!"
            );
            get_rm_code_with_offset(rm, offset[0].into())
        }
        0b10 => {
            let mut offset: [u8; 2] = [0; 2];
            assert!(
                matches!(f.read(&mut offset), Ok(2)),
                "Could not read next byte!"
            );
            get_rm_code_with_offset(rm, u16::from(offset[0]) + (u16::from(offset[1]) << 8))
        }
        0b00 => match rm {
            0 => "[bx + si]".to_string(),
            1 => "[bx + di]".to_string(),
            2 => "[bp + si]".to_string(),
            3 => "[bp + di]".to_string(),
            4 => "[si]".to_string(),
            5 => "[di]".to_string(),
            6 => get_direct_memory(f),
            7 => "[bx + si]".to_string(),
            _ => panic!("Impossible!"),
        },
        _ =>  panic!("Impossible!"),
    }
}

fn get_rm_code_with_offset(rm: u8, offset: u16) -> String {
    match rm {
        0 => format!("[bx + si + {offset}]"),
        1 => format!("[bx + di + {offset}]"),
        2 => format!("[bp + si + {offset}]"),
        3 => format!("[bp + di + {offset}]"),
        4 => format!("[si + {offset}]"),
        5 => format!("[di + {offset}]"),
        6 => format!("[bp + {offset}]"),
        7 => format!("[bx + {offset}]"),
        _ => panic!("Impossible!"),
    }
}

fn get_reg_code(instr: u8, wide: bool) -> &'static str {
    match (wide, instr) {
        (true, 0) => "ax",
        (true, 1) => "cx",
        (true, 2) => "dx",
        (true, 3) => "bx",
        (true, 4) => "sp",
        (true, 5) => "bp",
        (true, 6) => "si",
        (true, 7) => "di",
        (false, 0) => "al",
        (false, 1) => "cl",
        (false, 2) => "dl",
        (false, 3) => "bl",
        (false, 4) => "ah",
        (false, 5) => "ch",
        (false, 6) => "dh",
        (false, 7) => "bh",
        _ => panic!("Impossible!"),
    }
}

fn get_direct_memory(f: &mut File) -> String {
    let mut address: [u8; 2] = [0; 2];
    assert!(
        matches!(f.read(&mut address), Ok(2)),
        "Could not read next byte!"
    );
    format!("[{}]", u16::from(address[0]) + (u16::from(address[1]) << 8))
}

// get the reg and r/m from a register to/from r/m instruction kind
fn register_mem_to_register_mem(byte: u8, f: &mut File) -> (String, String) {
    let mut regrm = [0_u8; 1];
    let (d, w) = (((byte >> 1) & 1) == 1, (byte & 1) == 1);
    assert!(
        matches!(f.read(&mut regrm), Ok(1)),
        "Could not read next byte!"
    );
    let (r#mod, reg, rm) = (regrm[0] >> 6, (regrm[0] >> 3) & 0b111, regrm[0] & 0b111);
    let (reg, rm) = (
        get_reg_code(reg, w).to_owned(),
        get_rm_code(rm, w, r#mod, f),
    );
    if d { (reg, rm) } else { (rm, reg) }
}

fn immediate_to_register(byte: u8, f: &mut File) -> (String, &str) {
    let (w, reg) = ((byte >> 3) & 1, byte & 0b111);
    (get_immediate(w, false, f), get_reg_code(reg, w == 1))
}

fn immediate_to_rm(byte0: u8, byte1: u8, f: &mut File) -> (String, String) {
    let sw = byte0 & 0b11;
    let mut nextb = [0_u8; 1];
    assert!(
        matches!(f.read(&mut nextb), Ok(1)),
        "Could not read next bytes!"
    );
    (
        get_immediate(sw, (byte1 >> 6) != 0b11, f),
        get_rm_code(byte1 & 0b111, sw == 1, byte1 >> 6, f),
    )
}

fn immediate_to_acc(byte: u8, f: &mut File) -> (String, &str) {
    let w = byte & 1;
    (get_immediate(w, false, f), if w == 1 { "ax" } else { "al" })
}
