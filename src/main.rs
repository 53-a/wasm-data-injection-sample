use std::path::PathBuf;

use walrus as wr;
use anyhow::{anyhow, Result, bail, Ok};
use clap::Parser;

fn get_module_config() -> wr::ModuleConfig {
    wr::ModuleConfig::new()
        .generate_name_section(false) // A runtime error occurs if this is set to true 
        .generate_producers_section(false)
        .generate_synthetic_names_for_anonymous_items(false)
        .only_stable_features(false)
        .to_owned()
}

fn restore_exported_names(module: &mut wr::Module) {
    let mut list= Vec::new();
    module.exports.iter().for_each(|e| {
        list.push((e.item, e.name.clone()));
    });
    list.iter().for_each(|(it, name)| {
        if let wr::ExportItem::Function(f) = it {
            module.funcs.get_mut(*f).name.replace(name.clone());
        }
    });
}

fn replace_memory_init(module: &mut wr::Module, data: wr::DataId) -> Result<()> {
    let fid = module.funcs
        .by_name("__core_memory_init")
        .ok_or(anyhow!("function not found"))?;
    let f = module.funcs.get_mut(fid);

    match &mut f.kind {
        wr::FunctionKind::Import(_) => {
            bail!("__core_memory_init must be local function")
        }
        wr::FunctionKind::Local(f) => {
            let (dest, offset, size) = (f.args[0], f.args[1], f.args[2]);
            let mut b = f.builder_mut().func_body();
            b.instrs_mut().clear();
            let mem = module.memories
                .iter()
                .nth(0)
                .ok_or(anyhow!("no memory found"))?
                .id();
            b
                .local_get(dest)
                .local_get(offset)
                .local_get(size)
                .memory_init(mem, data);
        }
        wr::FunctionKind::Uninitialized(_) => todo!(),
    }
    Ok(())
}

fn remove_import(module: &mut wr::Module, name: &str) -> Result<()> {
    let id = module.imports.find("env", name).ok_or(anyhow!(""))?;
    module.imports.delete(id);
    Ok(())
}

fn ensure_function_not_exported(module: &mut wr::Module, name: &str) -> Result<()> {
    let fid = module.funcs
        .by_name(name)
        .ok_or(anyhow!("function {name} not found"))?;
    if let Some(e) = module.exports.get_exported_func(fid) {
        module.exports.delete(e.id())
    }
    Ok(())
}

#[derive(clap::Parser)]
#[command(name = "wasm-data-injection-sample")]
struct Args {
    #[arg(value_name = "SRC_FILE")]
    source_path: PathBuf,
    #[arg(value_name = "DEST_FILE")]
    dest_path: PathBuf,
    message: String,
}
fn main() -> Result<()> {
    let args = Args::parse();
    let mc = get_module_config();
    let mut m = mc.parse_file(args.source_path)?;

    restore_exported_names(&mut m);

    // Prepare data to append to given wasm
    let mut d = ((args.message.len() + 1) as u32).to_be_bytes().to_vec();
    d.append(&mut std::ffi::CString::new(args.message)?.as_bytes_with_nul().to_vec());
    let data_id = m.data.add(wr::DataKind::Passive, d);

    // Generate actual function
    replace_memory_init(&mut m, data_id)?;

    // Cleanups of dummy functions
    remove_import(&mut m, "__core_memory_init_dummy")?;
    ensure_function_not_exported(&mut m, "__core_memory_init")?;

    m.emit_wasm_file(args.dest_path.clone())?;
    println!("Wasm file generated: {}", args.dest_path.to_string_lossy());
    Ok(())
}
