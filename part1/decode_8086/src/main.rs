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
        println!("{}", base_8086_instr_read(bytes[0], &mut f));
    }
}

fn base_8086_instr_read(byte: u8, f: &mut File) -> String {
    match byte >> 5 {
        0b100 => match (byte >> 3) & 0b11 {
            0b01 => {
                if (byte >> 2) & 1 == 1 {
                    panic!("Not implemented!")
                } else {
                    register_mem_to_register_mem(byte, f)
                }
            }
            _ => panic!("Not implemented!"),
        },
        0b101 => {
            if (byte >> 4) & 1 == 1 {
                immediate_to_register(byte, f)
            } else {
                match (byte >> 1) & 0b111 {
                    0b000 => memory_to_accumulator(byte, f),
                    0b001 => accumulator_to_memory(byte, f),
                    _ => panic!("Not supported!"),
                }
            }
        }
        0b110 => match (byte >> 3) & 0b11 {
            0b00 => match (byte >> 1) & 0b11 {
                0b11 => immediate_to_register_memory(byte, f),
                _ => panic!("Not implemented!"),
            },
            _ => panic!("Not implemented!"),
        },
        _ => panic!("Impossible!"),
    }
}

fn register_mem_to_register_mem(byte: u8, f: &mut File) -> String {
    let (d, w) = (((byte >> 1) & 1) == 1, (byte & 1) == 1);

    // Split 2nd byte
    let mut regrm = [0_u8; 1];
    assert!(
        matches!(f.read(&mut regrm), Ok(1)),
        "Could not read next byte!"
    );
    let (r#mod, reg, rm) = (regrm[0] >> 6, (regrm[0] >> 3) & 0b111, regrm[0] & 0b111);
    let (reg, rm) = (
        get_reg_code(reg, w).to_owned(),
        get_rm_code(rm, w, r#mod, f),
    );

    let (src, dst) = if d { (rm, reg) } else { (reg, rm) };
    format!("mov {dst}, {src}")
}

fn immediate_to_register(byte: u8, f: &mut File) -> String {
    let (w, reg) = ((byte >> 3) & 1 == 1, byte & 0b111);
    format!(
        "mov {}, {}",
        get_reg_code(reg, w),
        get_immediate(w, false, f)
    )
}

fn immediate_to_register_memory(byte: u8, f: &mut File) -> String {
    let w = byte & 1 == 1;
    let mut nextb = [0_u8; 1];
    assert!(
        matches!(f.read(&mut nextb), Ok(1)),
        "Could not read next bytes!"
    );
    let r#mod = nextb[0] >> 6;
    format!(
        "mov {}, {}",
        get_rm_code(nextb[0] & 0b111, w, r#mod, f),
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
        _ => panic!("Impossible!"),
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

fn get_immediate(wide: bool, explicit: bool, f: &mut File) -> String {
    let immediate = if wide {
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
        format!("{} {immediate}", if wide { "word" } else { "byte" })
    } else {
        format!("{}", immediate)
    }
}
