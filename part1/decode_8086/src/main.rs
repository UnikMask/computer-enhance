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

        // Split 1st byte
        let instr = bytes[0];
        match instr {
            // Register/memory to register/memory
            _ if instr >> 2 == 0b100010 => {
                let (d, w) = (((bytes[0] >> 1) & 1) == 1, (bytes[0] & 1) == 1);

                // Split 2nd byte
                let mut regrm = [0_u8; 1];
                assert!(
                    matches!(f.read(&mut regrm), Ok(1)),
                    "Could not read next byte!"
                );
                let (r#mod, reg, rm) = (regrm[0] >> 6, (regrm[0] >> 3) & 0b111, regrm[0] & 0b111);
                let (reg, rm) = (
                    get_reg_code(reg, w).to_owned(),
                    get_rm_code(rm, w, r#mod, &mut f),
                );

                let (src, dst) = if d { (rm, reg) } else { (reg, rm) };
                println!("mov {dst}, {src}")
            }

            // Immediate register to memory
            _ if instr >> 1 == 0b1100011 => {
                let w = bytes[0] == 1;
                let mut nextb = [0_u8; 1];
                assert!(
                    matches!(f.read(&mut nextb), Ok(1)),
                    "Could not read next bytes!"
                );
                let r#mod = nextb[0] >> 6;
                println!(
                    "mov {}, {}",
                    get_rm_code(nextb[0] & 0b111, w, r#mod, &mut f),
                    get_immediate(w, r#mod != 0b11, &mut f)
                );
            }
            _ if instr >> 4 == 0b1011 => {
                let (w, reg) = (bytes[0] >> 3 == 1, bytes[0] & 0b111);
                println!(
                    "mov {}, {}",
                    get_reg_code(reg, w),
                    get_immediate(w, false, &mut f)
                );
            }
            _ => panic!("Not supported yet!"),
        }
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
            6 => {
                let mut address: [u8; 2] = [0; 2];
                assert!(
                    matches!(f.read(&mut address), Ok(2)),
                    "Could not read next byte!"
                );
                format!(
                    "[{:x}]",
                    u16::from(address[0]) + (u16::from(address[1]) << 8)
                )
            }
            7 => "[bx + si]".to_string(),
            _ => panic!("Impossible!"),
        },
        _ => panic!("Impossible!"),
    }
}

fn get_rm_code_with_offset(rm: u8, offset: u16) -> String {
    match rm {
        0 => format!("[bx + si + {offset:x}]"),
        1 => format!("[bx + di + {offset:x}]"),
        2 => format!("[bp + si + {offset:x}]"),
        3 => format!("[bp + di + {offset:x}]"),
        4 => format!("[si + {offset:x}]"),
        5 => format!("[di + {offset:x}]"),
        6 => format!("[bp + {offset:x}]"),
        7 => format!("[bx + {offset:x}]"),
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

fn get_immediate(wide: bool, explicit: bool, f: &mut File) -> String {
    let immediate = if wide {
        let mut nextb = [0_u8; 2];
        assert!(
            matches!(f.read(&mut nextb), Ok(2)),
            "Could not read next bytes!"
        );
        (u16::from(nextb[1]) << 8) + u16::from(nextb[0])
    } else {
        let mut nextb = [0_u8; 2];
        assert!(
            matches!(f.read(&mut nextb), Ok(2)),
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
