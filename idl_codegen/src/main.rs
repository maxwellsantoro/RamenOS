use anyhow::{Context, Result, anyhow};
use clap::{Parser, ValueEnum};
use serde::Deserialize;
use std::{
    fs,
    io::Write,
    path::{Path, PathBuf},
    process::{Command, Stdio},
};

#[derive(Parser, Debug)]
struct Args {
    /// Input IDL file (TOML)
    #[arg(long)]
    r#in: PathBuf,

    /// Output Rust file
    #[arg(long)]
    out: PathBuf,

    /// Output language. Defaults from --out extension (.h => c, otherwise rust).
    #[arg(long, value_enum)]
    lang: Option<Lang>,
}

#[derive(Debug, Clone, Copy, ValueEnum)]
enum Lang {
    Rust,
    C,
    /// WASM guest-side imports (SDK bindings)
    WasmImports,
    /// WASM host-side functions (native runner)
    WasmHost,
}

#[derive(Debug, Clone)]
struct Idl {
    namespace: String,
    version: String,
    protocol: u32,
    message: std::collections::BTreeMap<String, Msg>,
}

#[derive(Debug, Clone)]
struct Msg {
    msg_type: u32,
    fields: Vec<String>,
}

#[derive(Debug, Deserialize)]
struct IdlFile {
    namespace: String,
    version: String,
    protocol: Option<u32>,
    message: std::collections::BTreeMap<String, MsgFile>,
}

#[derive(Debug, Deserialize)]
struct MsgFile {
    msg_type: u32,
    fields: Vec<String>,
}

fn parse_idl(raw: &str) -> Result<Idl> {
    let file: IdlFile = toml::from_str(raw).with_context(|| "parse toml idl")?;
    let protocol = file.protocol.context("IDL protocol is required")?;
    if protocol == 0 {
        return Err(anyhow!("IDL protocol must be non-zero"));
    }
    let message = file
        .message
        .into_iter()
        .map(|(name, msg)| {
            if msg.msg_type == 0 {
                return Err(anyhow!("IDL message '{}' msg_type must be non-zero", name));
            }
            Ok((
                name,
                Msg {
                    msg_type: msg.msg_type,
                    fields: msg.fields,
                },
            ))
        })
        .collect::<Result<_>>()?;
    Ok(Idl {
        namespace: file.namespace,
        version: file.version,
        protocol,
        message,
    })
}

fn kernel_api_module_name(src_path: &std::path::Path) -> String {
    src_path
        .file_stem()
        .and_then(|s| s.to_str())
        .unwrap_or("generated")
        .to_string()
}

fn kernel_api_type_path(src_path: &std::path::Path, struct_name: &str) -> String {
    match kernel_api_module_name(src_path).as_str() {
        "echo_harness_v1" => {
            format!("kernel_api::generated::echo_harness_v1::{struct_name}")
        }
        "trace_service_v2" => {
            format!("kernel_api::generated::trace_service_v2::{struct_name}")
        }
        "semantic_state_v1" => {
            format!("kernel_api::generated::semantic_state_v1::{struct_name}")
        }
        _ => format!("kernel_api::generated::{struct_name}"),
    }
}

fn main() -> Result<()> {
    let args = Args::parse();
    let raw = fs::read_to_string(&args.r#in).with_context(|| "read idl")?;
    let idl = parse_idl(&raw)?;

    let lang = args.lang.unwrap_or_else(|| infer_lang(&args.out));
    let out = match lang {
        Lang::Rust => render_rust(&idl, &args.r#in)?,
        Lang::C => render_c(&idl, &args.r#in, &args.out)?,
        Lang::WasmImports => render_wasm_imports(&idl, &args.r#in)?,
        Lang::WasmHost => render_wasm_host(&idl, &args.r#in)?,
    };
    let out = match lang {
        Lang::Rust | Lang::WasmImports | Lang::WasmHost => rustfmt_source(&out)?,
        Lang::C => out,
    };

    if let Some(parent) = args.out.parent() {
        fs::create_dir_all(parent).ok();
    }
    fs::write(&args.out, out).with_context(|| "write output")?;
    Ok(())
}

fn infer_lang(out: &Path) -> Lang {
    match out.extension().and_then(|e| e.to_str()) {
        Some("h") => Lang::C,
        _ => Lang::Rust,
    }
}

fn source_display(src_path: &Path) -> String {
    if let Ok(cwd) = std::env::current_dir() {
        if let Ok(stripped) = src_path.strip_prefix(cwd) {
            return stripped.to_string_lossy().into_owned();
        }
    }
    src_path.to_string_lossy().into_owned()
}

fn rustfmt_source(source: &str) -> Result<String> {
    let mut child = Command::new("rustfmt")
        .arg("--edition")
        .arg("2021")
        .arg("--emit")
        .arg("stdout")
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .with_context(|| "spawn rustfmt")?;

    child
        .stdin
        .as_mut()
        .context("open rustfmt stdin")?
        .write_all(source.as_bytes())
        .with_context(|| "write generated source to rustfmt")?;

    let output = child
        .wait_with_output()
        .with_context(|| "wait for rustfmt")?;
    if !output.status.success() {
        return Err(anyhow!(
            "rustfmt failed: {}",
            String::from_utf8_lossy(&output.stderr)
        ));
    }
    String::from_utf8(output.stdout).with_context(|| "rustfmt emitted non-UTF8 output")
}

fn render_rust(idl: &Idl, src_path: &Path) -> Result<String> {
    let mut out = String::new();
    out.push_str("// GENERATED FILE. DO NOT EDIT BY HAND.\n");
    out.push_str(&format!("// Source: {}\n\n", source_display(src_path)));
    out.push_str(&format!(
        "// namespace = {}, version = {}\n\n",
        idl.namespace, idl.version
    ));
    out.push_str(&format!(
        "pub const {}_PROTOCOL_ID: u32 = {};\n\n",
        source_const_prefix(src_path),
        idl.protocol
    ));

    for (name, msg) in &idl.message {
        out.push_str(&format!(
            "pub const MSG_{}_{}: u32 = {};\n\n",
            source_const_prefix(src_path),
            const_name(name),
            msg.msg_type
        ));
        out.push_str("#[repr(C)]\n#[derive(Copy, Clone, Debug)]\n");
        out.push_str(&format!("pub struct {} {{\n", pascal(name)));
        for f in &msg.fields {
            let (field, ty) = f
                .split_once(':')
                .with_context(|| format!("field must be name:type: {f}"))?;
            let rust_ty = map_ty_rust(ty).with_context(|| {
                format!(
                    "unknown IDL type '{}' in message '{}' field '{}'",
                    ty, name, field
                )
            })?;
            out.push_str(&format!("    pub {}: {},\n", field, rust_ty));
        }
        out.push_str("}\n\n");
    }
    Ok(out)
}

fn render_c(idl: &Idl, src_path: &Path, out_path: &Path) -> Result<String> {
    let mut out = String::new();
    let guard = c_header_guard(out_path);

    out.push_str("/* GENERATED FILE. DO NOT EDIT BY HAND. */\n");
    out.push_str(&format!("/* Source: {} */\n", source_display(src_path)));
    out.push_str(&format!(
        "/* namespace = {}, version = {} */\n\n",
        idl.namespace, idl.version
    ));
    out.push_str(&format!("#ifndef {}\n#define {}\n\n", guard, guard));
    out.push_str("#include <stdint.h>\n\n");

    for (name, msg) in &idl.message {
        out.push_str("typedef struct {\n");
        for f in &msg.fields {
            let (field, ty) = f
                .split_once(':')
                .with_context(|| format!("field must be name:type: {f}"))?;
            let (c_ty, array_size) = map_ty_c(ty).with_context(|| {
                format!(
                    "unknown IDL type '{}' in message '{}' field '{}'",
                    ty, name, field
                )
            })?;
            if let Some(size) = array_size {
                // C array syntax: type name[size];
                out.push_str(&format!("    {} {}[{}];\n", c_ty, field, size));
            } else {
                out.push_str(&format!("    {} {};\n", c_ty, field));
            }
        }
        out.push_str(&format!("}} {};\n\n", pascal(name)));
    }

    out.push_str(&format!("#endif /* {} */\n", guard));
    Ok(out)
}

fn render_wasm_imports(idl: &Idl, src_path: &Path) -> Result<String> {
    let mut out = String::new();
    out.push_str("// GENERATED FILE. DO NOT EDIT BY HAND.\n");
    out.push_str(&format!("// Source: {}\n\n", source_display(src_path)));
    out.push_str(&format!(
        "// namespace = {}, version = {}\n\n",
        idl.namespace, idl.version
    ));
    out.push_str("//! WASM guest-side imports for SDK\n");
    out.push_str("//!\n");
    out.push_str("//! These functions are imported from the host runner and provide\n");
    out.push_str("//! access to harness endpoints via capability handles.\n\n");
    out.push_str("#![allow(dead_code)]\n\n");

    // Generate extern declarations for each message type
    for (name, msg) in &idl.message {
        out.push_str(&format!(
            "mod {} {{\n",
            name.to_lowercase().replace('-', "_")
        ));

        // Generate the extern "C" import for the harness call
        out.push_str(&format!(
            "    #[link(wasm_import_module = \"ramen::{}\")]\n",
            idl.namespace
        ));
        out.push_str("    extern \"C\" {\n");
        out.push_str(&format!(
            "        #[link_name = \"{}::call\"]\n",
            name.to_lowercase().replace('-', "_")
        ));
        out.push_str("        fn harness_call(\n");
        out.push_str("            cap: u64,\n");

        // Add parameters based on message fields
        for f in &msg.fields {
            let (field, ty) = f
                .split_once(':')
                .with_context(|| format!("field must be name:type: {f}"))?;
            if field == "cap_handle" {
                continue;
            }
            let wasm_ty = map_ty_wasm_param(ty).with_context(|| {
                format!(
                    "unknown IDL type '{}' in message '{}' field '{}'",
                    ty, name, field
                )
            })?;
            if ty == "string" || ty == "bytes" {
                out.push_str(&format!("            {}_ptr: i32,\n", field));
                out.push_str(&format!("            {}_len: u32,\n", field));
            } else {
                out.push_str(&format!("            {}: {},\n", field, wasm_ty));
            }
        }

        // Return parameters (out ptr and out len)
        out.push_str("            out_ptr: *mut u8,\n");
        out.push_str("            out_len: *mut i32,\n");
        out.push_str("        ) -> i32;\n");
        out.push_str("    }\n\n");

        // Generate a client wrapper type
        out.push_str(&format!(
            "    /// Client for {}::{} harness\n",
            idl.namespace, name
        ));
        out.push_str("    ///\n");
        out.push_str("    /// Created from a capability handle provided by the runner.\n");
        out.push_str(&format!("    pub struct {}Client {{\n", pascal(name)));
        out.push_str("        cap: u64,\n");
        out.push_str("    }\n\n");

        out.push_str(&format!("    impl {}Client {{\n", pascal(name)));
        out.push_str("        /// Create client from capability handle\n");
        out.push_str("        #[inline]\n");
        out.push_str("        pub const fn from_cap(cap: u64) -> Self {\n");
        out.push_str("            Self { cap }\n");
        out.push_str("        }\n\n");

        // Generate the send method
        out.push_str("        /// Call the harness endpoint\n");
        out.push_str("        ///\n");
        out.push_str("        /// Returns status plus the reply length written by the host.\n");
        out.push_str("        /// The reply bytes are written to the provided output buffer.\n");
        out.push_str("        #[inline]\n");
        out.push_str("        pub fn call(&self");

        // Add input parameters based on message fields
        for f in &msg.fields {
            let (field, ty) = f
                .split_once(':')
                .with_context(|| format!("field must be name:type: {f}"))?;
            if field == "cap_handle" {
                continue;
            }
            let rust_ty = map_ty_rust_wasm_import(ty).with_context(|| {
                format!(
                    "unknown IDL type '{}' in message '{}' field '{}'",
                    ty, name, field
                )
            })?;
            out.push_str(&format!(", {}: {}", field, rust_ty));
        }

        out.push_str(", out_buf: &mut [u8]) -> (crate::Status, usize) {\n");

        out.push_str("            let mut out_len: i32 = out_buf.len() as i32;\n");
        out.push_str("            let status = unsafe {\n");
        out.push_str("                harness_call(\n");
        out.push_str("                    self.cap,\n");

        // Pass each field
        for f in &msg.fields {
            let (field, ty) = f
                .split_once(':')
                .with_context(|| format!("field must be name:type: {f}"))?;
            if field == "cap_handle" {
                continue;
            }
            if ty == "string" || ty == "bytes" {
                out.push_str(&format!("                    {}.as_ptr() as i32,\n", field));
                out.push_str(&format!("                    {}.len() as u32,\n", field));
            } else if ty.starts_with("bytes") && ty.len() > 5 {
                // Fixed-size byte arrays: pass as pointer
                out.push_str(&format!("                    {}.as_ptr() as i32,\n", field));
            } else {
                out.push_str(&format!("                    {},\n", field));
            }
        }

        out.push_str("                    out_buf.as_mut_ptr(),\n");
        out.push_str("                    &mut out_len,\n");
        out.push_str("                )\n");
        out.push_str("            };\n");
        out.push_str(
            "            let reply_len = if out_len < 0 { 0 } else { core::cmp::min(out_len as usize, out_buf.len()) };\n",
        );
        out.push_str("            (crate::Status::from_raw(status), reply_len)\n");
        out.push_str("        }\n");
        out.push_str("    }\n");
        out.push_str("}\n\n");

        // Re-export the client
        out.push_str(&format!(
            "pub use {}::{}Client;\n\n",
            name.to_lowercase().replace('-', "_"),
            pascal(name)
        ));
    }

    Ok(out)
}

fn render_wasm_host(idl: &Idl, src_path: &Path) -> Result<String> {
    let mut out = String::new();
    out.push_str("// GENERATED FILE. DO NOT EDIT BY HAND.\n");
    out.push_str(&format!("// Source: {}\n\n", source_display(src_path)));
    out.push_str(&format!(
        "// namespace = {}, version = {}\n\n",
        idl.namespace, idl.version
    ));
    out.push_str("use wasmtime::*;\n");
    out.push_str("use crate::context::InstanceContext;\n");
    out.push_str("use crate::error::Status;\n");
    out.push_str("use kernel_api::ipc::Envelope;\n");
    out.push_str("use kernel_api::wire::write_payload;\n\n");

    out.push_str(&format!(
        "/// Register host functions for {} harness\n",
        idl.namespace
    ));
    out.push_str(&format!(
        "pub fn register_{}_host(linker: &mut Linker<InstanceContext>, memory: Memory) -> Result<()> {{\n",
        idl.namespace.replace('.', "_")
    ));

    for (name, msg) in &idl.message {
        let func_name = name.to_lowercase().replace('-', "_");
        out.push_str(&format!(
            "    // Host function for {}::{}\n",
            idl.namespace, name
        ));
        out.push_str("    linker.func_wrap(\n");
        out.push_str(&format!("        \"ramen::{}\",\n", idl.namespace));
        out.push_str(&format!("        \"{}::call\",\n", func_name));
        out.push_str("        move |mut caller: Caller<'_, InstanceContext>,\n");
        let cap_param_used = msg.fields.iter().any(|f| {
            f.split_once(':')
                .map(|(name, _)| name == "cap" || name == "cap_handle")
                .unwrap_or(false)
        });
        out.push_str(if cap_param_used {
            "        cap: u64,\n"
        } else {
            "        _cap: u64,\n"
        });

        // Add parameters based on message fields
        for f in &msg.fields {
            let (field, ty) = f
                .split_once(':')
                .with_context(|| format!("field must be name:type: {f}"))?;
            if field == "cap_handle" {
                continue;
            }
            let wasm_ty = map_ty_wasm_param(ty)?;
            if ty == "string" || ty == "bytes" {
                out.push_str(&format!("        {}: i32,\n", field));
                out.push_str(&format!("        {}_len: u32,\n", field));
            } else {
                out.push_str(&format!("        {}: {},\n", field, wasm_ty));
            }
        }

        if name == "shmem_write" {
            out.push_str("        _out_ptr: u32,\n");
            out.push_str("        _out_len_ptr: u32| -> i32 {\n");
        } else {
            out.push_str("        out_ptr: u32,\n");
            out.push_str("        out_len_ptr: u32| -> i32 {\n");
        }

        if name == "shmem_write" {
            out.push_str("        let data_slice = memory.data(&caller);\n");
            out.push_str("        let data_ptr = data_offset;\n");
            out.push_str("        let data_end = match (data_ptr as usize).checked_add(len as usize) { Some(e) => e, None => return Status::InvalidArgument as i32 };\n");
            out.push_str("        if data_end > data_slice.len() { return Status::InvalidArgument as i32; }\n");
            out.push_str("        let bytes = data_slice[data_ptr as usize..data_end].to_vec();\n");
            out.push_str("        match caller.data_mut().kernel_bridge.shmem_write(shm_cap, offset, &bytes) {\n");
            out.push_str("            Ok(_n) => Status::Ok as i32,\n");
            out.push_str("            Err(e) => e as i32,\n");
            out.push_str("        }\n");
        } else if name == "shmem_read" {
            out.push_str("        match caller.data_mut().kernel_bridge.shmem_read(shm_cap, offset, len as usize) {\n");
            out.push_str("            Ok(bytes) => {\n");
            out.push_str("                let data_mut = memory.data_mut(&mut caller);\n");
            out.push_str("                let copy_len = bytes.len().min(data_mut.len().saturating_sub(out_ptr as usize));\n");
            out.push_str("                if out_ptr as usize + copy_len <= data_mut.len() {\n");
            out.push_str("                    data_mut[out_ptr as usize..out_ptr as usize + copy_len].copy_from_slice(&bytes[..copy_len]);\n");
            out.push_str("                }\n");
            out.push_str("                if out_len_ptr as usize + 4 <= data_mut.len() {\n");
            out.push_str("                    data_mut[out_len_ptr as usize..out_len_ptr as usize + 4].copy_from_slice(&(copy_len as u32).to_le_bytes());\n");
            out.push_str("                }\n");
            out.push_str("                Status::Ok as i32\n");
            out.push_str("            },\n");
            out.push_str("            Err(e) => e as i32,\n");
            out.push_str("        }\n");
        } else {
            out.push_str(&format!(
                "        let mut env = Envelope::empty({}, {});\n",
                idl.protocol, msg.msg_type
            ));
            out.push_str(&format!(
                "        let req = {} {{ \n",
                kernel_api_type_path(src_path, &pascal(name))
            ));

            // Map IDL fields to struct fields
            for f in &msg.fields {
                let (field, _ty) = f.split_once(':').unwrap();
                let param = if field == "cap" || field == "cap_handle" {
                    "cap"
                } else {
                    field
                };
                if field == param {
                    out.push_str(&format!("            {field},\n"));
                } else {
                    out.push_str(&format!("            {field}: {param},\n"));
                }
            }
            out.push_str("        };\n\n");

            out.push_str("        if write_payload(&mut env, &req).is_err() { return Status::InternalError as i32; }\n");
            out.push_str(
                "        let reply_env = match caller.data_mut().kernel_bridge.transact(env) {\n",
            );
            out.push_str("            Ok(e) => e,\n");
            out.push_str("            Err(_e) => return Status::KernelError as i32,\n");
            out.push_str("        };\n\n");

            out.push_str("        // Write reply back to linear memory\n");
            out.push_str("        let data = memory.data_mut(&mut caller);\n");
            out.push_str("        let copy_len = reply_env.payload_len as usize;\n");
            out.push_str("        if out_ptr as usize + copy_len <= data.len() {\n");
            out.push_str("            data[out_ptr as usize..out_ptr as usize + copy_len].copy_from_slice(&reply_env.payload[..copy_len]);\n");
            out.push_str("        }\n");
            out.push_str("        if out_len_ptr as usize + 4 <= data.len() {\n");
            out.push_str("            data[out_len_ptr as usize..out_len_ptr as usize + 4].copy_from_slice(&(copy_len as u32).to_le_bytes());\n");
            out.push_str("        }\n");
            out.push_str("        Status::Ok as i32\n");
        }
        out.push_str("    })?;\n\n");
    }

    out.push_str("    Ok(())\n");
    out.push_str("}\n");

    Ok(out)
}

fn map_ty_wasm_param(t: &str) -> Result<&'static str> {
    match t {
        "u64" => Ok("u64"),
        "u32" => Ok("u32"),
        "u16" => Ok("u32"),    // WASM doesn't have i16, widen to i32
        "u8" => Ok("u32"),     // WASM doesn't have i8, widen to i32
        "string" => Ok("i32"), // ptr as i32
        "bytes" => Ok("i32"),  // ptr as i32
        // Fixed-size byte arrays: passed as pointers in WASM
        other if other.starts_with("bytes") && other.len() > 5 => {
            // Validate the size but treat as pointer
            let size_str = &other[5..];
            let size: usize = size_str
                .parse()
                .map_err(|_| anyhow!("invalid bytes size in type: {}", other))?;
            if size == 0 || size > 64 {
                return Err(anyhow!("bytes size must be 1-64, got: {}", size));
            }
            Ok("i32") // ptr as i32
        }
        _ => Err(anyhow!("unknown IDL type for WASM: {t}")),
    }
}

fn c_header_guard(out_path: &std::path::Path) -> String {
    let stem = out_path
        .file_stem()
        .and_then(|s| s.to_str())
        .unwrap_or("GENERATED");
    let mut guard = String::new();
    for ch in stem.chars() {
        if ch.is_ascii_alphanumeric() {
            guard.push(ch.to_ascii_uppercase());
        } else {
            guard.push('_');
        }
    }
    guard.push_str("_H");
    guard
}

fn map_ty_rust(t: &str) -> Result<String> {
    match t {
        "u64" => Ok("u64".to_string()),
        "u32" => Ok("u32".to_string()),
        "u16" => Ok("u16".to_string()),
        "u8" => Ok("u8".to_string()),
        "string" | "bytes" => Err(anyhow!(
            "dynamic IDL type '{}' is not wire-safe for direct Rust IPC structs",
            t
        )),
        // Fixed-size byte arrays: bytesN where N is the size
        other if other.starts_with("bytes") && other.len() > 5 => {
            let size_str = &other[5..];
            let size: usize = size_str
                .parse()
                .map_err(|_| anyhow!("invalid bytes size in type: {}", other))?;
            if size == 0 || size > 64 {
                return Err(anyhow!("bytes size must be 1-64, got: {}", size));
            }
            Ok(format!("[u8; {}]", size))
        }
        _ => Err(anyhow!("unknown IDL type: {t}")),
    }
}

fn map_ty_rust_wasm_import(t: &str) -> Result<String> {
    match t {
        "string" => Ok("&str".to_string()),
        "bytes" => Ok("&[u8]".to_string()),
        _ => map_ty_rust(t),
    }
}

/// Returns (base_type, optional_array_size) for C codegen.
/// Fixed-size byte arrays like bytes32 return ("uint8_t", Some(32))
/// so render_c can emit correct syntax: "uint8_t field[32];"
fn map_ty_c(t: &str) -> Result<(String, Option<usize>)> {
    match t {
        "u64" => Ok(("uint64_t".to_string(), None)),
        "u32" => Ok(("uint32_t".to_string(), None)),
        "u16" => Ok(("uint16_t".to_string(), None)),
        "u8" => Ok(("uint8_t".to_string(), None)),
        "string" => Ok(("const char*".to_string(), None)),
        "bytes" => Ok(("const uint8_t*".to_string(), None)),
        // Fixed-size byte arrays: bytesN where N is the size
        other if other.starts_with("bytes") && other.len() > 5 => {
            let size_str = &other[5..];
            let size: usize = size_str
                .parse()
                .map_err(|_| anyhow!("invalid bytes size in type: {}", other))?;
            if size == 0 || size > 64 {
                return Err(anyhow!("bytes size must be 1-64, got: {}", size));
            }
            Ok(("uint8_t".to_string(), Some(size)))
        }
        _ => Err(anyhow!("unknown IDL type: {t}")),
    }
}

fn pascal(s: &str) -> String {
    let mut out = String::new();
    let mut upper = true;
    for ch in s.chars() {
        if ch == '_' || ch == '-' {
            upper = true;
            continue;
        }
        if upper {
            out.push(ch.to_ascii_uppercase());
            upper = false;
        } else {
            out.push(ch);
        }
    }
    out
}

fn source_const_prefix(src_path: &std::path::Path) -> String {
    let stem = src_path
        .file_stem()
        .and_then(|s| s.to_str())
        .unwrap_or("generated");
    const_name(stem)
}

fn const_name(s: &str) -> String {
    let mut out = String::new();
    let mut previous_was_sep = false;
    for ch in s.chars() {
        if ch.is_ascii_alphanumeric() {
            out.push(ch.to_ascii_uppercase());
            previous_was_sep = false;
        } else if !previous_was_sep {
            out.push('_');
            previous_was_sep = true;
        }
    }
    while out.ends_with('_') {
        out.pop();
    }
    out
}

#[cfg(test)]
mod tests {
    use super::{
        Idl, Lang, Msg, const_name, infer_lang, map_ty_c, map_ty_rust, map_ty_wasm_param,
        parse_idl, pascal, render_c, render_rust, render_wasm_host, render_wasm_imports,
        source_const_prefix,
    };
    use std::path::Path;

    #[test]
    fn pascal_cases() {
        assert_eq!(pascal("ping"), "Ping");
        assert_eq!(pascal("ping_pong"), "PingPong");
        assert_eq!(pascal("ping-pong"), "PingPong");
    }

    #[test]
    fn map_ty_known_and_unknown() {
        assert_eq!(map_ty_rust("u64").unwrap(), "u64");
        assert_eq!(map_ty_rust("u8").unwrap(), "u8");
        assert_eq!(map_ty_rust("bytes32").unwrap(), "[u8; 32]");
        assert_eq!(map_ty_rust("bytes16").unwrap(), "[u8; 16]");
        assert!(map_ty_rust("unknown").is_err());
        assert!(map_ty_rust("bytes").is_err()); // dynamic bytes are not direct IPC wire types
        assert!(map_ty_rust("string").is_err()); // dynamic strings are not direct IPC wire types
        assert!(map_ty_rust("bytes0").is_err()); // zero size invalid
        assert!(map_ty_rust("bytes65").is_err()); // too large
        assert_eq!(map_ty_c("u32").unwrap(), ("uint32_t".to_string(), None));
        assert_eq!(
            map_ty_c("bytes32").unwrap(),
            ("uint8_t".to_string(), Some(32))
        );
        assert!(map_ty_c("unknown").is_err());
    }

    #[test]
    fn const_prefix_is_stable() {
        assert_eq!(
            source_const_prefix(Path::new("idl/harness/domain_manager_v1.toml")),
            "DOMAIN_MANAGER_V1"
        );
        assert_eq!(const_name("query-by_path"), "QUERY_BY_PATH");
    }

    #[test]
    fn map_ty_wasm_param_known_and_unknown() {
        // Integer types widen to i32 in WASM
        assert_eq!(map_ty_wasm_param("u64").unwrap(), "u64");
        assert_eq!(map_ty_wasm_param("u32").unwrap(), "u32");
        assert_eq!(map_ty_wasm_param("u16").unwrap(), "u32");
        assert_eq!(map_ty_wasm_param("u8").unwrap(), "u32");
        // Pointer types are i32 in WASM32
        assert_eq!(map_ty_wasm_param("string").unwrap(), "i32");
        assert_eq!(map_ty_wasm_param("bytes").unwrap(), "i32");
        // Fixed-size byte arrays are also passed as pointers
        assert_eq!(map_ty_wasm_param("bytes32").unwrap(), "i32");
        assert_eq!(map_ty_wasm_param("bytes16").unwrap(), "i32");
        // Unknown types fail
        assert!(map_ty_wasm_param("unknown").is_err());
    }

    #[test]
    fn render_rust_fails_closed_on_unknown_type() {
        let idl = Idl {
            namespace: "test".to_string(),
            version: "v0".to_string(),
            protocol: 0,
            message: std::collections::BTreeMap::from([(
                "bad".to_string(),
                Msg {
                    msg_type: 1,
                    fields: vec!["field:bogus".to_string()],
                },
            )]),
        };

        let err = render_rust(&idl, Path::new("bad.toml")).unwrap_err();
        let msg = err.to_string();
        assert!(msg.contains("unknown IDL type"));
        assert!(msg.contains("bogus"));
    }

    #[test]
    fn render_rust_rejects_dynamic_wire_types() {
        let idl = Idl {
            namespace: "test".to_string(),
            version: "v0".to_string(),
            protocol: 1,
            message: std::collections::BTreeMap::from([(
                "bad".to_string(),
                Msg {
                    msg_type: 1,
                    fields: vec!["field:string".to_string()],
                },
            )]),
        };

        let err = render_rust(&idl, Path::new("bad.toml")).unwrap_err();
        assert!(err.to_string().contains("unknown IDL type 'string'"));
    }

    #[test]
    fn parse_idl_rejects_zero_msg_type() {
        let err = parse_idl(
            r#"
namespace = "test"
version = "1"
protocol = 1

[message.bad]
msg_type = 0
fields = ["request_id:u64"]
"#,
        )
        .unwrap_err();

        assert!(err.to_string().contains("msg_type must be non-zero"));
    }

    #[test]
    fn render_c_fails_closed_on_unknown_type() {
        let idl = Idl {
            namespace: "test".to_string(),
            version: "v0".to_string(),
            protocol: 0,
            message: std::collections::BTreeMap::from([(
                "bad".to_string(),
                Msg {
                    msg_type: 1,
                    fields: vec!["field:bogus".to_string()],
                },
            )]),
        };

        let err = render_c(&idl, Path::new("bad.toml"), Path::new("out.h")).unwrap_err();
        let msg = err.to_string();
        assert!(msg.contains("unknown IDL type"));
        assert!(msg.contains("bogus"));
    }

    #[test]
    fn render_wasm_imports_fails_closed_on_unknown_type() {
        let idl = Idl {
            namespace: "test".to_string(),
            version: "v0".to_string(),
            protocol: 0,
            message: std::collections::BTreeMap::from([(
                "bad".to_string(),
                Msg {
                    msg_type: 1,
                    fields: vec!["field:bogus".to_string()],
                },
            )]),
        };

        let err = render_wasm_imports(&idl, Path::new("bad.toml")).unwrap_err();
        let msg = err.to_string();
        assert!(msg.contains("unknown IDL type"));
        assert!(msg.contains("bogus"));
    }

    #[test]
    fn render_wasm_imports_generates_client_struct() {
        let idl = Idl {
            namespace: "echo".to_string(),
            version: "v1".to_string(),
            protocol: 1,
            message: std::collections::BTreeMap::from([(
                "echo".to_string(),
                Msg {
                    msg_type: 1,
                    fields: vec!["data:bytes".to_string()],
                },
            )]),
        };

        let output = render_wasm_imports(&idl, Path::new("echo.toml")).unwrap();

        // Check for expected content
        assert!(output.contains("pub struct EchoClient"));
        assert!(output.contains("fn from_cap")); // matches "pub const fn from_cap"
        assert!(output.contains("pub fn call"));
        assert!(output.contains("wasm_import_module = \"ramen::echo\""));
        assert!(output.contains("harness_call"));
        assert!(output.contains("crate::Status"));
    }

    #[test]
    fn render_wasm_host_generates_host_functions() {
        let idl = Idl {
            namespace: "echo".to_string(),
            version: "v1".to_string(),
            protocol: 1,
            message: std::collections::BTreeMap::from([(
                "echo".to_string(),
                Msg {
                    msg_type: 1,
                    fields: vec!["data:bytes".to_string()],
                },
            )]),
        };

        let output = render_wasm_host(&idl, Path::new("echo_harness_v1.toml")).unwrap();

        assert!(output.contains("linker.func_wrap"));
        assert!(output.contains("kernel_api::generated::echo_harness_v1::Echo"));
        assert!(output.contains("kernel_bridge.transact"));
        assert!(output.contains("cap: u64"));
        assert!(output.contains("data: i32"));
        assert!(output.contains("out_ptr: u32"));
        assert!(output.contains("out_len_ptr: u32"));
    }

    #[test]
    fn infer_lang_by_extension() {
        assert!(matches!(infer_lang(Path::new("x.h")), Lang::C));
        assert!(matches!(
            infer_lang(Path::new("x.generated.rs")),
            Lang::Rust
        ));
    }
}
