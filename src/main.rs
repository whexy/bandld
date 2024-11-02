use flexi_logger::{FileSpec, Logger};
use log::{error, info, warn};
use std::collections::HashMap;
use std::env;
use std::fs;
use std::path::Path;
use std::process::{exit, Command};

fn main() {
    // Initialize the logger to write to /tmp/wrap_symbols.log
    Logger::try_with_str("info")
        .unwrap()
        .log_to_file(
            FileSpec::default()
                .directory("/tmp")
                .basename("wrap_symbols")
                .suffix("log"),
        )
        .use_utc()
        .start()
        .unwrap();

    info!("Starting the wrap symbol checker");

    let args: Vec<String> = env::args().collect();
    let mut wrap_symbols = Vec::new();
    let mut object_files = Vec::new();
    let mut other_args = Vec::new();
    let mut wrap_args = Vec::new();

    // Parse command line arguments
    for arg in &args[1..] {
        if arg.starts_with("--wrap=") {
            let symbol = arg.trim_start_matches("--wrap=");
            wrap_symbols.push(symbol.to_string());
            wrap_args.push(arg.clone());
            info!("Found wrap argument: {}", arg);
        } else if is_library_file(arg) {
            object_files.push(arg.to_string());
            info!("Found object file: {}", arg);
        } else {
            other_args.push(arg.to_string());
            info!("Found other argument: {}", arg);
        }
    }

    if wrap_symbols.is_empty() {
        // No wrap symbols, just call ld directly
        info!("No wrap symbols found, calling ld directly");
        call_ld(&args[1..]);
        return;
    }

    let mut generated_files = Vec::new();

    let mut symbol_map: HashMap<String, (bool, bool)> = HashMap::new();

    // Initialize the symbol_map with wrap_symbols
    for symbol in &wrap_symbols {
        symbol_map.insert(symbol.clone(), (false, false));
    }

    // Check each object file for the wrap symbol and usage of the symbol
    for obj_file in &object_files {
        let output = Command::new("nm")
            .arg(obj_file)
            .output()
            .expect("Failed to execute nm");

        let output_str = String::from_utf8_lossy(&output.stdout);
        for line in output_str.lines() {
            let parts: Vec<&str> = line.split_whitespace().collect();
            if parts.len() >= 2 {
                let sym = parts[parts.len() - 1];
                if let Some((_, symbol_used)) = symbol_map.get_mut(sym) {
                    *symbol_used = true;
                    info!("Found symbol {} in {}", sym, obj_file);
                }
                let wrap_sym = format!("__wrap_{}", sym);
                if let Some((wrap_symbol_found, _)) = symbol_map.get_mut(&wrap_sym) {
                    *wrap_symbol_found = true;
                    info!("Found wrap symbol {} in {}", wrap_sym, obj_file);
                }
            }
        }
    }

    for (symbol, (wrap_symbol_found, symbol_used)) in &symbol_map {
        if !wrap_symbol_found && *symbol_used {
            let wrap_symbol = format!("__wrap_{}", symbol);
            let temp_c_file = format!("/tmp/{}.c", wrap_symbol);
            let temp_o_file = format!("/tmp/{}.o", wrap_symbol);

            // Check if the .o file already exists
            if !Path::new(&temp_o_file).exists() {
                // Generate a default implementation for __wrap_symbol
                // Generate it as a weak symbol so that the real symbol can override it
                let default_impl = format!(
                    "void {wrap_symbol}() __attribute__((weak)) {{
                        extern void __real_{symbol}();
                        __real_{symbol}();
                    }}\n"
                );
                fs::write(&temp_c_file, default_impl).expect("Unable to write C file");
                info!(
                    "Generated C file for symbol {}: {}",
                    wrap_symbol, temp_c_file
                );

                // Compile the C file to an object file
                let compile_status = Command::new("cc")
                    .args(["-c", &temp_c_file, "-o", &temp_o_file])
                    .status()
                    .expect("Failed to compile temporary C file");

                if compile_status.success() {
                    info!("Successfully compiled {} to {}", temp_c_file, temp_o_file);
                } else {
                    error!("Failed to compile {} to {}", temp_c_file, temp_o_file);
                }
            } else {
                info!(
                    "Object file {} already exists, skipping compilation",
                    temp_o_file
                );
            }

            generated_files.push(temp_o_file);
        } else if !*symbol_used {
            warn!("Symbol {} is not used in any object files", symbol);
        }
    }

    // Prepare final ld command with updated object file list
    let mut ld_args = args[1..].to_vec();
    ld_args.extend(generated_files);

    info!("Calling ld with arguments: {:?}", ld_args);
    // Call ld with the new arguments
    call_ld(&ld_args);
}

fn is_library_file(filename: &str) -> bool {
    filename.ends_with(".o")
        || filename.ends_with(".a")
        || filename.ends_with(".so")
        || filename.ends_with(".dylib")
}

fn call_ld(args: &[String]) {
    let status = Command::new("ld-orig")
        .args(args)
        .status()
        .expect("Failed to execute ld-orig");

    if status.success() {
        info!("ld-orig completed successfully");
    } else {
        error!("ld-orig failed with status: {}", status);
        exit(status.code().unwrap_or(1));
    }
}
