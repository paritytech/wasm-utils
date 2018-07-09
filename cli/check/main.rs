extern crate parity_wasm;
extern crate pwasm_utils as utils;
extern crate pwasm_utils_cli as logger;
extern crate clap;

use clap::{App, Arg};
use parity_wasm::elements;

fn fail(msg: &str) -> ! {
    eprintln!("{}", msg);
    std::process::exit(1)
}

const ALLOWED_IMPORTS: &'static [&'static str] = &[
    "ret",
];

fn main() {
    logger::init_log();

    let matches = App::new("wasm-check")
                        .arg(Arg::with_name("input")
                            .index(1)
                            .required(true)
                            .help("Input WASM file"))
                        .get_matches();

    let input = matches.value_of("input").expect("is required; qed");

    let module = parity_wasm::deserialize_file(&input).expect("Input module deserialization failed");

    for section in module.sections() {
        match *section {
            elements::Section::Import(ref import_section) => {
                let mut has_imported_memory_properly_named = false;
                for entry in import_section.entries() {
                    if entry.module() != "env" {
                        fail("All imports should be from env");
                    }
                    match *entry.external() {
                        elements::External::Function(_) => {
                            if !ALLOWED_IMPORTS.contains(&entry.field()) {
                                fail(&format!("'{}' is not supported by the runtime", entry.field()));
                            }
                        },
                        elements::External::Memory(m) => {
                            if entry.field() == "memory" {
                                has_imported_memory_properly_named = true;
                            }

                            let max = if let Some(max) = m.limits().maximum() {
                                max
                            } else {
                                fail("There is a limit on memory in Parity runtime, and this program does not limit memory");
                            };

                            if max > 16 {
                                fail(&format!(
                                    "Parity runtime has 1Mb limit (16 pages) on max contract memory, this program speicifies {}",
                                    max
                                ));
                            }
                        },
                        elements::External::Global(_) => {
                            fail("Parity runtime does not provide any globals")
                        },
                        _ => { continue; }
                    }
                }

                if !has_imported_memory_properly_named {
                    fail("No imported memory from env::memory in the contract");
                }
            }
            _ => { continue; }
        }
    }
}
