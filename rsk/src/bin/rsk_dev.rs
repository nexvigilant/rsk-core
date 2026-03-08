use anyhow::{Context, Result};
use clap::{Parser, Subcommand};
use quote::ToTokens;
use std::fs;
use std::path::{Path, PathBuf};
use syn::{FnArg, Item, ReturnType, Type, Meta};
use regex::Regex;

#[derive(Parser, Debug)]
#[command(author, version, about = "RSK Development Tool", long_about = None)]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand, Debug)]
enum Commands {
    /// Generate Python bridge stubs from Rust modules
    GenerateBridge {
        /// Directory containing Rust modules
        #[arg(short, long, default_value = "src/modules")]
        input_dir: PathBuf,

        /// Output file (default: stdout)
        #[arg(short, long)]
        output: Option<PathBuf>,

        /// Comma-separated list of modules to process
        #[arg(short, long, default_value = "execution_engine,routing_engine,state_manager")]
        modules: String,
    },
    /// Synchronize rsk_bridge.py with PyO3 bindings
    Sync {
        /// Path to python_bindings.rs
        #[arg(long, default_value = "src/modules/python_bindings.rs")]
        bindings: PathBuf,

        /// Path to rsk_bridge.py
        #[arg(long)]
        bridge: Option<PathBuf>,

        /// Validate only and exit with error if out of sync
        #[arg(long)]
        validate: bool,

        /// Generate missing stubs
        #[arg(long)]
        generate: bool,
    },
}

struct RustFunction {
    name: String,
    params: Vec<(String, String)>,
    return_type: String,
    doc: String,
}

struct PyO3Function {
    python_name: String,
    params: Vec<(String, String)>,
    doc: String,
}

fn parse_rust_type(ty: &Type) -> String {
    let type_str = ty.to_token_stream().to_string().replace(' ', "");
    let type_str = type_str.replace('&', "");

    if type_str == "String" || type_str == "str" { return "str".to_string(); }
    if type_str == "bool" { return "bool".to_string(); }
    if type_str == "f32" || type_str == "f64" { return "float".to_string(); }
    if ["i32", "i64", "u32", "u64", "usize"].contains(&type_str.as_str()) { return "int".to_string(); }
    if type_str == "Vec<String>" || type_str == "Vec<str>" { return "list[str]".to_string(); }
    if type_str == "Vec<u8>" { return "bytes".to_string(); }
    if type_str.starts_with("Option<") {
        let inner = &type_str[7..type_str.len() - 1];
        return format!("Optional[{}]", inner); 
    }
    if type_str.starts_with("Vec<") {
        let inner = &type_str[4..type_str.len() - 1];
        return format!("list[{}]", inner); 
    }
    if type_str.starts_with("HashMap<") { return "dict[str, Any]".to_string(); }
    
    "Any".to_string()
}

fn extract_doc_comments(attrs: &[syn::Attribute]) -> String {
    attrs
        .iter()
        .filter_map(|attr| {
            if attr.path().is_ident("doc")
                && let syn::Meta::NameValue(nv) = &attr.meta
                && let syn::Expr::Lit(expr_lit) = &nv.value
                && let syn::Lit::Str(lit_str) = &expr_lit.lit
            {
                return Some(lit_str.value().trim().to_string());
            }
            None
        })
        .collect::<Vec<_>>()
        .join("\n    ")
}

fn parse_module(path: &Path) -> Result<Vec<RustFunction>> {
    let content = fs::read_to_string(path)?;
    let file = syn::parse_file(&content)?;
    let mut functions = Vec::new();

    for item in file.items {
        if let Item::Fn(func) = item
            && matches!(func.vis, syn::Visibility::Public(_))
        {
            let name = func.sig.ident.to_string();
            let doc = extract_doc_comments(&func.attrs);
            let mut params = Vec::new();
            for arg in func.sig.inputs {
                if let FnArg::Typed(pat_type) = arg {
                    let param_name = pat_type.pat.to_token_stream().to_string();
                    params.push((param_name, parse_rust_type(&pat_type.ty)));
                }
            }
            let return_type = match &func.sig.output {
                ReturnType::Default => "None".to_string(),
                ReturnType::Type(_, ty) => parse_rust_type(ty),
            };
            functions.push(RustFunction { name, params, return_type, doc });
        }
    }
    Ok(functions)
}

fn parse_pyo3_bindings(path: &Path) -> Result<Vec<PyO3Function>> {
    let content = fs::read_to_string(path)?;
    let file = syn::parse_file(&content)?;
    let mut functions = Vec::new();

    for item in file.items {
        if let Item::Fn(func) = item {
            let is_pyfunction = func.attrs.iter().any(|attr| attr.path().is_ident("pyfunction"));
            if is_pyfunction {
                let rust_name = func.sig.ident.to_string();
                let mut python_name = rust_name.clone();
                
                for attr in &func.attrs {
                    if attr.path().is_ident("pyo3")
                        && let Ok(list) = attr.parse_args_with(syn::punctuated::Punctuated::<Meta, syn::Token![,]>::parse_terminated)
                    {
                        for meta in list {
                            if let Meta::NameValue(nv) = meta
                                && nv.path.is_ident("name")
                                && let syn::Expr::Lit(expr_lit) = &nv.value
                                && let syn::Lit::Str(lit_str) = &expr_lit.lit
                            {
                                python_name = lit_str.value();
                            }
                        }
                    }
                }

                let doc = extract_doc_comments(&func.attrs);
                let mut params = Vec::new();
                for arg in func.sig.inputs {
                    if let FnArg::Typed(pat_type) = arg {
                        let param_name = pat_type.pat.to_token_stream().to_string();
                        if param_name == "py" || param_name == "_py" { continue; }
                        params.push((param_name, parse_rust_type(&pat_type.ty)));
                    }
                }

                functions.push(PyO3Function { python_name, params, doc });
            }
        }
    }
    Ok(functions)
}

fn get_bridge_names(path: &Path) -> Result<Vec<String>> {
    let content = fs::read_to_string(path)?;
    let re = Regex::new(r"(?m)^def\s+(\w+)\s*\(")?;
    let mut names = Vec::new();
    for cap in re.captures_iter(&content) {
        let name = cap[1].to_string();
        if !name.starts_with('_') {
            names.push(name);
        }
    }
    Ok(names)
}

fn generate_python_bridge(module_name: &str, functions: &[RustFunction]) -> String {
    let mut body = format!(
        r#""""RSK Bridge stubs for {} module.

Auto-generated by bridge_gen (Rust-native)
These functions delegate to the rsk CLI for Rust-speed execution.
"""

from pathlib import Path
from typing import Any, Optional, TypedDict
import subprocess
import json
import os
import shutil

"#,
        module_name
    );

    let subcommand = match module_name {
        "execution_engine" => "exec",
        "routing_engine" => "route",
        "state_manager" => "state",
        _ => module_name,
    };

    for func in functions {
        let py_params = func.params.iter().map(|(n, t)| format!("{}: {}", n, t)).collect::<Vec<_>>().join(", ");
        let p_names_map = func.params.iter().map(|(n, _)| format!("'{}': {}", n, n)).collect::<Vec<_>>().join(", ");
        let docstring = if func.doc.is_empty() { format!("Call rsk {} {}", subcommand, func.name) } else { func.doc.replace('"', "'") };

        body.push_str(&format!(
            r#"
def {}({}) -> {}:
    """{}

    Uses Rust implementation via CLI delegation.
    """
    RSK_BIN = shutil.which("rsk")
    if not RSK_BIN:
        raise RuntimeError("RSK binary not found on system path.")

    try:
        args = [RSK_BIN, "{}", "{}"]
        params = {{{}}}
        args.extend(["--input", json.dumps(params)])

        result = subprocess.run(args, capture_output=True, text=True, check=True, timeout=30)
        return json.loads(result.stdout)
    except (subprocess.CalledProcessError, json.JSONDecodeError, FileNotFoundError) as e:
        raise RuntimeError(f"RSK CLI call failed: {{e}}")
"#,
            func.name, py_params, func.return_type, docstring, subcommand, func.name, p_names_map
        ));
    }
    body
}

fn main() -> Result<()> {
    let cli = Cli::parse();

    match cli.command {
        Commands::GenerateBridge { input_dir, output, modules } => {
            let module_names: Vec<&str> = modules.split(',').collect();
            let mut all_output = String::new();

            for name in module_names {
                let path = input_dir.join(format!("{}.rs", name));
                if !path.exists() {
                    eprintln!("Warning: Module not found: {:?}", path);
                    continue;
                }

                let functions = parse_module(&path).with_context(|| format!("Failed to parse module {:?}", path))?;
                all_output.push_str(&generate_python_bridge(name, &functions));
                all_output.push_str("\n\n");
            }

            if let Some(output_path) = output {
                fs::write(output_path, all_output)?;
            } else {
                println!("{}", all_output);
            }
        }
        Commands::Sync { bindings, bridge, validate, generate } => {
            let bridge_path = bridge.unwrap_or_else(|| {
                dirs::home_dir().unwrap().join(".claude/skills/.shared/rsk_bridge.py")
            });

            if !bindings.exists() { anyhow::bail!("Bindings file not found: {:?}", bindings); }
            if !bridge_path.exists() { anyhow::bail!("Bridge file not found: {:?}", bridge_path); }

            let pyo3_funcs = parse_pyo3_bindings(&bindings)?;
            let bridge_names = get_bridge_names(&bridge_path)?;
            let pyo3_names: Vec<String> = pyo3_funcs.iter().map(|f| f.python_name.clone()).collect();
            
            let mut missing = Vec::new();
            for name in &pyo3_names {
                if !bridge_names.contains(name) { missing.push(name); }
            }

            println!("RSK Bridge Sync Report");
            println!("=====================");
            println!("PyO3 exports:    {}", pyo3_funcs.len());
            println!("Bridge functions: {}", bridge_names.len());
            println!("Missing in bridge: {}", missing.len());

            if !missing.is_empty() {
                println!("\nMissing functions:");
                for name in &missing { println!("  - {}", name); }

                if generate {
                    println!("\nGenerated stubs:");
                    for name in missing {
                        let func = pyo3_funcs.iter().find(|f| f.python_name == *name).unwrap();
                        let params = func.params.iter().map(|(n, t)| format!("{}: {}", n, t)).collect::<Vec<_>>().join(", ");
                        let p_names = func.params.iter().map(|(n, _)| n.as_str()).collect::<Vec<_>>().join(", ");
                        
                        println!(r#"
def {}({}) -> dict:
    """{}
    
    Uses Rust implementation via PyO3 if available, falls back to CLI.
    """
    if _USE_RUST:
        return _rsk.{}({})
    
    # CLI fallback logic here
    raise NotImplementedError("{} requires rsk or CLI")
"#, 
                            func.python_name, params, func.doc, func.python_name, p_names, func.python_name
                        );
                    }
                }
                if validate { anyhow::bail!("Bridge is out of sync!"); }
            } else {
                println!("\nStatus: IN_SYNC");
            }
        }
    }
    Ok(())
}
